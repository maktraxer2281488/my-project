//! Процедурные «шкуры» существ: тело (мех, перья, чешуя, хитин, ткань, камень…),
//! морда с глазами и носом/клювом, лапы. По три плитки атласа на каждое существо.

use crate::blocks::hash;
use crate::mobdefs::{GLOW, MOBS, MobDef, Plan};

type Px = [u8; 4];

fn c(t: (u8, u8, u8)) -> Px {
    [t.0, t.1, t.2, 255]
}

fn shade(p: Px, k: f32) -> Px {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    [f(p[0]), f(p[1]), f(p[2]), p[3]]
}

fn mix(a: Px, b: Px, t: f32) -> Px {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).clamp(0.0, 255.0) as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), 255]
}

#[derive(Clone, Copy, PartialEq)]
enum Mat {
    Fur,
    Wool,
    Feathers,
    Scales,
    Chitin,
    Slime,
    Glow,
    Stone,
    Cloth,
    Bone,
    Bandage,
}

fn material(i: usize, d: &MobDef) -> Mat {
    let n = d.name;
    if n.contains("Костяк") || n.contains("скелет") {
        return Mat::Bone;
    }
    if n.contains("Мумия") {
        return Mat::Bandage;
    }
    if n.contains("овечка") || n.contains("Пушистик") || n.contains("барашек") || n.contains("йети") {
        return Mat::Wool;
    }
    let _ = i;
    match d.plan {
        Plan::Quad => Mat::Fur,
        Plan::Bird => Mat::Feathers,
        Plan::Fish => Mat::Scales,
        Plan::Spider => Mat::Chitin,
        Plan::Slime => Mat::Slime,
        Plan::Wisp => Mat::Glow,
        Plan::Golem => Mat::Stone,
        Plan::Biped => Mat::Cloth,
    }
}

fn pattern(m: Mat, base: Px, alt: Px, x: i32, y: i32, seed: u32) -> Px {
    let n = hash(x, y, seed);
    let n2 = hash(x / 2, y / 2, seed + 1);
    match m {
        Mat::Fur => {
            // пряди шерсти: вертикальные штрихи
            let strand = hash(x, y / 3, seed + 2);
            let k = 0.78 + strand * 0.3 + n * 0.08;
            let p = shade(base, k);
            if (seed % 3 == 0) && hash(x / 4, y / 4, seed + 5) > 0.72 { mix(p, alt, 0.7) } else { p }
        }
        Mat::Wool => {
            let curl = ((x as f32 * 1.3).sin() + (y as f32 * 1.1).cos()) * 0.5;
            shade(base, 0.86 + curl * 0.1 + n * 0.08)
        }
        Mat::Feathers => {
            let row = y / 3;
            let off = if row % 2 == 0 { 0 } else { 2 };
            let fx = (x + off) % 4;
            let edge = fx == 0 || y % 3 == 2;
            let tip = y % 3 == 2 && fx == 2;
            if tip { shade(alt, 1.1) } else if edge { shade(base, 0.72) } else { shade(base, 0.92 + n * 0.12) }
        }
        Mat::Scales => {
            let row = y / 2;
            let off = if row % 2 == 0 { 0 } else { 1 };
            let edge = (x + off) % 3 == 0 || y % 2 == 1;
            let p = if edge { shade(base, 0.7) } else { shade(base, 1.0 + n * 0.15) };
            if y < 4 { mix(p, alt, 0.5) } else if y > 12 { mix(p, [235, 235, 225, 255], 0.35) } else { p }
        }
        Mat::Chitin => {
            let seg = y % 5 == 0;
            let gloss = (x + y) % 7 == 0;
            if seg { shade(base, 0.55) } else if gloss { shade(base, 1.45) } else if hash(x, y, seed + 3) > 0.93 { alt } else { shade(base, 0.85 + n * 0.15) }
        }
        Mat::Slime => {
            let r = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
            let bubble = hash(x / 3, y / 3, seed + 4) > 0.8 && (x % 3 == 1) && (y % 3 == 1);
            if bubble { shade(base, 1.4) } else { shade(base, 1.15 - r * 0.03 + n * 0.06) }
        }
        Mat::Glow => {
            let r = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
            mix(alt, base, (r / 9.0).min(1.0)).map_or_px(n)
        }
        Mat::Stone => {
            let (cx, cy) = (x / 5, y / 4);
            let crack = x % 5 == 0 || y % 4 == 0;
            if crack { shade(base, 0.6) } else { shade(base, 0.85 + hash(cx, cy, seed + 6) * 0.25 + n * 0.06) }
        }
        Mat::Cloth => {
            let weave = if (x + y) % 2 == 0 { 1.0 } else { 0.93 };
            let belt = y == 10 || y == 11;
            if belt {
                if x == 7 || x == 8 { [220, 190, 80, 255] } else { shade(alt, 0.8) }
            } else if y < 10 && x == 7 && y % 3 == 1 {
                shade(alt, 1.2)
            } else {
                shade(base, weave * (0.9 + n2 * 0.12))
            }
        }
        Mat::Bone => {
            let rib = y % 3 == 0 && x > 2 && x < 13;
            if rib { shade(base, 0.7) } else if (x == 7 || x == 8) && y > 1 { shade(base, 0.85) } else if hash(x, y, seed + 7) > 0.85 { shade(base, 0.6) } else { shade(base, 0.95 + n * 0.08) }
        }
        Mat::Bandage => {
            let band = (y + x / 4) % 3 == 0;
            if band { shade(base, 0.78) } else if hash(x, y, seed + 8) > 0.9 { shade(alt, 0.8) } else { shade(base, 0.95 + n * 0.08) }
        }
    }
}

trait OrPx {
    fn map_or_px(self, n: f32) -> Px;
}
impl OrPx for Px {
    fn map_or_px(self, n: f32) -> Px {
        shade(self, 0.95 + n * 0.1)
    }
}

fn face(i: usize, d: &MobDef, m: Mat, x: i32, y: i32) -> Px {
    let base = c(d.c1);
    let alt = c(d.c2);
    let eye = c(d.c3);
    let seed = 7000 + i as u32 * 13;
    let skin = pattern(m, base, alt, x, y, seed);
    let glow = d.flags & GLOW != 0;
    let dark = [20, 18, 22, 255];
    let white = [245, 245, 240, 255];
    match d.plan {
        Plan::Spider => {
            // четыре пары глаз
            for (ex, ey) in [(4, 5), (11, 5), (6, 3), (9, 3), (3, 8), (12, 8)] {
                if (x - ex).abs() <= 0 && (y - ey).abs() <= 0 || (x == ex + 1 && y == ey) {
                    return if x == ex { eye } else { shade(eye, 0.6) };
                }
            }
            if y >= 11 && (x == 6 || x == 9) { return shade(alt, 0.7) }
            skin
        }
        Plan::Bird => {
            let eye_px = (y == 5 || y == 6) && (x == 3 || x == 4 || x == 11 || x == 12);
            if eye_px {
                return if x == 4 || x == 11 { dark } else { if y == 5 { white } else { dark } };
            }
            // клюв
            if y >= 7 && y <= 10 && (x - 7).abs() + (y - 7) / 2 <= 2 && (x >= 6 && x <= 9) {
                return if y == 10 { shade(eye, 0.7) } else { eye };
            }
            skin
        }
        Plan::Wisp | Plan::Slime => {
            let ex = [4, 11];
            for e in ex {
                if (x == e || x == e + 1 - 2 * (e > 7) as i32) && (y == 6 || y == 7) {
                    return if glow { eye } else { dark };
                }
            }
            if y == 11 && (5..=10).contains(&x) && d.plan == Plan::Slime {
                return shade(base, 0.45);
            }
            skin
        }
        _ => {
            // глаза: белок + зрачок (+свечение)
            for ex in [4, 10] {
                if (x == ex || x == ex + 1) && (y == 6 || y == 7) {
                    if glow {
                        return if y == 6 { shade(eye, 1.3) } else { eye };
                    }
                    return if x == ex + 1 && y == 7 { dark } else if x == ex && y == 6 { white } else { eye };
                }
            }
            // брови/тень над глазами
            if y == 5 && ((4..=5).contains(&x) || (10..=11).contains(&x)) {
                return shade(skin, 0.7);
            }
            match d.plan {
                Plan::Quad => {
                    // морда-нос
                    if y >= 9 && y <= 13 && (5..=10).contains(&x) {
                        let nose = alt;
                        if y == 10 && (x == 6 || x == 9) {
                            return dark;
                        }
                        if y == 13 && (6..=9).contains(&x) {
                            return shade(nose, 0.6);
                        }
                        return shade(nose, 0.95 + hash(x, y, seed) * 0.08);
                    }
                }
                Plan::Biped => {
                    if y <= 2 {
                        return shade(alt, 0.8 + hash(x, y, seed) * 0.2); // волосы / капюшон
                    }
                    if y == 9 && (7..=8).contains(&x) {
                        return shade(skin, 0.75); // нос
                    }
                    if y == 11 && (6..=9).contains(&x) {
                        return shade(skin, 0.55); // рот
                    }
                }
                Plan::Golem => {
                    if y == 11 && (4..=11).contains(&x) {
                        return if glow { eye } else { shade(base, 0.45) };
                    }
                }
                _ => {}
            }
            skin
        }
    }
}

/// Создаёт плитки: [тело, морда, лапы] для каждого существа.
pub fn build(add: &mut dyn FnMut(Box<dyn Fn(i32, i32) -> [u8; 4]>) -> u16) -> Vec<[u16; 3]> {
    let mut out = Vec::with_capacity(MOBS.len());
    for (i, d) in MOBS.iter().enumerate() {
        let m = material(i, d);
        let (base, alt) = (c(d.c1), c(d.c2));
        let seed = 9000 + i as u32 * 17;
        let body = add(Box::new(move |x, y| pattern(m, base, alt, x, y, seed)));
        let dd = *d;
        let head = add(Box::new(move |x, y| face(i, &dd, m, x, y)));
        let limb_m = match m {
            Mat::Feathers => Mat::Scales,
            Mat::Wool => Mat::Fur,
            other => other,
        };
        let limb = add(Box::new(move |x, y| shade(pattern(limb_m, alt, base, x, y, seed + 3), 0.95)));
        out.push([body, head, limb]);
    }
    out
}
