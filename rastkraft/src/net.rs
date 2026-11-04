//! Сетевая игра по локальной сети (TCP, порт 7777): позиции игроков,
//! правки блоков и PvP-удары. Хост пересылает сообщения остальным.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

pub const PORT: u16 = 7777;

#[derive(Clone, Debug, PartialEq)]
pub struct PState {
    pub id: u8,
    pub name: String,
    pub dim: u8,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub held: u32,
    pub armor: [u32; 4],
    pub health: f32,
    pub walk: f32,
    pub speed: f32,
    pub swing: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Hello { name: String },
    Welcome { id: u8, seed: u32, creative: bool, time: f32, chunks: Vec<(u8, i32, i32, Vec<u8>)> },
    State(PState),
    Edit { dim: u8, x: i32, y: i32, z: i32, b: u8 },
    Hit { target: u8, from: u8, dmg: f32, knock: [f32; 3] },
    Chat { text: String },
    Leave { id: u8 },
}

// ---------- кодирование ----------
struct W(Vec<u8>);
impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
}

struct R<'a>(&'a [u8], usize);
impl R<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.0.get(self.1..self.1 + n)?;
        self.1 += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn bytes(&mut self) -> Option<Vec<u8>> {
        let n = self.u32()? as usize;
        if n > 64 << 20 {
            return None;
        }
        Some(self.take(n)?.to_vec())
    }
    fn str(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?).ok()
    }
}

pub fn encode(m: &Msg) -> Vec<u8> {
    let mut w = W(Vec::new());
    match m {
        Msg::Hello { name } => {
            w.u8(1);
            w.str(name);
        }
        Msg::Welcome { id, seed, creative, time, chunks } => {
            w.u8(2);
            w.u8(*id);
            w.u32(*seed);
            w.u8(*creative as u8);
            w.f32(*time);
            w.u32(chunks.len() as u32);
            for (d, x, z, data) in chunks {
                w.u8(*d);
                w.i32(*x);
                w.i32(*z);
                w.bytes(data);
            }
        }
        Msg::State(s) => {
            w.u8(3);
            w.u8(s.id);
            w.str(&s.name);
            w.u8(s.dim);
            for v in s.pos {
                w.f32(v);
            }
            w.f32(s.yaw);
            w.f32(s.pitch);
            w.u32(s.held);
            for a in s.armor {
                w.u32(a);
            }
            w.f32(s.health);
            w.f32(s.walk);
            w.f32(s.speed);
            w.f32(s.swing);
        }
        Msg::Edit { dim, x, y, z, b } => {
            w.u8(4);
            w.u8(*dim);
            w.i32(*x);
            w.i32(*y);
            w.i32(*z);
            w.u8(*b);
        }
        Msg::Hit { target, from, dmg, knock } => {
            w.u8(5);
            w.u8(*target);
            w.u8(*from);
            w.f32(*dmg);
            for v in knock {
                w.f32(*v);
            }
        }
        Msg::Chat { text } => {
            w.u8(6);
            w.str(text);
        }
        Msg::Leave { id } => {
            w.u8(7);
            w.u8(*id);
        }
    }
    let mut out = (w.0.len() as u32).to_le_bytes().to_vec();
    out.extend(w.0);
    out
}

pub fn decode(b: &[u8]) -> Option<Msg> {
    let mut r = R(b, 0);
    Some(match r.u8()? {
        1 => Msg::Hello { name: r.str()? },
        2 => {
            let id = r.u8()?;
            let seed = r.u32()?;
            let creative = r.u8()? == 1;
            let time = r.f32()?;
            let n = r.u32()? as usize;
            let mut chunks = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                chunks.push((r.u8()?, r.i32()?, r.i32()?, r.bytes()?));
            }
            Msg::Welcome { id, seed, creative, time, chunks }
        }
        3 => {
            let id = r.u8()?;
            let name = r.str()?;
            let dim = r.u8()?;
            let pos = [r.f32()?, r.f32()?, r.f32()?];
            let yaw = r.f32()?;
            let pitch = r.f32()?;
            let held = r.u32()?;
            let armor = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
            Msg::State(PState { id, name, dim, pos, yaw, pitch, held, armor, health: r.f32()?, walk: r.f32()?, speed: r.f32()?, swing: r.f32()? })
        }
        4 => Msg::Edit { dim: r.u8()?, x: r.i32()?, y: r.i32()?, z: r.i32()?, b: r.u8()? },
        5 => Msg::Hit { target: r.u8()?, from: r.u8()?, dmg: r.f32()?, knock: [r.f32()?, r.f32()?, r.f32()?] },
        6 => Msg::Chat { text: r.str()? },
        7 => Msg::Leave { id: r.u8()? },
        _ => return None,
    })
}

fn reader(mut s: TcpStream, id: u8, tx: Sender<(u8, Option<Msg>)>) {
    loop {
        let mut len = [0u8; 4];
        if s.read_exact(&mut len).is_err() {
            break;
        }
        let n = u32::from_le_bytes(len) as usize;
        if n > 64 << 20 {
            break;
        }
        let mut buf = vec![0u8; n];
        if s.read_exact(&mut buf).is_err() {
            break;
        }
        if let Some(m) = decode(&buf) {
            if tx.send((id, Some(m))).is_err() {
                return;
            }
        }
    }
    let _ = tx.send((id, None));
}

type Peers = Arc<Mutex<Vec<(u8, TcpStream)>>>;

pub struct Net {
    pub host: bool,
    pub my_id: u8,
    pub name: String,
    rx: Receiver<(u8, Option<Msg>)>,
    peers: Peers,
    pub info: String,
}

impl Net {
    /// Открыть мир для сети.
    pub fn host(name: &str) -> std::io::Result<Net> {
        let listener = TcpListener::bind(("0.0.0.0", PORT))?;
        let (tx, rx) = channel();
        let peers: Peers = Arc::new(Mutex::new(Vec::new()));
        let p2 = peers.clone();
        std::thread::spawn(move || {
            let mut next: u8 = 1;
            for s in listener.incoming().flatten() {
                let _ = s.set_nodelay(true);
                let id = next;
                next = next.wrapping_add(1).max(1);
                if let Ok(r) = s.try_clone() {
                    p2.lock().unwrap().push((id, s));
                    let tx = tx.clone();
                    std::thread::spawn(move || reader(r, id, tx));
                }
            }
        });
        Ok(Net { host: true, my_id: 0, name: name.to_string(), rx, peers, info: format!("Сервер открыт: {}:{}", local_ip(), PORT) })
    }

    /// Подключиться к хосту.
    pub fn join(addr: &str, name: &str) -> std::io::Result<Net> {
        let addr = if addr.contains(':') { addr.to_string() } else { format!("{addr}:{PORT}") };
        let s = TcpStream::connect(&addr)?;
        let _ = s.set_nodelay(true);
        let (tx, rx) = channel();
        let r = s.try_clone()?;
        std::thread::spawn(move || reader(r, 0, tx));
        let peers: Peers = Arc::new(Mutex::new(vec![(0, s)]));
        let n = Net { host: false, my_id: 255, name: name.to_string(), rx, peers, info: format!("Подключено к {addr}") };
        n.send_to(0, &Msg::Hello { name: name.to_string() });
        Ok(n)
    }

    pub fn send_to(&self, id: u8, m: &Msg) {
        let bytes = encode(m);
        let mut p = self.peers.lock().unwrap();
        p.retain_mut(|(pid, s)| *pid != id || s.write_all(&bytes).is_ok());
    }

    /// Хост — всем клиентам (кроме `except`), клиент — хосту.
    pub fn broadcast(&self, m: &Msg, except: Option<u8>) {
        let bytes = encode(m);
        let mut p = self.peers.lock().unwrap();
        p.retain_mut(|(pid, s)| Some(*pid) == except || s.write_all(&bytes).is_ok());
    }

    pub fn poll(&self) -> Vec<(u8, Option<Msg>)> {
        self.rx.try_iter().collect()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.lock().unwrap().len()
    }
}

pub fn local_ip() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".into())
}

/// Другой игрок, как его видим мы.
pub struct Remote {
    pub st: PState,
    pub shown: [f32; 3],
    pub hurt_t: f32,
}

pub type Remotes = HashMap<u8, Remote>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_roundtrip() {
        let msgs = vec![
            Msg::Hello { name: "Игрок".into() },
            Msg::Welcome { id: 3, seed: 42, creative: true, time: 0.25, chunks: vec![(0, -1, 2, vec![1, 2, 3])] },
            Msg::State(PState { id: 1, name: "А".into(), dim: 1, pos: [1.0, 2.0, 3.0], yaw: 0.5, pitch: -0.2, held: 7, armor: [1, 2, 3, 4], health: 18.0, walk: 3.0, speed: 1.0, swing: 0.5 }),
            Msg::Edit { dim: 0, x: -5, y: 60, z: 9, b: 4 },
            Msg::Hit { target: 2, from: 0, dmg: 7.5, knock: [1.0, 0.5, 0.0] },
            Msg::Chat { text: "привет".into() },
            Msg::Leave { id: 2 },
        ];
        for m in msgs {
            let e = encode(&m);
            assert_eq!(decode(&e[4..]).unwrap(), m);
        }
    }

    #[test]
    fn host_and_client_talk_over_tcp() {
        let host = Net::host("Хост").expect("порт свободен");
        let client = Net::join("127.0.0.1", "Гость").unwrap();
        let mut got = None;
        for _ in 0..200 {
            for (id, m) in host.poll() {
                if let Some(Msg::Hello { name }) = m {
                    got = Some((id, name));
                }
            }
            if got.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let (id, name) = got.expect("хост получил приветствие");
        assert_eq!(name, "Гость");
        host.send_to(id, &Msg::Chat { text: "добро пожаловать".into() });
        let mut ok = false;
        for _ in 0..200 {
            if client.poll().iter().any(|(_, m)| matches!(m, Some(Msg::Chat { .. }))) {
                ok = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(ok);
    }
}
