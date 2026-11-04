//! Живые существа: поведение, появление (до 50 мирных и 50 враждебных
//! одновременно), стрельба, анимация смерти и трофеи.

use crate::blocks::*;
use crate::entities::{Entities, Projectile};
use crate::items::{self, Item};
use crate::mobdefs::*;
use crate::model::{self, Anim};
use crate::player::{Body, Player};
use crate::world::{Dim, World};
use crate::worldgen::{self, Biome};
use macroquad::prelude::*;

static NEXT_UID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

/// События для других игроков (хост рассылает их по сети).
#[derive(Default)]
pub struct MobEvents {
    /// (игрок, урон, отбрасывание)
    pub hurts: Vec<(u8, f32, Vec3)>,
    /// (игрок, предмет, количество) — трофеи за убийство
    pub gives: Vec<(u8, Item, u16)>,
}

pub const MAX_FRIENDLY: usize = 50;
pub const MAX_HOSTILE: usize = 50;
/// Злые существа слабее табличных значений.
pub const HOSTILE_HP: f32 = 0.6;
pub const HOSTILE_DMG: f32 = 0.55;

pub struct Mob {
    pub def: usize,
    pub body: Body,
    pub yaw: f32,
    pub health: f32,
    pub hurt_t: f32,
    pub attack_cd: f32,
    pub attack_anim: f32,
    pub think_t: f32,
    pub wander: Vec3,
    pub phase: f32,
    pub flee_t: f32,
    pub dying: f32,
    pub angry: bool,
    pub home: Option<Vec3>,
    /// Сетевой номер существа.
    pub uid: u32,
    /// Кто из сетевых игроков нанёс смертельный удар.
    pub killer: Option<u8>,
    /// Куда двигаться (у клиента — позиция из снимка хоста).
    pub net_target: Option<Vec3>,
    /// Освещённость (обновляется не каждый кадр).
    pub light: f32,
    light_t: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
}

impl Mob {
    pub fn new(def: usize, pos: Vec3) -> Self {
        let d = &MOBS[def];
        let s = d.scale;
        let (hw, h) = match d.plan {
            Plan::Biped => (0.3, 1.9),
            Plan::Golem => (0.45, 2.1),
            Plan::Quad => (0.4, 1.0),
            Plan::Slime => (0.45, 0.9),
            Plan::Bird => (0.3, 0.8),
            Plan::Spider => (0.5, 0.7),
            Plan::Fish => (0.4, 0.6),
            Plan::Wisp => (0.35, 1.0),
        };
        Mob {
            def,
            body: Body::new(pos, hw * s, h * s),
            yaw: 0.0,
            health: if d.friendly { d.hp } else { d.hp * HOSTILE_HP },
            hurt_t: 0.0,
            attack_cd: 1.0,
            attack_anim: 0.0,
            think_t: 0.0,
            wander: Vec3::ZERO,
            phase: 0.0,
            flee_t: 0.0,
            dying: 0.0,
            angry: false,
            home: None,
            uid: NEXT_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            killer: None,
            net_target: None,
            light: 1.0,
            light_t: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
        }
    }

    pub fn d(&self) -> &'static MobDef {
        &MOBS[self.def]
    }

    pub fn hostile(&self) -> bool {
        let d = self.d();
        (!d.friendly && (d.flags & NEUTRAL == 0 || self.angry)) || (d.friendly && self.angry && d.flags & NEUTRAL != 0)
    }

    pub fn alive(&self) -> bool {
        self.dying <= 0.0
    }

    pub fn ray_hit(&self, o: Vec3, d: Vec3) -> Option<f32> {
        if !self.alive() {
            return None;
        }
        let min = self.body.pos - vec3(self.body.hw, 0.0, self.body.hw);
        let max = self.body.pos + vec3(self.body.hw, self.body.h, self.body.hw);
        ray_aabb(o, d, min, max)
    }

    pub fn damage(&mut self, amount: f32, from: Vec3) {
        if !self.alive() {
            return;
        }
        self.health -= amount;
        self.hurt_t = 0.3;
        let dir = (self.body.pos - from).with_y(0.0).normalize_or_zero();
        self.body.vel += dir * 6.0 + vec3(0.0, 4.5, 0.0);
        self.flee_t = 4.0;
        self.angry = true;
        if self.health <= 0.0 {
            self.dying = 0.01;
        }
    }
}

pub fn ray_aabb(o: Vec3, d: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
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

pub struct Mobs {
    frame: u32,
    pub list: Vec<Mob>,
    spawn_t: f32,
    rng: u32,
}

impl Mobs {
    pub fn new() -> Self {
        Mobs { frame: 0, list: Vec::new(), spawn_t: 0.0, rng: 0x1234_5678 }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0xffffff) as f32 / 16_777_215.0
    }

    fn pick(&mut self, filter: impl Fn(&MobDef) -> bool) -> Option<usize> {
        let ids: Vec<usize> = (0..MOBS.len()).filter(|&i| filter(&MOBS[i])).collect();
        if ids.is_empty() {
            return None;
        }
        Some(ids[(self.rand() * ids.len() as f32) as usize % ids.len()])
    }

    pub fn counts(&self) -> (usize, usize) {
        let f = self.list.iter().filter(|m| m.d().friendly).count();
        (f, self.list.len() - f)
    }

    fn try_spawn(&mut self, world: &World, anchor: Vec3, daylight: f32, difficulty: u8) {
        let (fr, ho) = self.counts();
        let hostile_cap = [0, 25, MAX_HOSTILE, MAX_HOSTILE][difficulty.min(3) as usize];
        let ang = self.rand() * std::f32::consts::TAU;
        let dist = 20.0 + self.rand() * 36.0;
        let p = anchor;
        let (x, z) = ((p.x + ang.cos() * dist).floor() as i32, (p.z + ang.sin() * dist).floor() as i32);
        if !world.is_loaded(x, z) {
            return;
        }
        let y0 = p.y as i32;
        let start = (y0 + 20).min(124);
        for y in ((y0 - 30).max(2)..start).rev() {
            let at = world.get(x, y, z);
            let below = world.get(x, y - 1, z);
            let water = at == WATER && world.get(x, y + 1, z) == WATER;
            let ground = is_solid(below) && at == AIR && world.get(x, y + 1, z) == AIR;
            if !water && !ground {
                continue;
            }
            let exposed = world.sky_exposed(x, y, z);
            let pos = vec3(x as f32 + 0.5, y as f32, z as f32 + 0.5);
            let biome = if world.dim == Dim::Heaven { Biome::Islands } else { worldgen::column(world.seed, x, z).biome };
            let night = daylight < 0.3;
            let want_hostile = ho < hostile_cap && (fr >= MAX_FRIENDLY || self.rand() < 0.5);
            let want_friendly = fr < MAX_FRIENDLY;
            let pick = if water {
                if want_hostile && self.rand() < 0.5 {
                    self.pick(|d| !d.friendly && d.hab == Hab::Ocean)
                } else if want_friendly {
                    self.pick(|d| d.friendly && d.hab == Hab::Ocean && d.flags & SWIM != 0)
                } else {
                    None
                }
            } else if world.dim == Dim::Heaven {
                if want_hostile && self.rand() < 0.35 {
                    self.pick(|d| !d.friendly && d.hab == Hab::Heaven)
                } else if want_friendly && is_solid(below) {
                    self.pick(|d| d.friendly && d.hab == Hab::Heaven)
                } else {
                    None
                }
            } else if !exposed {
                if want_hostile && self.rand() < 0.8 {
                    self.pick(|d| !d.friendly && d.hab == Hab::Cave)
                } else if want_friendly {
                    self.pick(|d| d.friendly && d.hab == Hab::Cave)
                } else {
                    None
                }
            } else {
                let hab = match biome {
                    Biome::Desert => Hab::Desert,
                    Biome::Taiga => Hab::Taiga,
                    Biome::Ocean => Hab::Ocean,
                    _ => Hab::Meadow,
                };
                if want_hostile && night {
                    if self.rand() < 0.6 {
                        self.pick(|d| !d.friendly && d.hab == Hab::Night)
                    } else {
                        self.pick(|d| !d.friendly && d.hab == hab)
                    }
                } else if want_hostile && self.rand() < 0.15 && hab != Hab::Meadow {
                    self.pick(|d| !d.friendly && d.hab == hab && d.flags & BURNS == 0)
                } else if want_friendly && !night {
                    self.pick(|d| d.friendly && d.hab == hab && d.flags & SWIM == 0)
                } else if want_friendly && night && self.rand() < 0.3 {
                    self.pick(|d| d.friendly && d.hab == hab && d.flags & GLOW != 0)
                } else {
                    None
                }
            };
            if let Some(id) = pick {
                if !(water && MOBS[id].flags & SWIM == 0 && MOBS[id].flags & FLY == 0) {
                    self.list.push(Mob::new(id, pos));
                }
            }
            return;
        }
    }

    /// Обитатели постройки появляются у неё один раз.
    pub fn spawn_inhabitant(&mut self, def: usize, pos: Vec3) {
        let mut m = Mob::new(def, pos);
        m.home = Some(pos);
        self.list.push(m);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(&mut self, world: &World, player: &mut Player, ents: &mut Entities, dt: f32, daylight: f32, difficulty: u8, creative: bool, remotes: &[(u8, Vec3)]) -> MobEvents {
        let mut ev = MobEvents::default();
        self.spawn_t -= dt;
        if self.spawn_t <= 0.0 {
            self.spawn_t = 0.12;
            for k in 0..3 {
                // существа появляются вокруг всех игроков по очереди
                let anchor = if remotes.is_empty() || k == 0 { player.body.pos } else { remotes[(self.rng as usize + k) % remotes.len()].1 };
                self.try_spawn(world, anchor, daylight, difficulty);
            }
        }
        if difficulty == 0 {
            self.list.retain(|m| m.d().friendly || m.home.is_some());
        }
        let ppos = player.body.pos;
        let dmg_k = [0.0, 0.5, 1.0, 1.4][difficulty.min(3) as usize] * HOSTILE_DMG;
        let n = self.list.len();
        let rands: Vec<f32> = (0..n * 4).map(|_| self.rand()).collect();

        self.frame = self.frame.wrapping_add(1);
        let frame = self.frame;
        for (i, m) in self.list.iter_mut().enumerate() {
            // далёкие существа думают и двигаются реже (каждый 3-й кадр)
            let mut near = (player.body.pos - m.body.pos).length_squared();
            for &(_, rp) in remotes {
                near = near.min((rp - m.body.pos).length_squared());
            }
            let far = near > 44.0 * 44.0;
            if far && m.dying <= 0.0 && (frame + i as u32) % 3 != 0 {
                continue;
            }
            let dt = if far { dt * 3.0 } else { dt };
            let d = &MOBS[m.def];
            if m.dying > 0.0 {
                m.dying += dt * 1.6;
                m.body.vel.x *= 0.9;
                m.body.vel.z *= 0.9;
                m.body.vel.y -= 20.0 * dt;
                m.body.step(world, dt);
                continue;
            }
            m.hurt_t = (m.hurt_t - dt).max(0.0);
            m.attack_cd -= dt;
            m.attack_anim = (m.attack_anim - dt * 3.0).max(0.0);
            m.think_t -= dt;
            m.flee_t -= dt;
            m.light_t -= dt;
            if m.light_t <= 0.0 {
                m.light_t = 0.5 + rands[i * 4] * 0.3;
                let (x, y, z) = (m.body.pos.x.floor() as i32, (m.body.pos.y + m.body.h * 0.6).floor() as i32, m.body.pos.z.floor() as i32);
                m.light = if world.sky_exposed(x, y, z) { 1.0 } else { 0.0 };
            }
            // ближайший игрок (свой или сетевой)
            let mut tgt: (Option<u8>, Vec3) = (None, ppos);
            let mut best = (ppos - m.body.pos).length_squared();
            for &(rid, rp) in remotes {
                let dd = (rp - m.body.pos).length_squared();
                if dd < best {
                    best = dd;
                    tgt = (Some(rid), rp);
                }
            }
            let ppos = tgt.1;
            let to = ppos - m.body.pos;
            let tgt_creative = tgt.0.is_none() && creative;
            // голова поворачивается к игроку, если он рядом
            {
                let eye = ppos + vec3(0.0, 1.5, 0.0) - (m.body.pos + vec3(0.0, m.body.h * 0.85, 0.0));
                let (ty, tp) = if eye.length() < 9.0 {
                    let ang = eye.z.atan2(eye.x);
                    let mut dy = ang - m.yaw;
                    while dy > std::f32::consts::PI {
                        dy -= std::f32::consts::TAU;
                    }
                    while dy < -std::f32::consts::PI {
                        dy += std::f32::consts::TAU;
                    }
                    (dy, (eye.y / eye.length().max(0.1)).asin())
                } else {
                    (0.0, 0.0)
                };
                let k = (dt * 6.0).min(1.0);
                m.head_yaw += (ty.clamp(-1.0, 1.0) - m.head_yaw) * k;
                m.head_pitch += (tp - m.head_pitch) * k;
            }
            let flat = to.with_y(0.0);
            let dist = flat.length();
            let dir_to = flat.normalize_or_zero();
            let chase = m.hostile() && !tgt_creative && dist < 22.0 && to.y.abs() < 10.0;
            let fly = d.flags & FLY != 0;
            let swim = d.flags & SWIM != 0;
            let in_water = world.get(m.body.pos.x.floor() as i32, (m.body.pos.y + 0.3).floor() as i32, m.body.pos.z.floor() as i32) == WATER;

            let mut wish;
            if chase {
                wish = dir_to;
                if d.flags & RANGED != 0 && dist < 7.0 {
                    wish = -dir_to * 0.6;
                }
            } else {
                if m.think_t <= 0.0 {
                    m.think_t = 2.0 + rands[i * 4] * 4.0;
                    let a = rands[i * 4 + 1] * std::f32::consts::TAU;
                    m.wander = if rands[i * 4 + 2] < 0.35 { Vec3::ZERO } else { vec3(a.cos(), 0.0, a.sin()) };
                    if let Some(h) = m.home {
                        let back = (h - m.body.pos).with_y(0.0);
                        if back.length() > 8.0 {
                            m.wander = back.normalize();
                        }
                    }
                }
                wish = m.wander;
                if m.flee_t > 0.0 && !m.hostile() {
                    wish = -dir_to;
                }
            }

            let mut speed = if chase { d.speed } else { d.speed * 0.45 };
            if m.flee_t > 0.0 && !m.hostile() {
                speed = d.speed * 1.4;
            }

            if d.flags & HOP != 0 && !fly {
                if m.body.on_ground {
                    m.body.vel.x *= 0.5;
                    m.body.vel.z *= 0.5;
                    if m.attack_cd < 0.5 && wish != Vec3::ZERO && rands[i * 4 + 3] < dt * 6.0 {
                        m.body.vel = wish * speed * 1.4 + vec3(0.0, 7.0, 0.0);
                    }
                }
            } else {
                let k = (dt * if m.body.on_ground || fly || in_water { 9.0 } else { 2.0 }).min(1.0);
                m.body.vel.x += (wish.x * speed - m.body.vel.x) * k;
                m.body.vel.z += (wish.z * speed - m.body.vel.z) * k;
                if m.body.hit_wall && m.body.on_ground && wish != Vec3::ZERO && !fly {
                    m.body.vel.y = 8.4;
                }
            }
            if fly {
                // держим высоту над землёй (или над игроком при погоне)
                let gx = m.body.pos.x.floor() as i32;
                let gz = m.body.pos.z.floor() as i32;
                let mut ground = m.body.pos.y as i32;
                while ground > 0 && !is_solid(world.get(gx, ground - 1, gz)) && (m.body.pos.y as i32 - ground) < 12 {
                    ground -= 1;
                }
                let target = if chase { ppos.y + 1.2 } else { ground as f32 + 3.0 + (m.phase * 0.3).sin() };
                m.body.vel.y += ((target - m.body.pos.y) * 2.0 - m.body.vel.y) * (dt * 3.0).min(1.0);
            } else if in_water {
                if swim {
                    let target = if chase { ppos.y } else { m.body.pos.y + (m.phase).sin() * 0.3 };
                    m.body.vel.y += ((target - m.body.pos.y) * 1.5 - m.body.vel.y) * (dt * 3.0).min(1.0);
                } else {
                    m.body.vel.y = (m.body.vel.y + 14.0 * dt).min(2.5);
                }
            } else {
                m.body.vel.y -= 28.0 * dt;
            }
            if wish != Vec3::ZERO {
                let target = if chase { dir_to.z.atan2(dir_to.x) } else { wish.z.atan2(wish.x) };
                let mut dd = target - m.yaw;
                while dd > std::f32::consts::PI {
                    dd -= std::f32::consts::TAU;
                }
                while dd < -std::f32::consts::PI {
                    dd += std::f32::consts::TAU;
                }
                m.yaw += dd * (dt * 8.0).min(1.0);
            }
            m.body.step(world, dt);
            m.phase += dt * (vec2(m.body.vel.x, m.body.vel.z).length() * 2.4 + if fly { 1.0 } else { 0.0 });

            // солнце, лава
            if d.flags & BURNS != 0 && daylight > 0.55 && world.dim == Dim::Overworld {
                let (x, y, z) = (m.body.pos.x.floor() as i32, (m.body.pos.y + m.body.h).floor() as i32, m.body.pos.z.floor() as i32);
                if world.sky_exposed(x, y, z) {
                    m.health -= dt * 3.0;
                    m.hurt_t = m.hurt_t.max(0.05);
                }
            }
            if d.flags & GLOW == 0 && m.body.touches(world, 0.0, |b| b == LAVA) {
                m.health -= dt * 6.0;
            }
            if m.health <= 0.0 && m.dying <= 0.0 {
                m.dying = 0.01;
            }

            // атака
            if m.hostile() && !tgt_creative && m.attack_cd <= 0.0 {
                let reach = 1.2 + m.body.hw;
                if dist < reach && (to.y + 0.5).abs() < 1.8 {
                    let knock = dir_to * 6.0 + vec3(0.0, 4.0, 0.0);
                    match tgt.0 {
                        None => {
                            player.hurt(d.dmg * dmg_k, knock, false);
                        }
                        Some(rid) => ev.hurts.push((rid, d.dmg * dmg_k, knock)),
                    }
                    m.attack_cd = 1.0;
                    m.attack_anim = 1.0;
                } else if d.flags & RANGED != 0 && dist < 16.0 && dist > 3.0 && tgt.0.is_some() {
                    ev.hurts.push((tgt.0.unwrap(), d.dmg * dmg_k * 0.8, dir_to * 3.0 + vec3(0.0, 2.0, 0.0)));
                    m.attack_cd = 3.0;
                    m.attack_anim = 1.0;
                } else if d.flags & RANGED != 0 && dist < 16.0 && dist > 3.0 {
                    let from = m.body.pos + vec3(0.0, m.body.h * 0.8, 0.0);
                    let target = ppos + vec3(0.0, 1.0, 0.0);
                    let dir = (target - from).normalize_or_zero();
                    let lift = dist * 0.18;
                    ents.projectiles.push(Projectile {
                        pos: from + dir * 0.6,
                        vel: dir * 16.0 + vec3(0.0, lift, 0.0),
                        damage: d.dmg * dmg_k,
                        color: Color::from_rgba(d.c3.0, d.c3.1, d.c3.2, 255),
                        age: 0.0,
                        from_player: false,
                    });
                    m.attack_cd = 3.0;
                    m.attack_anim = 1.0;
                }
            }
        }

        // смерть и трофеи
        let mut drops: Vec<(Item, u16, Vec3)> = Vec::new();
        let mut gives: Vec<(u8, Item, u16)> = Vec::new();
        let mut smoke_at: Vec<Vec3> = Vec::new();
        let ppos = player.body.pos;
        self.list.retain(|m| {
            if m.dying >= 1.0 {
                let mut out: Vec<(Item, u16)> = Vec::new();
                let d = &MOBS[m.def];
                let r = crate::blocks::hash(m.body.pos.x as i32, m.body.pos.z as i32, m.def as u32);
                let p = m.body.pos + vec3(0.0, 0.5, 0.0);
                if r < 0.75 {
                    out.push((Item::Thing(items::MOB_DROP0 + m.def as u16), 1 + (r < 0.2) as u16));
                }
                let extra = match d.extra {
                    Extra::None => None,
                    Extra::Beef => Some(items::RAW_BEEF),
                    Extra::Chicken => Some(items::RAW_CHICKEN),
                    Extra::Mutton => Some(items::RAW_MUTTON),
                    Extra::Fish => Some(items::RAW_FISH),
                    Extra::Leather => Some(items::LEATHER),
                    Extra::Fur => Some(items::FUR),
                    Extra::Bone => Some(items::BONE),
                    Extra::Feather => Some(items::FEATHER),
                    Extra::Slime => Some(items::SLIME),
                    Extra::Thread => Some(items::THREAD),
                    Extra::Coal => Some(items::COAL),
                    Extra::Gold => Some(items::GOLD_INGOT),
                    Extra::HeavenDust => Some(items::HEAVEN_DUST),
                    Extra::Egg => Some(items::EGG),
                };
                if let Some(e) = extra {
                    out.push((Item::Thing(e), 1 + (r * 2.5) as u16));
                }
                smoke_at.push(p);
                for (it, n) in out {
                    match m.killer {
                        Some(k) => gives.push((k, it, n)),
                        None => drops.push((it, n, p)),
                    }
                }
                return false;
            }
            let mut dd = (m.body.pos - ppos).length();
            for &(_, rp) in remotes {
                dd = dd.min((m.body.pos - rp).length());
            }
            dd < 120.0 && m.body.pos.y > -30.0
        });
        for (k, (it, n, p)) in drops.into_iter().enumerate() {
            ents.spawn_drop(it, n, p, k as f32 * 0.37);
        }
        let smoke = [Color::from_rgba(230, 230, 230, 255), Color::from_rgba(180, 180, 185, 255), Color::from_rgba(255, 255, 255, 255)];
        for p in smoke_at {
            ents.burst(p, 14, 0.8, &smoke, 0.13, 1.5);
        }
        ev.gives = gives;
        ev
    }

    /// Снимок существ рядом с точкой (для отправки клиенту).
    pub fn snapshot(&self, around: Vec3, radius: f32) -> Vec<crate::net::MobSnap> {
        self.list
            .iter()
            .filter(|m| (m.body.pos - around).length() < radius)
            .map(|m| crate::net::MobSnap {
                uid: m.uid,
                def: m.def as u16,
                pos: m.body.pos.to_array(),
                yaw: m.yaw,
                hurt: m.hurt_t > 0.0,
                attack: (m.attack_anim.clamp(0.0, 1.0) * 255.0) as u8,
                dying: (m.dying.clamp(0.0, 1.0) * 255.0) as u8,
            })
            .collect()
    }

    /// Клиент: применить снимок от хоста.
    pub fn apply_snapshot(&mut self, list: &[crate::net::MobSnap]) {
        let mut keep = Vec::with_capacity(list.len());
        for s in list {
            let pos = Vec3::from(s.pos);
            let idx = self.list.iter().position(|m| m.uid == s.uid);
            let mut m = match idx {
                Some(i) => self.list.swap_remove(i),
                None => {
                    let mut m = Mob::new((s.def as usize).min(MOBS.len() - 1), pos);
                    m.uid = s.uid;
                    m
                }
            };
            m.net_target = Some(pos);
            m.yaw = s.yaw;
            m.hurt_t = if s.hurt { 0.1 } else { 0.0 };
            m.attack_anim = s.attack as f32 / 255.0;
            m.dying = s.dying as f32 / 255.0;
            keep.push(m);
        }
        self.list = keep;
    }

    /// Клиент: плавное движение к позициям из снимка.
    pub fn client_tick(&mut self, world: &World, dt: f32, ppos: Vec3) {
        for m in self.list.iter_mut() {
            if let Some(t) = m.net_target {
                let old = m.body.pos;
                let k = (dt * 10.0).min(1.0);
                m.body.pos += (t - m.body.pos) * k;
                if (t - m.body.pos).length() > 8.0 {
                    m.body.pos = t;
                }
                m.body.vel = (m.body.pos - old) / dt.max(1e-3);
                m.body.on_ground = true;
            }
            m.phase += dt * vec2(m.body.vel.x, m.body.vel.z).length() * 2.4;
            m.light_t -= dt;
            if m.light_t <= 0.0 {
                m.light_t = 0.6;
                let (x, y, z) = (m.body.pos.x.floor() as i32, (m.body.pos.y + m.body.h * 0.6).floor() as i32, m.body.pos.z.floor() as i32);
                m.light = if world.sky_exposed(x, y, z) { 1.0 } else { 0.0 };
            }
            let eye = ppos + vec3(0.0, 1.5, 0.0) - (m.body.pos + vec3(0.0, m.body.h * 0.85, 0.0));
            if eye.length() < 9.0 {
                let mut dy = eye.z.atan2(eye.x) - m.yaw;
                while dy > std::f32::consts::PI {
                    dy -= std::f32::consts::TAU;
                }
                while dy < -std::f32::consts::PI {
                    dy += std::f32::consts::TAU;
                }
                m.head_yaw += (dy.clamp(-1.0, 1.0) - m.head_yaw) * (dt * 6.0).min(1.0);
            } else {
                m.head_yaw *= 1.0 - (dt * 6.0).min(1.0);
            }
        }
    }

    pub fn draw(&self, _world: &World, daylight: f32, t: f32, cam: Vec3, fwd: Vec3) {
        for m in &self.list {
            let p = m.body.pos;
            let to = p + vec3(0.0, m.body.h * 0.5, 0.0) - cam;
            let dist = to.length();
            // дальние и оказавшиеся за спиной не рисуем
            if dist > 72.0 || (dist > 4.0 && to.dot(fwd) < -m.body.h) {
                continue;
            }
            let d = m.d();
            let light = if d.flags & GLOW != 0 { 1.0 } else { (m.light * daylight).max(0.3) };
            let a = Anim {
                walk: m.phase,
                speed: vec2(m.body.vel.x, m.body.vel.z).length() / 2.0 + if d.flags & FLY != 0 { 0.5 } else { 0.0 },
                attack: m.attack_anim,
                hurt: m.hurt_t > 0.0 || m.dying > 0.0,
                dying: m.dying,
                t: t + m.def as f32,
                light,
                head_yaw: m.head_yaw,
                head_pitch: m.head_pitch,
            };
            if m.body.on_ground || d.flags & FLY != 0 {
                let alpha = if d.flags & FLY != 0 { 0.35 } else { 0.6 };
                let gy = if d.flags & FLY != 0 { p.y.floor() - 3.0 } else { p.y };
                if d.flags & FLY == 0 {
                    model::shadow(vec3(p.x, gy, p.z), m.body.hw * 1.6 + 0.2, alpha);
                }
            }
            model::draw_mob(m.def, p, m.yaw, &a);
        }
    }
}
