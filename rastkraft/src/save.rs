//! Сохранение миров: папка `saves/<имя>/` рядом с игрой.
//! `level.txt` — состояние игрока и мира, `ow/`, `hv/` — изменённые чанки (RLE).

use crate::world::{CHUNK_VOL, Chunk};
use std::collections::HashMap;
use std::path::PathBuf;

/// Папка для данных игры: рядом с exe на ПК, во внутренней памяти приложения на Android.
pub fn data_dir() -> PathBuf {
    if cfg!(target_os = "android") {
        let d = PathBuf::from("/data/data/com.maktraxer.rastkraft/files");
        let _ = std::fs::create_dir_all(&d);
        return d;
    }
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(|| ".".into())
}

pub fn saves_dir() -> PathBuf {
    data_dir().join("saves")
}

pub fn world_dir(name: &str) -> PathBuf {
    saves_dir().join(sanitize(name))
}

pub fn sanitize(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' }).collect();
    let s = s.trim().to_string();
    if s.is_empty() { "Мир".into() } else { s }
}

/// RLE чанка. Формат 2: заголовок [0, 0xB2], затем тройки (длина, младший байт, старший байт).
/// Старый формат 1 (пары длина + номер блока u8) читается для совместимости.
pub fn rle_encode(blocks: &[u16]) -> Vec<u8> {
    let mut out = vec![0, 0xB2];
    let mut i = 0;
    while i < blocks.len() {
        let b = blocks[i];
        let mut n = 1;
        while i + n < blocks.len() && blocks[i + n] == b && n < 255 {
            n += 1;
        }
        out.push(n as u8);
        out.push((b & 0xff) as u8);
        out.push((b >> 8) as u8);
        i += n;
    }
    out
}

pub fn rle_decode(data: &[u8]) -> Option<Vec<u16>> {
    let mut out = Vec::with_capacity(CHUNK_VOL);
    if data.len() >= 2 && data[0] == 0 && data[1] == 0xB2 {
        for t in data[2..].chunks_exact(3) {
            let b = t[1] as u16 | (t[2] as u16) << 8;
            if b as usize >= crate::blocks::BLOCK_COUNT {
                return None;
            }
            out.extend(std::iter::repeat_n(b, t[0] as usize));
        }
    } else {
        for pair in data.chunks_exact(2) {
            out.extend(std::iter::repeat_n(pair[1] as u16, pair[0] as usize));
        }
    }
    (out.len() == CHUNK_VOL).then_some(out)
}

/// Записать изменённые чанки измерения.
pub fn save_chunks(dir: &PathBuf, tag: &str, chunks: &[((i32, i32), &Chunk)]) -> std::io::Result<()> {
    let d = dir.join(tag);
    std::fs::create_dir_all(&d)?;
    for ((cx, cz), c) in chunks {
        std::fs::write(d.join(format!("{cx}_{cz}.bin")), rle_encode(&c.blocks))?;
    }
    Ok(())
}

pub fn load_chunks(dir: &PathBuf, tag: &str) -> HashMap<(i32, i32), Chunk> {
    let mut out = HashMap::new();
    let Ok(rd) = std::fs::read_dir(dir.join(tag)) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Some(stem) = name.strip_suffix(".bin") else { continue };
        let Some((a, b)) = stem.split_once('_') else { continue };
        let (Ok(cx), Ok(cz)) = (a.parse::<i32>(), b.parse::<i32>()) else { continue };
        if let Ok(data) = std::fs::read(e.path()) {
            if let Some(blocks) = rle_decode(&data) {
                out.insert((cx, cz), Chunk { blocks, spawns: Vec::new() });
            }
        }
    }
    out
}

/// Простое хранилище «ключ = значение».
#[derive(Default, Debug, Clone)]
pub struct Kv(pub Vec<(String, String)>);

impl Kv {
    pub fn set(&mut self, k: &str, v: impl ToString) {
        self.0.push((k.to_string(), v.to_string()));
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        self.0.iter().find(|(a, _)| a == k).map(|(_, b)| b.as_str())
    }
    pub fn f(&self, k: &str, def: f32) -> f32 {
        self.get(k).and_then(|v| v.parse().ok()).unwrap_or(def)
    }
    pub fn u(&self, k: &str, def: u32) -> u32 {
        self.get(k).and_then(|v| v.parse().ok()).unwrap_or(def)
    }
    pub fn floats(&self, k: &str) -> Vec<f32> {
        self.get(k).map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default()
    }
    pub fn ints(&self, k: &str) -> Vec<i64> {
        self.get(k).map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default()
    }
    pub fn to_text(&self) -> String {
        self.0.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
    }
    pub fn parse(text: &str) -> Kv {
        Kv(text.lines().filter_map(|l| l.split_once('=').map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))).collect())
    }
}

pub struct WorldInfo {
    pub name: String,
    pub creative: bool,
    pub seed: u32,
    pub modified: std::time::SystemTime,
}

pub fn list_worlds() -> Vec<WorldInfo> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(saves_dir()) else { return out };
    for e in rd.flatten() {
        let p = e.path().join("level.txt");
        if let Ok(text) = std::fs::read_to_string(&p) {
            let kv = Kv::parse(&text);
            out.push(WorldInfo {
                name: kv.get("name").unwrap_or("Мир").to_string(),
                creative: kv.get("creative") == Some("1"),
                seed: kv.u("seed", 0),
                modified: std::fs::metadata(&p).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH),
            });
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

pub fn delete_world(name: &str) {
    let _ = std::fs::remove_dir_all(world_dir(name));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_roundtrip() {
        let mut c = Chunk::empty();
        c.set(3, 10, 4, 7);
        c.set(0, 0, 0, 1);
        let enc = rle_encode(&c.blocks);
        assert!(enc.len() < 400);
        assert_eq!(rle_decode(&enc).unwrap(), c.blocks);
        // блок с номером больше 255
        c.set(5, 5, 5, crate::blocks::RUG_BLOCK);
        assert_eq!(rle_decode(&rle_encode(&c.blocks)).unwrap(), c.blocks);
        // старый формат (пары u8) читается
        let mut old = Vec::new();
        let mut left = CHUNK_VOL;
        while left > 0 {
            let n = left.min(255);
            old.push(n as u8);
            old.push(3);
            left -= n;
        }
        assert!(rle_decode(&old).unwrap().iter().all(|&b| b == 3));
        let kv = Kv::parse("a=1\nb=x,y\n");
        assert_eq!(kv.u("a", 0), 1);
        assert_eq!(kv.get("b"), Some("x,y"));
    }
}
