//! Построение геометрии чанка: освещение (небо + светящиеся блоки),
//! мягкие тени в углах, отсечение невидимых граней.

use crate::blocks::{self, Shape, *};
use crate::world::{CH, CS, Chunk};
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct V {
    pub pos: [f32; 3],
    /// Координаты внутри грани (в блоках): текстура повторяется по объединённой грани.
    pub uv: [f32; 2],
    /// Левый верхний угол плитки в атласе.
    pub tile: [f32; 2],
    /// r — свет неба, g — свет блоков, b — затенение грани, a — прозрачность.
    pub light: [u8; 4],
}

#[derive(Default)]
pub struct MeshData {
    pub opaque_v: Vec<V>,
    pub opaque_i: Vec<u32>,
    pub cutout_v: Vec<V>,
    pub cutout_i: Vec<u32>,
    pub water_v: Vec<V>,
    pub water_i: Vec<u32>,
    /// Полностью тёмные грани (глубокие пещеры): рисуются, только когда игрок под землёй или рядом.
    pub dark_v: Vec<V>,
    pub dark_i: Vec<u32>,
    /// Мелкие растения: рисуются только вблизи.
    pub detail_v: Vec<V>,
    pub detail_i: Vec<u32>,
}

/// Центральный чанк и его 8 соседей.
pub struct Nbhd {
    pub c: [Arc<Chunk>; 9],
}

impl Nbhd {
    #[inline]
    fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if y < 0 {
            return BEDROCK;
        }
        if y >= CH {
            return AIR;
        }
        let (dx, dz) = (x.div_euclid(CS), z.div_euclid(CS));
        let c = &self.c[((dz + 1) * 3 + (dx + 1)) as usize];
        c.get(x - dx * CS, y, z - dz * CS)
    }
}

const LM: i32 = 14;
const RW: i32 = CS + 2 * LM;

fn is_leaves(b: u8) -> bool {
    matches!(b, OAK_LEAVES | SPRUCE_LEAVES | BIRCH_LEAVES | SAKURA_LEAVES | PALM_LEAVES | GOLDEN_LEAVES)
}

struct Light {
    sky: Vec<u8>,
    blk: Vec<u8>,
    top: i32,
}

impl Light {
    #[inline]
    fn idx(x: i32, y: i32, z: i32) -> usize {
        ((y * RW + (z + LM)) * RW + (x + LM)) as usize
    }
    #[inline]
    fn at(&self, x: i32, y: i32, z: i32) -> (u8, u8) {
        if y > self.top || y >= CH {
            return (15, 0);
        }
        if y < 0 || x < -LM || z < -LM || x >= CS + LM || z >= CS + LM {
            return (0, 0);
        }
        let i = Self::idx(x, y, z);
        (self.sky[i], self.blk[i])
    }
}

fn compute_light(n: &Nbhd) -> Light {
    let reg = blocks::reg();
    // самая высокая непустая клетка в области
    let mut top = 0;
    for z in -LM..CS + LM {
        for x in -LM..CS + LM {
            for y in (top + 1..CH).rev() {
                if n.get(x, y, z) != AIR {
                    top = top.max(y);
                    break;
                }
            }
        }
    }
    let top = (top + 1).min(CH - 1);
    let size = (RW * RW * (top + 1)) as usize;
    let mut sky = vec![0u8; size];
    let mut blk = vec![0u8; size];
    let mut opq = vec![false; size];
    let mut q: Vec<u32> = Vec::with_capacity(size / 8);
    let mut sky_q: Vec<u32> = Vec::with_capacity(size / 8);
    // нижняя граница полного света неба в каждой колонке
    let mut bottom = vec![0i32; (RW * RW) as usize];

    for z in -LM..CS + LM {
        for x in -LM..CS + LM {
            let mut lvl: u8 = 15;
            let ci = ((z + LM) * RW + (x + LM)) as usize;
            bottom[ci] = top + 1;
            for y in (0..=top).rev() {
                let b = n.get(x, y, z);
                let i = Light::idx(x, y, z);
                if reg.opaque[b as usize] {
                    opq[i] = true;
                    lvl = 0;
                } else if is_leaves(b) || b == WATER || b == ICE {
                    lvl = lvl.saturating_sub(2);
                }
                sky[i] = lvl;
                if lvl == 15 {
                    bottom[ci] = y;
                } else if lvl > 1 {
                    sky_q.push(i as u32);
                }
                let e = reg.defs[b as usize].light;
                if e > 0 {
                    blk[i] = e;
                    q.push(i as u32);
                }
            }
        }
    }

    let spread = |arr: &mut Vec<u8>, q: &mut Vec<u32>| {
        let mut head = 0;
        while head < q.len() {
            let i = q[head] as i32;
            head += 1;
            let l = arr[i as usize];
            if l <= 1 {
                continue;
            }
            let x = i % RW;
            let z = (i / RW) % RW;
            let y = i / (RW * RW);
            let mut try_n = |ni: i32| {
                let ni = ni as usize;
                if !opq[ni] && arr[ni] + 1 < l {
                    arr[ni] = l - 1;
                    q.push(ni as u32);
                }
            };
            if x > 0 {
                try_n(i - 1);
            }
            if x < RW - 1 {
                try_n(i + 1);
            }
            if z > 0 {
                try_n(i - RW);
            }
            if z < RW - 1 {
                try_n(i + RW);
            }
            if y > 0 {
                try_n(i - RW * RW);
            }
            if y < top {
                try_n(i + RW * RW);
            }
        }
    };
    spread(&mut blk, &mut q);

    // свет неба растекается под навесы и в пещеры: начинаем только с «краёв»
    // полностью освещённых колонок, а не со всего воздуха над землёй
    q.clear();
    q.extend_from_slice(&sky_q);
    for zi in 0..RW {
        for xi in 0..RW {
            let ci = (zi * RW + xi) as usize;
            let b0 = bottom[ci];
            let mut ymax = b0;
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, nz) = (xi + dx, zi + dz);
                if (0..RW).contains(&nx) && (0..RW).contains(&nz) {
                    ymax = ymax.max(bottom[(nz * RW + nx) as usize]);
                }
            }
            for y in b0..=ymax.min(top) {
                let i = ((y * RW + zi) * RW + xi) as u32;
                if sky[i as usize] == 15 {
                    q.push(i);
                }
            }
        }
    }
    spread(&mut sky, &mut q);

    Light { sky, blk, top }
}

/// (нормаль, углы BL-BR-TR-TL, яркость грани)
const FACES: [([i32; 3], [[f32; 3]; 4], f32); 6] = [
    ([1, 0, 0], [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]], 0.80),
    ([-1, 0, 0], [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]], 0.80),
    ([0, 1, 0], [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]], 1.00),
    ([0, -1, 0], [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]], 0.55),
    ([0, 0, 1], [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]], 0.68),
    ([0, 0, -1], [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]], 0.68),
];
const AO_LEVELS: [f32; 4] = [0.45, 0.64, 0.82, 1.0];

fn face_visible(b: u8, nb: u8) -> bool {
    let reg = blocks::reg();
    if reg.opaque[nb as usize] {
        return false;
    }
    match reg.defs[b as usize].shape {
        Shape::Liquid => nb != b && reg.defs[nb as usize].shape != Shape::Liquid,
        Shape::Cutout if !is_leaves(b) => nb != b,
        // внутренние грани листвы не нужны — это главный источник лишней геометрии
        Shape::Cutout => !is_leaves(nb),
        _ => true,
    }
}

fn push_quad(v: &mut Vec<V>, idx: &mut Vec<u32>, q: [V; 4], flip: bool) {
    let b = v.len() as u32;
    v.extend_from_slice(&q);
    if flip {
        idx.extend_from_slice(&[b + 1, b + 2, b + 3, b + 1, b + 3, b]);
    } else {
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
}

#[inline]
fn lb(level: f32) -> u8 {
    (level / 15.0 * 255.0).round().clamp(0.0, 255.0) as u8
}

fn tile_origin(t: u16) -> [f32; 2] {
    let n = ATLAS_TILES as f32;
    [(t as usize % ATLAS_TILES) as f32 / n, (t as usize / ATLAS_TILES) as f32 / n]
}

/// Куда кладётся грань.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Opaque,
    Cutout,
    Water,
}

/// Грань, пригодная к объединению: одинаковые плитка, свет и затенение во всех углах.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    tile: u16,
    light: [u8; 4],
    pass: Pass,
    lowered: bool,
}

const SX: usize = CS as usize;
const SY: usize = CH as usize;

fn axis_size(a: usize) -> i32 {
    if a == 1 { CH } else { CS }
}

pub fn build(n: &Nbhd, smooth: bool) -> MeshData {
    let reg = blocks::reg();
    let light = compute_light(n);
    let mut out = MeshData::default();
    // объединяемые грани по направлениям: [dir][x][y][z]
    let mut mask: Vec<Option<Key>> = vec![None; 6 * SX * SY * SX];
    let mi = |dir: usize, x: i32, y: i32, z: i32| ((dir * SX + x as usize) * SY + y as usize) * SX + z as usize;

    for y in 0..CH {
        for z in 0..CS {
            for x in 0..CS {
                let b = n.get(x, y, z);
                if b == AIR {
                    continue;
                }
                let d = &reg.defs[b as usize];
                if d.shape == Shape::Cross {
                    let (s, l) = light.at(x, y, z);
                    let org = tile_origin(d.tex[1]);
                    let lt = [lb(s as f32), lb(l as f32), 230, 255];
                    let top_lt = [lt[0], lt[1], lt[2], 250];
                    let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                    let small = matches!(b, TALL_GRASS | FERN | DEAD_BUSH);
                    for (a, c) in [([0.15, 0.15], [0.85, 0.85]), ([0.85, 0.15], [0.15, 0.85])] {
                        let q = [
                            V { pos: [fx + a[0], fy, fz + a[1]], uv: [0.0, 1.0], tile: org, light: lt },
                            V { pos: [fx + c[0], fy, fz + c[1]], uv: [1.0, 1.0], tile: org, light: lt },
                            V { pos: [fx + c[0], fy + 1.0, fz + c[1]], uv: [1.0, 0.0], tile: org, light: top_lt },
                            V { pos: [fx + a[0], fy + 1.0, fz + a[1]], uv: [0.0, 0.0], tile: org, light: top_lt },
                        ];
                        if small {
                            push_quad(&mut out.detail_v, &mut out.detail_i, q, false);
                        } else {
                            push_quad(&mut out.cutout_v, &mut out.cutout_i, q, false);
                        }
                    }
                    continue;
                }
                let liquid = d.shape == Shape::Liquid;
                let lowered = liquid && n.get(x, y + 1, z) != b;
                let to_water = b == WATER;
                let leaves = is_leaves(b);
                let pass = if to_water {
                    Pass::Water
                } else if d.shape == Shape::Cutout {
                    Pass::Cutout
                } else {
                    Pass::Opaque
                };

                for (fi, (nrm, corners, shade)) in FACES.iter().enumerate() {
                    let nb = n.get(x + nrm[0], y + nrm[1], z + nrm[2]);
                    if !face_visible(b, nb) {
                        continue;
                    }
                    let tile = match fi {
                        2 => d.tex[0],
                        3 => d.tex[2],
                        _ => d.tex[1],
                    };
                    let na = nrm.iter().position(|&v| v != 0).unwrap();
                    let (ta, tb) = match na {
                        0 => (1, 2),
                        1 => (0, 2),
                        _ => (0, 1),
                    };
                    let p = [x + nrm[0], y + nrm[1], z + nrm[2]];
                    let mut lights = [[0u8; 4]; 4];
                    let mut ao = [3usize; 4];
                    for (i, c) in corners.iter().enumerate() {
                        let (mut sl, mut bl) = (0.0f32, 0.0f32);
                        if smooth && !liquid && !leaves {
                            let sa = if c[ta] > 0.5 { 1 } else { -1 };
                            let sb = if c[tb] > 0.5 { 1 } else { -1 };
                            let mut q1 = p;
                            q1[ta] += sa;
                            let mut q2 = p;
                            q2[tb] += sb;
                            let mut q3 = q1;
                            q3[tb] += sb;
                            let o1 = reg.opaque[n.get(q1[0], q1[1], q1[2]) as usize];
                            let o2 = reg.opaque[n.get(q2[0], q2[1], q2[2]) as usize];
                            let o3 = reg.opaque[n.get(q3[0], q3[1], q3[2]) as usize];
                            ao[i] = if o1 && o2 { 0 } else { 3 - (o1 as usize + o2 as usize + o3 as usize) };
                            let mut cnt = 0.0;
                            for (cell, opq) in [(p, false), (q1, o1), (q2, o2), (q3, o3 || (o1 && o2))] {
                                if !opq {
                                    let (s, l) = light.at(cell[0], cell[1], cell[2]);
                                    sl += s as f32;
                                    bl += l as f32;
                                    cnt += 1.0;
                                }
                            }
                            sl /= cnt;
                            bl /= cnt;
                        } else {
                            let (s, l) = light.at(p[0], p[1], p[2]);
                            let (s2, l2) = if liquid { light.at(x, y, z) } else { (0, 0) };
                            sl = s.max(s2) as f32;
                            bl = l.max(l2) as f32;
                        }
                        let k = shade * AO_LEVELS[ao[i]];
                        let alpha = if to_water {
                            if lowered && c[1] > 0.5 { 183 } else { 180 }
                        } else if leaves {
                            251
                        } else {
                            255
                        };
                        lights[i] = [lb(sl), lb(bl), (k * 255.0) as u8, alpha];
                    }
                    let uniform = lights.iter().all(|l| *l == lights[0]);
                    let mergeable = uniform && !(liquid && fi != 2);
                    if mergeable {
                        mask[mi(fi, x, y, z)] = Some(Key { tile, light: lights[0], pass, lowered });
                        continue;
                    }
                    // одиночная грань
                    let org = tile_origin(tile);
                    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
                    let mut q = [V { pos: [0.0; 3], uv: [0.0; 2], tile: org, light: [0; 4] }; 4];
                    for (i, c) in corners.iter().enumerate() {
                        let mut cy = c[1];
                        if lowered && cy > 0.5 {
                            cy = 0.875;
                        }
                        q[i] = V { pos: [x as f32 + c[0], y as f32 + cy, z as f32 + c[2]], uv: uvs[i], tile: org, light: lights[i] };
                    }
                    let flip = ao[0] + ao[2] < ao[1] + ao[3];
                    let dark = pass == Pass::Opaque && lights.iter().all(|l| l[0] == 0 && l[1] == 0);
                    let (vv, ii) = match pass {
                        Pass::Water => (&mut out.water_v, &mut out.water_i),
                        Pass::Cutout => (&mut out.cutout_v, &mut out.cutout_i),
                        Pass::Opaque if dark => (&mut out.dark_v, &mut out.dark_i),
                        Pass::Opaque => (&mut out.opaque_v, &mut out.opaque_i),
                    };
                    push_quad(vv, ii, q, flip);
                }
            }
        }
    }

    // жадное объединение одинаковых граней в прямоугольники
    for (fi, (nrm, corners, _)) in FACES.iter().enumerate() {
        let na = nrm.iter().position(|&v| v != 0).unwrap();
        let (ta, tb) = match na {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        // ось текстуры u — где отличаются углы 0 и 1, v — углы 0 и 3
        let uax = (0..3).find(|&a| corners[0][a] != corners[1][a]).unwrap();
        let vax = (0..3).find(|&a| corners[0][a] != corners[3][a]).unwrap();
        let (na_n, ta_n, tb_n) = (axis_size(na), axis_size(ta), axis_size(tb));
        let mut used = vec![false; (ta_n * tb_n) as usize];
        for sl in 0..na_n {
            used.iter_mut().for_each(|u| *u = false);
            let at = |a: i32, b: i32| -> Option<Key> {
                let mut c = [0i32; 3];
                c[na] = sl;
                c[ta] = a;
                c[tb] = b;
                mask[mi(fi, c[0], c[1], c[2])]
            };
            for b0 in 0..tb_n {
                for a0 in 0..ta_n {
                    if used[(b0 * ta_n + a0) as usize] {
                        continue;
                    }
                    let Some(key) = at(a0, b0) else { continue };
                    let max = if key.pass == Pass::Water || key.light[3] == 251 { 4 } else { 16 };
                    let mut w = 1;
                    while a0 + w < ta_n && w < max && !used[(b0 * ta_n + a0 + w) as usize] && at(a0 + w, b0) == Some(key) {
                        w += 1;
                    }
                    let mut h = 1;
                    'grow: while b0 + h < tb_n && h < max {
                        for a in a0..a0 + w {
                            if used[((b0 + h) * ta_n + a) as usize] || at(a, b0 + h) != Some(key) {
                                break 'grow;
                            }
                        }
                        h += 1;
                    }
                    for bb in b0..b0 + h {
                        for aa in a0..a0 + w {
                            used[(bb * ta_n + aa) as usize] = true;
                        }
                    }
                    let org = tile_origin(key.tile);
                    let (eu, ev) = if uax == ta { (w as f32, h as f32) } else { (h as f32, w as f32) };
                    let mut q = [V { pos: [0.0; 3], uv: [0.0; 2], tile: org, light: key.light }; 4];
                    for (i, c) in corners.iter().enumerate() {
                        let mut pos = [0.0f32; 3];
                        pos[na] = sl as f32 + c[na];
                        pos[ta] = a0 as f32 + c[ta] * w as f32;
                        pos[tb] = b0 as f32 + c[tb] * h as f32;
                        if key.lowered && fi == 2 {
                            pos[1] -= 0.125;
                        }
                        let u = if c[uax] == corners[1][uax] { eu } else { 0.0 };
                        let v = if c[vax] == corners[0][vax] { ev } else { 0.0 };
                        q[i] = V { pos, uv: [u, v], tile: org, light: key.light };
                    }
                    let dark = key.light[0] == 0 && key.light[1] == 0;
                    let (vv, ii) = match key.pass {
                        Pass::Water => (&mut out.water_v, &mut out.water_i),
                        Pass::Cutout => (&mut out.cutout_v, &mut out.cutout_i),
                        Pass::Opaque if dark => (&mut out.dark_v, &mut out.dark_i),
                        Pass::Opaque => (&mut out.opaque_v, &mut out.opaque_i),
                    };
                    push_quad(vv, ii, q, false);
                }
            }
        }
    }
    out
}
