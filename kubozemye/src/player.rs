//! Игрок: обзор мышью, ходьба, прыжки, плавание, полёт и столкновения.

use crate::world::{Block, World};
use macroquad::prelude::*;

pub const HALF_W: f32 = 0.3;
pub const HEIGHT: f32 = 1.8;
pub const EYE: f32 = 1.62;

pub struct Player {
    /// Центр ступней.
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub flying: bool,
    pub in_water: bool,
}

impl Player {
    pub fn new(pos: Vec3) -> Self {
        Player { pos, vel: Vec3::ZERO, yaw: 0.6, pitch: -0.15, on_ground: false, flying: false, in_water: false }
    }

    pub fn eye(&self) -> Vec3 {
        self.pos + vec3(0.0, EYE, 0.0)
    }

    pub fn forward(&self) -> Vec3 {
        vec3(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos())
    }

    pub fn look(&mut self, dx: f32, dy: f32) {
        const SENS: f32 = 0.0026;
        self.yaw += dx * SENS;
        self.pitch = (self.pitch - dy * SENS).clamp(-1.55, 1.55);
    }

    /// Пересекается ли тело игрока с твёрдыми блоками в позиции `p`.
    pub fn collides_at(world: &World, p: Vec3) -> bool {
        let min = p - vec3(HALF_W, 0.0, HALF_W);
        let max = p + vec3(HALF_W, HEIGHT, HALF_W);
        for y in min.y.floor() as i32..=(max.y - 1e-4).floor() as i32 {
            for z in min.z.floor() as i32..=(max.z - 1e-4).floor() as i32 {
                for x in min.x.floor() as i32..=(max.x - 1e-4).floor() as i32 {
                    if world.get(x, y, z).is_solid() {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Пересекает ли тело игрока клетку блока.
    pub fn overlaps_block(&self, b: IVec3) -> bool {
        let min = self.pos - vec3(HALF_W, 0.0, HALF_W);
        let max = self.pos + vec3(HALF_W, HEIGHT, HALF_W);
        let (bmin, bmax) = (b.as_vec3(), b.as_vec3() + Vec3::ONE);
        min.x < bmax.x && max.x > bmin.x && min.y < bmax.y && max.y > bmin.y && min.z < bmax.z && max.z > bmin.z
    }

    fn move_axis(&mut self, world: &World, axis: usize, delta: f32) {
        if delta == 0.0 {
            return;
        }
        let steps = (delta.abs() / 0.35).ceil().max(1.0) as i32;
        let step = delta / steps as f32;
        for _ in 0..steps {
            let before = self.pos;
            self.pos[axis] += step;
            if !Self::collides_at(world, self.pos) {
                continue;
            }
            // прижимаемся вплотную к стене блока
            const EPS: f32 = 1e-3;
            match (axis, step > 0.0) {
                (1, false) => {
                    self.pos.y = self.pos.y.floor() + 1.0;
                    self.on_ground = true;
                }
                (1, true) => self.pos.y = (self.pos.y + HEIGHT).floor() - HEIGHT - EPS,
                (a, true) => self.pos[a] = (self.pos[a] + HALF_W).floor() - HALF_W - EPS,
                (a, false) => self.pos[a] = (self.pos[a] - HALF_W).floor() + 1.0 + HALF_W + EPS,
            }
            if Self::collides_at(world, self.pos) {
                self.pos = before;
            }
            self.vel[axis] = 0.0;
            return;
        }
    }

    pub fn update(&mut self, world: &World, dt: f32) {
        let feet = self.pos + vec3(0.0, 0.4, 0.0);
        self.in_water = world.get(feet.x.floor() as i32, feet.y.floor() as i32, feet.z.floor() as i32) == Block::Water;

        // направление ходьбы
        let fwd = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
        let right = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
        let mut wish = Vec3::ZERO;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            wish += fwd;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            wish -= fwd;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            wish += right;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            wish -= right;
        }
        let wish = wish.normalize_or_zero();
        let sprint = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::R);
        let jump = is_key_down(KeyCode::Space);

        if self.flying {
            let speed = if sprint { 22.0 } else { 11.0 };
            let mut target = wish * speed;
            if jump {
                target.y = speed * 0.8;
            }
            if is_key_down(KeyCode::LeftShift) {
                target.y = -speed * 0.8;
            }
            self.vel = self.vel.lerp(target, (dt * 10.0).min(1.0));
        } else {
            let speed = if self.in_water { 3.0 } else if sprint { 7.0 } else { 4.6 };
            let target = wish * speed;
            let accel = if self.on_ground || self.in_water { 14.0 } else { 4.0 };
            let k = (dt * accel).min(1.0);
            self.vel.x += (target.x - self.vel.x) * k;
            self.vel.z += (target.z - self.vel.z) * k;

            if self.in_water {
                self.vel.y -= 9.0 * dt;
                self.vel.y *= 1.0 - (dt * 3.0).min(1.0);
                if jump {
                    self.vel.y = self.vel.y.max(3.6);
                }
            } else {
                self.vel.y -= 28.0 * dt;
                if jump && self.on_ground {
                    self.vel.y = 8.6;
                }
            }
            self.vel.y = self.vel.y.max(-50.0);
        }

        self.on_ground = false;
        let d = self.vel * dt;
        self.move_axis(world, 1, d.y);
        self.move_axis(world, 0, d.x);
        self.move_axis(world, 2, d.z);

        // невидимая стена по краю мира
        let lim = crate::world::WX as f32 - HALF_W - 0.01;
        self.pos.x = self.pos.x.clamp(HALF_W + 0.01, lim);
        self.pos.z = self.pos.z.clamp(HALF_W + 0.01, lim);

        // упал за край мира — возвращаемся на берег
        if self.pos.y < -20.0 {
            let (x, z) = (self.pos.x.clamp(1.0, 254.0), self.pos.z.clamp(1.0, 254.0));
            self.pos = vec3(x, world.surface_y(x as i32, z as i32) as f32 + 0.1, z);
            self.vel = Vec3::ZERO;
        }
    }
}
