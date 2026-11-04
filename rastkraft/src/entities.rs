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

#[derive(Default)]
pub struct Entities {
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

    pub fn draw(&self, atlas: &Texture2D, t: f32, cam_right: Vec3) {
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
            model::shadow(vec3(d.body.pos.x, d.body.pos.y, d.body.pos.z), 0.22, 0.45);
        }
    }
}

/// Что выпадает из сломанного блока.
pub fn block_drops(b: u8, held: Item, r: f32) -> Vec<(Item, u16)> {
    use items as it;
    let t = Item::Thing;
    let pick = matches!(held, Item::Tool(ToolKind::Pickaxe, _));
    let one = |x: Item| vec![(x, 1)];
    let needs_pick = blocks::def(b).tool == ToolNeed::Pickaxe;
    if needs_pick && !pick {
        return vec![];
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
        TALL_GRASS | FERN => if r < 0.12 { one(t(it::WHEAT)) } else { vec![] },
        DEAD_BUSH => if r < 0.5 { one(t(it::STICK)) } else { vec![] },
        WHEAT_CROP => vec![(t(it::WHEAT), 1 + (r * 3.0) as u16)],
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
        OAK_LEAVES | SAKURA_LEAVES | BIRCH_LEAVES | SPRUCE_LEAVES | PALM_LEAVES | GOLDEN_LEAVES => {
            if r < 0.05 {
                one(t(match b {
                    OAK_LEAVES => it::APPLE,
                    PALM_LEAVES => it::COCONUT,
                    GOLDEN_LEAVES => it::GOLDEN_APPLE,
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
            Item::Tool(items::TOOL_KINDS[(h(203) * 7.0) as usize % 7], m)
        } else {
            let am = if m == Mat::Stone { Mat::Leather } else { m };
            Item::Armor(SLOTS[(h(204) * 4.0) as usize % 4], am)
        };
        out.push((it, 1));
    }
    out
}
