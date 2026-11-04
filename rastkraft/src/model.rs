//! 3D-модели: кубики, текстурированные блоки, спрайты предметов, тела существ
//! и игрока с анимациями. Всё рисуется через macroquad в мировых координатах.

use crate::blocks::{self, Shape, tile_uv};
use crate::items::{Item, Mat};
use crate::mobdefs::{FLY, GLOW, HOP, MobDef, Plan};
use macroquad::prelude::*;

const FACES: [([f32; 3], [[f32; 3]; 4], f32, usize); 6] = [
    ([0., 1., 0.], [[-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5]], 1.0, 0),
    ([0., -1., 0.], [[-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5]], 0.5, 2),
    ([1., 0., 0.], [[0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5]], 0.82, 1),
    ([-1., 0., 0.], [[-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5]], 0.82, 1),
    ([0., 0., 1.], [[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]], 0.68, 1),
    ([0., 0., -1.], [[0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5]], 0.68, 1),
];

fn col(c: (u8, u8, u8)) -> Color {
    Color::from_rgba(c.0, c.1, c.2, 255)
}

fn scale_c(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

/// Единичный куб (центр в начале координат), преобразованный матрицей.
pub fn cube(m: Mat4, color: Color) {
    let mut v = Vec::with_capacity(24);
    let mut idx = Vec::with_capacity(36);
    for (_, corners, k, _) in FACES {
        let c = scale_c(color, k);
        let b = v.len() as u16;
        for p in corners {
            v.push(Vertex::new2(m.transform_point3(Vec3::from(p)), Vec2::ZERO, c));
        }
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    draw_mesh(&Mesh { vertices: v, indices: idx, texture: None });
}

/// Куб с текстурами блока (верх, бок, низ).
pub fn tex_cube(atlas: &Texture2D, m: Mat4, tex: [u16; 3], tint: Color) {
    let mut v = Vec::with_capacity(24);
    let mut idx = Vec::with_capacity(36);
    for (_, corners, k, ti) in FACES {
        let (u0, v0, u1, v1) = tile_uv(tex[ti]);
        let uv = [vec2(u0, v1), vec2(u1, v1), vec2(u1, v0), vec2(u0, v0)];
        let c = scale_c(tint, k);
        let b = v.len() as u16;
        for (i, p) in corners.iter().enumerate() {
            v.push(Vertex::new2(m.transform_point3(Vec3::from(*p)), uv[i], c));
        }
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    draw_mesh(&Mesh { vertices: v, indices: idx, texture: Some(atlas.clone()) });
}

/// Плоский спрайт (квадрат в плоскости XY, видно с обеих сторон).
pub fn sprite(atlas: &Texture2D, m: Mat4, tile: u16, tint: Color) {
    let (u0, v0, u1, v1) = tile_uv(tile);
    let p = [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]];
    let uv = [vec2(u0, v1), vec2(u1, v1), vec2(u1, v0), vec2(u0, v0)];
    let v: Vec<Vertex> = (0..4).map(|i| Vertex::new2(m.transform_point3(Vec3::from(p[i])), uv[i], tint)).collect();
    draw_mesh(&Mesh { vertices: v, indices: vec![0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2], texture: Some(atlas.clone()) });
}

pub fn item_tile(item: Item) -> u16 {
    match item {
        Item::Block(b) => blocks::def(b).tex[1],
        other => blocks::item_tile(other.sprite_index().unwrap_or(0)),
    }
}

/// Предмет в руке (в локальных координатах кисти).
pub fn held_item(atlas: &Texture2D, m: Mat4, item: Item, tint: Color) {
    match item {
        Item::Empty => {}
        Item::Block(b) if blocks::def(b).shape != Shape::Cross => {
            tex_cube(atlas, m * Mat4::from_scale(Vec3::splat(0.32)), blocks::def(b).tex, tint);
        }
        other => {
            // инструмент держим за рукоять: спрайт наклонён вперёд
            let s = if matches!(other, Item::Tool(..)) { 0.75 } else { 0.5 };
            let r = Mat4::from_translation(vec3(0.0, 0.18, 0.0)) * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2) * Mat4::from_rotation_z(-0.8);
            sprite(atlas, m * r * Mat4::from_scale(Vec3::splat(s)), item_tile(other), tint);
        }
    }
}

/// Часть тела: поворот `rot` вокруг точки `pivot`, затем смещение и размер.
fn part(root: Mat4, pivot: Vec3, rot: Mat4, offset: Vec3, size: Vec3, c: Color) {
    cube(root * Mat4::from_translation(pivot) * rot * Mat4::from_translation(offset) * Mat4::from_scale(size), c);
}

fn rx(a: f32) -> Mat4 {
    Mat4::from_rotation_x(a)
}
fn rz(a: f32) -> Mat4 {
    Mat4::from_rotation_z(a)
}

pub struct Anim {
    pub walk: f32,
    pub speed: f32,
    pub attack: f32,
    pub hurt: bool,
    pub dying: f32,
    pub t: f32,
    pub light: f32,
    pub head_pitch: f32,
}

fn root_m(pos: Vec3, yaw: f32, scale: f32, a: &Anim) -> Mat4 {
    let mut m = Mat4::from_translation(pos) * Mat4::from_rotation_y(-yaw);
    if a.dying > 0.0 {
        m *= Mat4::from_rotation_x(a.dying.min(1.0) * std::f32::consts::FRAC_PI_2);
    }
    m * Mat4::from_scale(Vec3::splat(scale))
}

/// Тело существа по описанию из таблицы.
pub fn draw_mob(def: &MobDef, pos: Vec3, yaw: f32, a: &Anim) {
    let lit = |c: Color| -> Color {
        let c = if a.hurt { Color::new(c.r * 0.4 + 0.6, c.g * 0.5, c.b * 0.5, c.a) } else { c };
        scale_c(c, a.light)
    };
    let c1 = lit(col(def.c1));
    let c2 = lit(col(def.c2));
    let glow = def.flags & GLOW != 0;
    let eye = if glow { col(def.c3) } else { lit(col(def.c3)) };
    let sw = (a.walk).sin() * 0.6 * a.speed.min(1.0);
    let atk = (a.attack * std::f32::consts::PI).sin();
    let bob = if def.flags & FLY != 0 { (a.t * 3.0).sin() * 0.12 } else { 0.0 };
    let r = root_m(pos + vec3(0.0, bob, 0.0), yaw, def.scale, a);
    let head = |root: Mat4, at: Vec3, s: f32, face: f32| {
        let hm = Mat4::from_translation(at) * rz(a.head_pitch * 0.5);
        part(root, Vec3::ZERO, hm, Vec3::ZERO, Vec3::splat(s), c1);
        part(root, Vec3::ZERO, hm, vec3(face, s * 0.1, -s * 0.22), vec3(0.03, s * 0.16, s * 0.18), eye);
        part(root, Vec3::ZERO, hm, vec3(face, s * 0.1, s * 0.22), vec3(0.03, s * 0.16, s * 0.18), eye);
    };
    match def.plan {
        Plan::Biped | Plan::Golem => {
            let g = def.plan == Plan::Golem;
            let (lw, lh) = if g { (0.32, 0.8) } else { (0.22, 0.75) };
            let bw = if g { 0.8 } else { 0.5 };
            let bh = if g { 0.9 } else { 0.72 };
            part(r, vec3(0.0, lh, -0.13), rx(0.0) * rz(sw), vec3(0.0, -lh / 2.0, 0.0), vec3(lw, lh, lw), c2);
            part(r, vec3(0.0, lh, 0.13), rz(-sw), vec3(0.0, -lh / 2.0, 0.0), vec3(lw, lh, lw), c2);
            part(r, vec3(0.0, lh + bh / 2.0, 0.0), Mat4::IDENTITY, Vec3::ZERO, vec3(0.32, bh, bw), c1);
            let sh = lh + bh - 0.08;
            let arm = vec3(0.2, bh * 0.95, 0.2);
            part(r, vec3(0.0, sh, -bw / 2.0 - 0.1), rz(-sw - atk * 1.6), vec3(0.0, -arm.y / 2.0, 0.0), arm, c2);
            part(r, vec3(0.0, sh, bw / 2.0 + 0.1), rz(sw - atk * 1.2), vec3(0.0, -arm.y / 2.0, 0.0), arm, c2);
            let hs = if g { 0.5 } else { 0.46 };
            head(r, vec3(0.0, lh + bh + hs / 2.0, 0.0), hs, hs / 2.0 + 0.01);
        }
        Plan::Quad => {
            let (bl, bh, bw) = (1.0, 0.55, 0.6);
            let leg_h = 0.45;
            for (lx, lz, ph) in [(0.36, 0.2, 1.0), (0.36, -0.2, -1.0), (-0.36, 0.2, -1.0), (-0.36, -0.2, 1.0)] {
                if def.flags & HOP != 0 {
                    let k = (a.walk * 0.5).sin().abs() * 0.5;
                    part(r, vec3(lx, leg_h, lz), rz(-k * ph), vec3(0.0, -leg_h / 2.0, 0.0), vec3(0.16, leg_h, 0.16), c2);
                } else {
                    part(r, vec3(lx, leg_h, lz), rz(sw * ph), vec3(0.0, -leg_h / 2.0, 0.0), vec3(0.16, leg_h, 0.16), c2);
                }
            }
            part(r, vec3(0.0, leg_h + bh / 2.0, 0.0), Mat4::IDENTITY, Vec3::ZERO, vec3(bl, bh, bw), c1);
            part(r, vec3(-bl / 2.0, leg_h + bh * 0.8, 0.0), rz(0.6 + (a.t * 6.0).sin() * 0.3), vec3(-0.12, 0.0, 0.0), vec3(0.25, 0.08, 0.08), c2);
            let nod = (a.walk * 2.0).sin() * 0.04 - atk * 0.2;
            head(r, vec3(bl / 2.0 + 0.15, leg_h + bh + 0.05 + nod, 0.0), 0.42, 0.22);
        }
        Plan::Slime => {
            let hop = (a.walk * 0.8).sin().abs();
            let sq = 1.0 + (1.0 - hop) * 0.15;
            let m = Mat4::from_translation(vec3(0.0, 0.45 * (2.0 - sq) + hop * 0.2, 0.0)) * Mat4::from_scale(vec3(0.9 * sq, 0.9 * (2.0 - sq), 0.9 * sq));
            cube(r * m * Mat4::from_scale(Vec3::splat(0.45)), c2);
            let mut outer = c1;
            outer.a = 0.75;
            cube(r * m, outer);
            part(r, Vec3::ZERO, Mat4::from_translation(vec3(0.46, 0.6, 0.0)), vec3(0.0, 0.0, -0.18), vec3(0.03, 0.14, 0.12), eye);
            part(r, Vec3::ZERO, Mat4::from_translation(vec3(0.46, 0.6, 0.0)), vec3(0.0, 0.0, 0.18), vec3(0.03, 0.14, 0.12), eye);
        }
        Plan::Bird => {
            let flap = if def.flags & FLY != 0 { (a.t * 14.0).sin() * 0.9 } else { (a.walk).sin() * 0.3 };
            let y0 = if def.flags & FLY != 0 { 0.3 } else { 0.3 };
            if def.flags & FLY == 0 {
                part(r, vec3(0.0, y0, -0.1), rz(sw), vec3(0.0, -0.15, 0.0), vec3(0.06, 0.3, 0.06), c2);
                part(r, vec3(0.0, y0, 0.1), rz(-sw), vec3(0.0, -0.15, 0.0), vec3(0.06, 0.3, 0.06), c2);
            }
            part(r, vec3(0.0, y0 + 0.2, 0.0), Mat4::IDENTITY, Vec3::ZERO, vec3(0.6, 0.38, 0.36), c1);
            part(r, vec3(0.0, y0 + 0.32, -0.18), rx(flap), vec3(0.0, 0.0, -0.3), vec3(0.4, 0.05, 0.6), c2);
            part(r, vec3(0.0, y0 + 0.32, 0.18), rx(-flap), vec3(0.0, 0.0, 0.3), vec3(0.4, 0.05, 0.6), c2);
            part(r, vec3(-0.34, y0 + 0.25, 0.0), rz(0.3), Vec3::ZERO, vec3(0.2, 0.06, 0.26), c2);
            head(r, vec3(0.35, y0 + 0.5, 0.0), 0.3, 0.16);
            part(r, vec3(0.53, y0 + 0.45, 0.0), Mat4::IDENTITY, Vec3::ZERO, vec3(0.12, 0.06, 0.08), col(def.c3));
        }
        Plan::Spider => {
            part(r, vec3(-0.35, 0.4, 0.0), Mat4::IDENTITY, Vec3::ZERO, vec3(0.6, 0.45, 0.55), c1);
            part(r, vec3(0.15, 0.35, 0.0), Mat4::IDENTITY, Vec3::ZERO, vec3(0.4, 0.3, 0.4), c2);
            for i in 0..4 {
                let x = 0.25 - i as f32 * 0.17;
                let w = (a.walk + i as f32 * 1.3).sin() * 0.35;
                for side in [-1.0f32, 1.0] {
                    part(r, vec3(x, 0.38, side * 0.18), rx(side * (1.0 + w)), vec3(0.0, 0.0, side * 0.3), vec3(0.06, 0.06, 0.6), c2);
                }
            }
            part(r, vec3(0.36, 0.4, -0.08), Mat4::IDENTITY, Vec3::ZERO, vec3(0.03, 0.07, 0.07), eye);
            part(r, vec3(0.36, 0.4, 0.08), Mat4::IDENTITY, Vec3::ZERO, vec3(0.03, 0.07, 0.07), eye);
        }
        Plan::Fish => {
            let segs = 4;
            for i in 0..segs {
                let x = 0.45 - i as f32 * 0.32;
                let wig = (a.t * 6.0 - i as f32 * 0.9).sin() * 0.12 * i as f32 / segs as f32 * 2.0;
                let s = 0.42 - i as f32 * 0.07;
                part(r, vec3(x, 0.3, wig), Mat4::IDENTITY, Vec3::ZERO, vec3(0.34, s, s * 0.8), if i % 2 == 0 { c1 } else { c2 });
            }
            let tw = (a.t * 6.0).sin() * 0.5;
            part(r, vec3(-0.85, 0.3, 0.0), Mat4::from_rotation_y(tw), vec3(-0.12, 0.0, 0.0), vec3(0.25, 0.32, 0.06), c2);
            part(r, vec3(0.63, 0.36, -0.12), Mat4::IDENTITY, Vec3::ZERO, vec3(0.03, 0.07, 0.07), eye);
            part(r, vec3(0.63, 0.36, 0.12), Mat4::IDENTITY, Vec3::ZERO, vec3(0.03, 0.07, 0.07), eye);
        }
        Plan::Wisp => {
            let pulse = 1.0 + (a.t * 4.0).sin() * 0.08;
            let core = if glow { col(def.c1) } else { c1 };
            cube(r * Mat4::from_translation(vec3(0.0, 0.6, 0.0)) * Mat4::from_rotation_y(a.t) * Mat4::from_scale(Vec3::splat(0.5 * pulse)), core);
            let mut halo = col(def.c2);
            halo.a = 0.45;
            cube(r * Mat4::from_translation(vec3(0.0, 0.6, 0.0)) * Mat4::from_rotation_y(-a.t * 0.7) * Mat4::from_scale(Vec3::splat(0.75 * pulse)), halo);
            for k in 0..3 {
                let ang = a.t * 2.0 + k as f32 * 2.094;
                let p = vec3(ang.cos() * 0.6, 0.6 + (a.t * 3.0 + k as f32).sin() * 0.15, ang.sin() * 0.6);
                cube(r * Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.1)), col(def.c3));
            }
            part(r, vec3(0.27, 0.68, -0.1), Mat4::IDENTITY, Vec3::ZERO, vec3(0.02, 0.08, 0.08), eye);
            part(r, vec3(0.27, 0.68, 0.1), Mat4::IDENTITY, Vec3::ZERO, vec3(0.02, 0.08, 0.08), eye);
        }
    }
}

pub fn armor_color(item: Item) -> Option<Color> {
    if let Item::Armor(_, m) = item {
        let (c, _, _) = m.colors();
        Some(Color::from_rgba(c[0], c[1], c[2], 255))
    } else {
        None
    }
}

pub struct PlayerLook {
    pub skin: Color,
    pub shirt: Color,
    pub pants: Color,
}

pub fn look_for(seed: u32) -> PlayerLook {
    let h = |k| crate::blocks::hash(seed as i32, k, 4242);
    let shirts = [(60, 120, 200), (200, 70, 60), (70, 160, 90), (230, 170, 40), (140, 80, 190), (40, 40, 50)];
    let sc = shirts[(h(1) * 6.0) as usize % 6];
    let skin = [(236, 190, 150), (200, 150, 110), (150, 100, 70), (250, 210, 180)][(h(2) * 4.0) as usize % 4];
    PlayerLook { skin: col(skin), shirt: col(sc), pants: col((50, 50, 90)) }
}

/// Модель игрока (для вида от третьего лица и других игроков по сети).
#[allow(clippy::too_many_arguments)]
pub fn draw_player(atlas: &Texture2D, pos: Vec3, yaw: f32, pitch: f32, walk: f32, speed: f32, swing: f32, held: Item, armor: &[Item; 4], look: &PlayerLook, light: f32, hurt: bool) {
    let lit = |c: Color| {
        let c = if hurt { Color::new(c.r * 0.4 + 0.6, c.g * 0.5, c.b * 0.5, 1.0) } else { c };
        scale_c(c, light)
    };
    let r = Mat4::from_translation(pos) * Mat4::from_rotation_y(-yaw);
    let sw = walk.sin() * 0.7 * speed.min(1.0);
    let atk = (swing * std::f32::consts::PI).sin();
    let legs = lit(armor_color(armor[2]).unwrap_or(look.pants));
    let boots = lit(armor_color(armor[3]).unwrap_or(Color::from_rgba(60, 40, 30, 255)));
    let shirt = lit(armor_color(armor[1]).unwrap_or(look.shirt));
    let skin = lit(look.skin);
    for (z, s) in [(-0.13f32, sw), (0.13, -sw)] {
        part(r, vec3(0.0, 0.75, z), rz(s), vec3(0.0, -0.3, 0.0), vec3(0.24, 0.6, 0.24), legs);
        part(r, vec3(0.0, 0.75, z), rz(s), vec3(0.0, -0.66, 0.0), vec3(0.26, 0.18, 0.26), boots);
    }
    part(r, vec3(0.0, 1.11, 0.0), Mat4::IDENTITY, Vec3::ZERO, vec3(0.28, 0.72, 0.5), shirt);
    // левая рука
    part(r, vec3(0.0, 1.42, -0.36), rz(-sw), vec3(0.0, -0.3, 0.0), vec3(0.2, 0.68, 0.2), shirt);
    // правая рука с предметом
    let arm_rot = rz(sw * 0.5 - atk * 1.8 - if held != Item::Empty { 0.35 } else { 0.0 });
    part(r, vec3(0.0, 1.42, 0.36), arm_rot, vec3(0.0, -0.3, 0.0), vec3(0.2, 0.68, 0.2), shirt);
    part(r, vec3(0.0, 1.42, 0.36), arm_rot, vec3(0.0, -0.6, 0.0), vec3(0.21, 0.14, 0.21), skin);
    let hand = r * Mat4::from_translation(vec3(0.0, 1.42, 0.36)) * arm_rot * Mat4::from_translation(vec3(0.12, -0.66, 0.0));
    held_item(atlas, hand, held, Color::new(light, light, light, 1.0));
    // голова
    let hm = Mat4::from_translation(vec3(0.0, 1.72, 0.0)) * rz(pitch);
    part(r, Vec3::ZERO, hm, Vec3::ZERO, Vec3::splat(0.48), skin);
    part(r, Vec3::ZERO, hm, vec3(0.0, 0.2, 0.0), vec3(0.5, 0.12, 0.5), lit(Color::from_rgba(70, 46, 30, 255)));
    part(r, Vec3::ZERO, hm, vec3(0.245, 0.04, -0.1), vec3(0.02, 0.07, 0.07), lit(Color::from_rgba(30, 40, 80, 255)));
    part(r, Vec3::ZERO, hm, vec3(0.245, 0.04, 0.1), vec3(0.02, 0.07, 0.07), lit(Color::from_rgba(30, 40, 80, 255)));
    if let Some(hc) = armor_color(armor[0]) {
        part(r, Vec3::ZERO, hm, vec3(-0.02, 0.12, 0.0), vec3(0.54, 0.3, 0.54), lit(hc));
    }
}

/// Рука и предмет от первого лица. `basis` — (вперёд, вправо, вверх) камеры.
#[allow(clippy::too_many_arguments)]
pub fn draw_first_person(atlas: &Texture2D, eye: Vec3, fwd: Vec3, right: Vec3, up: Vec3, held: Item, swing: f32, bob: Vec2, eat: f32, sleeve: Color, skin: Color, light: f32) {
    let atk = (swing * std::f32::consts::PI).sin();
    let eat_bob = if eat > 0.0 { (eat * 22.0).sin().abs() * 0.05 } else { 0.0 };
    let base = eye + fwd * 0.5 + right * (0.3 - atk * 0.12 - if eat > 0.0 { 0.2 } else { 0.0 }) + up * (-0.26 + bob.y + eat_bob + atk * 0.08) + right * bob.x;
    // базис камеры -> матрица: x=вперёд, y=вверх, z=вправо
    let cam = Mat4::from_cols(fwd.extend(0.0), up.extend(0.0), right.extend(0.0), base.extend(1.0));
    let tilt = Mat4::from_rotation_z(-0.25 - atk * 1.1) * Mat4::from_rotation_y(0.35 + atk * 0.3);
    let m = cam * tilt * Mat4::from_scale(Vec3::splat(0.42));
    let k = light;
    let sl = Color::new(sleeve.r * k, sleeve.g * k, sleeve.b * k, 1.0);
    let sk = Color::new(skin.r * k, skin.g * k, skin.b * k, 1.0);
    if held == Item::Empty {
        cube(m * Mat4::from_translation(vec3(0.0, -0.25, 0.0)) * Mat4::from_scale(vec3(0.16, 0.5, 0.16)), sl);
        cube(m * Mat4::from_translation(vec3(0.0, 0.05, 0.0)) * Mat4::from_scale(vec3(0.17, 0.14, 0.17)), sk);
    } else {
        cube(m * Mat4::from_translation(vec3(-0.05, -0.3, 0.05)) * Mat4::from_scale(vec3(0.15, 0.45, 0.15)), sl);
        cube(m * Mat4::from_translation(vec3(-0.05, -0.03, 0.05)) * Mat4::from_scale(vec3(0.16, 0.12, 0.16)), sk);
        held_item(atlas, m * Mat4::from_translation(vec3(0.05, 0.05, 0.0)), held, Color::new(k, k, k, 1.0));
    }
}

/// Цвет брони по материалу (для подсказок).
pub fn mat_color(m: Mat) -> Color {
    let (c, _, _) = m.colors();
    Color::from_rgba(c[0], c[1], c[2], 255)
}
