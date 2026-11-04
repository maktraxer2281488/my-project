//! Процедурные текстуры: каждая плитка 16×16 рисуется кодом при запуске.
//! Никаких сторонних картинок — весь стиль игры задаётся здесь.

pub const TILE: usize = 16;
pub const ATLAS_TILES: usize = 4;
pub const ATLAS: usize = TILE * ATLAS_TILES;

pub const T_GRASS_TOP: u8 = 0;
pub const T_GRASS_SIDE: u8 = 1;
pub const T_DIRT: u8 = 2;
pub const T_STONE: u8 = 3;
pub const T_SAND: u8 = 4;
pub const T_LOG_SIDE: u8 = 5;
pub const T_LOG_TOP: u8 = 6;
pub const T_LEAVES: u8 = 7;
pub const T_WATER: u8 = 8;
pub const T_PLANKS: u8 = 9;
pub const T_BRICKS: u8 = 10;
pub const T_SNOW: u8 = 11;
pub const T_COBBLE: u8 = 12;
pub const T_CRYSTAL: u8 = 13;
pub const T_GLASS: u8 = 14;
pub const T_SNOW_SIDE: u8 = 15;

/// Быстрый детерминированный хеш → [0, 1].
pub fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_215.0
}

type Px = [u8; 4];

const fn rgb(r: u8, g: u8, b: u8) -> Px {
    [r, g, b, 255]
}

fn shade(c: Px, k: f32) -> Px {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

fn mix(a: Px, b: Px, t: f32) -> Px {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).clamp(0.0, 255.0) as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])]
}

/// Крупнозернистый шум — пятна размером 2×2 пикселя.
fn blotch(x: i32, y: i32, seed: u32) -> f32 {
    0.6 * hash(x.div_euclid(2), y.div_euclid(2), seed) + 0.4 * hash(x, y, seed ^ 0x9e37)
}

fn dirt(x: i32, y: i32) -> Px {
    let base = rgb(134, 94, 62);
    let mut c = shade(base, 0.78 + blotch(x, y, 11) * 0.34);
    if hash(x, y, 12) > 0.94 {
        c = rgb(168, 142, 112); // камешки
    }
    c
}

fn grass(x: i32, y: i32) -> Px {
    let base = rgb(104, 172, 74);
    shade(base, 0.8 + blotch(x, y, 21) * 0.32)
}

fn stone(x: i32, y: i32, seed: u32) -> Px {
    let base = rgb(124, 130, 140);
    let mut c = shade(base, 0.8 + blotch(x, y, seed) * 0.3);
    // тонкие трещины
    let crack = ((x * 3 + y * 5 + (hash(y, 0, seed) * 6.0) as i32) % 13 == 0) && hash(x, y, seed + 1) > 0.4;
    if crack {
        c = shade(c, 0.72);
    }
    c
}

/// Ячеистый шум (Вороной) с заворачиванием на 16 пикселей — для булыжника.
fn cells(x: i32, y: i32, seed: u32) -> (f32, f32, i32) {
    const N: i32 = 4; // 4×4 ячейки по 4 пикселя
    const S: f32 = (TILE as i32 / N) as f32;
    let (cx, cy) = (x / (TILE as i32 / N), y / (TILE as i32 / N));
    let (mut d1, mut d2, mut id) = (f32::MAX, f32::MAX, 0);
    for oy in -1..=1 {
        for ox in -1..=1 {
            let (gx, gy) = (cx + ox, cy + oy);
            let (wx, wy) = (gx.rem_euclid(N), gy.rem_euclid(N));
            let px = gx as f32 * S + 0.6 + hash(wx, wy, seed) * (S - 1.2);
            let py = gy as f32 * S + 0.6 + hash(wx, wy, seed + 7) * (S - 1.2);
            let d = ((x as f32 + 0.5 - px).powi(2) + (y as f32 + 0.5 - py).powi(2)).sqrt();
            if d < d1 {
                d2 = d1;
                d1 = d;
                id = wy * N + wx;
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (d1, d2, id)
}

fn pixel(tile: u8, x: i32, y: i32) -> Px {
    let n = hash(x, y, tile as u32 * 31 + 7);
    match tile {
        T_GRASS_TOP => {
            let mut c = grass(x, y);
            let f = hash(x, y, 999);
            if f > 0.985 {
                // полевые цветы — фирменная деталь
                c = if hash(x, y, 5) > 0.5 { rgb(252, 222, 92) } else { rgb(214, 206, 238) };
            }
            c
        }
        T_GRASS_SIDE => {
            let depth = 3 + (hash(x, 0, 77) * 3.0) as i32;
            if y < depth {
                grass(x, y)
            } else if y == depth && hash(x, y, 3) > 0.45 {
                shade(grass(x, y), 0.75)
            } else {
                dirt(x, y)
            }
        }
        T_DIRT => dirt(x, y),
        T_STONE => stone(x, y, 31),
        T_SAND => {
            let mut c = shade(rgb(230, 210, 156), 0.92 + n * 0.12);
            if hash(x, y, 41) > 0.93 {
                c = rgb(196, 170, 120);
            }
            c
        }
        T_LOG_SIDE => {
            let stripe = hash(x, 0, 21);
            let mut c = shade(rgb(112, 80, 50), 0.68 + stripe * 0.42 + n * 0.08);
            if (x == 4 || x == 11) && (y % 7 == 2) {
                c = rgb(70, 48, 30); // сучки
            }
            c
        }
        T_LOG_TOP => {
            let d = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
            if d > 6.8 {
                shade(rgb(112, 80, 50), 0.8 + n * 0.2)
            } else {
                let ring = 0.86 + 0.14 * (d * 2.3).sin();
                shade(rgb(202, 160, 104), ring + n * 0.05)
            }
        }
        T_LEAVES => {
            if n < 0.16 {
                [0, 0, 0, 0]
            } else {
                let k = 0.62 + hash(x.div_euclid(2), y.div_euclid(2), 61) * 0.45;
                let mut c = shade(rgb(64, 140, 70), k);
                if hash(x, y, 62) > 0.96 {
                    c = rgb(232, 120, 96); // ягоды
                }
                c
            }
        }
        T_WATER => {
            let wave = ((x + (y / 4) * 5) % 8 == 0 || (x + (y / 4) * 5) % 8 == 1) && y % 4 == 1;
            let c = shade(rgb(54, 122, 204), 0.9 + n * 0.1);
            if wave { mix(c, rgb(170, 214, 255), 0.6) } else { c }
        }
        T_PLANKS => {
            let board = y / 4;
            let base = rgb(190, 144, 90);
            let mut c = shade(base, 0.82 + hash(x / 3, board, 91) * 0.16 + n * 0.06);
            let seam = (board * 5 + 3) % 16;
            if y % 4 == 3 || x == seam {
                c = shade(base, 0.58);
            } else if (x == (seam + 2) % 16 || x == (seam + 14) % 16) && y % 4 == 1 {
                c = rgb(96, 84, 74); // гвоздики
            }
            c
        }
        T_BRICKS => {
            let row = y / 4;
            let off = if row % 2 == 1 { 4 } else { 0 };
            if y % 4 == 3 || (x + off) % 8 == 7 {
                shade(rgb(206, 198, 186), 0.92 + n * 0.08)
            } else {
                let tone = hash((x + off) / 8, row, 101);
                shade(rgb(176, 78, 56), 0.82 + tone * 0.22 + n * 0.08)
            }
        }
        T_SNOW => {
            if hash(x, y, 111) > 0.97 {
                rgb(255, 255, 255)
            } else {
                shade(rgb(232, 240, 250), 0.95 + n * 0.05)
            }
        }
        T_SNOW_SIDE => {
            let depth = 2 + (hash(x, 0, 113) * 3.0) as i32;
            if y < depth { shade(rgb(232, 240, 250), 0.95 + n * 0.05) } else { dirt(x, y) }
        }
        T_COBBLE => {
            let (d1, d2, id) = cells(x, y, 121);
            if d2 - d1 < 0.85 {
                shade(rgb(82, 86, 94), 0.9 + n * 0.1)
            } else {
                let k = 0.82 + hash(id, 0, 122) * 0.3 - d1 * 0.04;
                shade(rgb(140, 144, 152), k + n * 0.06)
            }
        }
        T_CRYSTAL => {
            let seeds = [(4.0, 4.0, 2.2), (11.0, 6.0, 1.8), (7.0, 12.0, 2.0)];
            for (sx, sy, r) in seeds {
                let dx = x as f32 + 0.5 - sx;
                let dy = y as f32 + 0.5 - sy;
                if dx.abs() + dy.abs() < r * 1.25 {
                    let hl = if dx + dy < -0.4 { 1.3 } else { 1.0 };
                    return shade(mix(rgb(146, 92, 236), rgb(84, 214, 232), (sx / 16.0) as f32), hl);
                }
            }
            stone(x, y, 131)
        }
        T_GLASS => {
            let edge = x == 0 || y == 0 || x == 15 || y == 15;
            let glint = (x + y == 9 || x + y == 10) && (3..=7).contains(&x);
            if edge {
                rgb(196, 226, 238)
            } else if glint {
                [250, 254, 255, 255]
            } else {
                [0, 0, 0, 0]
            }
        }
        _ => rgb(255, 0, 255),
    }
}

/// Собирает атлас 64×64 (RGBA8).
pub fn build_atlas() -> Vec<u8> {
    let mut img = vec![0u8; ATLAS * ATLAS * 4];
    for tile in 0..(ATLAS_TILES * ATLAS_TILES) as u8 {
        let (tx, ty) = (tile as usize % ATLAS_TILES, tile as usize / ATLAS_TILES);
        for y in 0..TILE {
            for x in 0..TILE {
                let c = pixel(tile, x as i32, y as i32);
                let i = ((ty * TILE + y) * ATLAS + tx * TILE + x) * 4;
                img[i..i + 4].copy_from_slice(&c);
            }
        }
    }
    img
}

/// UV-прямоугольник плитки в атласе: (u0, v0, u1, v1), с небольшим отступом от краёв.
pub fn tile_uv(tile: u8) -> (f32, f32, f32, f32) {
    let (tx, ty) = ((tile as usize % ATLAS_TILES) as f32, (tile as usize / ATLAS_TILES) as f32);
    let s = 1.0 / ATLAS_TILES as f32;
    let e = 0.02 / ATLAS as f32;
    (tx * s + e, ty * s + e, (tx + 1.0) * s - e, (ty + 1.0) * s - e)
}
