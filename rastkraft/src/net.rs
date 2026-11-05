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
    /// Внешность: 4 номера палитр (255 — по имени).
    pub look: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Hello { name: String },
    Welcome { id: u8, seed: u32, creative: bool, time: f32, chunks: Vec<(u8, i32, i32, Vec<u8>)> },
    State(PState),
    Edit { dim: u8, x: i32, y: i32, z: i32, b: u16 },
    Hit { target: u8, from: u8, dmg: f32, knock: [f32; 3] },
    Chat { text: String },
    Leave { id: u8 },
    /// Снимок существ вокруг игрока + время суток (хост → клиент).
    Mobs { time: f32, weather: u8, list: Vec<MobSnap> },
    /// Клиент ударил существо.
    MobHit { uid: u32, dmg: f32 },
    /// Выдать предметы (трофей убитого клиентом существа).
    Give { item: u32, count: u16 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MobSnap {
    pub uid: u32,
    pub def: u16,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub hurt: bool,
    pub attack: u8,
    pub dying: u8,
    pub hp: u8,
}

/// Отправитель удара «существо» в сообщении Hit.
pub const FROM_MOB: u8 = 254;

// ---------- кодирование ----------
struct W(Vec<u8>);
impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
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
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
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
            w.u32(s.look);
        }
        Msg::Edit { dim, x, y, z, b } => {
            w.u8(4);
            w.u8(*dim);
            w.i32(*x);
            w.i32(*y);
            w.i32(*z);
            w.u16(*b);
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
        Msg::Mobs { time, weather, list } => {
            w.u8(8);
            w.f32(*time);
            w.u8(*weather);
            w.u32(list.len() as u32);
            for m in list {
                w.u32(m.uid);
                w.u32(m.def as u32);
                for v in m.pos {
                    w.f32(v);
                }
                w.f32(m.yaw);
                w.u8(m.hurt as u8);
                w.u8(m.attack);
                w.u8(m.dying);
                w.u8(m.hp);
            }
        }
        Msg::MobHit { uid, dmg } => {
            w.u8(9);
            w.u32(*uid);
            w.f32(*dmg);
        }
        Msg::Give { item, count } => {
            w.u8(10);
            w.u32(*item);
            w.u32(*count as u32);
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
            Msg::State(PState { id, name, dim, pos, yaw, pitch, held, armor, health: r.f32()?, walk: r.f32()?, speed: r.f32()?, swing: r.f32()?, look: r.u32()? })
        }
        4 => Msg::Edit { dim: r.u8()?, x: r.i32()?, y: r.i32()?, z: r.i32()?, b: r.u16()? },
        5 => Msg::Hit { target: r.u8()?, from: r.u8()?, dmg: r.f32()?, knock: [r.f32()?, r.f32()?, r.f32()?] },
        6 => Msg::Chat { text: r.str()? },
        7 => Msg::Leave { id: r.u8()? },
        8 => {
            let time = r.f32()?;
            let weather = r.u8()?;
            let n = r.u32()? as usize;
            let mut list = Vec::with_capacity(n.min(512));
            for _ in 0..n {
                list.push(MobSnap {
                    uid: r.u32()?,
                    def: r.u32()? as u16,
                    pos: [r.f32()?, r.f32()?, r.f32()?],
                    yaw: r.f32()?,
                    hurt: r.u8()? == 1,
                    attack: r.u8()?,
                    dying: r.u8()?,
                    hp: r.u8()?,
                });
            }
            Msg::Mobs { time, weather, list }
        }
        9 => Msg::MobHit { uid: r.u32()?, dmg: r.f32()? },
        10 => Msg::Give { item: r.u32()?, count: r.u32()? as u16 },
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
    /// Строка для маяка в локальной сети (имя хоста, мир, игроки).
    pub beacon: Option<std::sync::Arc<std::sync::Mutex<String>>>,
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
        let beacon = std::sync::Arc::new(std::sync::Mutex::new(format!("{name}|игроков: 1")));
        start_beacon(beacon.clone());
        Ok(Net { beacon: Some(beacon), host: true, my_id: 0, name: name.to_string(), rx, peers, info: format!("Сервер открыт: {}:{}", local_ip(), PORT) })
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
        let n = Net { beacon: None, host: false, my_id: 255, name: name.to_string(), rx, peers, info: format!("Подключено к {addr}") };
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

pub const BEACON_PORT: u16 = 7778;

/// Хост раз в секунду сообщает в локальную сеть: «здесь идёт игра».
pub fn start_beacon(info: std::sync::Arc<std::sync::Mutex<String>>) {
    std::thread::spawn(move || {
        let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") else { return };
        let _ = sock.set_broadcast(true);
        loop {
            let msg = format!("RASTKRAFT1|{}", info.lock().unwrap());
            let _ = sock.send_to(msg.as_bytes(), ("255.255.255.255", BEACON_PORT));
            let _ = sock.send_to(msg.as_bytes(), ("127.0.0.1", BEACON_PORT));
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });
}

/// Найденная игра в локальной сети.
#[derive(Clone, Debug)]
pub struct Found {
    pub addr: String,
    pub info: String,
    pub seen: std::time::Instant,
}

/// Слушает маяки хостов в локальной сети.
pub struct Finder {
    pub found: std::sync::Arc<std::sync::Mutex<Vec<Found>>>,
}

impl Finder {
    pub fn start() -> Option<Finder> {
        let sock = std::net::UdpSocket::bind(("0.0.0.0", BEACON_PORT)).ok()?;
        let found = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Found>::new()));
        let f2 = found.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 512];
            while let Ok((n, from)) = sock.recv_from(&mut buf) {
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let Some(info) = text.strip_prefix("RASTKRAFT1|") else { continue };
                let addr = from.ip().to_string();
                let mut list = f2.lock().unwrap();
                list.retain(|f| f.addr != addr);
                list.push(Found { addr, info: info.to_string(), seen: std::time::Instant::now() });
            }
        });
        Some(Finder { found })
    }

    pub fn list(&self) -> Vec<Found> {
        let mut l = self.found.lock().unwrap();
        l.retain(|f| f.seen.elapsed().as_secs() < 5);
        // одна и та же игра может прийти по двум адресам — оставляем один (не 127.0.0.1)
        let mut out: Vec<Found> = Vec::new();
        for f in l.iter() {
            if let Some(o) = out.iter_mut().find(|o| o.info == f.info) {
                if o.addr.starts_with("127.") {
                    *o = f.clone();
                }
            } else {
                out.push(f.clone());
            }
        }
        out
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
            Msg::State(PState { id: 1, name: "А".into(), dim: 1, pos: [1.0, 2.0, 3.0], yaw: 0.5, pitch: -0.2, held: 7, armor: [1, 2, 3, 4], health: 18.0, walk: 3.0, speed: 1.0, swing: 0.5, look: 0x01020304 }),
            Msg::Edit { dim: 0, x: -5, y: 60, z: 9, b: 4 },
            Msg::Hit { target: 2, from: 0, dmg: 7.5, knock: [1.0, 0.5, 0.0] },
            Msg::Chat { text: "привет".into() },
            Msg::Leave { id: 2 },
            Msg::Mobs { time: 0.3, weather: 1, list: vec![MobSnap { uid: 7, def: 12, pos: [1.0, 2.0, 3.0], yaw: 0.4, hurt: true, attack: 128, dying: 0, hp: 200 }] },
            Msg::MobHit { uid: 9, dmg: 3.5 },
            Msg::Give { item: 0x0400_0010, count: 3 },
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

#[cfg(test)]
mod lan_tests {
    use super::*;

    #[test]
    fn beacon_is_found() {
        let f = Finder::start().expect("порт 7778 свободен");
        let info = std::sync::Arc::new(std::sync::Mutex::new("Хост · мир · игроков: 1".to_string()));
        start_beacon(info);
        let mut ok = false;
        for _ in 0..40 {
            if !f.list().is_empty() {
                ok = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(ok, "маяк не найден");
    }
}
