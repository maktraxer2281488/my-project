//! Фигурные блоки: полублоки, ступеньки, двери, люки, заборы, калитки, стеклянные панели.
//! Каждое положение (сторона, открыто/закрыто, верх/низ) — отдельный номер блока,
//! поэтому в чанке по-прежнему хранится только номер. Геометрия — набор коробок.

use crate::blocks::*;
use macroquad::prelude::{Vec3, vec3};

/// Материалы полублоков и ступенек.
pub const SLAB_MATS: [u16; 16] = [
    OAK_PLANKS, SPRUCE_LOG + 1, BIRCH_LOG + 1, SAKURA_LOG + 1, PALM_LOG + 1, MAPLE_LOG + 1, EBONY_LOG + 1, COBBLE,
    STONE, STONE_BRICKS, BRICKS, SANDSTONE, QUARTZ, POLISHED_GRANITE, POLISHED_MARBLE, DARK_BRICKS,
];
/// Двери и люки: 11 пород дерева и железо.
pub const DOOR_MATS: [u16; 12] = [
    OAK_PLANKS, SPRUCE_LOG + 1, BIRCH_LOG + 1, SAKURA_LOG + 1, PALM_LOG + 1, MAPLE_LOG + 1, EBONY_LOG + 1,
    WILLOW_LOG + 1, ACACIA_LOG + 1, MANGROVE_LOG + 1, CEDAR_LOG + 1, IRON_BLOCK,
];
/// Заборы и калитки — 11 пород дерева.
pub const FENCE_MATS: [u16; 11] = [
    OAK_PLANKS, SPRUCE_LOG + 1, BIRCH_LOG + 1, SAKURA_LOG + 1, PALM_LOG + 1, MAPLE_LOG + 1, EBONY_LOG + 1,
    WILLOW_LOG + 1, ACACIA_LOG + 1, MANGROVE_LOG + 1, CEDAR_LOG + 1,
];

pub const SLAB0: u16 = 382;
pub const STAIRS0: u16 = SLAB0 + 16 * 2;
pub const DOOR0: u16 = STAIRS0 + 16 * 8;
pub const TRAP0: u16 = DOOR0 + 12 * 16;
pub const FENCE0: u16 = TRAP0 + 12 * 16;
pub const GATE0: u16 = FENCE0 + 11;
pub const PANE: u16 = GATE0 + 11 * 4;
pub const PLATE0: u16 = PANE + 1;
pub const WIRE0: u16 = PLATE0 + 2;
pub const FORM_END: u16 = WIRE0 + 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Slab { m: u8, top: bool },
    /// dir — сторона высокой части: 0 +x, 1 −x, 2 +z, 3 −z.
    Stairs { m: u8, dir: u8, top: bool },
    Door { m: u8, dir: u8, open: bool, upper: bool },
    Trapdoor { m: u8, dir: u8, open: bool, top: bool },
    Fence { m: u8 },
    /// axis 0 — калитка поперёк оси z (перегораживает проход по z), 1 — поперёк x.
    Gate { m: u8, axis: u8, open: bool },
    Pane,
    /// Нажимная плита.
    Plate { on: bool },
    /// Сигнальный провод.
    Wire { on: bool },
}

pub fn form(b: u16) -> Option<Form> {
    if !(SLAB0..FORM_END).contains(&b) {
        return None;
    }
    let f = if b < STAIRS0 {
        let k = b - SLAB0;
        Form::Slab { m: (k / 2) as u8, top: k % 2 == 1 }
    } else if b < DOOR0 {
        let k = b - STAIRS0;
        Form::Stairs { m: (k / 8) as u8, dir: ((k % 8) / 2) as u8, top: k % 2 == 1 }
    } else if b < TRAP0 {
        let k = b - DOOR0;
        Form::Door { m: (k / 16) as u8, dir: ((k % 16) / 4) as u8, open: (k % 4) / 2 == 1, upper: k % 2 == 1 }
    } else if b < FENCE0 {
        let k = b - TRAP0;
        Form::Trapdoor { m: (k / 16) as u8, dir: ((k % 16) / 4) as u8, open: (k % 4) / 2 == 1, top: k % 2 == 1 }
    } else if b < GATE0 {
        Form::Fence { m: (b - FENCE0) as u8 }
    } else if b < PANE {
        let k = b - GATE0;
        Form::Gate { m: (k / 4) as u8, axis: ((k % 4) / 2) as u8, open: k % 2 == 1 }
    } else if b == PANE {
        Form::Pane
    } else if b < WIRE0 {
        Form::Plate { on: b == PLATE0 + 1 }
    } else {
        Form::Wire { on: b == WIRE0 + 1 }
    };
    Some(f)
}

pub fn id(f: Form) -> u16 {
    match f {
        Form::Slab { m, top } => SLAB0 + m as u16 * 2 + top as u16,
        Form::Stairs { m, dir, top } => STAIRS0 + m as u16 * 8 + dir as u16 * 2 + top as u16,
        Form::Door { m, dir, open, upper } => DOOR0 + m as u16 * 16 + dir as u16 * 4 + open as u16 * 2 + upper as u16,
        Form::Trapdoor { m, dir, open, top } => TRAP0 + m as u16 * 16 + dir as u16 * 4 + open as u16 * 2 + top as u16,
        Form::Fence { m } => FENCE0 + m as u16,
        Form::Gate { m, axis, open } => GATE0 + m as u16 * 4 + axis as u16 * 2 + open as u16,
        Form::Pane => PANE,
        Form::Plate { on } => PLATE0 + on as u16,
        Form::Wire { on } => WIRE0 + on as u16,
    }
}

/// Какой предмет получается из блока (и какой лежит в инвентаре).
pub fn canon(b: u16) -> u16 {
    match form(b) {
        Some(Form::Slab { m, .. }) => id(Form::Slab { m, top: false }),
        Some(Form::Stairs { m, .. }) => id(Form::Stairs { m, dir: 0, top: false }),
        Some(Form::Door { m, .. }) => id(Form::Door { m, dir: 0, open: false, upper: false }),
        Some(Form::Trapdoor { m, .. }) => id(Form::Trapdoor { m, dir: 0, open: false, top: false }),
        Some(Form::Gate { m, .. }) => id(Form::Gate { m, axis: 0, open: false }),
        Some(Form::Plate { .. }) => PLATE0,
        Some(Form::Wire { .. }) => WIRE0,
        _ => b,
    }
}

/// Блок, чья текстура используется.
pub fn base(f: Form) -> u16 {
    match f {
        Form::Slab { m, .. } | Form::Stairs { m, .. } => SLAB_MATS[m as usize],
        Form::Door { m, .. } | Form::Trapdoor { m, .. } => DOOR_MATS[m as usize],
        Form::Fence { m } | Form::Gate { m, .. } => FENCE_MATS[m as usize],
        Form::Pane => GLASS,
        Form::Plate { .. } => STONE,
        Form::Wire { on } => WIRE0 + on as u16,
    }
}

/// Открывается ли по ПКМ.
pub fn toggled(b: u16) -> Option<u16> {
    match form(b)? {
        Form::Door { m, dir, open, upper } if m != 11 => Some(id(Form::Door { m, dir, open: !open, upper })),
        Form::Trapdoor { m, dir, open, top } if m != 11 => Some(id(Form::Trapdoor { m, dir, open: !open, top })),
        Form::Gate { m, axis, open } => Some(id(Form::Gate { m, axis, open: !open })),
        _ => None,
    }
}

/// Сторона по направлению взгляда игрока (куда он смотрит).
pub fn facing(yaw: f32) -> u8 {
    let (c, s) = (yaw.cos(), yaw.sin());
    if c.abs() > s.abs() {
        if c > 0.0 { 0 } else { 1 }
    } else if s > 0.0 {
        2
    } else {
        3
    }
}

/// Вариант блока при установке: по взгляду игрока и точке попадания на грани.
pub fn placed(item: u16, yaw: f32, hit_y_frac: f32, hit_face_down: bool) -> u16 {
    let dir = facing(yaw);
    let top = hit_face_down || (hit_y_frac > 0.5 && hit_y_frac < 0.999);
    match form(item) {
        Some(Form::Slab { m, .. }) => id(Form::Slab { m, top }),
        Some(Form::Stairs { m, .. }) => id(Form::Stairs { m, dir, top }),
        Some(Form::Door { m, .. }) => id(Form::Door { m, dir: dir ^ 1, open: false, upper: false }),
        Some(Form::Trapdoor { m, .. }) => id(Form::Trapdoor { m, dir: dir ^ 1, open: false, top }),
        Some(Form::Gate { m, .. }) => id(Form::Gate { m, axis: if dir < 2 { 1 } else { 0 }, open: false }),
        _ => item,
    }
}

pub type Bx = (Vec3, Vec3);

const P: f32 = 1.0 / 16.0;

fn side_plate(dir: u8, t: f32, y0: f32, y1: f32) -> Bx {
    match dir {
        0 => (vec3(1.0 - t, y0, 0.0), vec3(1.0, y1, 1.0)),
        1 => (vec3(0.0, y0, 0.0), vec3(t, y1, 1.0)),
        2 => (vec3(0.0, y0, 1.0 - t), vec3(1.0, y1, 1.0)),
        _ => (vec3(0.0, y0, 0.0), vec3(1.0, y1, t)),
    }
}

fn arm(side: usize, w0: f32, w1: f32, y0: f32, y1: f32) -> Bx {
    match side {
        0 => (vec3(w1, y0, w0), vec3(1.0, y1, w1)),
        1 => (vec3(0.0, y0, w0), vec3(w0, y1, w1)),
        2 => (vec3(w0, y0, w1), vec3(w1, y1, 1.0)),
        _ => (vec3(w0, y0, 0.0), vec3(w1, y1, w0)),
    }
}

/// Коробки блока (в долях клетки). `conn` — соединения забора/панели: +x, −x, +z, −z.
/// `collide` — для столкновений (забор и закрытая калитка выше: 1.5).
pub fn boxes(f: Form, conn: [bool; 4], collide: bool, out: &mut Vec<Bx>) {
    out.clear();
    match f {
        Form::Slab { top, .. } => out.push(if top { (vec3(0.0, 0.5, 0.0), Vec3::ONE) } else { (Vec3::ZERO, vec3(1.0, 0.5, 1.0)) }),
        Form::Stairs { dir, top, .. } => {
            let (y0, y1) = if top { (0.0, 0.5) } else { (0.5, 1.0) };
            out.push(if top { (vec3(0.0, 0.5, 0.0), Vec3::ONE) } else { (Vec3::ZERO, vec3(1.0, 0.5, 1.0)) });
            out.push(side_plate(dir, 0.5, y0, y1));
        }
        Form::Door { dir, open, .. } => {
            let side = if open { [2, 3, 1, 0][dir as usize] } else { dir };
            out.push(side_plate(side, 3.0 * P, 0.0, 1.0));
        }
        Form::Trapdoor { dir, open, top, .. } => out.push(if open {
            side_plate(dir, 3.0 * P, 0.0, 1.0)
        } else if top {
            (vec3(0.0, 13.0 * P, 0.0), Vec3::ONE)
        } else {
            (Vec3::ZERO, vec3(1.0, 3.0 * P, 1.0))
        }),
        Form::Fence { .. } => {
            let h = if collide { 1.5 } else { 1.0 };
            out.push((vec3(6.0 * P, 0.0, 6.0 * P), vec3(10.0 * P, h, 10.0 * P)));
            for (s, c) in conn.iter().enumerate() {
                if !c {
                    continue;
                }
                if collide {
                    out.push(arm(s, 6.0 * P, 10.0 * P, 0.0, 1.5));
                } else {
                    out.push(arm(s, 7.0 * P, 9.0 * P, 6.0 * P, 9.0 * P));
                    out.push(arm(s, 7.0 * P, 9.0 * P, 12.0 * P, 15.0 * P));
                }
            }
        }
        Form::Gate { axis, open, .. } => {
            if collide {
                if !open {
                    out.push(if axis == 0 { (vec3(0.0, 0.0, 7.0 * P), vec3(1.0, 1.5, 9.0 * P)) } else { (vec3(7.0 * P, 0.0, 0.0), vec3(9.0 * P, 1.5, 1.0)) });
                }
            } else {
                // столбики по краям и перекладины (у открытой — развёрнуты внутрь)
                let posts = if axis == 0 {
                    [(vec3(0.0, 5.0 * P, 7.0 * P), vec3(2.0 * P, 1.0, 9.0 * P)), (vec3(14.0 * P, 5.0 * P, 7.0 * P), vec3(1.0, 1.0, 9.0 * P))]
                } else {
                    [(vec3(7.0 * P, 5.0 * P, 0.0), vec3(9.0 * P, 1.0, 2.0 * P)), (vec3(7.0 * P, 5.0 * P, 14.0 * P), vec3(9.0 * P, 1.0, 1.0))]
                };
                out.extend_from_slice(&posts);
                for (y0, y1) in [(6.0 * P, 9.0 * P), (12.0 * P, 15.0 * P)] {
                    out.push(match (axis, open) {
                        (0, false) => (vec3(2.0 * P, y0, 7.0 * P), vec3(14.0 * P, y1, 9.0 * P)),
                        (0, true) => (vec3(0.0, y0, 9.0 * P), vec3(2.0 * P, y1, 15.0 * P)),
                        (_, false) => (vec3(7.0 * P, y0, 2.0 * P), vec3(9.0 * P, y1, 14.0 * P)),
                        (_, true) => (vec3(9.0 * P, y0, 0.0), vec3(15.0 * P, y1, 2.0 * P)),
                    });
                }
            }
        }
        Form::Plate { on } => {
            if !collide {
                out.push((vec3(P, 0.0, P), vec3(15.0 * P, if on { 0.5 * P } else { P }, 15.0 * P)));
            }
        }
        Form::Wire { .. } => {
            if !collide {
                out.push((Vec3::ZERO, vec3(1.0, 0.5 * P, 1.0)));
            }
        }
        Form::Pane => {
            out.push((vec3(7.0 * P, 0.0, 7.0 * P), vec3(9.0 * P, 1.0, 9.0 * P)));
            let any = conn.iter().any(|c| *c);
            for (s, c) in conn.iter().enumerate() {
                if *c || !any {
                    out.push(arm(s, 7.0 * P, 9.0 * P, 0.0, 1.0));
                }
            }
        }
    }
}

/// Соединяется ли забор/панель с соседом.
pub fn connects(f: Form, nb: u16) -> bool {
    if is_opaque(nb) {
        return true;
    }
    match (f, form(nb)) {
        (Form::Fence { .. }, Some(Form::Fence { .. } | Form::Gate { .. })) => true,
        (Form::Pane, Some(Form::Pane)) => true,
        (Form::Pane, None) => nb == GLASS || def(nb).name.contains("стекло"),
        _ => false,
    }
}

/// Соединения забора/панели в точке мира (соседи по +x, −x, +z, −z).
pub fn conn_at(get: impl Fn(i32, i32, i32) -> u16, f: Form, x: i32, y: i32, z: i32) -> [bool; 4] {
    match f {
        Form::Fence { .. } | Form::Pane => [
            connects(f, get(x + 1, y, z)),
            connects(f, get(x - 1, y, z)),
            connects(f, get(x, y, z + 1)),
            connects(f, get(x, y, z - 1)),
        ],
        _ => [false; 4],
    }
}

/// Двери, люки и панели рисуются с прозрачностью.
pub fn cutout(f: Form) -> bool {
    matches!(f, Form::Door { .. } | Form::Trapdoor { .. } | Form::Pane | Form::Wire { .. })
}
