//! Предметы на земле, снаряды, правила выпадения из блоков и добыча из сундуков.

use crate::blocks::{self, *};
use crate::items::{self, Item, Mat, SLOTS, ToolKind};
use crate::model;
use crate::player::Body;
use crate::world::World;
use macroquad::prelude::*;

pub struct ItemDrop {
    pub item: Item,
    pub count: u16,
    pub body: Body,
    pub age: f32,
}

pub struct Projectile {
    pub pos: Vec3,
    pub vel: Vec3,
    pub damage: f32,
    pub color: Color,
    pub age: f32,
    pub from_player: bool,
}

pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub max: f32,
    pub color: Color,
    pub size: f32,
    pub gravity: f32,
}

/// Шарик опыта.
pub struct Orb {
    pub pos: Vec3,
    pub vel: Vec3,
    pub value: u32,
    pub age: f32,
}

/// Лодка или вагонетка.
pub struct Vehicle {
    pub boat: bool,
    pub pos: Vec3,
    pub yaw: f32,
    pub speed: f32,
    /// Направление вагонетки по рельсам.
    pub dir: IVec3,
    pub vy: f32,
}

#[derive(Default)]
pub struct Entities {
    pub vehicles: Vec<Vehicle>,
    pub orbs: Vec<Orb>,
    pub drops: Vec<ItemDrop>,
    pub projectiles: Vec<Projectile>,
    pub particles: Vec<Particle>,
    seed: u32,
}

/// Цвет пикселя плитки атласа (для частиц).
pub fn tile_color(tile: u16, k: u32) -> Color {
    let px = blocks::atlas_pixels();
    let (tx, ty) = (tile as usize % blocks::ATLAS_TILES, tile as usize / blocks::ATLAS_TILES);
    let (x, y) = (4 + (k * 7 % 8) as usize, 4 + (k * 5 % 8) as usize);
    let i = ((ty * blocks::TILE + y) * blocks::ATLAS + tx * blocks::TILE + x) * 4;
    Color::from_rgba(px[i], px[i + 1], px[i + 2], 255)
}

impl Entities {
    fn rnd(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / 16_777_216.0
    }

    /// Россыпь частиц (обломки блока, кровь, искры).
    pub fn burst(&mut self, at: Vec3, n: usize, spread: f32, colors: &[Color], size: f32, speed: f32) {
        let n = ((n as f32 * crate::opt::get().particles).round() as usize).max(1);
        if self.particles.len() > 600 {
            return;
        }
        for k in 0..n {
            let (a, b, c) = (self.rnd(), self.rnd(), self.rnd());
            let dir = vec3(a - 0.5, b * 0.8 + 0.2, c - 0.5).normalize_or_zero();
            let life = 0.5 + self.rnd() * 0.6;
            let sp = 0.5 + self.rnd();
            let sz = 0.7 + self.rnd() * 0.6;
            self.particles.push(Particle {
                pos: at + vec3(a - 0.5, b - 0.5, c - 0.5) * spread,
                vel: dir * speed * sp,
                life,
                max: life,
                color: colors[k % colors.len().max(1)],
                size: size * sz,
                gravity: 14.0,
            });
        }
    }

    /// Обломки блока: цвета берутся из его текстуры.
    pub fn block_burst(&mut self, at: Vec3, tile: u16) {
        let cols: Vec<Color> = (0..6).map(|k| tile_color(tile, k)).collect();
        self.burst(at, 16, 0.8, &cols, 0.11, 3.0);
    }

    pub fn update_particles(&mut self, world: &World, dt: f32) {
        for p in self.particles.iter_mut() {
            p.life -= dt;
            p.vel.y -= p.gravity * dt;
            let next = p.pos + p.vel * dt;
            if blocks::is_solid(world.get(next.x.floor() as i32, next.y.floor() as i32, next.z.floor() as i32)) {
                p.vel *= -0.2;
                p.vel.x *= 0.5;
            } else {
                p.pos = next;
            }
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    pub fn spawn_drop(&mut self, item: Item, count: u16, pos: Vec3, rnd: f32) {
        if item == Item::Empty || count == 0 {
            return;
        }
        let a = rnd * std::f32::consts::TAU;
        let mut body = Body::new(pos, 0.12, 0.25);
        body.vel = vec3(a.cos() * 2.0, 4.0, a.sin() * 2.0);
        self.drops.push(ItemDrop { item, count, body, age: 0.0 });
    }

    /// Шаг физики. Возвращает предметы, подобранные игроком.
    pub fn update(&mut self, world: &World, player_pos: Vec3, dt: f32, can_pickup: bool) -> Vec<(Item, u16)> {
        let mut picked = Vec::new();
        for d in self.drops.iter_mut() {
            d.age += dt;
            let to = player_pos + vec3(0.0, 0.8, 0.0) - d.body.pos;
            if can_pickup && d.age > 0.6 && to.length() < 1.8 {
                d.body.vel = to.normalize_or_zero() * 9.0;
                d.body.pos += d.body.vel * dt;
                if to.length() < 0.6 {
                    picked.push((d.item, d.count));
                    d.count = 0;
                }
                continue;
            }
            d.body.vel.y -= 20.0 * dt;
            if d.body.on_ground {
                d.body.vel.x *= 0.8;
                d.body.vel.z *= 0.8;
            }
            d.body.step(world, dt);
        }
        // слияние одинаковых рядом лежащих стопок
        let n = self.drops.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let (a, b) = self.drops.split_at_mut(j);
                let (di, dj) = (&mut a[i], &mut b[0]);
                if di.count > 0 && dj.count > 0 && di.item == dj.item && (di.body.pos - dj.body.pos).length() < 0.8 && di.count + dj.count <= di.item.max_stack() {
                    di.count += dj.count;
                    dj.count = 0;
                }
            }
        }
        self.drops.retain(|d| d.count > 0 && d.age < 300.0 && d.body.pos.y > -40.0);
        picked
    }

    /// Снаряды; возвращает урон, нанесённый игроку (и направление).
    /// Рассыпать опыт шариками.
    pub fn spawn_xp(&mut self, at: Vec3, total: u32) {
        let mut left = total;
        let mut k = 0;
        while left > 0 && k < 24 {
            let v = if left > 40 { 20 } else if left > 8 { 5 } else { 1 }.min(left);
            left -= v;
            let a = k as f32 * 2.4;
            let extra = if k == 23 { std::mem::take(&mut left) } else { 0 };
            self.orbs.push(Orb { pos: at + vec3(0.0, 0.4, 0.0), vel: vec3(a.cos() * 1.5, 3.0 + (k % 3) as f32, a.sin() * 1.5), value: v + extra, age: 0.0 });
            k += 1;
        }
    }

    /// Шарики летят к игроку; возвращает собранный опыт.
    pub fn update_orbs(&mut self, world: &World, player: Vec3, dt: f32) -> u32 {
        let mut got = 0;
        let target = player + vec3(0.0, 0.8, 0.0);
        for o in self.orbs.iter_mut() {
            o.age += dt;
            let to = target - o.pos;
            let d = to.length();
            if d < 7.0 && o.age > 0.5 {
                o.vel += to.normalize_or_zero() * (14.0 / d.max(0.8)) * dt * 6.0;
                o.vel *= 0.92;
            } else {
                o.vel.y -= 12.0 * dt;
                o.vel.x *= 0.96;
                o.vel.z *= 0.96;
            }
            let next = o.pos + o.vel * dt;
            if blocks::is_solid(world.get(next.x.floor() as i32, next.y.floor() as i32, next.z.floor() as i32)) {
                o.vel = vec3(o.vel.x * 0.5, 0.0, o.vel.z * 0.5);
            } else {
                o.pos = next;
            }
            if d < 1.1 && o.age > 0.5 {
                got += o.value;
                o.age = 999.0;
            }
        }
        self.orbs.retain(|o| o.age < 120.0);
        got
    }

    pub fn update_projectiles(&mut self, world: &World, player_pos: Vec3, dt: f32) -> Vec<(f32, Vec3)> {
        let mut hits = Vec::new();
        for p in self.projectiles.iter_mut() {
            p.age += dt;
            p.vel.y -= 6.0 * dt;
            p.pos += p.vel * dt;
            let b = world.get(p.pos.x.floor() as i32, p.pos.y.floor() as i32, p.pos.z.floor() as i32);
            if blocks::is_solid(b) {
                p.age = 99.0;
                continue;
            }
            let c = player_pos + vec3(0.0, 0.9, 0.0);
            if !p.from_player && (p.pos - c).abs().cmple(vec3(0.45, 1.0, 0.45)).all() {
                hits.push((p.damage, p.vel.normalize_or_zero()));
                p.age = 99.0;
            }
        }
        self.projectiles.retain(|p| p.age < 6.0);
        hits
    }

    /// Движение лодок и вагонеток. `input` — (вперёд/назад, направление взгляда) для той, на которой едет игрок.
    pub fn update_vehicles(&mut self, world: &World, riding: Option<usize>, input: (f32, Vec3), dt: f32) {
        for (i, v) in self.vehicles.iter_mut().enumerate() {
            let me = riding == Some(i);
            let cell = v.pos.floor().as_ivec3();
            let at = |p: IVec3| world.get(p.x, p.y, p.z);
            if v.boat {
                let in_water = blocks::fluid(at(cell)).is_some_and(|(w, _)| w) || blocks::fluid(at(cell - IVec3::Y)).is_some_and(|(w, _)| w);
                if me {
                    v.yaw = input.1.z.atan2(input.1.x);
                    v.speed = (v.speed + input.0 * dt * if in_water { 6.0 } else { 1.5 }).clamp(-3.0, if in_water { 8.0 } else { 1.5 });
                }
                v.speed *= 1.0 - dt * if in_water { 0.6 } else { 4.0 };
                // держимся на поверхности воды
                let surface = if blocks::fluid(at(cell)).is_some() { cell.y as f32 + 0.85 } else { v.pos.y };
                if blocks::fluid(at(cell)).is_some() {
                    v.vy = (surface + 0.05 - v.pos.y) * 4.0;
                } else if !blocks::is_solid(at(cell - IVec3::Y)) && !in_water {
                    v.vy -= 20.0 * dt;
                } else {
                    v.vy = v.vy.max(0.0) * 0.5;
                }
                v.pos.y += v.vy * dt;
            } else {
                let on_rail = at(cell) == blocks::RAIL;
                if me && on_rail {
                    let along = vec3(v.dir.x as f32, 0.0, v.dir.z as f32);
                    let push = input.0 * input.1.with_y(0.0).normalize_or_zero().dot(along).signum();
                    v.speed = (v.speed + push * dt * 8.0).clamp(-9.0, 9.0);
                }
                v.speed *= 1.0 - dt * if on_rail { 0.15 } else { 3.0 };
                if on_rail {
                    // по рельсам: вперёд, а на тупике — поворот налево или направо
                    let ahead = cell + v.dir * if v.speed >= 0.0 { 1 } else { -1 };
                    if at(ahead) != blocks::RAIL {
                        let left = IVec3::new(v.dir.z, 0, -v.dir.x);
                        let sign = if v.speed >= 0.0 { 1 } else { -1 };
                        let c = v.pos.floor() + Vec3::splat(0.5);
                        let near_center = (v.pos.with_y(0.0) - c.with_y(0.0)).length() < 0.25;
                        if near_center && at(cell + left * sign) == blocks::RAIL {
                            v.dir = left;
                            v.pos = vec3(c.x, v.pos.y, c.z);
                        } else if near_center && at(cell - left * sign) == blocks::RAIL {
                            v.dir = -left;
                            v.pos = vec3(c.x, v.pos.y, c.z);
                        } else if near_center {
                            v.speed = 0.0;
                        }
                    }
                    v.yaw = (v.dir.z as f32).atan2(v.dir.x as f32);
                    // держимся середины рельса
                    let c = v.pos.floor() + Vec3::splat(0.5);
                    if v.dir.x == 0 {
                        v.pos.x += (c.x - v.pos.x) * (dt * 8.0).min(1.0);
                    } else {
                        v.pos.z += (c.z - v.pos.z) * (dt * 8.0).min(1.0);
                    }
                }
                if !blocks::is_solid(at(cell - IVec3::Y)) && !on_rail {
                    v.vy -= 20.0 * dt;
                    v.pos.y += v.vy * dt;
                } else {
                    v.vy = 0.0;
                }
            }
            let fwd = if v.boat { vec3(v.yaw.cos(), 0.0, v.yaw.sin()) } else { vec3(v.dir.x as f32, 0.0, v.dir.z as f32) };
            let next = v.pos + fwd * v.speed * dt;
            let nc = (next + fwd * 0.5 * v.speed.signum()).floor().as_ivec3();
            if blocks::is_solid(world.get(nc.x, nc.y, nc.z)) {
                v.speed = 0.0;
            } else {
                v.pos = next;
            }
        }
    }

    pub fn draw(&self, atlas: &Texture2D, t: f32, cam_right: Vec3) {
        for v in &self.vehicles {
            let r = Mat4::from_translation(v.pos) * Mat4::from_rotation_y(-v.yaw);
            let b = |m: Mat4, c: Color| model::cube(m, c);
            let sc = |x: f32, y: f32, z: f32| Mat4::from_scale(vec3(x, y, z));
            let tr = |x: f32, y: f32, z: f32| Mat4::from_translation(vec3(x, y, z));
            if v.boat {
                let wood = Color::from_rgba(176, 128, 76, 255);
                let dark = Color::from_rgba(120, 82, 44, 255);
                b(r * tr(0.0, 0.1, 0.0) * sc(1.4, 0.12, 0.8), dark);
                b(r * tr(0.0, 0.3, 0.38) * sc(1.4, 0.35, 0.08), wood);
                b(r * tr(0.0, 0.3, -0.38) * sc(1.4, 0.35, 0.08), wood);
                b(r * tr(0.68, 0.3, 0.0) * sc(0.08, 0.35, 0.8), wood);
                b(r * tr(-0.68, 0.3, 0.0) * sc(0.08, 0.35, 0.8), wood);
                b(r * tr(0.0, 0.32, 0.0) * sc(0.25, 0.06, 0.7), dark);
            } else {
                let iron = Color::from_rgba(150, 154, 164, 255);
                let dark = Color::from_rgba(70, 70, 80, 255);
                b(r * tr(0.0, 0.15, 0.0) * sc(0.9, 0.1, 0.7), dark);
                for (x, z, sx, sz) in [(0.0, 0.33, 0.9, 0.06), (0.0, -0.33, 0.9, 0.06), (0.43, 0.0, 0.06, 0.7), (-0.43, 0.0, 0.06, 0.7)] {
                    b(r * tr(x, 0.4, z) * sc(sx, 0.45, sz), iron);
                }
                for (x, z) in [(0.3, 0.36), (-0.3, 0.36), (0.3, -0.36), (-0.3, -0.36)] {
                    b(r * tr(x, 0.1, z) * sc(0.18, 0.18, 0.05), Color::from_rgba(40, 40, 44, 255));
                }
            }
        }
        for o in &self.orbs {
            let pulse = 0.85 + (t * 6.0 + o.pos.x).sin() * 0.15;
            let sz = 0.12 + (o.value as f32).sqrt() * 0.03;
            let m = Mat4::from_translation(o.pos + vec3(0.0, (t * 3.0 + o.pos.z).sin() * 0.05, 0.0)) * Mat4::from_rotation_y(t * 2.0) * Mat4::from_scale(Vec3::splat(sz));
            model::cube(m, Color::new(0.55 * pulse, 1.0 * pulse, 0.3 * pulse, 1.0));
        }
        for d in &self.drops {
            let bob = (t * 2.5 + d.body.pos.x).sin() * 0.06 + 0.12;
            let p = d.body.pos + vec3(0.0, bob, 0.0);
            let spin = t * 1.6 + d.body.pos.z;
            let m = Mat4::from_translation(p) * Mat4::from_rotation_y(spin);
            let copies = if d.count > 16 { 3 } else if d.count > 1 { 2 } else { 1 };
            for k in 0..copies {
                let off = Mat4::from_translation(vec3(k as f32 * 0.07, k as f32 * 0.07, k as f32 * 0.05));
                match d.item {
                    Item::Block(b) if blocks::def(b).shape != Shape::Cross => {
                        model::tex_cube(atlas, m * off * Mat4::from_scale(Vec3::splat(0.26)), blocks::def(b).tex, WHITE);
                    }
                    other => {
                        let tile = model::item_tile(other);
                        let _ = cam_right;
                        model::sprite(atlas, m * off * Mat4::from_scale(Vec3::splat(0.42)), tile, WHITE);
                    }
                }
            }
        }
        for p in &self.projectiles {
            model::cube(Mat4::from_translation(p.pos) * Mat4::from_rotation_y(t * 9.0) * Mat4::from_scale(Vec3::splat(0.2)), p.color);
        }
        for p in &self.particles {
            let s = p.size * (p.life / p.max).sqrt();
            model::cube(Mat4::from_translation(p.pos) * Mat4::from_rotation_y(p.pos.x * 3.0) * Mat4::from_scale(Vec3::splat(s)), p.color);
        }
        for d in &self.drops {
            if crate::opt::get().shadows {
                model::shadow(vec3(d.body.pos.x, d.body.pos.y, d.body.pos.z), 0.22, 0.45);
            }
        }
    }
}

/// Что выпадает из сломанного блока.
pub fn block_drops(b: u16, held: Item, r: f32) -> Vec<(Item, u16)> {
    use items as it;
    let t = Item::Thing;
    let pick = matches!(held, Item::Tool(ToolKind::Pickaxe, _));
    let one = |x: Item| vec![(x, 1)];
    let needs_pick = blocks::def(b).tool == ToolNeed::Pickaxe;
    if needs_pick && !pick {
        return vec![];
    }
    match crate::shapes::form(b) {
        Some(crate::shapes::Form::Door { upper: true, .. }) => return vec![],
        Some(crate::shapes::Form::Pane) => return vec![],
        Some(_) => return one(Item::Block(crate::shapes::canon(b))),
        None => {}
    }
    match b {
        AIR | WATER | LAVA | BEDROCK | PORTAL => vec![],
        GRASS | SNOW_GRASS | FARMLAND | PODZOL => one(Item::Block(DIRT)),
        HEAVEN_GRASS => one(Item::Block(HEAVEN_DIRT)),
        STONE => one(Item::Block(COBBLE)),
        COAL_ORE => one(t(it::COAL)),
        RUBY_ORE => one(t(it::RUBY)),
        CRYSTAL_ORE => one(t(it::CRYSTAL)),
        SAPPHIRE_ORE => one(t(it::SAPPHIRE)),
        MALACHITE_ORE => one(t(it::MALACHITE)),
        HEAVEN_CRYSTAL => vec![(t(it::HEAVEN_DUST), 2 + (r * 3.0) as u16)],
        GLASS | ICE | GLASS_RED | GLASS_BLUE | GLASS_GREEN | GLASS_YELLOW | CELESTIAL_GLASS => vec![],
        x if (GLASS_EXTRA0..GLASS_EXTRA0 + 12).contains(&x) => vec![],
        CLAY => vec![(t(it::CLAY_BALL), 4)],
        GRAVEL => one(if r < 0.12 { t(it::FLINT) } else { Item::Block(GRAVEL) }),
        BOOKSHELF => vec![(t(it::BOOK), 3)],
        TALL_GRASS | FERN => if r < 0.15 { one(t(it::WHEAT_SEEDS)) } else { vec![] },
        DEAD_BUSH => if r < 0.5 { one(t(it::STICK)) } else { vec![] },
        WHEAT_CROP => vec![(t(it::WHEAT), 1 + (r * 3.0) as u16), (t(it::WHEAT_SEEDS), 1 + (r * 2.5) as u16)],
        POTATO_CROP => vec![(t(it::POTATO), 2 + (r * 3.0) as u16)],
        x if crop(x).is_some() => match crop(x).unwrap().0 {
            0 => one(t(it::WHEAT_SEEDS)),
            1 => one(t(it::CARROT)),
            _ => one(t(it::POTATO)),
        },
        BED => one(Item::Block(BED)),
        CARROT_CROP => vec![(t(it::CARROT), 1 + (r * 3.0) as u16)],
        BERRY_BUSH => vec![(t(it::BERRIES), 2 + (r * 3.0) as u16)],
        REEDS => one(t(it::REED)),
        COBWEB => one(t(it::THREAD)),
        BEEHIVE => vec![(t(it::HONEYCOMB), 3)],
        GLOWSTONE => vec![(t(it::HEAVEN_DUST), 2)],
        SALT_BLOCK => vec![(t(it::SALT), 4)],
        AMBER_BLOCK => vec![(t(it::AMBER), 3)],
        MELON => vec![(t(it::MELON_SLICE), 3 + (r * 4.0) as u16)],
        CACTUS => if r < 0.3 { vec![(Item::Block(CACTUS), 1), (t(it::CACTUS_FRUIT), 1)] } else { one(Item::Block(CACTUS)) },
        x if blocks::is_leaves(x) => {
            if r < 0.05 {
                one(t(match b {
                    OAK_LEAVES => it::APPLE,
                    PALM_LEAVES => it::COCONUT,
                    GOLDEN_LEAVES => it::GOLDEN_APPLE,
                    MAPLE_LEAVES => it::MAPLE_SYRUP,
                    WILLOW_LEAVES => it::PEAR,
                    ACACIA_LEAVES => it::NUTS,
                    CEDAR_LEAVES => it::NUTS,
                    MANGROVE_LEAVES => it::ORANGE,
                    EBONY_LEAVES => it::PLUM,
                    _ => it::STICK,
                }))
            } else if r < 0.15 {
                one(t(it::STICK))
            } else if r < 0.18 && b == PALM_LEAVES {
                one(t(it::DATES))
            } else {
                vec![]
            }
        }
        CORAL_PINK | CORAL_PURPLE if r < 0.3 => vec![(Item::Block(b), 1), (t(it::CORAL_SHARD), 1)],
        STAINED_WINDOW => vec![],
        SPRUCE_LOG if r < 0.1 => vec![(Item::Block(SPRUCE_LOG), 1), (t(it::RESIN), 1)],
        _ => one(Item::Block(b)),
    }
}

/// Содержимое сундука в постройке: зависит от «богатства» постройки (0..3).
pub fn chest_loot(tier: u32, seed: u32) -> Vec<(Item, u16)> {
    use items as it;
    let t = Item::Thing;
    let h = |k: u32| crate::blocks::hash(seed as i32, k as i32, 9191);
    let common = [
        (t(it::BREAD), 3), (t(it::APPLE), 4), (t(it::COAL), 6), (t(it::STICK), 8), (t(it::IRON_INGOT), 3),
        (t(it::COPPER_INGOT), 4), (Item::Block(TORCH), 8), (t(it::STEAK), 2), (t(it::THREAD), 4), (t(it::PAPER), 5),
        (t(it::CARROT), 4), (t(it::COOKIE), 6), (t(it::BOTTLE), 2), (t(it::LEATHER), 3), (t(it::BONE), 4),
    ];
    let rare = [
        (t(it::GOLD_INGOT), 3), (t(it::RUBY), 2), (t(it::SAPPHIRE), 2), (t(it::MALACHITE), 2), (t(it::CRYSTAL), 1),
        (t(it::GOLDEN_APPLE), 1), (t(it::POTION0), 1), (t(it::POTION0 + 2), 1), (t(it::POTION0 + 4), 1), (t(it::BOOK), 2),
        (t(it::HEAVEN_DUST), 4), (t(it::AMBROSIA), 1), (t(it::CAKE), 2), (t(it::HONEY_JAR), 2),
    ];
    let mats = [Mat::Stone, Mat::Copper, Mat::Iron, Mat::Gold, Mat::Ruby, Mat::Sapphire, Mat::Crystal, Mat::Heaven];
    let mut out = Vec::new();
    let n = 3 + (h(0) * 4.0) as u32;
    for k in 0..n {
        let (item, max) = common[(h(k + 1) * common.len() as f32) as usize % common.len()];
        out.push((item, 1 + (h(k + 50) * max as f32) as u16));
    }
    for k in 0..tier {
        let (item, max) = rare[(h(k + 100) * rare.len() as f32) as usize % rare.len()];
        out.push((item, 1 + (h(k + 150) * max as f32) as u16));
    }
    if h(200) < 0.25 + tier as f32 * 0.15 {
        let mi = ((tier as f32 * 1.6 + h(201) * 3.0) as usize).min(mats.len() - 1);
        let m = mats[mi];
        let it = if h(202) < 0.5 {
            Item::Tool(items::TOOL_KINDS[(h(203) * 13.0) as usize % items::TOOL_KINDS.len()], m)
        } else {
            let am = if m == Mat::Stone { Mat::Leather } else { m };
            Item::Armor(SLOTS[(h(204) * 4.0) as usize % 4], am)
        };
        out.push((it, 1));
    }
    out
}
