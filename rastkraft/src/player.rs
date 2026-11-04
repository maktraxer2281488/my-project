//! Физическое тело (игрок и существа) и сам игрок: здоровье, броня, движение.

use crate::blocks::{self, *};
use crate::items::{Item, Slot, ToolKind, Mat};
use crate::world::World;
use macroquad::prelude::*;

pub struct Body {
    pub pos: Vec3,
    pub vel: Vec3,
    pub hw: f32,
    pub h: f32,
    pub on_ground: bool,
    pub hit_wall: bool,
}

impl Body {
    pub fn new(pos: Vec3, hw: f32, h: f32) -> Self {
        Body { pos, vel: Vec3::ZERO, hw, h, on_ground: false, hit_wall: false }
    }

    pub fn collides_at(&self, world: &World, p: Vec3) -> bool {
        let min = p - vec3(self.hw, 0.0, self.hw);
        let max = p + vec3(self.hw, self.h, self.hw);
        for y in min.y.floor() as i32..=(max.y - 1e-4).floor() as i32 {
            for z in min.z.floor() as i32..=(max.z - 1e-4).floor() as i32 {
                for x in min.x.floor() as i32..=(max.x - 1e-4).floor() as i32 {
                    if world.solid_at(x, y, z) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Пересекает ли тело клетку блока.
    pub fn overlaps_block(&self, b: IVec3) -> bool {
        let min = self.pos - vec3(self.hw, 0.0, self.hw);
        let max = self.pos + vec3(self.hw, self.h, self.hw);
        let (bmin, bmax) = (b.as_vec3(), b.as_vec3() + Vec3::ONE);
        min.x < bmax.x && max.x > bmin.x && min.y < bmax.y && max.y > bmin.y && min.z < bmax.z && max.z > bmin.z
    }

    /// Какие блоки касаются тела (с небольшим запасом).
    pub fn touching(&self, world: &World, pad: f32) -> Vec<u8> {
        let min = self.pos - vec3(self.hw + pad, pad, self.hw + pad);
        let max = self.pos + vec3(self.hw + pad, self.h + pad, self.hw + pad);
        let mut out = Vec::new();
        for y in min.y.floor() as i32..=max.y.floor() as i32 {
            for z in min.z.floor() as i32..=max.z.floor() as i32 {
                for x in min.x.floor() as i32..=max.x.floor() as i32 {
                    out.push(world.get(x, y, z));
                }
            }
        }
        out
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
            if !self.collides_at(world, self.pos) {
                continue;
            }
            const EPS: f32 = 1e-3;
            match (axis, step > 0.0) {
                (1, false) => {
                    self.pos.y = self.pos.y.floor() + 1.0;
                    self.on_ground = true;
                }
                (1, true) => self.pos.y = (self.pos.y + self.h).floor() - self.h - EPS,
                (a, true) => self.pos[a] = (self.pos[a] + self.hw).floor() - self.hw - EPS,
                (a, false) => self.pos[a] = (self.pos[a] - self.hw).floor() + 1.0 + self.hw + EPS,
            }
            if self.collides_at(world, self.pos) {
                self.pos = before;
            }
            if axis != 1 {
                self.hit_wall = true;
            }
            self.vel[axis] = 0.0;
            return;
        }
    }

    pub fn step(&mut self, world: &World, dt: f32) {
        self.on_ground = false;
        self.hit_wall = false;
        let d = self.vel * dt;
        self.move_axis(world, 1, d.y);
        self.move_axis(world, 0, d.x);
        self.move_axis(world, 2, d.z);
    }
}

pub const EYE: f32 = 1.62;
pub const MAX_HEALTH: f32 = 20.0;

pub struct Player {
    pub body: Body,
    pub yaw: f32,
    pub pitch: f32,
    pub flying: bool,
    pub in_water: bool,
    pub eye_in_water: bool,
    pub health: f32,
    pub hurt_t: f32,
    pub invuln: f32,
    pub regen_t: f32,
    pub fall_start: Option<f32>,
    pub hazard_t: f32,
    pub hotbar: [Item; 9],
    pub selected: usize,
    pub armor: [Item; 4],
    pub walk_phase: f32,
}

impl Player {
    pub fn new(pos: Vec3) -> Self {
        Player {
            body: Body::new(pos, 0.3, 1.8),
            yaw: 0.6,
            pitch: -0.1,
            flying: false,
            in_water: false,
            eye_in_water: false,
            health: MAX_HEALTH,
            hurt_t: 0.0,
            invuln: 0.0,
            regen_t: 0.0,
            fall_start: None,
            hazard_t: 0.0,
            hotbar: [
                Item::Tool(ToolKind::Sword, Mat::Iron),
                Item::Tool(ToolKind::Pickaxe, Mat::Iron),
                Item::Block(TORCH),
                Item::Block(OAK_PLANKS),
                Item::Block(COBBLE),
                Item::Block(GLASS),
                Item::Block(STONE_BRICKS),
                Item::Block(GLOWSTONE),
                Item::Tool(ToolKind::Axe, Mat::Stone),
            ],
            selected: 0,
            armor: [Item::Empty; 4],
            walk_phase: 0.0,
        }
    }

    pub fn eye(&self) -> Vec3 {
        self.body.pos + vec3(0.0, EYE, 0.0)
    }

    pub fn forward(&self) -> Vec3 {
        vec3(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos())
    }

    pub fn held(&self) -> Item {
        self.hotbar[self.selected]
    }

    pub fn armor_points(&self) -> i32 {
        self.armor.iter().map(|a| a.armor_points()).sum()
    }

    pub fn equip(&mut self, item: Item) {
        if let Item::Armor(slot, _) = item {
            let i = match slot {
                Slot::Helmet => 0,
                Slot::Chest => 1,
                Slot::Legs => 2,
                Slot::Boots => 3,
            };
            self.armor[i] = item;
        }
    }

    /// Получить урон с учётом брони. Возвращает true, если урон прошёл.
    pub fn hurt(&mut self, amount: f32, knock: Vec3, ignore_armor: bool) -> bool {
        if self.invuln > 0.0 || amount <= 0.0 {
            return false;
        }
        let k = if ignore_armor { 1.0 } else { 1.0 - (self.armor_points().min(20) as f32) * 0.04 };
        self.health -= amount * k;
        self.hurt_t = 0.35;
        self.invuln = 0.5;
        self.regen_t = 0.0;
        self.body.vel += knock;
        true
    }

    pub fn look(&mut self, dx: f32, dy: f32, sens: f32) {
        let s = 0.0026 * sens;
        self.yaw += dx * s;
        self.pitch = (self.pitch - dy * s).clamp(-1.55, 1.55);
    }

    pub fn update(&mut self, world: &World, dt: f32, creative: bool, can_move: bool) {
        let b = &self.body;
        let feet = b.pos + vec3(0.0, 0.3, 0.0);
        let at = |p: Vec3| world.get(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let feet_b = at(feet);
        self.in_water = feet_b == WATER || feet_b == LAVA;
        let in_lava = self.body.touching(world, 0.0).contains(&LAVA);
        self.eye_in_water = at(self.eye()) == WATER;
        let in_web = self.body.touching(world, 0.0).contains(&COBWEB);

        let fwd = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
        let right = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
        let mut wish = Vec3::ZERO;
        let mut jump = false;
        let mut sprint = false;
        let mut sneak = false;
        if can_move {
            if is_key_down(KeyCode::W) {
                wish += fwd;
            }
            if is_key_down(KeyCode::S) {
                wish -= fwd;
            }
            if is_key_down(KeyCode::D) {
                wish += right;
            }
            if is_key_down(KeyCode::A) {
                wish -= right;
            }
            jump = is_key_down(KeyCode::Space);
            sprint = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::R);
            sneak = is_key_down(KeyCode::LeftShift);
        }
        let wish = wish.normalize_or_zero();
        if !creative {
            self.flying = false;
        }

        if self.flying {
            let speed = if sprint { 24.0 } else { 11.0 };
            let mut target = wish * speed;
            if jump {
                target.y = speed * 0.75;
            }
            if sneak {
                target.y = -speed * 0.75;
            }
            self.body.vel = self.body.vel.lerp(target, (dt * 10.0).min(1.0));
        } else {
            let mut speed = if self.in_water { 3.0 } else if sprint { 6.6 } else if sneak { 1.8 } else { 4.4 };
            if in_web {
                speed *= 0.25;
            }
            let target = wish * speed;
            let accel = if self.body.on_ground || self.in_water { 14.0 } else { 3.5 };
            let k = (dt * accel).min(1.0);
            self.body.vel.x += (target.x - self.body.vel.x) * k;
            self.body.vel.z += (target.z - self.body.vel.z) * k;
            if self.in_water {
                self.body.vel.y -= 9.0 * dt;
                self.body.vel.y *= 1.0 - (dt * 3.0).min(1.0);
                if jump {
                    self.body.vel.y = self.body.vel.y.max(3.4);
                }
            } else {
                self.body.vel.y -= 28.0 * dt;
                if jump && self.body.on_ground {
                    self.body.vel.y = 8.6;
                }
            }
            if in_web {
                self.body.vel.y = self.body.vel.y.max(-1.5);
            }
            self.body.vel.y = self.body.vel.y.max(-55.0);
        }

        // аккуратный шаг с края в присяде
        let was_ground = self.body.on_ground;
        let prev = self.body.pos;
        self.body.step(world, dt);
        if sneak && was_ground && !self.flying && !self.body.on_ground && self.body.vel.y <= 0.0 {
            let probe = self.body.pos - vec3(0.0, 0.05, 0.0);
            if !self.body.collides_at(world, probe) {
                self.body.pos.x = prev.x;
                self.body.pos.z = prev.z;
                self.body.vel.x = 0.0;
                self.body.vel.z = 0.0;
                self.body.on_ground = true;
            }
        }
        let moving = vec2(self.body.vel.x, self.body.vel.z).length();
        if self.body.on_ground && moving > 0.5 {
            self.walk_phase += dt * moving * 1.9;
        }

        // урон от падения
        if creative || self.flying || self.in_water || in_web {
            self.fall_start = None;
        } else if !self.body.on_ground {
            let y = self.body.pos.y;
            self.fall_start = Some(self.fall_start.map_or(y, |s| s.max(y)));
        } else if let Some(s) = self.fall_start.take() {
            let h = s - self.body.pos.y;
            if h > 3.5 {
                self.hurt((h - 3.0).floor(), Vec3::ZERO, true);
            }
        }

        // лава и кактусы
        self.hazard_t -= dt;
        if !creative && self.hazard_t <= 0.0 {
            if in_lava {
                self.hurt(4.0, Vec3::ZERO, false);
                self.hazard_t = 0.5;
            } else if self.body.touching(world, 0.05).contains(&CACTUS) {
                self.hurt(1.0, Vec3::ZERO, false);
                self.hazard_t = 0.5;
            }
        }

        // восстановление здоровья
        self.hurt_t = (self.hurt_t - dt).max(0.0);
        self.invuln = (self.invuln - dt).max(0.0);
        self.regen_t += dt;
        if self.regen_t > 4.0 && self.health < MAX_HEALTH {
            self.health = (self.health + 1.0).min(MAX_HEALTH);
            self.regen_t = 3.0;
        }
        if creative {
            self.health = MAX_HEALTH;
        }
    }
}

/// Время ломания блока данным предметом (секунды; None — нельзя сломать).
pub fn break_time(block: u8, held: Item) -> Option<f32> {
    let d = blocks::def(block);
    if d.hardness < 0.0 {
        return None;
    }
    let mut t = d.hardness / held.mining_speed(d.tool);
    if d.tool == ToolNeed::Pickaxe && !matches!(held, Item::Tool(ToolKind::Pickaxe, _)) {
        t *= 3.0;
    }
    Some(t.max(0.0))
}
