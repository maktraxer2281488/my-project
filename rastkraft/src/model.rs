//! 3D-модели существ, игроков и предметов. Все треугольники за кадр собираются
//! в один большой буфер и рисуются несколькими крупными вызовами (батчинг).

use crate::blocks::{self, Shape, tile_uv};
use crate::items::{Item, Mat};
use crate::mobdefs::{FLY, GLOW, HOP, MOBS, Plan};
use macroquad::prelude::*;
use std::cell::RefCell;

const MAX_BATCH: usize = 36_000;

struct Batch {
    v: Vec<Vertex>,
    i: Vec<u16>,
    atlas: Option<Texture2D>,
    white: Vec2,
}

thread_local! {
    static B: RefCell<Batch> = RefCell::new(Batch { v: Vec::with_capacity(MAX_BATCH), i: Vec::with_capacity(MAX_BATCH * 3 / 2), atlas: None, white: Vec2::ZERO });
}

/// Подготовить батч к кадру (атлас нужен для текстур).
pub fn begin(atlas: &Texture2D) {
    B.with(|b| {
        let mut b = b.borrow_mut();
        if b.atlas.is_none() {
            b.atlas = Some(atlas.clone());
            let (u0, v0, u1, v1) = tile_uv(blocks::white_tile());
            b.white = vec2((u0 + u1) / 2.0, (v0 + v1) / 2.0);
        }
    });
}

/// Отрисовать всё накопленное.
pub fn flush() {
    B.with(|b| {
        let mut b = b.borrow_mut();
        if b.i.is_empty() {
            return;
        }
        let mesh = Mesh { vertices: std::mem::take(&mut b.v), indices: std::mem::take(&mut b.i), texture: b.atlas.clone() };
        draw_mesh(&mesh);
        b.v = mesh.vertices;
        b.i = mesh.indices;
        b.v.clear();
        b.i.clear();
    });
}

fn quad(p: [Vec3; 4], uv: [Vec2; 4], c: Color, two_sided: bool) {
    let full = B.with(|b| b.borrow().v.len() + 4 > MAX_BATCH);
    if full {
        flush();
    }
    B.with(|b| {
        let mut b = b.borrow_mut();
        let base = b.v.len() as u16;
        for k in 0..4 {
            b.v.push(Vertex::new2(p[k], uv[k], c));
        }
        b.i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        if two_sided {
            b.i.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    });
}

fn white_uv() -> Vec2 {
    B.with(|b| b.borrow().white)
}

/// (углы, яркость, индекс плитки: 0 — верх, 1 — бок, 2 — низ, 3 — перед)
const FACES: [([[f32; 3]; 4], f32, usize); 6] = [
    ([[-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5]], 1.0, 0),
    ([[-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5]], 0.5, 2),
    ([[0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5]], 0.86, 3),
    ([[-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5]], 0.78, 1),
    ([[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]], 0.7, 1),
    ([[0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5]], 0.7, 1),
];

fn col(c: (u8, u8, u8)) -> Color {
    Color::from_rgba(c.0, c.1, c.2, 255)
}

fn scale_c(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

/// Одноцветный куб.
pub fn cube(m: Mat4, color: Color) {
    let w = white_uv();
    for (corners, k, _) in FACES {
        let p = corners.map(|c| m.transform_point3(Vec3::from(c)));
        quad(p, [w; 4], scale_c(color, k), false);
    }
}

/// Куб с плитками: [верх, бок, низ, перед].
pub fn tex_box(m: Mat4, tiles: [u16; 4], tint: Color) {
    for (corners, k, ti) in FACES {
        let (u0, v0, u1, v1) = tile_uv(tiles[ti]);
        let uv = [vec2(u0, v1), vec2(u1, v1), vec2(u1, v0), vec2(u0, v0)];
        let p = corners.map(|c| m.transform_point3(Vec3::from(c)));
        quad(p, uv, scale_c(tint, k), false);
    }
}

/// Куб с текстурами блока (верх, бок, низ).
pub fn tex_cube(_atlas: &Texture2D, m: Mat4, tex: [u16; 3], tint: Color) {
    tex_box(m, [tex[0], tex[1], tex[2], tex[1]], tint);
}

/// Плоский спрайт (квадрат в плоскости XY, виден с обеих сторон).
pub fn sprite(_atlas: &Texture2D, m: Mat4, tile: u16, tint: Color) {
    let (u0, v0, u1, v1) = tile_uv(tile);
    let p = [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]].map(|c| m.transform_point3(Vec3::from(c)));
    quad(p, [vec2(u0, v1), vec2(u1, v1), vec2(u1, v0), vec2(u0, v0)], tint, true);
}

/// Мягкая тень на земле.
pub fn shadow(pos: Vec3, r: f32, alpha: f32) {
    let (u0, v0, u1, v1) = tile_uv(blocks::shadow_tile());
    let y = pos.y + 0.03;
    let p = [vec3(pos.x - r, y, pos.z + r), vec3(pos.x + r, y, pos.z + r), vec3(pos.x + r, y, pos.z - r), vec3(pos.x - r, y, pos.z - r)];
    quad(p, [vec2(u0, v1), vec2(u1, v1), vec2(u1, v0), vec2(u0, v0)], Color::new(1.0, 1.0, 1.0, alpha), false);
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
            let s = if matches!(other, Item::Tool(..)) { 0.75 } else { 0.5 };
            let r = Mat4::from_translation(vec3(0.0, 0.18, 0.0)) * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2) * Mat4::from_rotation_z(-0.8);
            sprite(atlas, m * r * Mat4::from_scale(Vec3::splat(s)), item_tile(other), tint);
        }
    }
}

fn rx(a: f32) -> Mat4 {
    Mat4::from_rotation_x(a)
}
fn ry(a: f32) -> Mat4 {
    Mat4::from_rotation_y(a)
}
fn rz(a: f32) -> Mat4 {
    Mat4::from_rotation_z(a)
}
fn t(x: f32, y: f32, z: f32) -> Mat4 {
    Mat4::from_translation(vec3(x, y, z))
}
fn s3(x: f32, y: f32, z: f32) -> Mat4 {
    Mat4::from_scale(vec3(x, y, z))
}

pub struct Anim {
    pub walk: f32,
    pub speed: f32,
    pub attack: f32,
    pub hurt: bool,
    pub dying: f32,
    pub t: f32,
    pub light: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
}

/// Тело существа № `mob` из таблицы.
pub fn draw_mob(mob: usize, pos: Vec3, yaw: f32, a: &Anim) {
    let def = &MOBS[mob];
    let [body_t, face_t, limb_t] = blocks::mob_tiles(mob);
    let glow = def.flags & GLOW != 0;
    let light = if glow { 1.0 } else { a.light };
    let tint = {
        let c = if a.hurt { Color::new(1.0, 0.45, 0.45, 1.0) } else { WHITE };
        scale_c(c, light)
    };
    let body = |m: Mat4| tex_box(m, [body_t, body_t, body_t, body_t], tint);
    let limb = |m: Mat4| tex_box(m, [limb_t, limb_t, limb_t, limb_t], tint);
    let head = |m: Mat4| tex_box(m, [body_t, body_t, body_t, face_t], tint);
    let accent = scale_c(col(def.c3), if glow { 1.0 } else { light });
    let dark = scale_c(col(def.c2), light * 0.55);

    let sp = a.speed.min(1.2);
    let sw = a.walk.sin() * 0.6 * sp;
    let atk = (a.attack * std::f32::consts::PI).sin();
    let breath = 1.0 + (a.t * 2.2).sin() * 0.018;
    let fly = def.flags & FLY != 0;
    let bob = if fly { (a.t * 3.0).sin() * 0.12 } else { (a.walk * 2.0).sin().abs() * 0.04 * sp };
    let mut root = Mat4::from_translation(pos + vec3(0.0, bob, 0.0)) * ry(-yaw);
    if a.dying > 0.0 {
        root *= rx(a.dying.min(1.0) * std::f32::consts::FRAC_PI_2);
    }
    root *= Mat4::from_scale(Vec3::splat(def.scale));
    let r = root;
    let hy = a.head_yaw.clamp(-0.9, 0.9);
    let hp = a.head_pitch.clamp(-0.6, 0.6);
    let look = ry(-hy) * rz(hp);

    match def.plan {
        Plan::Biped | Plan::Golem => {
            let g = def.plan == Plan::Golem;
            let (lw, lh) = if g { (0.32, 0.8) } else { (0.22, 0.75) };
            let bw = if g { 0.8 } else { 0.5 };
            let bh = if g { 0.9 } else { 0.72 };
            for (z, s) in [(-0.13f32, sw), (0.13, -sw)] {
                let leg = r * t(0.0, lh, z * if g { 1.4 } else { 1.0 }) * rz(s);
                limb(leg * t(0.0, -lh / 2.0, 0.0) * s3(lw, lh, lw));
                cube(leg * t(0.04, -lh + 0.05, 0.0) * s3(lw + 0.08, 0.1, lw + 0.02), dark);
            }
            let torso = r * t(0.0, lh + bh / 2.0, 0.0) * s3(1.0, breath, 1.0);
            body(torso * s3(0.34, bh, bw));
            if g {
                // наплечники и светящееся ядро
                body(r * t(0.0, lh + bh - 0.05, -bw / 2.0 - 0.05) * s3(0.4, 0.22, 0.3));
                body(r * t(0.0, lh + bh - 0.05, bw / 2.0 + 0.05) * s3(0.4, 0.22, 0.3));
                if glow {
                    cube(r * t(0.18, lh + bh * 0.55, 0.0) * s3(0.04, 0.18, 0.18), accent);
                }
            }
            let sh = lh + bh - 0.08;
            let arm_len = bh * if g { 1.15 } else { 0.95 };
            for (z, s, k) in [(-bw / 2.0 - 0.11, -sw - atk * 1.6, 1.0f32), (bw / 2.0 + 0.11, sw - atk * 1.2, -1.0)] {
                let arm = r * t(0.0, sh, z) * rz(s) * rx(0.06 * k);
                limb(arm * t(0.0, -arm_len / 2.0, 0.0) * s3(0.2, arm_len, 0.2));
                let hand = if g { scale_c(col(def.c2), light * 0.8) } else { scale_c(col(def.c3), light).with_alpha(1.0) };
                let hand = if glow && !g { scale_c(col(def.c2), light) } else { hand };
                cube(arm * t(0.0, -arm_len - 0.04, 0.0) * s3(0.21, 0.1, 0.21), hand);
            }
            let hs = if g { 0.5 } else { 0.46 };
            let hm = r * t(0.0, lh + bh + hs / 2.0, 0.0) * look;
            head(hm * Mat4::from_scale(Vec3::splat(hs)));
            if def.hab == crate::mobdefs::Hab::Village && def.friendly {
                // шляпа жителя
                let hat = scale_c(col(def.c2), light * 0.9);
                cube(hm * t(0.0, hs / 2.0 + 0.02, 0.0) * s3(hs + 0.16, 0.04, hs + 0.16), hat);
                cube(hm * t(0.0, hs / 2.0 + 0.1, 0.0) * s3(hs * 0.7, 0.14, hs * 0.7), hat);
            }
            if glow && !g {
                cube(hm * t(hs / 2.0 + 0.005, hs * 0.08, -0.1) * s3(0.02, 0.06, 0.07), accent);
                cube(hm * t(hs / 2.0 + 0.005, hs * 0.08, 0.1) * s3(0.02, 0.06, 0.07), accent);
            }
        }
        Plan::Quad => {
            let (bl, bh, bw) = (1.0, 0.55, 0.6);
            let leg_h = 0.45;
            let hop = def.flags & HOP != 0;
            for (lx, lz, ph) in [(0.36, 0.2, 1.0), (0.36, -0.2, -1.0), (-0.36, 0.2, -1.0), (-0.36, -0.2, 1.0)] {
                let ang = if hop { -(a.walk * 0.5).sin().abs() * 0.5 * ph } else { sw * ph };
                let leg = r * t(lx, leg_h, lz) * rz(ang);
                limb(leg * t(0.0, -leg_h / 2.0, 0.0) * s3(0.16, leg_h, 0.16));
                cube(leg * t(0.0, -leg_h + 0.04, 0.0) * s3(0.18, 0.08, 0.18), dark);
            }
            let torso = r * t(0.0, leg_h + bh / 2.0, 0.0) * s3(1.0, breath, 1.0);
            body(torso * s3(bl, bh, bw));
            if matches!(def.drop_icon, crate::icons::Icon::Mane) {
                // пушистая шерсть
                body(torso * s3(bl + 0.12, bh + 0.12, bw + 0.12));
            }
            // хвост
            let wag = (a.t * 7.0).sin() * 0.4;
            limb(r * t(-bl / 2.0, leg_h + bh * 0.8, 0.0) * ry(wag) * rz(0.7) * t(-0.12, 0.0, 0.0) * s3(0.26, 0.09, 0.09));
            // шея и голова
            let nod = (a.walk * 2.0).sin() * 0.04 - atk * 0.25;
            let hm = r * t(bl / 2.0 + 0.12, leg_h + bh + 0.05 + nod, 0.0) * look;
            head(hm * Mat4::from_scale(Vec3::splat(0.42)));
            limb(hm * t(0.24, -0.08, 0.0) * s3(0.14, 0.18, 0.24));
            let ear = (a.t * 3.0 + pos.x).sin().max(0.0) * 0.3;
            for z in [-0.14f32, 0.14] {
                body(hm * t(-0.06, 0.24, z) * rx(z.signum() * (0.3 + ear)) * s3(0.08, 0.16, 0.1));
            }
            if matches!(def.drop_icon, crate::icons::Icon::Horn) {
                let horn = scale_c(Color::from_rgba(230, 220, 190, 255), light);
                for z in [-0.12f32, 0.12] {
                    let base = hm * t(-0.02, 0.22, z) * rx(z.signum() * 0.4);
                    cube(base * t(0.0, 0.18, 0.0) * s3(0.05, 0.36, 0.05), horn);
                    cube(base * t(0.0, 0.3, 0.0) * rz(0.8) * t(0.0, 0.08, 0.0) * s3(0.04, 0.16, 0.04), horn);
                }
            }
        }
        Plan::Slime => {
            let hopv = (a.walk * 0.8).sin().abs();
            let sq = 1.0 + (1.0 - hopv) * 0.15;
            let m = r * t(0.0, 0.45 * (2.0 - sq) + hopv * 0.2, 0.0) * s3(0.9 * sq, 0.9 * (2.0 - sq), 0.9 * sq);
            limb(m * Mat4::from_scale(Vec3::splat(0.42)));
            let mut outer = tint;
            outer.a = 0.72;
            tex_box(m, [body_t, body_t, body_t, face_t], outer);
        }
        Plan::Bird => {
            let flap = if fly { (a.t * 14.0).sin() * 0.9 } else { (a.walk).sin() * 0.25 };
            let y0 = 0.3;
            if !fly {
                for (z, s) in [(-0.1f32, sw), (0.1, -sw)] {
                    let leg = r * t(0.0, y0, z) * rz(s);
                    cube(leg * t(0.0, -0.15, 0.0) * s3(0.05, 0.3, 0.05), accent);
                    cube(leg * t(0.05, -0.3, 0.0) * s3(0.14, 0.03, 0.1), accent);
                }
            }
            let torso = r * t(0.0, y0 + 0.2, 0.0) * s3(1.0, breath, 1.0);
            body(torso * s3(0.6, 0.38, 0.36));
            for (z, f) in [(-0.18f32, flap), (0.18, -flap)] {
                let wing = r * t(0.0, y0 + 0.32, z) * rx(f);
                body(wing * t(0.0, 0.0, z.signum() * 0.25) * s3(0.42, 0.05, 0.5));
                limb(wing * t(-0.06, 0.0, z.signum() * 0.55) * s3(0.3, 0.04, 0.18));
            }
            for k in 0..3 {
                let fan = (k as f32 - 1.0) * 0.35;
                limb(r * t(-0.34, y0 + 0.24, 0.0) * ry(fan) * rz(0.25) * t(-0.12, 0.0, 0.0) * s3(0.26, 0.04, 0.09));
            }
            let hm = r * t(0.34, y0 + 0.5, 0.0) * look;
            head(hm * Mat4::from_scale(Vec3::splat(0.3)));
            cube(hm * t(0.2, -0.04, 0.0) * s3(0.14, 0.07, 0.09), accent);
            if def.friendly {
                body(hm * t(-0.04, 0.18, 0.0) * rz(-0.4) * s3(0.12, 0.08, 0.04));
            }
        }
        Plan::Spider => {
            let torso = r * t(-0.35, 0.42, 0.0) * s3(1.0, breath, 1.0);
            body(torso * s3(0.62, 0.48, 0.58));
            body(r * t(0.12, 0.36, 0.0) * s3(0.36, 0.3, 0.38));
            head(r * t(0.36, 0.38, 0.0) * look * s3(0.24, 0.24, 0.3));
            cube(r * t(0.48, 0.27, -0.06) * rz(0.4) * s3(0.03, 0.1, 0.03), accent);
            cube(r * t(0.48, 0.27, 0.06) * rz(0.4) * s3(0.03, 0.1, 0.03), accent);
            for i in 0..4 {
                let x = 0.24 - i as f32 * 0.15;
                let w = (a.walk + i as f32 * 1.3).sin() * 0.3 * sp;
                for side in [-1.0f32, 1.0] {
                    let hip = r * t(x, 0.4, side * 0.16) * ry(side * (w + (i as f32 - 1.5) * 0.25)) * rx(side * -0.6);
                    limb(hip * t(0.0, 0.0, side * 0.22) * s3(0.06, 0.06, 0.44));
                    let knee = hip * t(0.0, 0.0, side * 0.42) * rx(side * 1.4);
                    limb(knee * t(0.0, 0.0, side * 0.2) * s3(0.05, 0.05, 0.42));
                }
            }
        }
        Plan::Fish => {
            let segs = 4;
            for i in 0..segs {
                let x = 0.45 - i as f32 * 0.32;
                let wig = (a.t * 6.0 - i as f32 * 0.9).sin() * 0.12 * i as f32 / segs as f32 * 2.0;
                let sz = 0.42 - i as f32 * 0.07;
                let m = r * t(x, 0.3, wig) * s3(0.34, sz, sz * 0.8);
                if i == 0 { head(m) } else { body(m) }
            }
            let tw = (a.t * 6.0).sin() * 0.5;
            limb(r * t(-0.85, 0.3, 0.0) * ry(tw) * t(-0.12, 0.0, 0.0) * s3(0.25, 0.36, 0.05));
            limb(r * t(0.0, 0.55, 0.0) * rz(-0.3) * s3(0.3, 0.16, 0.04));
            for z in [-0.2f32, 0.2] {
                limb(r * t(0.3, 0.2, z) * rx(z.signum() * (0.6 + (a.t * 5.0).sin() * 0.3)) * s3(0.16, 0.03, 0.14));
            }
        }
        Plan::Wisp => {
            let pulse = 1.0 + (a.t * 4.0).sin() * 0.08;
            let core = r * t(0.0, 0.6, 0.0) * ry(a.t) * Mat4::from_scale(Vec3::splat(0.5 * pulse));
            tex_box(core, [body_t, body_t, body_t, face_t], WHITE);
            let mut halo = col(def.c2);
            halo.a = 0.35;
            cube(r * t(0.0, 0.6, 0.0) * ry(-a.t * 0.7) * Mat4::from_scale(Vec3::splat(0.78 * pulse)), halo);
            for k in 0..4 {
                let ang = a.t * 2.0 + k as f32 * 1.571;
                let p = vec3(ang.cos() * 0.62, 0.6 + (a.t * 3.0 + k as f32).sin() * 0.18, ang.sin() * 0.62);
                cube(r * Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.09)), accent);
            }
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
    pub hair: Color,
}

pub fn look_for(seed: u32) -> PlayerLook {
    let h = |k| crate::blocks::hash(seed as i32, k, 4242);
    let shirts = [(60, 120, 200), (200, 70, 60), (70, 160, 90), (230, 170, 40), (140, 80, 190), (40, 40, 50)];
    let sc = shirts[(h(1) * 6.0) as usize % 6];
    let skin = [(236, 190, 150), (200, 150, 110), (150, 100, 70), (250, 210, 180)][(h(2) * 4.0) as usize % 4];
    let hair = [(70, 46, 30), (30, 26, 24), (200, 160, 80), (150, 60, 30)][(h(3) * 4.0) as usize % 4];
    PlayerLook { skin: col(skin), shirt: col(sc), pants: col((50, 50, 90)), hair: col(hair) }
}

/// Модель игрока (вид от третьего лица и другие игроки по сети).
#[allow(clippy::too_many_arguments)]
pub fn draw_player(atlas: &Texture2D, pos: Vec3, yaw: f32, pitch: f32, walk: f32, speed: f32, swing: f32, held: Item, armor: &[Item; 4], look: &PlayerLook, light: f32, hurt: bool) {
    let lit = |c: Color| {
        let c = if hurt { Color::new(c.r * 0.4 + 0.6, c.g * 0.5, c.b * 0.5, 1.0) } else { c };
        scale_c(c, light)
    };
    shadow(pos, 0.45, 0.7);
    let r = Mat4::from_translation(pos) * ry(-yaw);
    let sw = walk.sin() * 0.7 * speed.min(1.0);
    let atk = (swing * std::f32::consts::PI).sin();
    let legs = lit(armor_color(armor[2]).unwrap_or(look.pants));
    let boots = lit(armor_color(armor[3]).unwrap_or(Color::from_rgba(60, 40, 30, 255)));
    let shirt = lit(armor_color(armor[1]).unwrap_or(look.shirt));
    let skin = lit(look.skin);
    for (z, s) in [(-0.13f32, sw), (0.13, -sw)] {
        let leg = r * t(0.0, 0.75, z) * rz(s);
        cube(leg * t(0.0, -0.3, 0.0) * s3(0.24, 0.6, 0.24), legs);
        cube(leg * t(0.03, -0.66, 0.0) * s3(0.3, 0.18, 0.26), boots);
    }
    cube(r * t(0.0, 1.11, 0.0) * s3(0.28, 0.72, 0.5), shirt);
    cube(r * t(0.0, 0.8, 0.0) * s3(0.3, 0.08, 0.52), lit(Color::from_rgba(60, 40, 30, 255)));
    let la = r * t(0.0, 1.42, -0.36) * rz(-sw);
    cube(la * t(0.0, -0.3, 0.0) * s3(0.2, 0.62, 0.2), shirt);
    cube(la * t(0.0, -0.64, 0.0) * s3(0.2, 0.12, 0.2), skin);
    let arm_rot = rz(sw * 0.5 - atk * 1.8 - if held != Item::Empty { 0.35 } else { 0.0 });
    let ra = r * t(0.0, 1.42, 0.36) * arm_rot;
    cube(ra * t(0.0, -0.3, 0.0) * s3(0.2, 0.62, 0.2), shirt);
    cube(ra * t(0.0, -0.64, 0.0) * s3(0.21, 0.12, 0.21), skin);
    held_item(atlas, ra * t(0.12, -0.66, 0.0), held, Color::new(light, light, light, 1.0));
    let hm = r * t(0.0, 1.72, 0.0) * rz(pitch);
    cube(hm * Mat4::from_scale(Vec3::splat(0.48)), skin);
    cube(hm * t(-0.03, 0.17, 0.0) * s3(0.5, 0.16, 0.5), lit(look.hair));
    cube(hm * t(-0.2, 0.0, 0.0) * s3(0.12, 0.4, 0.5), lit(look.hair));
    for z in [-0.1f32, 0.1] {
        cube(hm * t(0.245, 0.03, z) * s3(0.02, 0.07, 0.08), lit(Color::from_rgba(250, 250, 250, 255)));
        cube(hm * t(0.25, 0.03, z + 0.02 * z.signum() * -1.0) * s3(0.02, 0.06, 0.04), lit(Color::from_rgba(30, 40, 80, 255)));
    }
    cube(hm * t(0.25, -0.12, 0.0) * s3(0.02, 0.03, 0.14), lit(Color::from_rgba(150, 80, 70, 255)));
    if let Some(hc) = armor_color(armor[0]) {
        cube(hm * t(-0.02, 0.12, 0.0) * s3(0.54, 0.3, 0.54), lit(hc));
    }
}

/// Рука и предмет от первого лица.
#[allow(clippy::too_many_arguments)]
pub fn draw_first_person(atlas: &Texture2D, eye: Vec3, fwd: Vec3, right: Vec3, up: Vec3, held: Item, swing: f32, bob: Vec2, eat: f32, sleeve: Color, skin: Color, light: f32) {
    let atk = (swing * std::f32::consts::PI).sin();
    let eat_bob = if eat > 0.0 { (eat * 22.0).sin().abs() * 0.05 } else { 0.0 };
    let base = eye + fwd * 0.5 + right * (0.3 - atk * 0.12 - if eat > 0.0 { 0.2 } else { 0.0 }) + up * (-0.26 + bob.y + eat_bob + atk * 0.08) + right * bob.x;
    let cam = Mat4::from_cols(fwd.extend(0.0), up.extend(0.0), right.extend(0.0), base.extend(1.0));
    let tilt = rz(-0.25 - atk * 1.1) * ry(0.35 + atk * 0.3);
    let m = cam * tilt * Mat4::from_scale(Vec3::splat(0.42));
    let k = light;
    let sl = Color::new(sleeve.r * k, sleeve.g * k, sleeve.b * k, 1.0);
    let sk = Color::new(skin.r * k, skin.g * k, skin.b * k, 1.0);
    if held == Item::Empty {
        cube(m * t(0.0, -0.25, 0.0) * s3(0.16, 0.5, 0.16), sl);
        cube(m * t(0.0, 0.05, 0.0) * s3(0.17, 0.14, 0.17), sk);
    } else {
        cube(m * t(-0.05, -0.3, 0.05) * s3(0.15, 0.45, 0.15), sl);
        cube(m * t(-0.05, -0.03, 0.05) * s3(0.16, 0.12, 0.16), sk);
        held_item(atlas, m * t(0.05, 0.05, 0.0), held, Color::new(k, k, k, 1.0));
    }
}

#[allow(dead_code)]
pub fn mat_color(m: Mat) -> Color {
    let (c, _, _) = m.colors();
    Color::from_rgba(c[0], c[1], c[2], 255)
}
