//! Пул фоновых потоков: генерация чанков и построение их геометрии,
//! чтобы игра не подтормаживала при подгрузке мира.

use crate::worldgen;
use crate::mesher::{self, MeshData, Nbhd};
use crate::world::{Chunk, Dim};
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};

pub enum Job {
    Generate { dim: Dim, seed: u32, cx: i32, cz: i32 },
    Mesh { dim: Dim, cx: i32, cz: i32, version: u32, epoch: u32, smooth: bool, n: Box<Nbhd> },
}

pub enum Done {
    Generated { dim: Dim, seed: u32, cx: i32, cz: i32, chunk: Chunk },
    Meshed { dim: Dim, cx: i32, cz: i32, version: u32, epoch: u32, mesh: MeshData },
}

struct Shared {
    queue: Mutex<VecDeque<Job>>,
    cv: Condvar,
}

pub struct Pool {
    shared: Arc<Shared>,
    rx: Receiver<Done>,
    pub threads: usize,
    pub in_flight: usize,
}

impl Pool {
    pub fn new() -> Self {
        // на слабых процессорах (2 ядра / 4 потока) оставляем ядро игре и драйверу видеокарты,
        // иначе генерация мира отнимает время у кадра
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
        let threads = if cores <= 4 { (cores / 2).max(1) } else { cores - 2 }.clamp(1, 6);
        let shared = Arc::new(Shared { queue: Mutex::new(VecDeque::new()), cv: Condvar::new() });
        let (tx, rx) = channel::<Done>();
        for _ in 0..threads {
            let shared = shared.clone();
            let tx: Sender<Done> = tx.clone();
            std::thread::spawn(move || worker(shared, tx));
        }
        Pool { shared, rx, threads, in_flight: 0 }
    }

    pub fn submit(&mut self, job: Job) {
        self.in_flight += 1;
        self.shared.queue.lock().unwrap().push_back(job);
        self.shared.cv.notify_one();
    }

    /// Срочное задание — в начало очереди (перестройка после правки игрока).
    pub fn submit_front(&mut self, job: Job) {
        self.in_flight += 1;
        self.shared.queue.lock().unwrap().push_front(job);
        self.shared.cv.notify_one();
    }

    /// Сколько ещё заданий можно поставить, чтобы очередь не разрасталась.
    pub fn capacity(&self) -> usize {
        (self.threads * 3).saturating_sub(self.in_flight)
    }

    pub fn poll(&mut self) -> Vec<Done> {
        let out: Vec<Done> = self.rx.try_iter().collect();
        self.in_flight = self.in_flight.saturating_sub(out.len());
        out
    }
}

fn worker(shared: Arc<Shared>, tx: Sender<Done>) {
    loop {
        let job = {
            let mut q = shared.queue.lock().unwrap();
            loop {
                if let Some(j) = q.pop_front() {
                    break j;
                }
                q = shared.cv.wait(q).unwrap();
            }
        };
        let done = match job {
            Job::Generate { dim, seed, cx, cz } => Done::Generated { dim, seed, cx, cz, chunk: worldgen::generate(dim, seed, cx, cz) },
            Job::Mesh { dim, cx, cz, version, epoch, smooth, n } => Done::Meshed { dim, cx, cz, version, epoch, mesh: mesher::build(&n, smooth) },
        };
        if tx.send(done).is_err() {
            return;
        }
    }
}
