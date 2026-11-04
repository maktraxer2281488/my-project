//! Существа: враждебные Мракун (боится солнца) и Прыгун (живёт в пещерах),
//! мирный Пушистик.

use crate::blocks::*;
use crate::player::{Body, Player};
use crate::world::{Dim, World};
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Shade,
    Hopper,
    Fluff,
}

impl Kind {
    pub fn hostile(self) -> bool {
        !matches!(self, Kind::Fluff)
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::Shade => "Мракун",
            Kind::Hopper => "Прыгун",
            Kind::Fluff => "Пушистик",
        }
    }
}

pub struct Mob {
    pub kind: Kind,
    pub body: Body,
    pub yaw: f32,
    pub health: f32,
    pub hurt_t: f32,
    pub attack_cd: f32,
    pub think_t: f32,
    pub wander: Vec3,
    pub phase: f32,
    pub flee_t: f32,
}

impl Mob {
    pub fn new(kind: Kind, pos: Vec3) -> Self {
        let (hw, h, hp) = match kind {
            Kind::Shade => (0.3, 1.9, 16.0),
            Kind::Hopper => (0.45, 0.9, 8.0),
            Kind::Fluff => (0.45, 1.0, 8.0),
        };
        Mob { kind, body: Body::new(pos, hw, h), yaw: 0.0, health: hp, hurt_t: 0.0, attack_cd: 1.0, think_t: 0.0, wander: Vec3::ZERO, phase: 0.0, flee_t: 0.0 }
    }

    /// Пересечение луча с телом: расстояние до попадания.
    pub fn ray_hit(&self, o: Vec3, d: Vec3) -> Option<f32> {
        let min = self.body.pos - vec3(self.body.hw, 0.0, self.body.hw);
        let max = self.body.pos + vec3(self.body.hw, self.body.h, self.body.hw);
        let (mut t0, mut t1) = (0.0f32, f32::MAX);
        for a in 0..3 {
            if d[a].abs() < 1e-6 {
                if o[a] < min[a] || o[a] > max[a] {
                    return None;
                }
            } else {
                let (mut ta, mut tb) = ((min[a] - o[a]) / d[a], (max[a] - o[a]) / d[a]);
                if ta > tb {
                    std::mem::swap(&mut ta, &mut tb);
                }
                t0 = t0.max(ta);
                t1 = t1.min(tb);
                if t0 > t1 {
                    return None;
                }
            }
        }
        Some(t0)
    }

    pub fn damage(&mut self, amount: f32, from: Vec3) {
        self.health -= amount;
        self.hurt_t = 0.3;
        let dir = (self.body.pos - from).with_y(0.0).normalize_or_zero();
        self.body.vel += dir * 7.0 + vec3(0.0, 5.0, 0.0);
        self.flee_t = 3.0;
    }
}

pub struct Mobs {
    pub list: Vec<Mob>,
    spawn_t: f32,
    rng: u32,
}

impl Mobs {
    pub fn new() -> Self {
        Mobs { list: Vec::new(), spawn_t: 0.0, rng: 0x1234_5678 }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0xffffff) as f32 / 16_777_215.0
    }

    fn try_spawn(&mut self, world: &World, player: &Player, daylight: f32, difficulty: u8) {
        let hostile = self.list.iter().filter(|m| m.kind.hostile()).count();
        let passive = self.list.len() - hostile;
        let ang = self.rand() * std::f32::consts::TAU;
        let dist = 22.0 + self.rand() * 26.0;
        let p = player.body.pos;
        let (x, z) = ((p.x + ang.cos() * dist).floor() as i32, (p.z + ang.sin() * dist).floor() as i32);
        if !world.is_loaded(x, z) {
            return;
        }
        let y0 = p.y as i32;
        for y in ((y0 - 24).max(1)..(y0 + 16).min(126)).rev() {
            let below = world.get(x, y - 1, z);
            if !is_solid(below) || world.get(x, y, z) != AIR || world.get(x, y + 1, z) != AIR {
                continue;
            }
            let exposed = world.sky_exposed(x, y, z);
            let pos = vec3(x as f32 + 0.5, y as f32, z as f32 + 0.5);
            let r = self.rand();
            if world.dim == Dim::Heaven {
                if passive < 5 && below == HEAVEN_GRASS {
                    self.list.push(Mob::new(Kind::Fluff, pos));
                }
                return;
            }
            if difficulty > 0 && hostile < 6 + difficulty as usize * 3 {
                if exposed && daylight < 0.3 && r < 0.8 {
                    self.list.push(Mob::new(Kind::Shade, pos));
                    return;
                }
                if !exposed {
                    let k = if r < 0.55 { Kind::Shade } else { Kind::Hopper };
                    self.list.push(Mob::new(k, pos));
                    return;
                }
            }
            if exposed && daylight > 0.5 && passive < 6 && below == GRASS && r < 0.35 {
                self.list.push(Mob::new(Kind::Fluff, pos));
            }
            return;
        }
    }

    /// Обновление; возвращает урон, нанесённый игроку.
    pub fn update(&mut self, world: &World, player: &mut Player, dt: f32, daylight: f32, difficulty: u8, creative: bool) {
        self.spawn_t -= dt;
        if self.spawn_t <= 0.0 {
            self.spawn_t = 0.7;
            self.try_spawn(world, player, daylight, difficulty);
        }
        if difficulty == 0 {
            self.list.retain(|m| !m.kind.hostile());
        }
        let ppos = player.body.pos;
        let dmg_k = [0.0, 0.5, 1.0, 1.5][difficulty.min(3) as usize];
        let rands: Vec<f32> = (0..self.list.len() * 3).map(|_| self.rand()).collect();

        for (i, m) in self.list.iter_mut().enumerate() {
            m.hurt_t = (m.hurt_t - dt).max(0.0);
            m.attack_cd -= dt;
            m.think_t -= dt;
            m.flee_t -= dt;
            let to = (ppos - m.body.pos).with_y(0.0);
            let dist = to.length();
            let dir_to = to.normalize_or_zero();
            let mut wish;
            let mut speed = 0.0;
            let chase = m.kind.hostile() && !creative && dist < 20.0 && (ppos.y - m.body.pos.y).abs() < 8.0;

            if chase {
                wish = dir_to;
            } else {
                if m.think_t <= 0.0 {
                    m.think_t = 2.0 + rands[i * 3] * 4.0;
                    let a = rands[i * 3 + 1] * std::f32::consts::TAU;
                    m.wander = if rands[i * 3 + 2] < 0.4 { Vec3::ZERO } else { vec3(a.cos(), 0.0, a.sin()) };
                }
                wish = m.wander;
                if m.flee_t > 0.0 {
                    wish = -dir_to;
                }
            }

            match m.kind {
                Kind::Shade => {
                    speed = if chase { 3.3 } else { 1.2 };
                    let (x, y, z) = (m.body.pos.x.floor() as i32, (m.body.pos.y + 1.5).floor() as i32, m.body.pos.z.floor() as i32);
                    if daylight > 0.55 && world.dim == Dim::Overworld && world.sky_exposed(x, y, z) {
                        m.health -= dt * 3.0;
                        m.hurt_t = m.hurt_t.max(0.05);
                    }
                }
                Kind::Hopper => {
                    // передвигается прыжками
                    if m.body.on_ground {
                        m.body.vel.x *= 0.6;
                        m.body.vel.z *= 0.6;
                        if m.attack_cd < 0.3 && wish != Vec3::ZERO {
                            m.body.vel = wish * if chase { 4.5 } else { 2.0 } + vec3(0.0, 7.5, 0.0);
                            m.attack_cd = 1.2;
                        }
                    }
                }
                Kind::Fluff => {
                    speed = if m.flee_t > 0.0 { 4.0 } else { 1.1 };
                }
            }

            if m.kind != Kind::Hopper {
                let k = (dt * if m.body.on_ground { 10.0 } else { 2.0 }).min(1.0);
                m.body.vel.x += (wish.x * speed - m.body.vel.x) * k;
                m.body.vel.z += (wish.z * speed - m.body.vel.z) * k;
                if m.body.hit_wall && m.body.on_ground && wish != Vec3::ZERO {
                    m.body.vel.y = 8.4;
                }
            }
            if wish != Vec3::ZERO {
                let target = wish.z.atan2(wish.x);
                let mut d = target - m.yaw;
                while d > std::f32::consts::PI {
                    d -= std::f32::consts::TAU;
                }
                while d < -std::f32::consts::PI {
                    d += std::f32::consts::TAU;
                }
                m.yaw += d * (dt * 8.0).min(1.0);
            }
            let in_water = world.get(m.body.pos.x.floor() as i32, (m.body.pos.y + 0.4).floor() as i32, m.body.pos.z.floor() as i32) == WATER;
            if in_water {
                m.body.vel.y = (m.body.vel.y + 14.0 * dt).min(2.5);
            } else {
                m.body.vel.y -= 28.0 * dt;
            }
            m.body.step(world, dt);
            m.phase += dt * vec2(m.body.vel.x, m.body.vel.z).length() * 2.2;
            if m.body.touching(world, 0.0).contains(&LAVA) {
                m.health -= dt * 6.0;
            }

            // атака
            if m.kind.hostile() && !creative {
                let dy = ppos.y - m.body.pos.y;
                let reach = if m.kind == Kind::Hopper { 1.0 } else { 1.3 };
                if dist < reach + m.body.hw && dy.abs() < 1.6 && m.attack_cd <= 0.0 {
                    let base = if m.kind == Kind::Shade { 3.0 } else { 2.0 };
                    player.hurt(base * dmg_k, dir_to * 6.0 + vec3(0.0, 4.0, 0.0), false);
                    m.attack_cd = 1.0;
                }
            }
        }
        self.list.retain(|m| {
            let d = (m.body.pos - ppos).length();
            m.health > 0.0 && d < 110.0 && m.body.pos.y > -20.0
        });
    }

    pub fn draw(&self, world: &World, daylight: f32, t: f32) {
        for m in &self.list {
            let p = m.body.pos;
            let (x, y, z) = (p.x.floor() as i32, (p.y + 1.0).floor() as i32, p.z.floor() as i32);
            let exposed = world.sky_exposed(x, y, z);
            let light = if exposed { daylight.max(0.2) } else { 0.28 };
            let hurt = m.hurt_t > 0.0;
            let tint = |c: Color| -> Color {
                let c = if hurt { Color::new(c.r * 0.4 + 0.6, c.g * 0.5, c.b * 0.5, c.a) } else { c };
                Color::new(c.r * light, c.g * light, c.b * light, c.a)
            };
            let glow = |c: Color| c; // светящиеся детали не темнеют
            let swing = (m.phase).sin() * 0.25;
            match m.kind {
                Kind::Shade => {
                    let body = Color::from_rgba(52, 38, 78, 255);
                    let dark = Color::from_rgba(30, 22, 46, 255);
                    draw_box(p, m.yaw, vec3(0.0, 0.0, -0.12), vec3(0.22, 0.85, 0.22), vec3(swing, 0.0, 0.0), tint(dark));
                    draw_box(p, m.yaw, vec3(0.0, 0.0, 0.12), vec3(0.22, 0.85, 0.22), vec3(-swing, 0.0, 0.0), tint(dark));
                    draw_box(p, m.yaw, vec3(0.0, 0.85, 0.0), vec3(0.32, 0.6, 0.52), Vec3::ZERO, tint(body));
                    draw_box(p, m.yaw, vec3(0.05, 0.85, -0.38), vec3(0.2, 0.55, 0.18), vec3(0.0, 0.0, 0.0), tint(dark));
                    draw_box(p, m.yaw, vec3(0.05, 0.85, 0.38), vec3(0.2, 0.55, 0.18), vec3(0.0, 0.0, 0.0), tint(dark));
                    draw_box(p, m.yaw, vec3(0.0, 1.45, 0.0), vec3(0.46, 0.44, 0.46), Vec3::ZERO, tint(body));
                    let pulse = 0.75 + (t * 4.0).sin() * 0.25;
                    let eye = glow(Color::new(0.45 * pulse, 0.95 * pulse, 1.0, 1.0));
                    draw_box(p, m.yaw, vec3(0.235, 1.62, -0.11), vec3(0.02, 0.07, 0.1), Vec3::ZERO, eye);
                    draw_box(p, m.yaw, vec3(0.235, 1.62, 0.11), vec3(0.02, 0.07, 0.1), Vec3::ZERO, eye);
                }
                Kind::Hopper => {
                    let squash = if m.body.on_ground { 1.0 } else { 1.12 };
                    let s = vec3(0.9, 0.86 * squash, 0.9);
                    draw_box(p, m.yaw, vec3(0.0, 0.2, 0.0), vec3(0.4, 0.4, 0.4), Vec3::ZERO, tint(Color::from_rgba(40, 110, 50, 255)));
                    draw_box(p, m.yaw, Vec3::ZERO, s, Vec3::ZERO, tint(Color::from_rgba(110, 210, 90, 170)));
                    let eye = tint(Color::from_rgba(20, 30, 20, 255));
                    draw_box(p, m.yaw, vec3(0.455, 0.55, -0.18), vec3(0.02, 0.14, 0.12), Vec3::ZERO, eye);
                    draw_box(p, m.yaw, vec3(0.455, 0.55, 0.18), vec3(0.02, 0.14, 0.12), Vec3::ZERO, eye);
                }
                Kind::Fluff => {
                    let wool = Color::from_rgba(244, 240, 232, 255);
                    let leg = Color::from_rgba(70, 60, 56, 255);
                    for (lx, lz, ph) in [(0.35, 0.22, 1.0), (0.35, -0.22, -1.0), (-0.35, 0.22, -1.0), (-0.35, -0.22, 1.0)] {
                        draw_box(p, m.yaw, vec3(lx, 0.0, lz), vec3(0.16, 0.4, 0.16), vec3(swing * ph, 0.0, 0.0), tint(leg));
                    }
                    draw_box(p, m.yaw, vec3(0.0, 0.35, 0.0), vec3(1.05, 0.62, 0.8), Vec3::ZERO, tint(wool));
                    draw_box(p, m.yaw, vec3(0.6, 0.62, 0.0), vec3(0.36, 0.36, 0.4), Vec3::ZERO, tint(Color::from_rgba(150, 110, 90, 255)));
                    draw_box(p, m.yaw, vec3(0.55, 0.92, 0.0), vec3(0.26, 0.12, 0.46), Vec3::ZERO, tint(wool));
                    let eye = tint(Color::from_rgba(20, 20, 20, 255));
                    draw_box(p, m.yaw, vec3(0.785, 0.78, -0.1), vec3(0.01, 0.06, 0.06), Vec3::ZERO, eye);
                    draw_box(p, m.yaw, vec3(0.785, 0.78, 0.1), vec3(0.01, 0.06, 0.06), Vec3::ZERO, eye);
                }
            }
        }
    }
}

/// Рисует повёрнутый по `yaw` параллелепипед. `local` — смещение низа-центра
/// в системе существа (x — вперёд), `swing` — поворот вокруг оси z у верхней грани.
fn draw_box(origin: Vec3, yaw: f32, local: Vec3, size: Vec3, swing: Vec3, color: Color) {
    let (hx, hz) = (size.x / 2.0, size.z / 2.0);
    let mut corners = [Vec3::ZERO; 8];
    for i in 0..8 {
        let x = if i & 1 == 0 { -hx } else { hx };
        let y = if i & 2 == 0 { 0.0 } else { size.y };
        let z = if i & 4 == 0 { -hz } else { hz };
        let mut c = vec3(x, y - size.y, z);
        // качание конечности вокруг верхней точки
        let (s, co) = swing.x.sin_cos();
        c = vec3(c.x * co - c.y * s, c.x * s + c.y * co, c.z);
        c.y += size.y;
        let p = local + c;
        let (sy, cy) = yaw.sin_cos();
        corners[i] = origin + vec3(p.x * cy - p.z * sy, p.y, p.x * sy + p.z * cy);
    }
    let faces: [([usize; 4], f32); 6] = [
        ([2, 3, 7, 6], 1.0),
        ([0, 4, 5, 1], 0.5),
        ([1, 5, 7, 3], 0.82),
        ([0, 2, 6, 4], 0.82),
        ([4, 6, 7, 5], 0.68),
        ([0, 1, 3, 2], 0.68),
    ];
    let mut verts = Vec::with_capacity(24);
    let mut idx = Vec::with_capacity(36);
    for (f, k) in faces {
        let c = Color::new(color.r * k, color.g * k, color.b * k, color.a);
        let b = verts.len() as u16;
        for &i in &f {
            verts.push(Vertex::new2(corners[i], Vec2::ZERO, c));
        }
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    draw_mesh(&Mesh { vertices: verts, indices: idx, texture: None });
}
