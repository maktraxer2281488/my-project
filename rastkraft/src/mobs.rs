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
    /// Сообщения в чат (пробуждение и гибель боссов).
    pub say: Vec<String>,
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
    /// Боссы: таймеры особых атак, вторая фаза, сколько раз делился/звал помощь.
    pub skill_t: f32,
    pub skill2_t: f32,
    pub enraged: bool,
    pub splits: u8,
    pub slam: bool,
    pub stuck_t: f32,
    /// Доля здоровья из снимка хоста (у клиента).
    pub hp_net: Option<f32>,
    /// Разведение: >0 — ищет пару, <0 — отдыхает после.
    pub love: f32,
    /// Детёныш: сколько секунд ещё расти.
    pub baby: f32,
    /// Питомец: чей (255 — местный игрок), сидит ли.
    pub owner: Option<u8>,
    pub sitting: bool,
}

/// Игрок на этом компьютере (для питомцев).
pub const OWNER_LOCAL: u8 = 255;

/// Кого можно приручить и чем.
pub fn tame_food(def: usize) -> Option<u16> {
    match MOBS[def].name {
        "Волк-одиночка" => Some(items::BONE),
        "Рысь" | "Облачный кот" => Some(items::RAW_FISH),
        "Лисёнок" => Some(items::BERRIES),
        "Райская птица" | "Кудахта" => Some(items::WHEAT_SEEDS),
        _ => None,
    }
}

/// Животное, которое можно разводить (дружелюбные звери и птицы, не жители).
pub fn breedable(def: usize) -> bool {
    let d = &MOBS[def];
    d.friendly && matches!(d.plan, Plan::Quad | Plan::Bird) && d.flags & (FLY | SWIM) == 0
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
            skill_t: 4.0,
            skill2_t: 9.0,
            enraged: false,
            splits: 0,
            slam: false,
            stuck_t: 0.0,
            hp_net: None,
            love: 0.0,
            baby: 0.0,
            owner: None,
            sitting: false,
        }
    }

    pub fn is_boss(&self) -> bool {
        self.d().flags & BOSS != 0
    }

    pub fn max_health(&self) -> f32 {
        let d = self.d();
        if d.friendly { d.hp } else { d.hp * HOSTILE_HP }
    }

    /// Доля оставшегося здоровья (0..1).
    pub fn health_frac(&self) -> f32 {
        self.hp_net.unwrap_or((self.health / self.max_health()).clamp(0.0, 1.0))
    }

    pub fn d(&self) -> &'static MobDef {
        &MOBS[self.def]
    }

    pub fn hostile(&self) -> bool {
        if self.owner.is_some() {
            return false;
        }
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
        if self.is_boss() {
            // босса почти не отбросить, и он не убегает
            self.body.vel += dir * 0.8;
        } else {
            self.body.vel += dir * 6.0 + vec3(0.0, 4.5, 0.0);
            self.flee_t = 4.0;
        }
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
    pending_say: Vec<String>,
}

impl Mobs {
    pub fn new() -> Self {
        Mobs { frame: 0, list: Vec::new(), spawn_t: 0.0, rng: 0x1234_5678, pending_say: Vec::new() }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0xffffff) as f32 / 16_777_215.0
    }

    fn pick(&mut self, filter: impl Fn(&MobDef) -> bool) -> Option<usize> {
        let ids: Vec<usize> = (0..MOBS.len()).filter(|&i| MOBS[i].flags & BOSS == 0 && filter(&MOBS[i])).collect();
        if ids.is_empty() {
            return None;
        }
        Some(ids[(self.rand() * ids.len() as f32) as usize % ids.len()])
    }

    pub fn counts(&self) -> (usize, usize) {
        let f = self.list.iter().filter(|m| m.d().friendly).count();
        (f, self.list.len() - f)
    }

    pub fn boss_alive(&self) -> bool {
        self.list.iter().any(|m| m.is_boss() && m.alive())
    }

    /// Призвать босса рядом с точкой (предметом призыва или изредка сам).
    pub fn summon_boss(&mut self, world: &World, def: usize, near: Vec3, dir: Vec3) -> Option<String> {
        let flat = dir.with_y(0.0).normalize_or_zero();
        let base = near + flat * 8.0;
        let (x, z) = (base.x.floor() as i32, base.z.floor() as i32);
        // ищем землю не дальше 20 блоков вниз; если чанк не загружен — на уровне игрока
        let top = (near.y as i32 + 10).min(124);
        let mut y = top;
        while y > top - 30 && y > 2 && !is_solid(world.get(x, y - 1, z)) {
            y -= 1;
        }
        if !world.is_loaded(x, z) || y <= top - 30 || y <= 2 {
            y = near.y as i32 + 1;
        }
        // не застревать в стене: поднимаемся, пока над ногами не будет свободно
        while y < 124 && (is_solid(world.get(x, y, z)) || is_solid(world.get(x, y + 1, z))) {
            y += 1;
        }
        let fly = MOBS[def].flags & FLY != 0;
        let pos = vec3(x as f32 + 0.5, y as f32 + if fly { 3.0 } else { 0.2 }, z as f32 + 0.5);
        let mut m = Mob::new(def, pos);
        m.angry = true;
        m.yaw = (-flat.z).atan2(-flat.x);
        self.list.push(m);
        Some(format!("{} пробуждается!", MOBS[def].name))
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
            // очень редко сам пробуждается босс своего края
            if difficulty > 0 && !water && self.rand() < 0.00003 && !self.boss_alive() {
                let want = if world.dim == Dim::Heaven {
                    Some(BOSS0 + 5)
                } else if !exposed {
                    Some(if y < 40 { BOSS0 + 1 } else { BOSS0 + 2 })
                } else {
                    match biome {
                        Biome::Desert if night => Some(BOSS0 + 3),
                        Biome::Taiga => Some(BOSS0 + 4),
                        Biome::Ocean => None,
                        _ if night => Some(BOSS0),
                        _ => None,
                    }
                };
                if let Some(b) = want {
                    let mut m = Mob::new(b, pos + vec3(0.0, if MOBS[b].flags & FLY != 0 { 3.0 } else { 0.0 }, 0.0));
                    m.angry = true;
                    self.list.push(m);
                    self.pending_say.push(format!("Где-то рядом пробудился босс: {}!", MOBS[b].name));
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
        ev.say.append(&mut self.pending_say);
        let mut new_mobs: Vec<(usize, Vec3)> = Vec::new();
        let minions = self.list.len();
        for (i, m) in self.list.iter_mut().enumerate() {
            // далёкие существа думают и двигаются реже (каждый 3-й кадр)
            let mut near = (player.body.pos - m.body.pos).length_squared();
            for &(_, rp) in remotes {
                near = near.min((rp - m.body.pos).length_squared());
            }
            let mob_far = crate::opt::get().mob_far;
            let far = near > mob_far * mob_far;
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
            let boss = d.flags & BOSS != 0;
            let chase = m.hostile() && !tgt_creative && dist < if boss { 48.0 } else { 22.0 } && to.y.abs() < if boss { 24.0 } else { 10.0 };
            // питомец: идёт за хозяином, сидит по команде
            let mut pet_target: Option<Vec3> = None;
            if let Some(o) = m.owner {
                let opos = if o == OWNER_LOCAL { player.body.pos } else { remotes.iter().find(|r| r.0 == o).map_or(m.body.pos, |r| r.1) };
                let dd = (opos - m.body.pos).length();
                if dd > 28.0 && !m.sitting {
                    m.body.pos = opos + vec3(1.0, 0.5, 1.0);
                    m.body.vel = Vec3::ZERO;
                } else if dd > 4.0 && !m.sitting {
                    pet_target = Some(opos);
                }
            }
            if m.baby > 0.0 {
                m.baby -= dt;
            }
            if m.love != 0.0 {
                m.love = if m.love > 0.0 { (m.love - dt).max(0.0) } else { (m.love + dt).min(0.0) };
            }
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
                if m.flee_t > 0.0 && !m.hostile() && m.owner.is_none() {
                    wish = -dir_to;
                }
                if let Some(t) = pet_target {
                    wish = (t - m.body.pos).with_y(0.0).normalize_or_zero();
                }
                if m.sitting {
                    wish = Vec3::ZERO;
                }
            }

            let mut speed = if chase { d.speed } else { d.speed * 0.45 };
            if m.enraged {
                speed *= 1.3;
            }
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

            if boss && std::env::var("RK_BOSSLOG").is_ok() && frame % 30 == 0 {
                eprintln!("BOSS {} pos={:?} vel={:?} ground={} chase={} hp={:.0} dist={:.1} stuck={:.1} wall={}", d.name, m.body.pos, m.body.vel, m.body.on_ground, chase, m.health, dist, m.stuck_t, m.body.hit_wall);
            }
            // особые атаки боссов
            if boss && chase {
                let boss_dmg = d.dmg * dmg_k * if m.enraged { 1.25 } else { 1.0 };
                let (r, sk) = boss_skill(m, i, dist, dir_to, ppos, tgt.0, boss_dmg, dt, minions, ents, &mut ev, &mut new_mobs);
                if let Some((dmg, knock)) = r {
                    match tgt.0 {
                        None => {
                            player.hurt(dmg, knock, false);
                        }
                        Some(rid) => ev.hurts.push((rid, dmg, knock)),
                    }
                }
                let _ = sk;
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

        // разведение: две особи одного вида рядом — детёныш
        let lovers: Vec<usize> = (0..self.list.len()).filter(|&i| self.list[i].love > 0.0 && self.list[i].alive()).collect();
        let mut born: Vec<(usize, Vec3)> = Vec::new();
        let mut used = std::collections::HashSet::new();
        for (k, &i) in lovers.iter().enumerate() {
            for &j in &lovers[k + 1..] {
                if used.contains(&i) || used.contains(&j) {
                    continue;
                }
                let (a, b) = (&self.list[i], &self.list[j]);
                if a.def == b.def && (a.body.pos - b.body.pos).length() < 8.0 {
                    born.push((a.def, (a.body.pos + b.body.pos) * 0.5));
                    used.insert(i);
                    used.insert(j);
                }
            }
        }
        for i in used {
            self.list[i].love = -60.0;
        }
        let hearts = [Color::from_rgba(250, 80, 110, 255), Color::from_rgba(255, 150, 170, 255)];
        for (def, p) in born {
            let mut baby = Mob::new(def, p + vec3(0.0, 0.3, 0.0));
            baby.baby = 180.0;
            ents.burst(p + vec3(0.0, 1.0, 0.0), 12, 0.6, &hearts, 0.1, 2.0);
            self.list.push(baby);
        }
        // питомцы защищают хозяина: бьют злых существ рядом
        let mut pet_hits: Vec<(usize, usize)> = Vec::new();
        for (i, m) in self.list.iter().enumerate() {
            if m.owner.is_none() || m.sitting || !m.alive() || m.attack_cd > 0.0 {
                continue;
            }
            let best = self.list.iter().enumerate().filter(|(j, o)| *j != i && o.alive() && o.hostile() && !o.is_boss() && (o.body.pos - m.body.pos).length() < 10.0).min_by(|a, b| (a.1.body.pos - m.body.pos).length().total_cmp(&(b.1.body.pos - m.body.pos).length()));
            if let Some((j, _)) = best {
                pet_hits.push((i, j));
            }
        }
        for (i, j) in pet_hits {
            let tp = self.list[j].body.pos;
            let me = &mut self.list[i];
            let to = (tp - me.body.pos).with_y(0.0);
            if to.length() < 1.8 + me.body.hw {
                let dmg = me.d().dmg.max(3.0);
                let from = me.body.pos;
                me.attack_cd = 1.0;
                me.attack_anim = 1.0;
                self.list[j].damage(dmg, from);
            } else {
                let dir = to.normalize_or_zero();
                let sp = me.d().speed * 1.2;
                me.body.vel.x = dir.x * sp;
                me.body.vel.z = dir.z * sp;
                me.yaw = dir.z.atan2(dir.x);
            }
        }

        // смерть и трофеи
        let mut drops: Vec<(Item, u16, Vec3)> = Vec::new();
        let mut gives: Vec<(u8, Item, u16)> = Vec::new();
        let mut smoke_at: Vec<Vec3> = Vec::new();
        let mut boss_dead: Vec<&'static str> = Vec::new();
        let ppos = player.body.pos;
        self.list.retain(|m| {
            if m.dying >= 1.0 {
                let mut out: Vec<(Item, u16)> = Vec::new();
                let d = &MOBS[m.def];
                let r = crate::blocks::hash(m.body.pos.x as i32, m.body.pos.z as i32, m.def as u32);
                let p = m.body.pos + vec3(0.0, 0.5, 0.0);
                if r < 0.75 {
                    out.push((Item::Thing(items::mob_drop(m.def)), 1 + (r < 0.2) as u16));
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
                if d.flags & BOSS != 0 {
                    out.extend(boss_loot(m.def, r));
                    boss_dead.push(d.name);
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
            (m.owner.is_some() || dd < if m.is_boss() { 260.0 } else { 120.0 }) && m.body.pos.y > -30.0
        });
        for name in boss_dead {
            ev.say.push(format!("Босс {name} повержен!"));
        }
        for (def, pos) in new_mobs {
            let mut m = Mob::new(def, pos);
            m.angry = true;
            self.list.push(m);
        }
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
                hp: (m.health_frac() * 255.0) as u8,
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
            m.hp_net = Some(s.hp as f32 / 255.0);
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
        let o = crate::opt::get();
        for m in &self.list {
            let p = m.body.pos;
            let to = p + vec3(0.0, m.body.h * 0.5, 0.0) - cam;
            let dist = to.length();
            // дальние и оказавшиеся за спиной не рисуем
            let max_d = if m.is_boss() { o.mob_dist.max(64.0) } else { o.mob_dist };
            if dist > max_d || (dist > 4.0 && to.dot(fwd) < -m.body.h) {
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
                scale: if m.baby > 0.0 { 0.55 } else { 1.0 },
            };
            if m.body.on_ground || d.flags & FLY != 0 {
                let alpha = if d.flags & FLY != 0 { 0.35 } else { 0.6 };
                let gy = if d.flags & FLY != 0 { p.y.floor() - 3.0 } else { p.y };
                if d.flags & FLY == 0 && o.shadows {
                    model::shadow(vec3(p.x, gy, p.z), m.body.hw * 1.6 + 0.2, alpha);
                }
            }
            model::draw_mob(m.def, p, m.yaw, &a);
            if d.flags & BOSS != 0 && m.dying <= 0.0 {
                // парящая корона над боссом
                let top = p + vec3(0.0, m.body.h + 0.35 + (t * 2.0).sin() * 0.08, 0.0);
                let r = (m.body.hw * 0.55).max(0.35);
                let gold = if m.enraged { Color::new(1.0, 0.35, 0.25, 1.0) } else { Color::new(1.0, 0.82, 0.25, 1.0) };
                let spin = t * 0.8;
                for k in 0..10 {
                    let ang = spin + k as f32 / 10.0 * std::f32::consts::TAU;
                    let c = top + vec3(ang.cos() * r, 0.0, ang.sin() * r);
                    let h = if k % 2 == 0 { 0.32 } else { 0.16 };
                    let mm = Mat4::from_translation(c + vec3(0.0, h * 0.5, 0.0)) * Mat4::from_rotation_y(-ang) * Mat4::from_scale(vec3(0.12, h, 0.12));
                    model::cube(mm, gold);
                    if k % 2 == 0 {
                        let gem = Mat4::from_translation(c + vec3(0.0, h + 0.05, 0.0)) * Mat4::from_scale(Vec3::splat(0.07));
                        model::cube(gem, Color::from_rgba(d.c3.0, d.c3.1, d.c3.2, 255));
                    }
                }
            }
        }
    }
}


fn shoot(ents: &mut Entities, from: Vec3, dir: Vec3, speed: f32, damage: f32, color: Color) {
    ents.projectiles.push(Projectile { pos: from + dir * 0.8, vel: dir * speed + vec3(0.0, 1.0, 0.0), damage, color, age: 0.0, from_player: false });
}

/// Поворот направления вокруг вертикали.
fn rot_y(v: Vec3, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    vec3(v.x * c - v.z * s, v.y, v.x * s + v.z * c)
}

/// Особые атаки боссов. Возвращает удар по цели (урон, отбрасывание), если он был.
#[allow(clippy::too_many_arguments)]
fn boss_skill(
    m: &mut Mob,
    _i: usize,
    dist: f32,
    dir_to: Vec3,
    ppos: Vec3,
    remote: Option<u8>,
    dmg: f32,
    dt: f32,
    minions: usize,
    ents: &mut Entities,
    ev: &mut MobEvents,
    new_mobs: &mut Vec<(usize, Vec3)>,
) -> (Option<(f32, Vec3)>, ()) {
    let d = m.d();
    let kind = m.def - BOSS0;
    let col = Color::from_rgba(d.c3.0, d.c3.1, d.c3.2, 255);
    let col2 = Color::from_rgba(d.c1.0, d.c1.1, d.c1.2, 255);
    let center = m.body.pos + vec3(0.0, m.body.h * 0.6, 0.0);
    let mut hit: Option<(f32, Vec3)> = None;
    // ярость на половине здоровья
    if !m.enraged && m.health < m.max_health() * 0.5 {
        m.enraged = true;
        ev.say.push(format!("{} в ярости!", d.name));
        ents.burst(center, 40, m.body.hw * 1.5, &[col, col2, WHITE], 0.16, 6.0);
        if kind == 5 {
            m.health += m.max_health() * 0.2;
        }
    }
    m.skill_t -= dt;
    m.skill2_t -= dt;
    // огромное тело цепляется за деревья: сначала большой прыжок, потом рывок к игроку
    let hspeed = vec2(m.body.vel.x, m.body.vel.z).length();
    if d.flags & FLY == 0 && dist > 3.0 && (hspeed < 0.5 || m.body.hit_wall) {
        m.stuck_t += dt;
    } else {
        m.stuck_t = (m.stuck_t - dt * 0.5).max(0.0);
    }
    if m.stuck_t > 1.5 && m.stuck_t < 1.5 + dt * 2.0 + 0.05 && m.body.on_ground {
        m.body.vel = dir_to * 7.0 + vec3(0.0, 15.0, 0.0);
    }
    if m.stuck_t > 4.0 {
        m.stuck_t = 0.0;
        ents.burst(center, 30, m.body.hw, &[col, col2], 0.14, 4.0);
        m.body.pos = ppos - dir_to * 3.0 + vec3(0.0, 1.0, 0.0);
        m.body.vel = Vec3::ZERO;
    }
    let fast = if m.enraged { 0.65 } else { 1.0 };
    let aim = |from: Vec3| ((ppos + vec3(0.0, 1.0, 0.0)) - from).normalize_or_zero();
    // по сетевому игроку снаряды не долетят — бьём напрямую с меньшим уроном
    let ranged = |ents: &mut Entities, hit: &mut Option<(f32, Vec3)>, dirs: &[Vec3], speed: f32, k: f32| {
        if remote.is_some() {
            if dist < 24.0 {
                *hit = Some((dmg * k * 0.6, dir_to * 3.0 + vec3(0.0, 2.0, 0.0)));
            }
        } else {
            for dd in dirs {
                shoot(ents, center, *dd, speed, dmg * k, col);
            }
        }
    };
    match kind {
        0 => {
            // Король слизней: делится при потере здоровья и прыгает с ударной волной
            let want = (((1.0 - m.health / m.max_health()) * 4.0) as u8).min(3);
            while m.splits < want {
                m.splits += 1;
                for k in 0..3 {
                    let a = k as f32 * 2.1;
                    new_mobs.push((by_name("Прыгун"), m.body.pos + vec3(a.cos() * 2.0, 1.0, a.sin() * 2.0)));
                }
                ents.burst(center, 30, 1.5, &[col2, col], 0.14, 5.0);
            }
            if m.slam && m.body.on_ground && m.body.vel.y <= 0.0 {
                m.slam = false;
                ents.burst(m.body.pos + vec3(0.0, 0.3, 0.0), 40, 3.0, &[col2, WHITE], 0.12, 7.0);
                if dist < 5.0 {
                    hit = Some((dmg * 1.1, dir_to * 9.0 + vec3(0.0, 8.0, 0.0)));
                }
            }
            if m.skill_t <= 0.0 && m.body.on_ground {
                m.skill_t = 5.0 * fast;
                m.body.vel = dir_to * (dist.min(12.0) * 0.9 + 3.0) + vec3(0.0, 13.0, 0.0);
                m.slam = true;
                m.attack_anim = 1.0;
            }
        }
        1 => {
            // Каменный титан: удар о землю и брошенные валуны
            if m.skill_t <= 0.0 && dist < 9.0 {
                m.skill_t = 6.0 * fast;
                m.attack_anim = 1.0;
                for k in 0..24 {
                    let a = k as f32 / 24.0 * std::f32::consts::TAU;
                    let p = m.body.pos + vec3(a.cos() * 3.5, 0.3, a.sin() * 3.5);
                    ents.burst(p, 2, 0.4, &[col2, Color::from_rgba(90, 80, 70, 255)], 0.18, 4.0);
                }
                if dist < 7.0 {
                    hit = Some((dmg * 1.2, dir_to * 6.0 + vec3(0.0, 11.0, 0.0)));
                }
            }
            if m.skill2_t <= 0.0 && dist > 8.0 {
                m.skill2_t = 4.0 * fast;
                m.attack_anim = 1.0;
                let a = aim(center);
                ranged(ents, &mut hit, &[a], 20.0, 0.9);
            }
        }
        2 => {
            // Королева пауков: зовёт пауков и плюётся ядом
            if m.skill_t <= 0.0 {
                m.skill_t = 9.0 * fast;
                if minions < 95 {
                    for k in 0..2 {
                        let a = k as f32 * 3.1 + m.phase;
                        new_mobs.push((by_name("Пещерный паук"), m.body.pos + vec3(a.cos() * 2.5, 0.5, a.sin() * 2.5)));
                    }
                    ents.burst(center, 20, 1.5, &[col, col2], 0.12, 3.0);
                }
            }
            if m.skill2_t <= 0.0 && dist > 4.0 {
                m.skill2_t = 2.5 * fast;
                m.attack_anim = 1.0;
                let a = aim(center);
                ranged(ents, &mut hit, &[rot_y(a, -0.15), a, rot_y(a, 0.15)], 17.0, 0.6);
            }
        }
        3 => {
            // Фараон: веер проклятых снарядов, телепорт за спину, мумии
            if m.skill_t <= 0.0 {
                m.skill_t = 6.0 * fast;
                m.attack_anim = 1.0;
                let a = aim(center);
                let dirs: Vec<Vec3> = (-2..=2).map(|k| rot_y(a, k as f32 * 0.18)).collect();
                ranged(ents, &mut hit, &dirs, 18.0, 0.7);
            }
            if m.skill2_t <= 0.0 {
                m.skill2_t = 13.0 * fast;
                ents.burst(center, 30, 1.2, &[col, col2], 0.14, 4.0);
                m.body.pos = ppos - dir_to * 4.0 + vec3(0.0, 0.5, 0.0);
                m.body.vel = Vec3::ZERO;
                for k in 0..2 {
                    let a = k as f32 * 3.1;
                    new_mobs.push((by_name("Мумия"), ppos + vec3(a.cos() * 4.0, 0.5, a.sin() * 4.0)));
                }
            }
        }
        4 => {
            // Ледяной дракон: залп ледяных осколков и пикирование
            if m.skill_t <= 0.0 {
                m.skill_t = 5.0 * fast;
                m.attack_anim = 1.0;
                let a = aim(center);
                let dirs: Vec<Vec3> = (-3..=3).map(|k| rot_y(a, k as f32 * 0.12)).collect();
                ranged(ents, &mut hit, &dirs, 20.0, 0.55);
            }
            if m.enraged && m.skill2_t <= 0.0 {
                m.skill2_t = 7.0;
                m.body.vel = (ppos + vec3(0.0, 1.0, 0.0) - center).normalize_or_zero() * 18.0;
            }
        }
        _ => {
            // Падший серафим: кольцо света и прицельные лучи
            if m.skill_t <= 0.0 {
                m.skill_t = 6.0 * fast;
                m.attack_anim = 1.0;
                let n = if m.enraged { 16 } else { 12 };
                let dirs: Vec<Vec3> = (0..n).map(|k| {
                    let a = k as f32 / n as f32 * std::f32::consts::TAU;
                    vec3(a.cos(), -0.12, a.sin())
                }).collect();
                if remote.is_none() {
                    for dd in &dirs {
                        shoot(ents, center, *dd, 14.0, dmg * 0.6, col);
                    }
                } else if dist < 16.0 {
                    hit = Some((dmg * 0.5, dir_to * 3.0 + vec3(0.0, 2.0, 0.0)));
                }
            }
            if m.skill2_t <= 0.0 {
                m.skill2_t = 2.2 * fast;
                let a = aim(center);
                ranged(ents, &mut hit, &[a], 24.0, 0.7);
            }
        }
    }
    (hit, ())
}

/// Добыча с босса (кроме трофея).
fn boss_loot(def: usize, r: f32) -> Vec<(Item, u16)> {
    use crate::items::{Mat, ToolKind, Slot};
    let t = Item::Thing;
    let n = |k: f32| 2 + (r * k) as u16;
    let mut v = match def - BOSS0 {
        0 => vec![(t(items::SLIME), 24), (Item::Block(JELLY_BLOCK), n(4.0)), (t(items::GOLD_INGOT), n(5.0)), (t(items::MALACHITE), n(4.0))],
        1 => vec![(t(items::IRON_INGOT), 16), (t(items::RUBY), n(4.0)), (Item::Block(OBSIDIAN), 8), (t(items::STEEL_INGOT), n(6.0))],
        2 => vec![(t(items::THREAD), 32), (t(items::SILK), n(8.0)), (t(items::CRYSTAL), n(3.0)), (t(items::PEARL), n(3.0))],
        3 => vec![(t(items::GOLD_INGOT), 16), (t(items::SAPPHIRE), n(4.0)), (t(items::AMBER), n(6.0)), (t(items::GEM_DUST), n(6.0))],
        4 => vec![(t(items::SAPPHIRE), n(6.0)), (Item::Block(PACKED_ICE), 16), (t(items::CRYSTAL), n(4.0)), (t(items::GEM_DUST), n(6.0))],
        _ => vec![(t(items::HEAVEN_INGOT), n(6.0)), (t(items::HEAVEN_DUST), 24), (t(items::CRYSTAL), n(5.0)), (t(items::STARFRUIT), n(4.0))],
    };
    // и одна редкая вещь
    let kinds = [ToolKind::Katana, ToolKind::Battleaxe, ToolKind::Warhammer, ToolKind::Scythe, ToolKind::Halberd, ToolKind::Rapier];
    let mat = if def - BOSS0 >= 4 { Mat::Heaven } else { Mat::Crystal };
    let k = kinds[((r * 977.0) as usize + def) % kinds.len()];
    if r < 0.6 {
        v.push((Item::Tool(k, mat), 1));
    } else {
        let s = [Slot::Helmet, Slot::Chest, Slot::Legs, Slot::Boots][((r * 131.0) as usize) % 4];
        v.push((Item::Armor(s, mat), 1));
    }
    v
}
