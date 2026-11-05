//! Генерация мира: климат и биомы, рельеф, пещеры, руды, заброшенные шахты,
//! растительность. Отдельный генератор — для измерения «Рай».

use crate::blocks::*;
use crate::shapes;
use crate::world::{CH, CS, Chunk, Dim};

pub const SEA: i32 = 48;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Biome {
    Ocean,
    Meadow,
    Desert,
    Taiga,
    Islands,
}

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Ocean => "Океан",
            Biome::Meadow => "Цветущие луга",
            Biome::Desert => "Знойная пустыня",
            Biome::Taiga => "Снежная тайга",
            Biome::Islands => "Парящие острова",
        }
    }
}

// ---------- шум ----------
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    smooth(((x - a) / (b - a)).clamp(0.0, 1.0))
}

pub fn noise2(x: f32, z: f32, seed: u32) -> f32 {
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

pub fn fbm2(x: f32, z: f32, seed: u32, oct: u32) -> f32 {
    let (mut s, mut a, mut f, mut n) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..oct {
        s += noise2(x * f, z * f, seed.wrapping_add(o * 1013)) * a;
        n += a;
        a *= 0.5;
        f *= 2.03;
    }
    s / n
}

fn h3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    hash(x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663), z, seed)
}

pub fn noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (x0, y0, z0) = (x.floor(), y.floor(), z.floor());
    let (fx, fy, fz) = (smooth(x - x0), smooth(y - y0), smooth(z - z0));
    let (ix, iy, iz) = (x0 as i32, y0 as i32, z0 as i32);
    let l = |dx: i32, dy: i32, dz: i32| h3(ix + dx, iy + dy, iz + dz, seed);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = lerp(l(0, 0, 0), l(1, 0, 0), fx);
    let x10 = lerp(l(0, 1, 0), l(1, 1, 0), fx);
    let x01 = lerp(l(0, 0, 1), l(1, 0, 1), fx);
    let x11 = lerp(l(0, 1, 1), l(1, 1, 1), fx);
    lerp(lerp(x00, x10, fy), lerp(x01, x11, fy), fz)
}

/// Растягивает fbm (который сгущается около 0.5) на весь диапазон 0..1.
fn stretch(v: f32, k: f32) -> f32 {
    ((v - 0.5) * k + 0.5).clamp(0.0, 1.0)
}

// ---------- климат и рельеф ----------
pub struct Column {
    pub height: i32,
    pub biome: Biome,
    pub temp: f32,
}

pub fn column(seed: u32, x: i32, z: i32) -> Column {
    let (fx, fz) = (x as f32, z as f32);
    let temp = stretch(fbm2(fx * 0.0021, fz * 0.0021, seed.wrapping_add(11), 3), 2.6);
    let cont = stretch(fbm2(fx * 0.0014, fz * 0.0014, seed.wrapping_add(21), 4), 2.4);

    let w_taiga = smoothstep(0.42, 0.30, temp);
    let w_desert = smoothstep(0.58, 0.70, temp);
    let w_meadow = (1.0 - w_taiga - w_desert).max(0.0);

    let detail = fbm2(fx * 0.03, fz * 0.03, seed.wrapping_add(31), 2);
    let hills = fbm2(fx * 0.009, fz * 0.009, seed.wrapping_add(41), 4);
    let h_meadow = 52.0 + stretch(hills, 1.6) * 18.0 + detail * 4.0;
    let dunes = (fbm2(fx * 0.02, fz * 0.012, seed.wrapping_add(51), 2) * 6.28).sin() * 2.5;
    let h_desert = 51.0 + stretch(hills, 1.4) * 9.0 + dunes;
    let mnt = fbm2(fx * 0.0045, fz * 0.0045, seed.wrapping_add(61), 4);
    let mountain = ((stretch(mnt, 2.0) - 0.5).max(0.0) * 2.0).powf(1.5) * 52.0;
    let h_taiga = 53.0 + stretch(hills, 1.6) * 16.0 + detail * 5.0 + mountain;

    let land = w_meadow * h_meadow + w_desert * h_desert + w_taiga * h_taiga;
    let ocean_floor = 33.0 + detail * 6.0 + hills * 4.0;
    let k = smoothstep(0.30, 0.44, cont);
    let h = ocean_floor + (land - ocean_floor) * k;
    let height = (h as i32).clamp(4, CH - 8);

    // биом с лёгким «дрожанием» на границах
    let jitter = (hash(x, z, seed.wrapping_add(71)) - 0.5) * 0.06;
    let t = temp + jitter;
    let biome = if height < SEA - 1 {
        Biome::Ocean
    } else if t < 0.36 {
        Biome::Taiga
    } else if t > 0.64 {
        Biome::Desert
    } else {
        Biome::Meadow
    };
    Column { height, biome, temp }
}

/// Точка появления: ближайшая к началу координат суша на лугах (или любая суша).
pub fn find_spawn(seed: u32) -> (i32, i32) {
    let mut fallback = None;
    for r in (0..8000).step_by(16) {
        let n = (r / 8).max(1);
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let (x, z) = ((a.cos() * r as f32) as i32, (a.sin() * r as f32) as i32);
            let c = column(seed, x, z);
            if c.height > SEA + 2 {
                if c.biome == Biome::Meadow {
                    return (x, z);
                }
                fallback.get_or_insert((x, z));
            }
        }
        if r > 2000 {
            if let Some(f) = fallback {
                return f;
            }
        }
    }
    fallback.unwrap_or((0, 0))
}

// ---------- генерация чанка ----------
pub(crate) struct Gen<'a> {
    pub(crate) c: &'a mut Chunk,
    pub(crate) x0: i32,
    pub(crate) z0: i32,
}

impl Gen<'_> {
    pub(crate) fn inside(&self, x: i32, y: i32, z: i32) -> bool {
        x >= self.x0 && x < self.x0 + CS && z >= self.z0 && z < self.z0 + CS && (0..CH).contains(&y)
    }
    pub(crate) fn get(&self, x: i32, y: i32, z: i32) -> u16 {
        if self.inside(x, y, z) { self.c.get(x - self.x0, y, z - self.z0) } else { AIR }
    }
    pub(crate) fn set(&mut self, x: i32, y: i32, z: i32, b: u16) {
        if self.inside(x, y, z) {
            self.c.set(x - self.x0, y, z - self.z0, b);
        }
    }
    /// Ставит блок только в пустоту (или в траву/листву для стволов).
    fn put(&mut self, x: i32, y: i32, z: i32, b: u16) {
        if self.inside(x, y, z) {
            let cur = self.get(x, y, z);
            if cur == AIR || def(cur).shape == Shape::Cross || (def(cur).shape == Shape::Cutout && def(b).shape == Shape::Cube) {
                self.set(x, y, z, b);
            }
        }
    }
}

pub fn generate(dim: Dim, seed: u32, cx: i32, cz: i32) -> Chunk {
    match dim {
        Dim::Overworld => gen_overworld(seed, cx, cz),
        Dim::Heaven => gen_heaven(seed, cx, cz),
        Dim::Abyss => gen_abyss(seed, cx, cz),
    }
}

/// Бездна: огромные пещеры между дном и потолком, лавовое море, пепел и базальт,
/// светокамень на сводах, мосты-крепости. У начала координат — площадка с порталом назад.
fn gen_abyss(seed: u32, cx: i32, cz: i32) -> Chunk {
    let mut chunk = Chunk::empty();
    let (x0, z0) = (cx * CS, cz * CS);
    const FLOOR: i32 = 4;
    const ROOF: i32 = 100;
    const LAVA_SEA: i32 = 30;
    for lz in 0..CS {
        for lx in 0..CS {
            let (x, z) = (x0 + lx, z0 + lz);
            let (fx, fz) = (x as f32, z as f32);
            let ground = 34 + (fbm2(fx * 0.02, fz * 0.02, seed.wrapping_add(701), 3) * 26.0) as i32;
            let ceil = 84 + (fbm2(fx * 0.025, fz * 0.025, seed.wrapping_add(702), 3) * 14.0) as i32;
            for y in 0..CH {
                let b = if y < FLOOR {
                    BEDROCK
                } else if y >= ROOF {
                    if y < ROOF + 3 { BEDROCK } else { AIR }
                } else {
                    let n = noise3(fx * 0.045, y as f32 * 0.07, fz * 0.045, seed.wrapping_add(703));
                    let solid = y < ground || y > ceil || n > 0.72;
                    if solid {
                        let k = hash(x * 7 + y, z, seed.wrapping_add(704));
                        if k > 0.996 { GOLD_ORE } else if k > 0.993 { CRYSTAL_ORE } else if k > 0.985 && y < 50 { MAGMA } else if (y / 6 + (fx * 0.05).sin() as i32) % 3 == 0 { BASALT } else { ASH_BLOCK }
                    } else if y <= LAVA_SEA {
                        LAVA
                    } else {
                        AIR
                    }
                };
                if b != AIR {
                    chunk.set(lx, y, lz, b);
                }
            }
            // светокамень на сводах и грибы на земле
            for y in (FLOOR + 1..ROOF - 1).rev() {
                let here = chunk.get(lx, y, lz);
                let below = chunk.get(lx, y - 1, lz);
                if here != AIR && below == AIR && hash(x, z * 3 + y, seed.wrapping_add(705)) < 0.03 {
                    chunk.set(lx, y - 1, lz, GLOWSTONE);
                }
                if here == AIR && (below == ASH_BLOCK || below == BASALT) && hash(x * 5 + y, z, seed.wrapping_add(706)) < 0.01 {
                    chunk.set(lx, y, lz, if hash(x, z, 707) < 0.5 { BLUE_MUSHROOM } else { FIREFLOWER });
                }
            }
        }
    }
    let mut g = Gen { c: &mut chunk, x0, z0 };
    // мосты крепостей: прямые пролёты тёмного кирпича над лавой
    let region = 80;
    let (rx0, rz0) = ((x0 - 40).div_euclid(region), (z0 - 40).div_euclid(region));
    for rz in rz0..=rz0 + 1 {
        for rx in rx0..=rx0 + 1 {
            if hash(rx, rz, seed.wrapping_add(708)) > 0.55 {
                continue;
            }
            let (ox, oz) = (rx * region + 20, rz * region + 20);
            let y = 52 + (hash(rx, rz, seed.wrapping_add(709)) * 8.0) as i32;
            for t in 0..48 {
                for w in -1..=1 {
                    g.set(ox + t, y, oz + w, DARK_BRICKS);
                    g.set(ox + w + 24, y, oz + t - 24, DARK_BRICKS);
                }
                if t % 8 == 0 {
                    for py in LAVA_SEA - 2..y {
                        g.set(ox + t, py, oz, DARK_BRICKS);
                    }
                }
                g.set(ox + t, y + 1, oz - 2, shapes::id(shapes::Form::Fence { m: 6 }));
                g.set(ox + t, y + 1, oz + 2, shapes::id(shapes::Form::Fence { m: 6 }));
            }
            g.c.spawns.push((crate::mobdefs::by_name("Огненный бес") as u16, [ox + 10, y + 1, oz]));
            g.c.spawns.push((crate::mobdefs::by_name("Рыцарь-скелет") as u16, [ox + 30, y + 1, oz]));
            for k in 0..2 {
                let (cx2, cz2) = (ox + 6 + k * 30, oz + 1);
                if g.inside(cx2, y + 1, cz2) {
                    g.set(cx2, y + 1, cz2, CHEST);
                }
            }
        }
    }
    // площадка прибытия с порталом назад
    for x in -3..=3 {
        for z in -3..=3 {
            for y in 60..66 {
                g.set(x, y, z, AIR);
            }
            g.set(x, 59, z, DARK_BRICKS);
        }
    }
    for y in 60..63 {
        for x in -1..=1 {
            g.set(x, y, 3, if x == 0 && y < 62 { ABYSS_PORTAL } else { OBSIDIAN });
        }
    }
    g.set(0, 62, 3, OBSIDIAN);
    chunk
}

/// Подземелье-лабиринт: 6×6 комнат на глубине 20–26, вход — шахта с лестницей,
/// в тупиках сундуки и ловушки (провалы в лаву под нажимными плитами), в дальней комнате — босс.
pub(crate) fn dungeons(g: &mut Gen, seed: u32) {
    const REG: i32 = 112;
    const ROOM: i32 = 8;
    const N: i32 = 6;
    let (x0, z0) = (g.x0, g.z0);
    for rz in (z0 - 60).div_euclid(REG)..=(z0 + 76).div_euclid(REG) {
        for rx in (x0 - 60).div_euclid(REG)..=(x0 + 76).div_euclid(REG) {
            if hash(rx, rz, seed.wrapping_add(801)) > 0.3 {
                continue;
            }
            let ox = rx * REG + 16;
            let oz = rz * REG + 16;
            if ox + N * ROOM < x0 || ox > x0 + CS || oz + N * ROOM < z0 || oz > z0 + CS {
                continue;
            }
            let base = 20;
            // лабиринт обходом в глубину
            let mut open = [[0u8; (N * N) as usize]; 1];
            let mut seen = vec![false; (N * N) as usize];
            let mut stack = vec![0i32];
            seen[0] = true;
            let mut depth = vec![0i32; (N * N) as usize];
            let mut rng = (rx as u32).wrapping_mul(73_856_093) ^ (rz as u32).wrapping_mul(19_349_663) ^ seed;
            let mut next = || {
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                rng
            };
            while let Some(&c) = stack.last() {
                let (cx, cz) = (c % N, c / N);
                let mut opts = Vec::new();
                for (d, (dx, dz)) in [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().enumerate() {
                    let (nx, nz) = (cx + dx, cz + dz);
                    if (0..N).contains(&nx) && (0..N).contains(&nz) && !seen[(nz * N + nx) as usize] {
                        opts.push((d, nz * N + nx));
                    }
                }
                if opts.is_empty() {
                    stack.pop();
                    continue;
                }
                let (d, n) = opts[next() as usize % opts.len()];
                open[0][c as usize] |= 1 << d;
                open[0][n as usize] |= 1 << (d ^ 1);
                seen[n as usize] = true;
                depth[n as usize] = depth[c as usize] + 1;
                stack.push(n);
            }
            let far = (0..(N * N) as usize).max_by_key(|&i| depth[i]).unwrap_or(0) as i32;
            for c in 0..N * N {
                let (cx, cz) = (c % N, c / N);
                let (bx, bz) = (ox + cx * ROOM, oz + cz * ROOM);
                let o = open[0][c as usize];
                let dead_end = o.count_ones() == 1 && c != 0;
                for x in bx..bx + ROOM {
                    for z in bz..bz + ROOM {
                        let (lx, lz) = (x - bx, z - bz);
                        let wall_x = lx == 0 || lx == ROOM - 1;
                        let wall_z = lz == 0 || lz == ROOM - 1;
                        let door = (lx == ROOM - 1 && o & 1 != 0 || lx == 0 && o & 2 != 0) && (3..=4).contains(&lz)
                            || (lz == ROOM - 1 && o & 4 != 0 || lz == 0 && o & 8 != 0) && (3..=4).contains(&lx);
                        for y in base - 1..base + 6 {
                            let edge = y == base - 1 || y == base + 5;
                            let b = if edge {
                                if hash(x, z * 3 + y, seed.wrapping_add(802)) > 0.7 { MOSSY_COBBLE } else { STONE_BRICKS }
                            } else if (wall_x || wall_z) && !(door && y < base + 3) {
                                if hash(x * 3 + y, z, seed.wrapping_add(803)) > 0.8 { CRACKED_STONE_BRICKS } else if hash(x, z + y, 804) > 0.85 { MOSSY_STONE_BRICKS } else { STONE_BRICKS }
                            } else {
                                AIR
                            };
                            g.set(x, y, z, b);
                        }
                        if !wall_x && !wall_z && hash(x * 7, z * 5, seed.wrapping_add(805)) > 0.96 {
                            g.set(x, base + 4, z, COBWEB);
                        }
                    }
                }
                // факелы по углам
                g.set(bx + 1, base, bz + 1, TORCH);
                let mid = (bx + ROOM / 2, bz + ROOM / 2);
                if c == 0 {
                    // вход: шахта с лестницей до поверхности
                    for y in base..118 {
                        if g.get(mid.0, y, mid.1) == AIR && y > base + 6 && g.get(mid.0, y + 1, mid.1) == AIR && g.get(mid.0, y + 2, mid.1) == AIR {
                            break;
                        }
                        g.set(mid.0, y, mid.1, AIR);
                        g.set(mid.0 + 1, y, mid.1, AIR);
                        g.set(mid.0 - 1, y, mid.1, LADDER);
                    }
                } else if c == far {
                    // логово босса
                    let boss = crate::mobdefs::BOSS0 + [0usize, 1, 2][(next() % 3) as usize];
                    if g.inside(mid.0, base, mid.1) {
                        g.c.spawns.push((boss as u16, [mid.0, base, mid.1]));
                    }
                    g.set(mid.0 + 2, base, mid.1 + 2, CHEST);
                    g.set(mid.0 - 2, base, mid.1 - 2, CHEST);
                } else if dead_end {
                    g.set(mid.0, base, mid.1, CHEST);
                } else if next() % 3 == 0 {
                    // ловушка: под плитой — яма с лавой
                    g.set(mid.0, base - 1, mid.1, AIR);
                    g.set(mid.0, base - 2, mid.1, LAVA);
                    g.set(mid.0, base - 3, mid.1, STONE_BRICKS);
                    g.set(mid.0, base, mid.1, shapes::PLATE0);
                } else if next() % 2 == 0 && g.inside(mid.0, base, mid.1) {
                    let pool = ["Костяк", "Рыцарь-скелет", "Рудокоп-призрак", "Тёмный гном"];
                    let m = crate::mobdefs::by_name(pool[(next() % 4) as usize]);
                    g.c.spawns.push((m as u16, [mid.0, base, mid.1]));
                }
            }
        }
    }
}

fn gen_overworld(seed: u32, cx: i32, cz: i32) -> Chunk {
    let mut chunk = Chunk::empty();
    let (x0, z0) = (cx * CS, cz * CS);
    let mut cols = Vec::with_capacity(256);
    for lz in 0..CS {
        for lx in 0..CS {
            cols.push(column(seed, x0 + lx, z0 + lz));
        }
    }

    // --- рельеф и поверхность ---
    for lz in 0..CS {
        for lx in 0..CS {
            let (x, z) = (x0 + lx, z0 + lz);
            let col = &cols[(lz * CS + lx) as usize];
            let h = col.height;
            let beach = h <= SEA + 1 && h >= SEA - 3;
            let (top, filler, depth) = match col.biome {
                Biome::Ocean => {
                    let n = hash(x / 3, z / 3, seed.wrapping_add(81));
                    let b = if n < 0.5 { SAND } else if n < 0.8 { GRAVEL } else { CLAY };
                    (b, b, 3)
                }
                Biome::Desert => (SAND, SAND, 4),
                Biome::Taiga if beach => (GRAVEL, GRAVEL, 2),
                _ if beach => (SAND, SAND, 3),
                Biome::Taiga if h > 96 => (SNOW_BLOCK, STONE, 1),
                Biome::Taiga if h > 86 => (STONE, STONE, 1),
                Biome::Taiga => (SNOW_GRASS, DIRT, 3),
                _ => (GRASS, DIRT, 3),
            };
            for y in 0..=h {
                let b = if y == 0 || (y < 3 && hash(x, z * 3 + y, seed) < 0.5) {
                    BEDROCK
                } else if y == h {
                    top
                } else if y > h - depth {
                    filler
                } else if col.biome == Biome::Desert && y > h - depth - 4 {
                    SANDSTONE
                } else if y < 18 {
                    SLATE
                } else {
                    STONE
                };
                chunk.set(lx, y, lz, b);
            }
            for y in (h + 1)..=SEA {
                let b = if y == SEA && col.biome == Biome::Taiga && col.temp < 0.32 { ICE } else { WATER };
                chunk.set(lx, y, lz, b);
            }
        }
    }

    // --- вкрапления пород ---
    for lz in 0..CS {
        for lx in 0..CS {
            let (x, z) = (x0 + lx, z0 + lz);
            let h = cols[(lz * CS + lx) as usize].height;
            for y in 3..(h - 4).max(3) {
                let cur = chunk.get(lx, y, lz);
                if cur != STONE && cur != SLATE {
                    continue;
                }
                let n = noise3(x as f32 * 0.06, y as f32 * 0.09, z as f32 * 0.06, seed.wrapping_add(91));
                if n > 0.8 {
                    let kind = hash(x / 24, z / 24, seed.wrapping_add(92));
                    let b = if kind < 0.25 { GRANITE } else if kind < 0.5 { MARBLE } else if kind < 0.75 { TUFF } else { BASALT };
                    chunk.set(lx, y, lz, b);
                } else if n < 0.12 && y > 30 {
                    chunk.set(lx, y, lz, if hash(x, z, 93) < 0.5 { GRAVEL } else { DIRT });
                }
            }
        }
    }

    let mut g = Gen { c: &mut chunk, x0, z0 };

    // --- руды ---
    let ores: [(u16, u32, i32, i32, u32); 8] = [
        (COAL_ORE, 18, 5, 110, 10),
        (IRON_ORE, 12, 5, 70, 7),
        (COPPER_ORE, 10, 20, 80, 8),
        (GOLD_ORE, 4, 3, 34, 6),
        (MALACHITE_ORE, 3, 5, 50, 5),
        (CRYSTAL_ORE, 3, 3, 40, 4),
        (RUBY_ORE, 2, 3, 20, 4),
        (SAPPHIRE_ORE, 2, 3, 20, 4),
    ];
    let mut rng = Rng::new(seed ^ (cx as u32).wrapping_mul(0x9E37_79B9) ^ (cz as u32).wrapping_mul(0x85EB_CA6B));
    for (ore, tries, ymin, ymax, size) in ores {
        for _ in 0..tries {
            let (mut x, mut y, mut z) = (x0 + rng.range(0, CS), rng.range(ymin, ymax), z0 + rng.range(0, CS));
            for _ in 0..size {
                let cur = g.get(x, y, z);
                if cur == STONE || cur == SLATE || cur == GRANITE || cur == TUFF {
                    g.set(x, y, z, ore);
                }
                match rng.range(0, 6) {
                    0 => x += 1,
                    1 => x -= 1,
                    2 => y += 1,
                    3 => y -= 1,
                    4 => z += 1,
                    _ => z -= 1,
                }
            }
        }
    }

    // --- пещеры ---
    for lz in 0..CS {
        for lx in 0..CS {
            let (x, z) = (x0 + lx, z0 + lz);
            let col = &cols[(lz * CS + lx) as usize];
            let top = if col.height < SEA + 3 { (SEA - 8).min(col.height - 5) } else { col.height - 1 };
            for y in 4..top.max(4) {
                let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                let a = noise3(fx * 0.03, fy * 0.05, fz * 0.03, seed.wrapping_add(101)) - 0.5;
                let b = noise3(fx * 0.03, fy * 0.05, fz * 0.03, seed.wrapping_add(102)) - 0.5;
                let tunnel = a * a + b * b < 0.0016;
                let cavern = y < 46 && noise3(fx * 0.018, fy * 0.03, fz * 0.018, seed.wrapping_add(103)) > 0.76;
                if tunnel || cavern {
                    let cur = g.get(x, y, z);
                    if cur != BEDROCK && cur != WATER && cur != ICE {
                        g.set(x, y, z, if y <= 10 { LAVA } else { AIR });
                    }
                }
            }
        }
    }

    // --- заброшенные шахты ---
    carve_mineshafts(&mut g, seed, &cols);

    // --- растительность (с запасом вокруг чанка, чтобы кроны не обрезались) ---
    const M: i32 = 3;
    for gz in (z0 - M)..(z0 + CS + M) {
        for gx in (x0 - M)..(x0 + CS + M) {
            let inside = gx >= x0 && gx < x0 + CS && gz >= z0 && gz < z0 + CS;
            let col = if inside { None } else { Some(column(seed, gx, gz)) };
            let c = col.as_ref().unwrap_or_else(|| &cols[((gz - z0) * CS + (gx - x0)) as usize]);
            let h = c.height;
            if h <= SEA {
                continue;
            }
            if inside && g.get(gx, h, gz) == AIR {
                continue; // поверхность выедена пещерой
            }
            let r = hash(gx, gz, seed.wrapping_add(5));
            let (fx, fz) = (gx as f32, gz as f32);
            match c.biome {
                Biome::Meadow => {
                    let forest = fbm2(fx * 0.02 + 300.0, fz * 0.02, seed.wrapping_add(9), 2);
                    let grove = fbm2(fx * 0.01, fz * 0.01 + 500.0, seed.wrapping_add(10), 2);
                    let chance = if forest > 0.56 { 0.035 } else { 0.003 };
                    if r < chance && h > SEA + 1 {
                        let t = hash(gx, gz, seed.wrapping_add(6));
                        if grove > 0.64 {
                            tree_sakura(&mut g, gx, h, gz, t);
                        } else if h <= SEA + 3 && t > 0.5 {
                            tree_willow(&mut g, gx, h, gz, t);
                        } else if forest > 0.72 && t > 0.85 {
                            tree_ebony(&mut g, gx, h, gz, t);
                        } else if grove < 0.3 && t > 0.6 {
                            tree_maple(&mut g, gx, h, gz, t);
                        } else if t < 0.3 {
                            tree_birch(&mut g, gx, h, gz, t);
                        } else {
                            tree_oak(&mut g, gx, h, gz, t);
                        }
                    } else if inside && g.get(gx, h + 1, gz) == AIR && g.get(gx, h, gz) == GRASS {
                        let p = hash(gx, gz, seed.wrapping_add(7));
                        let patch = fbm2(fx * 0.08, fz * 0.08, seed.wrapping_add(12), 1);
                        if h <= SEA + 1 && p < 0.08 {
                            g.set(gx, h + 1, gz, CATTAIL);
                        } else if p < 0.10 {
                            g.set(gx, h + 1, gz, TALL_GRASS);
                        } else if p < 0.13 && patch > 0.55 {
                            let fl = [DAISY, POPPY, CORNFLOWER, DANDELION, TULIP_RED, TULIP_ORANGE, TULIP_PINK, LAVENDER, BLUEBELL, SUNFLOWER, ORCHID, PEONY, CLOVER, THISTLE][(patch * 97.0) as usize % 14];
                            g.set(gx, h + 1, gz, fl);
                        } else if p > 0.996 && p < 0.998 {
                            g.set(gx, h + 1, gz, if p < 0.997 { BERRY_BUSH } else { ROSE_BUSH });
                        } else if p > 0.9993 {
                            g.set(gx, h + 1, gz, PUMPKIN);
                        } else if p > 0.9986 {
                            g.set(gx, h + 1, gz, MELON);
                        }
                    }
                }
                Biome::Taiga => {
                    if r < 0.02 && h > SEA + 1 && h < 86 {
                        let t = hash(gx, gz, seed.wrapping_add(6));
                        if t > 0.78 {
                            tree_cedar(&mut g, gx, h, gz, t);
                        } else {
                            tree_spruce(&mut g, gx, h, gz, t);
                        }
                    } else if inside && g.get(gx, h + 1, gz) == AIR && g.get(gx, h, gz) == SNOW_GRASS {
                        let p = hash(gx, gz, seed.wrapping_add(7));
                        if p < 0.004 {
                            g.set(gx, h + 1, gz, if p < 0.002 { SNOW_FLOWER } else { BERRY_BUSH });
                        } else if p < 0.06 {
                            g.set(gx, h + 1, gz, FERN);
                        } else if p < 0.065 {
                            g.set(gx, h + 1, gz, if p < 0.0625 { RED_MUSHROOM } else { BROWN_MUSHROOM });
                        }
                    }
                }
                Biome::Desert => {
                    if h <= SEA + 3 && r < 0.02 {
                        let t = hash(gx, gz, seed.wrapping_add(6));
                        if t > 0.8 {
                            tree_mangrove(&mut g, gx, h, gz, t);
                        } else {
                            tree_palm(&mut g, gx, h, gz, t);
                        }
                    } else if r > 0.9985 && h > SEA + 4 {
                        tree_acacia(&mut g, gx, h, gz, hash(gx, gz, seed.wrapping_add(6)));
                    } else if inside && g.get(gx, h + 1, gz) == AIR && g.get(gx, h, gz) == SAND {
                        let p = hash(gx, gz, seed.wrapping_add(7));
                        if p < 0.006 {
                            let tall = 1 + (hash(gx, gz, 8) * 3.0) as i32;
                            for y in 1..=tall {
                                g.set(gx, h + y, gz, CACTUS);
                            }
                        } else if p < 0.016 {
                            g.set(gx, h + 1, gz, DEAD_BUSH);
                        } else if p < 0.018 {
                            g.set(gx, h + 1, gz, DESERT_ROSE);
                        }
                    }
                }
                _ => {}
            }
            // тростник у воды
            if inside && h == SEA + 1 && g.get(gx, h + 1, gz) == AIR {
                let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dz)| g.get(gx + dx, h, gz + dz) == WATER);
                if near && hash(gx, gz, seed.wrapping_add(13)) < 0.25 && matches!(g.get(gx, h, gz), GRASS | SAND) {
                    for y in 1..=(2 + (hash(gx, gz, 14) * 2.0) as i32) {
                        g.set(gx, h + y, gz, REEDS);
                    }
                }
            }
        }
    }
    // светящиеся грибы в пещерах
    for lz in 0..CS {
        for lx in 0..CS {
            let (x, z) = (x0 + lx, z0 + lz);
            for y in 6..40 {
                if g.get(x, y, z) == AIR && matches!(g.get(x, y - 1, z), STONE | SLATE | TUFF) && hash(x * 3 + y, z, seed.wrapping_add(15)) < 0.012 {
                    g.set(x, y, z, if hash(x, z * 5 + y, 951) < 0.3 { BLUE_MUSHROOM } else { GLOW_MUSHROOM });
                }
            }
        }
    }
    crate::structures::place(&mut g, Dim::Overworld, seed);
    dungeons(&mut g, seed);
    chunk
}

struct Rng(u32);
impl Rng {
    fn new(s: u32) -> Self {
        Rng(s | 1)
    }
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn range(&mut self, a: i32, b: i32) -> i32 {
        a + (self.next() % ((b - a).max(1)) as u32) as i32
    }
}

// ---------- деревья ----------
fn leaves_blob(g: &mut Gen, x: i32, top: i32, z: i32, leaf: u16, rnd: f32) {
    for dy in -2..=1 {
        let r: i32 = if dy >= 0 { 1 } else { 2 };
        for dz in -r..=r {
            for dx in -r..=r {
                let corner = dx.abs() == r && dz.abs() == r;
                if corner && (dy == 1 || hash(x + dx, z + dz + dy, (rnd * 1000.0) as u32) > 0.5) {
                    continue;
                }
                g.put(x + dx, top + dy, z + dz, leaf);
            }
        }
    }
}

fn trunk(g: &mut Gen, x: i32, h: i32, z: i32, len: i32, log: u16) {
    for y in 1..=len {
        if g.inside(x, h + y, z) {
            g.set(x, h + y, z, log);
        }
    }
    if g.inside(x, h, z) {
        g.set(x, h, z, DIRT);
    }
}

fn tree_oak(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    let len = 4 + (t * 3.0) as i32;
    leaves_blob(g, x, h + len, z, OAK_LEAVES, t);
    trunk(g, x, h, z, len - 1, OAK_LOG);
}

fn tree_birch(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    let len = 5 + (t * 6.0) as i32;
    leaves_blob(g, x, h + len, z, BIRCH_LEAVES, t);
    trunk(g, x, h, z, len - 1, BIRCH_LOG);
}

fn tree_spruce(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    let len = 7 + (t * 4.0) as i32;
    let top = h + len + 1;
    g.put(x, top, z, SPRUCE_LEAVES);
    let mut r: i32 = 0;
    for y in (h + 3..top).rev() {
        r = if (top - y) % 2 == 1 { (r + 1).min(3) } else { (r - 1).max(1) };
        if top - y <= 1 {
            r = 1;
        }
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() + dz.abs() <= r + (r > 1) as i32 {
                    g.put(x + dx, y, z + dz, SPRUCE_LEAVES);
                }
            }
        }
    }
    trunk(g, x, h, z, len, SPRUCE_LOG);
}

fn tree_sakura(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    let len = 4 + (t * 2.0) as i32;
    let top = h + len;
    for dy in 0..=2 {
        let r: i32 = if dy == 2 { 2 } else { 3 };
        for dz in -r..=r {
            for dx in -r..=r {
                if dx * dx + dz * dz <= r * r + 1 && hash(x + dx, z + dz, 900 + dy as u32) > 0.12 {
                    g.put(x + dx, top + dy, z + dz, SAKURA_LEAVES);
                }
            }
        }
    }
    trunk(g, x, h, z, len, SAKURA_LOG);
}


fn tree_maple(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    // широкая округлая крона
    let len = 5 + (t * 3.0) as i32;
    let top = h + len;
    for dy in -2i32..=2 {
        let r: i32 = if dy.abs() == 2 { 2 } else { 3 };
        for dz in -r..=r {
            for dx in -r..=r {
                if dx * dx + dz * dz + dy * dy * 2 <= r * r + 2 && hash(x + dx * 7, z + dz + dy * 3, 931) > 0.08 {
                    g.put(x + dx, top + dy, z + dz, MAPLE_LEAVES);
                }
            }
        }
    }
    trunk(g, x, h, z, len, MAPLE_LOG);
}

fn tree_ebony(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    // высокое тёмное дерево с несколькими ярусами
    let len = 9 + (t * 5.0) as i32;
    for k in 0..3 {
        let y = h + len - k * 3;
        let r = 1 + k;
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() + dz.abs() <= r + 1 {
                    g.put(x + dx, y, z + dz, EBONY_LEAVES);
                }
            }
        }
    }
    g.put(x, h + len + 1, z, EBONY_LEAVES);
    trunk(g, x, h, z, len, EBONY_LOG);
}

fn tree_willow(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    // крона с длинными свисающими ветвями
    let len = 5 + (t * 2.0) as i32;
    let top = h + len;
    for dz in -3i32..=3 {
        for dx in -3i32..=3 {
            let d = dx * dx + dz * dz;
            if d <= 10 {
                g.put(x + dx, top, z + dz, WILLOW_LEAVES);
                if d <= 4 {
                    g.put(x + dx, top + 1, z + dz, WILLOW_LEAVES);
                }
                if d >= 5 {
                    let hang = 1 + (hash(x + dx, z + dz, 941) * 4.0) as i32;
                    for k in 1..=hang {
                        g.put(x + dx, top - k, z + dz, WILLOW_LEAVES);
                    }
                }
            }
        }
    }
    trunk(g, x, h, z, len, WILLOW_LOG);
}

fn tree_acacia(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    // наклонный ствол и плоская крона-зонтик
    let len = 4 + (t * 2.0) as i32;
    let (dx, dz) = if t < 0.5 { (1, 0) } else { (0, 1) };
    trunk(g, x, h, z, len, ACACIA_LOG);
    let (tx, tz) = (x + dx * 2, z + dz * 2);
    g.put(x + dx, h + len + 1, z + dz, ACACIA_LOG);
    g.put(tx, h + len + 2, tz, ACACIA_LOG);
    let top = h + len + 3;
    for oz in -3i32..=3 {
        for ox in -3i32..=3 {
            if ox.abs() + oz.abs() <= 4 {
                g.put(tx + ox, top, tz + oz, ACACIA_LEAVES);
            }
            if ox.abs() + oz.abs() <= 1 {
                g.put(tx + ox, top + 1, tz + oz, ACACIA_LEAVES);
            }
        }
    }
}

fn tree_mangrove(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    // ствол на корнях-ходулях
    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        for k in 0..2 {
            g.put(x + dx * (2 - k), h + 1 + k, z + dz * (2 - k), MANGROVE_LOG);
        }
    }
    let len = 6 + (t * 2.0) as i32;
    leaves_blob(g, x, h + len, z, MANGROVE_LEAVES, t);
    for (dx, dz) in [(2, 0), (-2, 0), (0, 2), (0, -2)] {
        g.put(x + dx, h + len - 1, z + dz, MANGROVE_LEAVES);
    }
    trunk(g, x, h, z, len - 1, MANGROVE_LOG);
}

fn tree_cedar(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    // голый ствол и пышная верхушка
    let len = 8 + (t * 4.0) as i32;
    let top = h + len;
    for dy in -3..=1 {
        let r: i32 = if dy == 1 { 0 } else if dy == -3 { 1 } else { 2 };
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() + dz.abs() <= r + (r > 1) as i32 {
                    g.put(x + dx, top + dy, z + dz, CEDAR_LEAVES);
                }
            }
        }
    }
    trunk(g, x, h, z, len - 1, CEDAR_LOG);
}

fn tree_palm(g: &mut Gen, x: i32, h: i32, z: i32, t: f32) {
    let len = 5 + (t * 3.0) as i32;
    let top = h + len + 1;
    g.put(x, top, z, PALM_LEAVES);
    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        for i in 1..=3 {
            let droop = if i == 3 { -1 } else { 0 };
            g.put(x + dx * i, top + droop, z + dz * i, PALM_LEAVES);
        }
    }
    for (dx, dz) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
        g.put(x + dx, top, z + dz, PALM_LEAVES);
    }
    trunk(g, x, h, z, len, PALM_LOG);
}

// ---------- шахты ----------
const REGION: i32 = 64;

struct Seg {
    along_x: bool,
    a: i32,
    b: i32,
    fixed: i32,
    y: i32,
}

fn shaft_segments(seed: u32, rx: i32, rz: i32) -> Option<(Vec<Seg>, (i32, i32, i32))> {
    let s = seed.wrapping_add(777);
    if hash(rx, rz, s) > 0.55 {
        return None;
    }
    let mut rng = Rng::new(s ^ (rx as u32).wrapping_mul(0x27D4_EB2D) ^ (rz as u32).wrapping_mul(0x1656_67B1));
    let ox = rx * REGION + rng.range(16, 48);
    let oz = rz * REGION + rng.range(16, 48);
    let y = rng.range(16, 36);
    let mut segs = vec![
        Seg { along_x: true, a: ox - rng.range(16, 48), b: ox + rng.range(16, 48), fixed: oz, y },
        Seg { along_x: false, a: oz - rng.range(16, 48), b: oz + rng.range(16, 48), fixed: ox, y },
    ];
    for _ in 0..3 {
        let along = rng.range(0, 2) == 0;
        let off = rng.range(8, 30) * if rng.range(0, 2) == 0 { 1 } else { -1 };
        let len = rng.range(10, 26);
        let dir = if rng.range(0, 2) == 0 { 1 } else { -1 };
        if along {
            // ответвление вдоль Z от главного коридора по X
            let at = ox + off;
            let (a, b) = if dir > 0 { (oz, oz + len) } else { (oz - len, oz) };
            segs.push(Seg { along_x: false, a, b, fixed: at, y });
        } else {
            let at = oz + off;
            let (a, b) = if dir > 0 { (ox, ox + len) } else { (ox - len, ox) };
            segs.push(Seg { along_x: true, a, b, fixed: at, y });
        }
    }
    Some((segs, (ox, y, oz)))
}

fn carve_mineshafts(g: &mut Gen, seed: u32, cols: &[Column]) {
    let (x0, z0) = (g.x0, g.z0);
    let surface = |g: &Gen, x: i32, z: i32| -> i32 {
        if x >= g.x0 && x < g.x0 + CS && z >= g.z0 && z < g.z0 + CS {
            cols[((z - g.z0) * CS + (x - g.x0)) as usize].height
        } else {
            CH
        }
    };
    let r0x = (x0 - 70).div_euclid(REGION);
    let r1x = (x0 + CS + 70).div_euclid(REGION);
    let r0z = (z0 - 70).div_euclid(REGION);
    let r1z = (z0 + CS + 70).div_euclid(REGION);
    for rz in r0z..=r1z {
        for rx in r0x..=r1x {
            let Some((segs, (ox, oy, oz))) = shaft_segments(seed, rx, rz) else { continue };
            // центральный зал
            for y in oy..oy + 4 {
                for z in oz - 3..=oz + 3 {
                    for x in ox - 3..=ox + 3 {
                        if g.inside(x, y, z) && y < surface(g, x, z) - 4 && g.get(x, y, z) != WATER {
                            g.set(x, y, z, AIR);
                        }
                    }
                }
            }
            for s in &segs {
                for t in s.a..=s.b {
                    for w in -1..=1 {
                        let (x, z) = if s.along_x { (t, s.fixed + w) } else { (s.fixed + w, t) };
                        if !g.inside(x, s.y, z) || s.y + 3 >= surface(g, x, z) - 3 {
                            continue;
                        }
                        let support = t.rem_euclid(5) == 0;
                        for dy in 0..3 {
                            let y = s.y + dy;
                            if g.get(x, y, z) == WATER {
                                continue;
                            }
                            let b = if support && w != 0 && dy < 2 {
                                BEAM
                            } else if support && dy == 2 {
                                if w == 0 && hash(t, s.fixed, seed.wrapping_add(31)) < 0.25 { LANTERN } else { OAK_PLANKS }
                            } else if dy == 2 && w != 0 && hash(x * 7 + y, z, seed.wrapping_add(32)) < 0.05 {
                                COBWEB
                            } else if dy == 0 && w != 0 && !support && hash(x, z, seed.wrapping_add(33)) < 0.012 {
                                CHEST
                            } else {
                                AIR
                            };
                            g.set(x, y, z, b);
                        }
                        // мостки над пустотой
                        if !is_solid(g.get(x, s.y - 1, z)) {
                            g.set(x, s.y - 1, z, OAK_PLANKS);
                        }
                    }
                }
            }
        }
    }
}

// ---------- Рай ----------
pub fn heaven_island(seed: u32, x: i32, z: i32) -> Option<(i32, i32)> {
    let (fx, fz) = (x as f32, z as f32);
    let mut m = stretch(fbm2(fx * 0.011, fz * 0.011, seed.wrapping_add(500), 4), 2.2);
    let d = (fx * fx + fz * fz).sqrt();
    m += 0.45 * (1.0 - d / 70.0).max(0.0);
    if m < 0.56 {
        return None;
    }
    let e = m - 0.56;
    let top = 68 + (e * 40.0) as i32 + (fbm2(fx * 0.05, fz * 0.05, seed.wrapping_add(501), 2) * 5.0) as i32;
    let bottom = top - 3 - (e * 90.0 * (0.6 + noise2(fx * 0.15, fz * 0.15, seed.wrapping_add(502)) * 0.8)) as i32;
    Some((top.min(CH - 12), bottom.max(4)))
}

fn gen_heaven(seed: u32, cx: i32, cz: i32) -> Chunk {
    let mut chunk = Chunk::empty();
    let (x0, z0) = (cx * CS, cz * CS);
    for lz in 0..CS {
        for lx in 0..CS {
            let (x, z) = (x0 + lx, z0 + lz);
            if let Some((top, bottom)) = heaven_island(seed, x, z) {
                for y in bottom..=top {
                    let b = if y == top {
                        HEAVEN_GRASS
                    } else if y > top - 4 {
                        HEAVEN_DIRT
                    } else if hash(x * 13 + y, z, seed.wrapping_add(503)) > 0.985 {
                        HEAVEN_CRYSTAL
                    } else if hash(x, z * 7 + y, seed.wrapping_add(507)) > 0.993 {
                        GLOWSTONE
                    } else {
                        HEAVEN_STONE
                    };
                    chunk.set(lx, y, lz, b);
                }
            }
            // облачные поля
            let (fx, fz) = (x as f32, z as f32);
            let cl = fbm2(fx * 0.02, fz * 0.02, seed.wrapping_add(504), 3);
            if cl > 0.62 {
                let base = 44 + (noise2(fx * 0.03, fz * 0.03, seed.wrapping_add(505)) * 4.0) as i32;
                let th = ((cl - 0.62) * 25.0) as i32 + 1;
                for y in base..base + th.min(4) {
                    if chunk.get(lx, y, lz) == AIR {
                        chunk.set(lx, y, lz, CLOUD);
                    }
                }
            }
        }
    }
    let mut g = Gen { c: &mut chunk, x0, z0 };
    const M: i32 = 3;
    for gz in (z0 - M)..(z0 + CS + M) {
        for gx in (x0 - M)..(x0 + CS + M) {
            let Some((top, _)) = heaven_island(seed, gx, gz) else { continue };
            let inside = gx >= x0 && gx < x0 + CS && gz >= z0 && gz < z0 + CS;
            let r = hash(gx, gz, seed.wrapping_add(506));
            if r < 0.012 {
                let t = hash(gx, gz, 507);
                let len = 4 + (t * 3.0) as i32;
                leaves_blob(&mut g, gx, top + len, gz, GOLDEN_LEAVES, t);
                trunk(&mut g, gx, top, gz, len - 1, LIGHT_LOG);
                if g.inside(gx, top, gz) {
                    g.set(gx, top, gz, HEAVEN_DIRT);
                }
            } else if inside && r < 0.06 {
                let fl = [DAISY, HEAVEN_LILY, CORNFLOWER, TALL_GRASS, HEAVEN_LILY][(r * 1000.0) as usize % 5];
                g.put(gx, top + 1, gz, fl);
            }
        }
    }
    crate::structures::place(&mut g, Dim::Heaven, seed);
    chunk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_land_biomes_exist() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..4000 {
            let (x, z) = ((i % 63) * 97 - 3000, (i / 63) * 97 - 3000);
            seen.insert(column(42, x, z).biome);
        }
        assert!(seen.contains(&Biome::Meadow));
        assert!(seen.contains(&Biome::Desert));
        assert!(seen.contains(&Biome::Taiga));
        assert!(seen.contains(&Biome::Ocean));
    }

    #[test]
    fn mineshafts_carve_beams() {
        let mut beams = 0;
        for cz in -6..6 {
            for cx in -6..6 {
                let c = generate(Dim::Overworld, 7, cx, cz);
                beams += c.blocks.iter().filter(|&&b| b == BEAM).count();
            }
        }
        assert!(beams > 20, "beams = {beams}");
    }

    #[test]
    fn heaven_spawn_island_exists() {
        assert!(heaven_island(3, 0, 0).is_some());
        let c = generate(Dim::Heaven, 3, 0, 0);
        assert!(c.blocks.iter().any(|&b| b == HEAVEN_GRASS));
    }
}

#[cfg(test)]
mod probe {
    use super::*;
    #[test]
    #[ignore]
    fn print_biome_locations() {
        let seed = crate::hash_seed("2024");
        for b in [Biome::Desert, Biome::Taiga, Biome::Meadow] {
            'o: for r in (0..6000).step_by(40) {
                for i in 0..(r / 20).max(1) {
                    let a = i as f32 / (r / 20).max(1) as f32 * 6.283;
                    let (x, z) = ((a.cos() * r as f32) as i32, (a.sin() * r as f32) as i32);
                    let c = column(seed, x, z);
                    if c.biome == b && c.height > SEA + 3 && column(seed, x + 40, z).biome == b && column(seed, x - 40, z).biome == b {
                        println!("{:?} {} {} h={}", b, x, z, c.height);
                        break 'o;
                    }
                }
            }
        }
        let (sx, sz) = find_spawn(seed);
        println!("spawn {sx} {sz}");
        // шахта рядом со спавном
        for rz in -3..3 {
            for rx in -3..3 {
                if let Some((_, (ox, oy, oz))) = shaft_segments(seed, rx, rz) {
                    println!("shaft {ox} {oy} {oz} surface {}", column(seed, ox, oz).height);
                }
            }
        }
    }
}

#[cfg(test)]
mod dungeon_find {
    #[test]
    #[ignore]
    fn find() {
        let seed = crate::hash_seed("2024");
        for rz in -3..3 {
            for rx in -3..3 {
                if super::hash(rx, rz, seed.wrapping_add(801)) <= 0.3 {
                    println!("DUNGEON at x={} z={} y=20", rx * 112 + 16, rz * 112 + 16);
                }
            }
        }
    }
}
