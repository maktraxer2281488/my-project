//! Мир из блоков: хранение, генерация ландшафта и трассировка луча.

use crate::textures::{self, hash};
use macroquad::prelude::*;

pub const CHUNK: usize = 16;
pub const CHUNKS: usize = 16;
pub const WX: usize = CHUNK * CHUNKS;
pub const WZ: usize = CHUNK * CHUNKS;
pub const WY: usize = 96;
pub const WATER_LEVEL: i32 = 22;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Block {
    Air,
    Grass,
    Dirt,
    Stone,
    Sand,
    Log,
    Leaves,
    Water,
    Planks,
    Bricks,
    Snow,
    Cobble,
    Crystal,
    Glass,
}

impl Block {
    /// Твёрдый — с ним сталкивается игрок.
    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water)
    }

    /// Полностью закрывает соседние грани.
    pub fn is_opaque(self) -> bool {
        !matches!(self, Block::Air | Block::Water | Block::Leaves | Block::Glass)
    }

    /// Можно навести прицел и сломать.
    pub fn is_targetable(self) -> bool {
        !matches!(self, Block::Air | Block::Water)
    }

    /// Плитки атласа: (верх, бок, низ).
    pub fn tiles(self) -> (u8, u8, u8) {
        use textures::*;
        match self {
            Block::Grass => (T_GRASS_TOP, T_GRASS_SIDE, T_DIRT),
            Block::Dirt => (T_DIRT, T_DIRT, T_DIRT),
            Block::Stone => (T_STONE, T_STONE, T_STONE),
            Block::Sand => (T_SAND, T_SAND, T_SAND),
            Block::Log => (T_LOG_TOP, T_LOG_SIDE, T_LOG_TOP),
            Block::Leaves => (T_LEAVES, T_LEAVES, T_LEAVES),
            Block::Water => (T_WATER, T_WATER, T_WATER),
            Block::Planks => (T_PLANKS, T_PLANKS, T_PLANKS),
            Block::Bricks => (T_BRICKS, T_BRICKS, T_BRICKS),
            Block::Snow => (T_SNOW, T_SNOW_SIDE, T_DIRT),
            Block::Cobble => (T_COBBLE, T_COBBLE, T_COBBLE),
            Block::Crystal => (T_CRYSTAL, T_CRYSTAL, T_CRYSTAL),
            Block::Glass => (T_GLASS, T_GLASS, T_GLASS),
            Block::Air => (0, 0, 0),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Block::Air => "Воздух",
            Block::Grass => "Луговая трава",
            Block::Dirt => "Земля",
            Block::Stone => "Камень",
            Block::Sand => "Песок",
            Block::Log => "Бревно",
            Block::Leaves => "Листва",
            Block::Water => "Вода",
            Block::Planks => "Доски",
            Block::Bricks => "Кирпич",
            Block::Snow => "Снег",
            Block::Cobble => "Булыжник",
            Block::Crystal => "Кристаллическая руда",
            Block::Glass => "Стекло",
        }
    }
}

pub struct World {
    blocks: Vec<Block>,
    #[allow(dead_code)]
    pub seed: u32,
}

fn idx(x: usize, y: usize, z: usize) -> usize {
    (y * WZ + z) * WX + x
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Значимый шум (value noise) с плавной интерполяцией.
pub fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let (x0, z0) = (x.floor(), z.floor());
    let (fx, fz) = (smooth(x - x0), smooth(z - z0));
    let (ix, iz) = (x0 as i32, z0 as i32);
    let a = hash(ix, iz, seed);
    let b = hash(ix + 1, iz, seed);
    let c = hash(ix, iz + 1, seed);
    let d = hash(ix + 1, iz + 1, seed);
    let top = a + (b - a) * fx;
    let bot = c + (d - c) * fx;
    top + (bot - top) * fz
}

pub fn fbm(x: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += value_noise(x * freq, z * freq, seed.wrapping_add(o * 1013)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

impl World {
    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        if y < 0 {
            return Block::Stone;
        }
        if x < 0 || z < 0 || x >= WX as i32 || z >= WZ as i32 || y >= WY as i32 {
            return Block::Air;
        }
        self.blocks[idx(x as usize, y as usize, z as usize)]
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, b: Block) -> bool {
        if x < 0 || y < 0 || z < 0 || x >= WX as i32 || y >= WY as i32 || z >= WZ as i32 {
            return false;
        }
        self.blocks[idx(x as usize, y as usize, z as usize)] = b;
        true
    }

    /// Высота поверхности (до генерации деревьев и воды).
    pub fn terrain_height(seed: u32, x: i32, z: i32) -> i32 {
        let (fx, fz) = (x as f32, z as f32);
        let n = fbm(fx * 0.013, fz * 0.013, seed, 4);
        let m = fbm(fx * 0.006 + 50.0, fz * 0.006 + 50.0, seed.wrapping_add(1), 3);
        let mountain = ((m - 0.5).max(0.0) * 2.6).powf(1.4) * 44.0;
        let mut h = 13.0 + n * 20.0 + mountain;
        // мир окружён океаном
        let edge = (x.min(z).min(WX as i32 - 1 - x).min(WZ as i32 - 1 - z)) as f32;
        let t = smooth((edge / 36.0).clamp(0.0, 1.0));
        h = 9.0 + (h - 9.0) * t;
        (h as i32).clamp(1, WY as i32 - 14)
    }

    pub fn generate(seed: u32) -> World {
        let mut w = World { blocks: vec![Block::Air; WX * WY * WZ], seed };
        let mut heights = vec![0i32; WX * WZ];

        for z in 0..WZ as i32 {
            for x in 0..WX as i32 {
                let h = Self::terrain_height(seed, x, z);
                heights[z as usize * WX + x as usize] = h;
                let rocky = h > 40 && hash(x, z, seed ^ 0x51) > 0.35;
                for y in 0..=h {
                    let b = if y == h {
                        if h <= WATER_LEVEL + 1 {
                            Block::Sand
                        } else if h >= 52 {
                            Block::Snow
                        } else if rocky {
                            Block::Stone
                        } else {
                            Block::Grass
                        }
                    } else if y >= h - 3 {
                        if h <= WATER_LEVEL + 1 { Block::Sand } else { Block::Dirt }
                    } else if y < h - 6 && hash(x * 7 + y, z * 3 - y, seed ^ 0xC0FFEE) > 0.988 {
                        Block::Crystal
                    } else {
                        Block::Stone
                    };
                    w.set(x, y, z, b);
                }
                for y in (h + 1)..=WATER_LEVEL {
                    w.set(x, y, z, Block::Water);
                }
            }
        }

        // деревья
        for z in 3..WZ as i32 - 3 {
            for x in 3..WX as i32 - 3 {
                let h = heights[z as usize * WX + x as usize];
                let forest = fbm(x as f32 * 0.02 + 300.0, z as f32 * 0.02, seed.wrapping_add(9), 2);
                let chance = if forest > 0.55 { 0.03 } else { 0.0025 };
                if w.get(x, h, z) != Block::Grass || hash(x, z, seed.wrapping_add(5)) > chance {
                    continue;
                }
                let trunk = 4 + (hash(x, z, seed.wrapping_add(6)) * 3.0) as i32;
                let top = h + trunk;
                for dy in -2..=1 {
                    let r: i32 = if dy >= 0 { 1 } else { 2 };
                    for dz in -r..=r {
                        for dx in -r..=r {
                            let corner = dx.abs() == r && dz.abs() == r;
                            if corner && (dy == 1 || hash(x + dx, z + dz + dy, seed) > 0.5) {
                                continue;
                            }
                            let (lx, ly, lz) = (x + dx, top + dy, z + dz);
                            if w.get(lx, ly, lz) == Block::Air {
                                w.set(lx, ly, lz, Block::Leaves);
                            }
                        }
                    }
                }
                for y in (h + 1)..top {
                    w.set(x, y, z, Block::Log);
                }
                w.set(x, h, z, Block::Dirt);
            }
        }
        w
    }

    /// Высота первого свободного блока над колонкой.
    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        for y in (0..WY as i32).rev() {
            if self.get(x, y, z).is_solid() {
                return y + 1;
            }
        }
        0
    }

    /// DDA-трассировка: (блок под прицелом, соседняя пустая клетка перед ним).
    pub fn raycast(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<(IVec3, IVec3)> {
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
            if self.get(p.x, p.y, p.z).is_targetable() {
                return Some((p, prev));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_has_land_water_and_trees() {
        let w = World::generate(1337);
        let mut land = 0;
        let mut water = 0;
        let mut logs = 0;
        for z in 0..WZ as i32 {
            for x in 0..WX as i32 {
                if w.get(x, WATER_LEVEL, z) == Block::Water {
                    water += 1;
                }
                if w.get(x, WATER_LEVEL + 1, z).is_solid() {
                    land += 1;
                }
                for y in 0..WY as i32 {
                    if w.get(x, y, z) == Block::Log {
                        logs += 1;
                    }
                }
            }
        }
        assert!(land > 10_000, "land = {land}");
        assert!(water > 5_000, "water = {water}");
        assert!(logs > 100, "logs = {logs}");
    }

    #[test]
    fn raycast_hits_ground_below() {
        let mut w = World { blocks: vec![Block::Air; WX * WY * WZ], seed: 0 };
        w.set(10, 5, 10, Block::Stone);
        let hit = w.raycast(vec3(10.5, 9.5, 10.5), vec3(0.0, -1.0, 0.0), 8.0);
        assert_eq!(hit, Some((ivec3(10, 5, 10), ivec3(10, 6, 10))));
        let miss = w.raycast(vec3(10.5, 9.5, 10.5), vec3(1.0, 0.0, 0.0), 8.0);
        assert_eq!(miss, None);
    }

    #[test]
    fn world_edges_are_ocean() {
        let w = World::generate(7);
        for i in 0..WX as i32 {
            assert!(w.get(i, WATER_LEVEL, 0) == Block::Water);
        }
    }
}
