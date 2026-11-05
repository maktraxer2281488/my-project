//! Построение геометрии чанка: освещение (небо + светящиеся блоки),
//! мягкие тени в углах, отсечение невидимых граней.

use crate::blocks::{self, Shape, *};
use crate::world::{CH, CS, Chunk};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// «Быстрая листва»: листья рисуются сплошными блоками без прозрачности
/// и закрывают грани соседей (меняется в настройках оптимизации).
pub static FAST_LEAVES: AtomicBool = AtomicBool::new(false);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct V {
    /// Позиция в 1/64 блока (компактно: 8 байт вместо 12).
    pub pos: [u16; 4],
    /// u, v внутри грани (в блоках) и номер плитки атласа (младший, старший байт).
    pub uvt: [u8; 4],
    /// r — свет неба, g — свет блоков, b — затенение грани, a — прозрачность.
    pub light: [u8; 4],
}

impl V {
    #[inline]
    pub fn new(pos: [f32; 3], uv: [f32; 2], tile: u16, light: [u8; 4]) -> V {
        let q = |v: f32| (v * 64.0).round().clamp(0.0, 65535.0) as u16;
        // u, v хранятся в 1/16 блока (до 15.9 блока): так можно задавать половинки граней
        let t = |v: f32| (v * 16.0).round().clamp(0.0, 255.0) as u8;
        V { pos: [q(pos[0]), q(pos[1]), q(pos[2]), 0], uvt: [t(uv[0]), t(uv[1]), (tile & 0xff) as u8, (tile >> 8) as u8], light }
    }
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
    /// Упрощённая копия для дальних расстояний: клетки 2×2×2, без пещер и растений.
    pub lod_v: Vec<V>,
    pub lod_i: Vec<u32>,
    pub lod_cut_v: Vec<V>,
    pub lod_cut_i: Vec<u32>,
    pub lod_water_v: Vec<V>,
    pub lod_water_i: Vec<u32>,
}

/// Центральный чанк и его 8 соседей.
pub struct Nbhd {
    pub c: [Arc<Chunk>; 9],
}

impl Nbhd {
    #[inline]
    fn get(&self, x: i32, y: i32, z: i32) -> u16 {
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

fn face_visible(b: u16, nb: u16, fast: bool) -> bool {
    let reg = blocks::reg();
    if reg.opaque[nb as usize] || (fast && is_leaves(nb) && reg.defs[b as usize].shape != Shape::Liquid) {
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

fn tile_origin(t: u16) -> u16 {
    t
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
    /// Насколько опущена поверхность жидкости (в восьмых блока).
    drop: u8,
}

const SX: usize = CS as usize;
const SY: usize = CH as usize;

fn axis_size(a: usize) -> i32 {
    if a == 1 { CH } else { CS }
}

pub fn build(n: &Nbhd, smooth: bool) -> MeshData {
    let reg = blocks::reg();
    let fast = FAST_LEAVES.load(Ordering::Relaxed);
    let light = compute_light(n);
    let mut out = MeshData::default();
    build_lod(n, &light, &mut out);
    // объединяемые грани по направлениям: [dir][x][y][z]
    let mut mask: Vec<Option<Key>> = vec![None; 6 * SX * SY * SX];
    let mut form_boxes: Vec<crate::shapes::Bx> = Vec::with_capacity(8);
    let mi = |dir: usize, x: i32, y: i32, z: i32| ((dir * SX + x as usize) * SY + y as usize) * SX + z as usize;

    for y in 0..CH {
        for z in 0..CS {
            for x in 0..CS {
                let b = n.get(x, y, z);
                if b == AIR {
                    continue;
                }
                let d = &reg.defs[b as usize];
                if d.shape == Shape::Form {
                    emit_form(n, &light, x, y, z, b, &mut out, &mut form_boxes);
                    continue;
                }
                if d.shape == Shape::Cross {
                    let (s, l) = light.at(x, y, z);
                    let org = tile_origin(d.tex[1]);
                    let lt = [lb(s as f32), lb(l as f32), 230, 255];
                    let top_lt = [lt[0], lt[1], lt[2], 250];
                    let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                    let small = matches!(b, TALL_GRASS | FERN | DEAD_BUSH);
                    for (a, c) in [([0.15, 0.15], [0.85, 0.85]), ([0.85, 0.15], [0.15, 0.85])] {
                        let q = [
                            V::new([fx + a[0], fy, fz + a[1]], [0.0, 1.0], org, lt),
                            V::new([fx + c[0], fy, fz + c[1]], [1.0, 1.0], org, lt),
                            V::new([fx + c[0], fy + 1.0, fz + c[1]], [1.0, 0.0], org, top_lt),
                            V::new([fx + a[0], fy + 1.0, fz + a[1]], [0.0, 0.0], org, top_lt),
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
                // поверхность жидкости ниже, чем дальше от источника
                let fl = fluid(b);
                let drop: u8 = match fl {
                    Some((w, lvl)) if fluid(n.get(x, y + 1, z)).is_none_or(|(w2, _)| w2 != w) => (1 + lvl * if w { 1 } else { 2 }).min(7),
                    _ => 0,
                };
                let lowered = drop > 0;
                let to_water = fl.is_some_and(|(w, _)| w);
                let leaves = is_leaves(b);
                let pass = if to_water {
                    Pass::Water
                } else if leaves && fast {
                    Pass::Opaque
                } else if d.shape == Shape::Cutout {
                    Pass::Cutout
                } else {
                    Pass::Opaque
                };

                for (fi, (nrm, corners, shade)) in FACES.iter().enumerate() {
                    let nb = n.get(x + nrm[0], y + nrm[1], z + nrm[2]);
                    if !face_visible(b, nb, fast) {
                        continue;
                    }
                    let tile = match fi {
                        _ if leaves && fast => reg.solid_leaf[b as usize],
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
                        mask[mi(fi, x, y, z)] = Some(Key { tile, light: lights[0], pass, drop });
                        continue;
                    }
                    // одиночная грань
                    let org = tile_origin(tile);
                    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
                    let mut q = [V::new([0.0; 3], [0.0; 2], org, [0; 4]); 4];
                    for (i, c) in corners.iter().enumerate() {
                        let mut cy = c[1];
                        if lowered && cy > 0.5 {
                            cy = 1.0 - drop as f32 / 8.0;
                        }
                        q[i] = V::new([x as f32 + c[0], y as f32 + cy, z as f32 + c[2]], uvs[i], org, lights[i]);
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
                    let max = if key.pass == Pass::Water || key.light[3] == 251 { 4 } else { 15 };
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
                    let mut q = [V::new([0.0; 3], [0.0; 2], org, key.light); 4];
                    for (i, c) in corners.iter().enumerate() {
                        let mut pos = [0.0f32; 3];
                        pos[na] = sl as f32 + c[na];
                        pos[ta] = a0 as f32 + c[ta] * w as f32;
                        pos[tb] = b0 as f32 + c[tb] * h as f32;
                        if key.drop > 0 && fi == 2 {
                            pos[1] -= key.drop as f32 / 8.0;
                        }
                        let u = if c[uax] == corners[1][uax] { eu } else { 0.0 };
                        let v = if c[vax] == corners[0][vax] { ev } else { 0.0 };
                        q[i] = V::new(pos, [u, v], org, key.light);
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


/// Клетка 2×2×2 для дальней копии: сплошной блок (что виднее сверху), вода или пусто.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Air,
    Water,
    Solid(u16),
}

fn lod_cell(n: &Nbhd, cx: i32, cy: i32, cz: i32) -> Cell {
    let reg = blocks::reg();
    let (mut solid, mut water) = (0, 0);
    let mut top: Option<(i32, u16)> = None;
    for dy in 0..2 {
        for dz in 0..2 {
            for dx in 0..2 {
                let (x, y, z) = (cx * 2 + dx, cy * 2 + dy, cz * 2 + dz);
                let b = n.get(x, y, z);
                match reg.defs[b as usize].shape {
                    Shape::Cube | Shape::Cutout => {
                        solid += 1;
                        if top.is_none_or(|(ty, _)| y > ty) {
                            top = Some((y, b));
                        }
                    }
                    Shape::Liquid if b == WATER => water += 1,
                    _ => {}
                }
            }
        }
    }
    match top {
        Some((_, b)) if solid >= 3 => Cell::Solid(b),
        _ if water >= 4 => Cell::Water,
        _ => Cell::Air,
    }
}

/// Дальняя копия чанка: вдвое крупнее «пиксели», грани без света в пещерах не строятся,
/// соседние одинаковые грани в строке склеиваются.
fn build_lod(n: &Nbhd, light: &Light, out: &mut MeshData) {
    let reg = blocks::reg();
    const NX: i32 = CS / 2;
    const NY: i32 = CH / 2;
    // клетки с запасом в одну клетку вокруг чанка
    let w = NX + 2;
    let mut cells = vec![Cell::Air; (w * w * NY) as usize];
    let ci = |x: i32, y: i32, z: i32| ((y * w + (z + 1)) * w + (x + 1)) as usize;
    for y in 0..NY {
        for z in -1..=NX {
            for x in -1..=NX {
                cells[ci(x, y, z)] = lod_cell(n, x, y, z);
            }
        }
    }
    let get = |x: i32, y: i32, z: i32| -> Cell {
        if y < 0 {
            Cell::Solid(BEDROCK)
        } else if y >= NY {
            Cell::Air
        } else {
            cells[ci(x, y, z)]
        }
    };
    for (fi, (nrm, corners, shade)) in FACES.iter().enumerate() {
        for y in 0..NY {
            for z in 0..NX {
                let mut x = 0;
                while x < NX {
                    let c = get(x, y, z);
                    // на краю чанка соседа считаем пустым: стенка-бортик закрывает щели
                    // между упрощённым и обычным чанком (тёмные части бортика не строятся)
                    let edge = |x2: i32, z2: i32| !(0..NX).contains(&x2) || !(0..NX).contains(&z2);
                    let nb = if edge(x + nrm[0], z + nrm[2]) { Cell::Air } else { get(x + nrm[0], y + nrm[1], z + nrm[2]) };
                    let face = |c: Cell, nb: Cell| -> Option<(u16, [u8; 4], bool)> {
                        let (tile, water) = match c {
                            Cell::Air => return None,
                            Cell::Water => {
                                if fi != 2 || nb != Cell::Air {
                                    return None;
                                }
                                (reg.defs[WATER as usize].tex[0], true)
                            }
                            Cell::Solid(b) => {
                                if matches!(nb, Cell::Solid(_)) {
                                    return None;
                                }
                                let d = &reg.defs[b as usize];
                                let t = if is_leaves(b) { reg.solid_leaf[b as usize] } else { match fi { 2 => d.tex[0], 3 => d.tex[2], _ => d.tex[1] } };
                                (t, false)
                            }
                        };
                        // свет снаружи грани: лучший из соседних клеток-блоков
                        let (mut sl, mut bl) = (0u8, 0u8);
                        for k in 0..4 {
                            let ox = x * 2 + if nrm[0] > 0 { 2 } else if nrm[0] < 0 { -1 } else { k & 1 };
                            let oy = y * 2 + if nrm[1] > 0 { 2 } else if nrm[1] < 0 { -1 } else { (k >> 1) & 1 };
                            let oz = z * 2 + if nrm[2] > 0 { 2 } else if nrm[2] < 0 { -1 } else if nrm[0] != 0 { k & 1 } else { (k >> 1) & 1 };
                            let (a, b2) = light.at(ox, oy, oz);
                            sl = sl.max(a);
                            bl = bl.max(b2);
                        }
                        if sl == 0 && bl == 0 && !water {
                            return None; // тёмные пещеры вдали не нужны
                        }
                        let alpha = if water { 180 } else { 255 };
                        Some((tile, [lb(sl as f32), lb(bl as f32), (shade * 255.0) as u8, alpha], water))
                    };
                    let Some(key) = face(c, nb) else {
                        x += 1;
                        continue;
                    };
                    // склеиваем одинаковые грани вдоль x (для граней ±x — только по одной)
                    let mut len = 1;
                    if nrm[0] == 0 {
                        while x + len < NX && len < 7 {
                            let c2 = get(x + len, y, z);
                            let nb2 = if edge(x + len + nrm[0], z + nrm[2]) { Cell::Air } else { get(x + len + nrm[0], y + nrm[1], z + nrm[2]) };
                            if face(c2, nb2) != Some(key) {
                                break;
                            }
                            len += 1;
                        }
                    }
                    let (tile, lt, water) = key;
                    let mut q = [V::new([0.0; 3], [0.0; 2], tile, lt); 4];
                    let uax = (0..3).find(|&a| corners[0][a] != corners[1][a]).unwrap();
                    let vax = (0..3).find(|&a| corners[0][a] != corners[3][a]).unwrap();
                    for (i, cr) in corners.iter().enumerate() {
                        let sx = if nrm[0] == 0 { len as f32 * 2.0 } else { 2.0 };
                        let pos = [x as f32 * 2.0 + cr[0] * sx, y as f32 * 2.0 + cr[1] * 2.0 - if water { 0.25 } else { 0.0 }, z as f32 * 2.0 + cr[2] * 2.0];
                        let ext = |a: usize| if a == 0 { sx } else { 2.0 };
                        let u = if cr[uax] == corners[1][uax] { ext(uax) } else { 0.0 };
                        let v = if cr[vax] == corners[0][vax] { ext(vax) } else { 0.0 };
                        q[i] = V::new(pos, [u, v], tile, lt);
                    }
                    let leaves = matches!(c, Cell::Solid(b) if reg.defs[b as usize].shape == Shape::Cutout && !is_leaves(b));
                    let (vv, ii) = if water {
                        (&mut out.lod_water_v, &mut out.lod_water_i)
                    } else if leaves {
                        (&mut out.lod_cut_v, &mut out.lod_cut_i)
                    } else {
                        (&mut out.lod_v, &mut out.lod_i)
                    };
                    push_quad(vv, ii, q, false);
                    x += len;
                }
            }
        }
    }
}


/// Фигурный блок: грани каждой коробки, свет — без сглаживания.
#[allow(clippy::too_many_arguments)]
fn emit_form(n: &Nbhd, light: &Light, x: i32, y: i32, z: i32, b: u16, out: &mut MeshData, bx: &mut Vec<crate::shapes::Bx>) {
    use crate::shapes::{self as sh, Form};
    let reg = blocks::reg();
    let Some(f) = sh::form(b) else { return };
    let conn = sh::conn_at(|a, b2, c| n.get(a, b2, c), f, x, y, z);
    sh::boxes(f, conn, false, bx);
    let base = &reg.defs[sh::base(f) as usize];
    let door = match f {
        Form::Door { m, upper, .. } => Some(reg.door_tex[m as usize][upper as usize]),
        Form::Trapdoor { m, .. } => Some(reg.door_tex[m as usize][2]),
        _ => None,
    };
    let cut = sh::cutout(f);
    let (s0, l0) = light.at(x, y, z);
    for (min, max) in bx.iter() {
        for (fi, (nrm, corners, shade)) in FACES.iter().enumerate() {
            let na = nrm.iter().position(|&v| v != 0).unwrap();
            let pos_n = nrm[na] > 0;
            let on_edge = if pos_n { max[na] >= 0.999 } else { min[na] <= 0.001 };
            if on_edge {
                let nb = n.get(x + nrm[0], y + nrm[1], z + nrm[2]);
                if reg.opaque[nb as usize] {
                    continue;
                }
            }
            let (s1, l1) = light.at(x + nrm[0], y + nrm[1], z + nrm[2]);
            let lt = [lb(s0.max(s1) as f32), lb(l0.max(l1) as f32), (shade * 255.0) as u8, 255];
            // большие грани двери/люка — своей плиткой, рёбра — текстурой материала
            let face_area = {
                let mut a = 1.0;
                for k in 0..3 {
                    if k != na {
                        a *= max[k] - min[k];
                    }
                }
                a
            };
            let tile = match door {
                Some(t) if face_area > 0.5 => t,
                _ => match fi {
                    2 => base.tex[0],
                    3 => base.tex[2],
                    _ => base.tex[1],
                },
            };
            let uax = (0..3).find(|&a| corners[0][a] != corners[1][a]).unwrap();
            let vax = (0..3).find(|&a| corners[0][a] != corners[3][a]).unwrap();
            let c0 = corners[0];
            let mut q = [V::new([0.0; 3], [0.0; 2], tile, lt); 4];
            for (i, c) in corners.iter().enumerate() {
                let mut p = [0.0f32; 3];
                for k in 0..3 {
                    p[k] = if c[k] > 0.5 { max[k] } else { min[k] };
                }
                let u = (p[uax] - c0[uax]).abs();
                let v = 1.0 - (p[vax] - c0[vax]).abs();
                q[i] = V::new([x as f32 + p[0], y as f32 + p[1], z as f32 + p[2]], [u, v], tile, lt);
            }
            if cut {
                push_quad(&mut out.cutout_v, &mut out.cutout_i, q, false);
            } else {
                push_quad(&mut out.opaque_v, &mut out.opaque_i, q, false);
            }
        }
    }
}
