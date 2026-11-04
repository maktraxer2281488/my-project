//! Мир, разбитый на чанки 16×16×128. Чанки создаются по мере надобности,
//! поэтому мир практически бесконечный.

use crate::blocks::{self, AIR, BEDROCK};
use macroquad::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

pub const CS: i32 = 16;
pub const CH: i32 = 128;
pub const CHUNK_VOL: usize = (CS * CS * CH) as usize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Dim {
    Overworld,
    Heaven,
}

impl Dim {
    pub fn name(self) -> &'static str {
        match self {
            Dim::Overworld => "Верхний мир",
            Dim::Heaven => "Рай",
        }
    }
}

#[derive(Clone)]
pub struct Chunk {
    pub blocks: Vec<u8>,
}

impl Chunk {
    pub fn empty() -> Self {
        Chunk { blocks: vec![AIR; CHUNK_VOL] }
    }

    #[inline]
    pub fn idx(x: i32, y: i32, z: i32) -> usize {
        ((y * CS + z) * CS + x) as usize
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        self.blocks[Self::idx(x, y, z)]
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, b: u8) {
        self.blocks[Self::idx(x, y, z)] = b;
    }
}

pub fn chunk_of(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(CS), z.div_euclid(CS))
}

pub struct World {
    pub dim: Dim,
    pub seed: u32,
    pub chunks: HashMap<(i32, i32), Arc<Chunk>>,
    /// Номер правки чанка (растёт при каждом изменении блока).
    pub versions: HashMap<(i32, i32), u32>,
}

impl World {
    pub fn new(dim: Dim, seed: u32) -> Self {
        World { dim, seed, chunks: HashMap::new(), versions: HashMap::new() }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if y < 0 {
            return if self.dim == Dim::Overworld { BEDROCK } else { AIR };
        }
        if y >= CH {
            return AIR;
        }
        let (cx, cz) = chunk_of(x, z);
        match self.chunks.get(&(cx, cz)) {
            Some(c) => c.get(x - cx * CS, y, z - cz * CS),
            None => AIR,
        }
    }

    pub fn is_loaded(&self, x: i32, z: i32) -> bool {
        self.chunks.contains_key(&chunk_of(x, z))
    }

    /// Твёрдо ли для столкновений (незагруженные чанки — стена).
    pub fn solid_at(&self, x: i32, y: i32, z: i32) -> bool {
        if y >= 0 && y < CH && !self.is_loaded(x, z) {
            return true;
        }
        blocks::is_solid(self.get(x, y, z))
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, b: u8) -> bool {
        if !(0..CH).contains(&y) {
            return false;
        }
        let (cx, cz) = chunk_of(x, z);
        let Some(c) = self.chunks.get_mut(&(cx, cz)) else { return false };
        Arc::make_mut(c).set(x - cx * CS, y, z - cz * CS, b);
        *self.versions.entry((cx, cz)).or_insert(0) += 1;
        true
    }

    /// Высота первого свободного места над колонкой.
    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        for y in (0..CH).rev() {
            if blocks::is_solid(self.get(x, y, z)) {
                return y + 1;
            }
        }
        -1
    }

    /// Открыта ли клетка небу (для спавна существ и затемнения).
    pub fn sky_exposed(&self, x: i32, y: i32, z: i32) -> bool {
        for yy in (y + 1).max(0)..CH {
            if blocks::is_opaque(self.get(x, yy, z)) {
                return false;
            }
        }
        true
    }

    /// DDA-трассировка: (блок под прицелом, соседняя клетка перед ним, дистанция).
    pub fn raycast(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<(IVec3, IVec3, f32)> {
        let mut p = origin.floor().as_ivec3();
        let step = ivec3(
            dir.x.signum() as i32 * (dir.x != 0.0) as i32,
            dir.y.signum() as i32 * (dir.y != 0.0) as i32,
            dir.z.signum() as i32 * (dir.z != 0.0) as i32,
        );
        let inv = |d: f32| if d != 0.0 { 1.0 / d.abs() } else { f32::INFINITY };
        let t_delta = vec3(inv(dir.x), inv(dir.y), inv(dir.z));
        let first = |o: f32, c: i32, d: f32, td: f32| {
            if d > 0.0 {
                (c as f32 + 1.0 - o) * td
            } else if d < 0.0 {
                (o - c as f32) * td
            } else {
                f32::INFINITY
            }
        };
        let mut t_max = vec3(
            first(origin.x, p.x, dir.x, t_delta.x),
            first(origin.y, p.y, dir.y, t_delta.y),
            first(origin.z, p.z, dir.z, t_delta.z),
        );
        let mut prev = p;
        let mut t = 0.0;
        while t <= max {
            let b = self.get(p.x, p.y, p.z);
            if b != AIR && blocks::def(b).shape != blocks::Shape::Liquid {
                return Some((p, prev, t));
            }
            prev = p;
            if t_max.x < t_max.y && t_max.x < t_max.z {
                p.x += step.x;
                t = t_max.x;
                t_max.x += t_delta.x;
            } else if t_max.y < t_max.z {
                p.y += step.y;
                t = t_max.y;
                t_max.y += t_delta.y;
            } else {
                p.z += step.z;
                t = t_max.z;
                t_max.z += t_delta.z;
            }
        }
        None
    }
}
