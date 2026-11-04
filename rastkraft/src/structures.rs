//! Постройки: по 10 видов на луга, пустыню, тайгу и Рай. Каждая строится
//! детерминированно из зерна, частями — по чанкам, и заселяется своими жителями.

use crate::blocks::*;
use crate::mobdefs::by_name;
use crate::world::{CH, Dim};
use crate::worldgen::{self, Biome, Gen, SEA};

const REGION: i32 = 64;
const REACH: i32 = 14;

pub struct SB<'a, 'b> {
    g: &'a mut Gen<'b>,
    ox: i32,
    oy: i32,
    oz: i32,
    seed: u32,
}

impl SB<'_, '_> {
    fn p(&mut self, x: i32, y: i32, z: i32, b: u16) {
        self.g.set(self.ox + x, self.oy + y, self.oz + z, b);
    }
    fn get(&self, x: i32, y: i32, z: i32) -> u16 {
        self.g.get(self.ox + x, self.oy + y, self.oz + z)
    }
    fn fill(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, b: u16) {
        for y in y0.min(y1)..=y0.max(y1) {
            for z in z0.min(z1)..=z0.max(z1) {
                for x in x0.min(x1)..=x0.max(x1) {
                    self.p(x, y, z, b);
                }
            }
        }
    }
    fn walls(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, b: u16) {
        for y in y0..=y1 {
            for z in z0..=z1 {
                for x in x0..=x1 {
                    if x == x0 || x == x1 || z == z0 || z == z1 {
                        self.p(x, y, z, b);
                    }
                }
            }
        }
    }
    fn clear(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) {
        self.fill(x0, y0, z0, x1, y1, z1, AIR);
    }
    /// Фундамент: заполняет пустоту под площадкой до земли.
    fn found(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, b: u16) {
        for z in z0..=z1 {
            for x in x0..=x1 {
                let mut y = -1;
                while self.oy + y > 1 && y > -20 {
                    let cur = self.get(x, y, z);
                    if is_opaque(cur) && cur != OAK_LEAVES && cur != SPRUCE_LEAVES {
                        break;
                    }
                    self.p(x, y, z, b);
                    y -= 1;
                }
            }
        }
    }
    /// Двускатная крыша вдоль X.
    fn gable(&mut self, x0: i32, x1: i32, z0: i32, z1: i32, y: i32, b: u16) {
        let mut k = 0;
        while z0 + k <= z1 - k {
            self.fill(x0, y + k, z0 + k, x1, y + k, z0 + k, b);
            self.fill(x0, y + k, z1 - k, x1, y + k, z1 - k, b);
            k += 1;
        }
    }
    fn pyramid(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, y: i32, b: u16, hollow: bool) {
        let mut k = 0;
        while x0 + k <= x1 - k && z0 + k <= z1 - k {
            if hollow {
                self.walls(x0 + k, y + k, z0 + k, x1 - k, y + k, z1 - k, b);
            } else {
                self.fill(x0 + k, y + k, z0 + k, x1 - k, y + k, z1 - k, b);
            }
            k += 1;
        }
    }
    fn cyl(&mut self, r: f32, y0: i32, y1: i32, b: u16, hollow: bool) {
        let ri = r.ceil() as i32;
        for z in -ri..=ri {
            for x in -ri..=ri {
                let d = ((x * x + z * z) as f32).sqrt();
                if d <= r && (!hollow || d > r - 1.0) {
                    self.fill(x, y0, z, x, y1, z, b);
                }
            }
        }
    }
    fn dome(&mut self, r: f32, y: i32, b: u16) {
        let ri = r.ceil() as i32;
        for dy in 0..=ri {
            for z in -ri..=ri {
                for x in -ri..=ri {
                    let d = ((x * x + dy * dy + z * z) as f32).sqrt();
                    if d <= r && d > r - 1.2 {
                        self.p(x, y + dy, z, b);
                    }
                }
            }
        }
    }
    fn mob(&mut self, name: &str, x: i32, y: i32, z: i32) {
        let (wx, wy, wz) = (self.ox + x, self.oy + y, self.oz + z);
        if self.g.inside(wx, wy, wz) {
            self.g.c.spawns.push((by_name(name) as u16, [wx, wy, wz]));
        }
    }
    fn r(&self, k: u32) -> f32 {
        hash(self.ox + k as i32 * 31, self.oz, self.seed.wrapping_add(k))
    }
}

pub const MEADOW: [&str; 10] = ["Деревенский дом", "Колодец", "Ферма", "Мельница", "Кузница", "Разрушенная башня", "Лагерь разбойников", "Пасека", "Заброшенная часовня", "Амбар"];
pub const DESERT: [&str; 10] = ["Пирамида", "Оазис", "Шатёр кочевников", "Колодец пустыни", "Храм солнца", "Обелиск", "Саманный дом", "Гробница", "Базар", "Разбитый караван"];
pub const TAIGA: [&str; 10] = ["Бревенчатая изба", "Иглу", "Охотничий лагерь", "Сторожевая вышка", "Ледяной шпиль", "Берлога", "Обитель отшельника", "Святилище духов", "Логово йети", "Рыбацкая хижина"];
pub const HEAVEN: [&str; 10] = ["Небесный храм", "Золотая беседка", "Облачный мост", "Сад ангелов", "Кристальное святилище", "Парящая библиотека", "Алтарь солнца", "Маяк света", "Небесный фонтан", "Обсерватория"];

/// Где и какая постройка в регионе (для отладки и карты).
pub fn region_site(dim: Dim, seed: u32, rx: i32, rz: i32) -> Option<(i32, i32, i32, Biome, usize)> {
    let s = seed.wrapping_add(4040);
    if hash(rx, rz, s) > 0.72 {
        return None;
    }
    let ox = rx * REGION + 14 + (hash(rx, rz, s + 1) * (REGION - 28) as f32) as i32;
    let oz = rz * REGION + 14 + (hash(rx, rz, s + 2) * (REGION - 28) as f32) as i32;
    let kind = (hash(rx, rz, s + 3) * 10.0) as usize % 10;
    match dim {
        Dim::Overworld => {
            let c = worldgen::column(seed, ox, oz);
            if c.biome == Biome::Ocean || c.height <= SEA + 1 || c.height > 90 {
                return None;
            }
            Some((ox, c.height + 1, oz, c.biome, kind))
        }
        Dim::Heaven => {
            let y = worldgen::heaven_island(seed, ox, oz).map_or(70, |(t, _)| t + 1);
            Some((ox, y, oz, Biome::Islands, kind))
        }
    }
}

pub fn place(g: &mut Gen, dim: Dim, seed: u32) {
    let (x0, z0) = (g.x0, g.z0);
    let r0x = (x0 - REACH).div_euclid(REGION);
    let r1x = (x0 + 16 + REACH).div_euclid(REGION);
    let r0z = (z0 - REACH).div_euclid(REGION);
    let r1z = (z0 + 16 + REACH).div_euclid(REGION);
    for rz in r0z..=r1z {
        for rx in r0x..=r1x {
            let Some((ox, oy, oz, biome, kind)) = region_site(dim, seed, rx, rz) else { continue };
            if (ox + REACH < x0) || (ox - REACH >= x0 + 16) || (oz + REACH < z0) || (oz - REACH >= z0 + 16) {
                continue;
            }
            if oy + 20 >= CH {
                continue;
            }
            let mut sb = SB { g, ox, oy, oz, seed: seed ^ (rx as u32).wrapping_mul(7919) ^ (rz as u32).wrapping_mul(104729) };
            match biome {
                Biome::Meadow | Biome::Ocean => meadow(&mut sb, kind),
                Biome::Desert => desert(&mut sb, kind),
                Biome::Taiga => taiga(&mut sb, kind),
                Biome::Islands => heaven(&mut sb, kind),
            }
        }
    }
}

fn meadow(s: &mut SB, kind: usize) {
    match kind {
        0 => {
            // деревенский дом
            s.found(-4, -3, 4, 3, COBBLE);
            s.clear(-5, 0, -4, 5, 9, 4);
            s.fill(-4, -1, -3, 4, -1, 3, COBBLE);
            s.walls(-4, 0, -3, 4, 3, 3, TIMBER_FRAME);
            for (x, z) in [(-4, -3), (4, -3), (-4, 3), (4, 3)] {
                s.fill(x, 0, z, x, 3, z, OAK_LOG);
            }
            s.fill(-3, 0, -2, 3, 0, 2, OAK_PLANKS);
            s.fill(0, 1, -3, 0, 2, -3, AIR);
            s.p(-2, 2, 3, GLASS);
            s.p(2, 2, 3, GLASS);
            s.p(-4, 2, 0, GLASS);
            s.p(4, 2, 0, GLASS);
            s.gable(-5, 5, -4, 4, 4, THATCH);
            s.p(3, 1, 2, CHEST);
            s.p(-3, 1, 2, WORKBENCH);
            s.p(-3, 1, -2, LANTERN);
            s.mob("Селянин", 0, 1, 0);
            s.mob("Селянин", 2, 1, -5);
        }
        1 => {
            s.found(-2, -2, 2, 2, COBBLE);
            s.clear(-2, 0, -2, 2, 5, 2);
            s.walls(-2, -3, -2, 2, 0, 2, COBBLE);
            s.fill(-1, -3, -1, 1, -1, 1, WATER);
            for (x, z) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                s.fill(x, 1, z, x, 3, z, OAK_LOG);
            }
            s.pyramid(-2, -2, 2, 2, 4, OAK_PLANKS, false);
            s.p(0, 3, 0, ROPE);
            s.mob("Селянин", 3, 0, 0);
        }
        2 => {
            s.found(-6, -5, 6, 5, DIRT);
            s.clear(-6, 0, -5, 6, 4, 5);
            s.fill(-6, -1, -5, 6, -1, 5, FARMLAND);
            s.fill(-6, -1, 0, 6, -1, 0, WATER);
            for z in [-4, -3, -2, -1, 1, 2, 3, 4] {
                for x in -5..=5 {
                    s.p(x, 0, z, if z < 0 { WHEAT_CROP } else { CARROT_CROP });
                }
            }
            s.walls(-7, 0, -6, 7, 0, 6, LATTICE);
            s.fill(0, 0, -6, 0, 0, -6, AIR);
            s.p(6, 0, -5, HAY);
            s.p(6, 1, -5, HAY);
            s.mob("Кудахта", 0, 0, -7);
            s.mob("Кудахта", 2, 0, -8);
            s.mob("Бурёнка", -3, 0, 8);
        }
        3 => {
            s.found(-3, -3, 3, 3, COBBLE);
            s.clear(-3, 0, -3, 3, 12, 3);
            s.cyl(3.0, -1, -1, COBBLE, false);
            s.cyl(3.0, 0, 8, WHITEWASH, true);
            s.fill(0, 0, -3, 0, 1, -3, AIR);
            s.pyramid(-3, -3, 3, 3, 9, DARK_BRICKS, false);
            // крылья мельницы
            for k in -5..=5 {
                s.p(-4, 6 + k, 0, WOOL0);
                s.p(-4, 6, k, WOOL0);
            }
            s.p(-4, 6, 0, OAK_LOG);
            s.mob("Мельник", 0, 0, 0);
        }
        4 => {
            s.found(-4, -3, 4, 3, STONE_BRICKS);
            s.clear(-4, 0, -3, 4, 6, 3);
            s.fill(-4, -1, -3, 4, -1, 3, STONE_BRICKS);
            for (x, z) in [(-4, -3), (4, -3), (-4, 3), (4, 3)] {
                s.fill(x, 0, z, x, 3, z, BEAM);
            }
            s.fill(-4, 4, -3, 4, 4, 3, DARK_BRICKS);
            s.fill(-4, 0, 3, 4, 3, 3, STONE_BRICKS);
            s.p(-2, 0, 2, ANVIL);
            s.p(0, 0, 2, FURNACE);
            s.p(1, 0, 2, FURNACE);
            s.p(3, -1, 2, LAVA);
            s.p(3, 0, 1, BARREL);
            s.p(2, 0, -2, CHEST);
            s.mob("Кузнец", 0, 0, 0);
        }
        5 => {
            s.found(-3, -3, 3, 3, STONE_BRICKS);
            s.clear(-3, 0, -3, 3, 10, 3);
            s.walls(-3, 0, -3, 3, 8, 3, STONE_BRICKS);
            for y in 0..9 {
                for z in -3..=3 {
                    for x in -3..=3 {
                        if s.r((x + 5 + (y + 3) * 11 + (z + 5) * 97) as u32) < 0.18 + y as f32 * 0.04 {
                            s.p(x, y, z, if y > 5 { AIR } else { CRACKED_STONE_BRICKS });
                        }
                    }
                }
            }
            s.fill(0, 0, -3, 0, 2, -3, AIR);
            s.fill(-2, 4, -2, 2, 4, 2, OAK_PLANKS);
            s.fill(-2, 0, 2, -2, 4, 2, LADDER);
            s.p(1, 5, 1, CHEST);
            s.p(1, 0, 1, COBWEB);
            s.mob("Рыцарь-скелет", 0, 0, 0);
            s.mob("Рыцарь-скелет", 1, 5, -1);
        }
        6 => {
            s.found(-6, -6, 6, 6, DIRT);
            s.clear(-6, 0, -6, 6, 5, 6);
            s.p(0, -1, 0, MAGMA);
            for (cx, cz, c) in [(-4, -3, WOOL0 + 5), (4, -2, WOOL0 + 12), (0, 4, WOOL0 + 4)] {
                s.fill(cx - 1, 0, cz - 1, cx + 1, 0, cz + 1, AIR);
                s.pyramid(cx - 2, cz - 2, cx + 2, cz + 2, 0, c, true);
                s.fill(cx - 2, 0, cz, cx - 2, 1, cz, AIR);
            }
            s.p(2, 0, 2, CRATE);
            s.p(3, 0, 2, BARREL);
            s.p(-2, 0, 1, CHEST);
            s.mob("Разбойник", 1, 0, 0);
            s.mob("Разбойник", -2, 0, -1);
            s.mob("Разбойник", 2, 0, -3);
        }
        7 => {
            s.found(-5, -5, 5, 5, DIRT);
            s.clear(-5, 0, -5, 5, 5, 5);
            s.fill(-5, -1, -5, 5, -1, 5, GRASS);
            let fl = [TULIP_RED, TULIP_PINK, LAVENDER, SUNFLOWER, DAISY, BLUEBELL];
            for z in -5i32..=5 {
                for x in -5i32..=5 {
                    if (x + z) % 2 == 0 && (x.abs() > 1 || z.abs() > 1) {
                        s.p(x, 0, z, fl[((x + 7) * 3 + z + 7) as usize % fl.len()]);
                    }
                }
            }
            for (x, z) in [(-3, -3), (3, -3), (-3, 3), (3, 3)] {
                s.p(x, 0, z, BEEHIVE);
            }
            s.walls(-1, 0, -1, 1, 2, 1, LATTICE);
            s.fill(0, 0, -1, 0, 1, -1, AIR);
            s.fill(-1, 3, -1, 1, 3, 1, OAK_PLANKS);
            s.p(0, 0, 0, HONEY_BLOCK);
            s.mob("Пчёлка-трудяга", 0, 2, -3);
            s.mob("Пчёлка-трудяга", 2, 2, 2);
        }
        8 => {
            s.found(-3, -5, 3, 5, QUARTZ);
            s.clear(-3, 0, -5, 3, 9, 5);
            s.fill(-3, -1, -5, 3, -1, 5, POLISHED_MARBLE);
            s.walls(-3, 0, -5, 3, 4, 5, QUARTZ);
            for z in [-3, 0, 3] {
                s.p(-3, 2, z, GLASS_YELLOW);
                s.p(3, 2, z, GLASS_YELLOW);
            }
            s.fill(0, 0, -5, 0, 2, -5, AIR);
            s.gable(-4, 4, -5, 5, 5, DARK_BRICKS);
            s.pyramid(-1, -1, 1, 1, 5, QUARTZ_PILLAR, false);
            s.p(0, 0, 4, GOLD_BLOCK);
            s.p(-1, 0, 4, CANDLE);
            s.p(1, 0, 4, CANDLE);
            s.p(2, 0, 3, CHEST);
            s.mob("Культист", 0, 0, 1);
            s.mob("Культист", -1, 0, -2);
        }
        _ => {
            s.found(-5, -4, 5, 4, OAK_PLANKS);
            s.clear(-5, 0, -4, 5, 10, 4);
            s.fill(-5, -1, -4, 5, -1, 4, OAK_PLANKS);
            s.walls(-5, 0, -4, 5, 4, 4, OAK_PLANKS);
            for x in [-5, 0, 5] {
                for z in [-4, 4] {
                    s.fill(x, 0, z, x, 4, z, OAK_LOG);
                }
            }
            s.fill(-1, 0, -4, 1, 3, -4, AIR);
            s.gable(-6, 6, -5, 5, 5, DARK_BRICKS);
            for x in -4..=-2 {
                for z in 1..=3 {
                    s.p(x, 0, z, HAY);
                }
            }
            s.p(-4, 1, 3, HAY);
            s.p(4, 0, 3, CHEST);
            s.mob("Ослик Тимоша", 1, 0, 0);
            s.mob("Хрюндель", -2, 0, -2);
        }
    }
}

fn desert(s: &mut SB, kind: usize) {
    match kind {
        0 => {
            s.found(-9, -9, 9, 9, SANDSTONE);
            s.clear(-9, 0, -9, 9, 10, 9);
            s.pyramid(-9, -9, 9, 9, 0, SANDSTONE_BRICKS, false);
            s.fill(-5, 0, -5, 5, 4, 5, AIR);
            s.fill(-5, -1, -5, 5, -1, 5, GOLD_TILES);
            s.fill(0, 0, -9, 0, 2, -5, AIR);
            s.fill(-1, 0, -9, 1, 2, -8, AIR);
            for (x, z) in [(-5, -5), (5, -5), (-5, 5), (5, 5)] {
                s.fill(x, 0, z, x, 4, z, CHISELED_SANDSTONE);
            }
            s.p(0, 0, 4, CHEST);
            s.p(-2, 0, 4, CHEST);
            s.p(0, 9, 0, GOLD_BLOCK);
            s.p(3, 0, 0, LANTERN);
            s.p(-3, 0, 0, LANTERN);
            s.mob("Мумия", 2, 0, 2);
            s.mob("Мумия", -2, 0, -2);
            s.mob("Страж пирамиды", 0, 0, 0);
        }
        1 => {
            s.found(-6, -6, 6, 6, SAND);
            s.clear(-6, 0, -6, 6, 6, 6);
            s.cyl(6.0, -1, -1, GRASS, false);
            s.cyl(3.5, -2, -1, WATER, false);
            for (x, z) in [(-5, 0), (5, 1), (0, 5), (1, -5)] {
                s.fill(x, 0, z, x, 4, z, PALM_LOG);
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (0, 0)] {
                    s.p(x + dx, 5, z + dz, PALM_LEAVES);
                    s.p(x + dx * 2, 4, z + dz * 2, PALM_LEAVES);
                }
            }
            s.p(4, 0, -4, REEDS);
            s.p(-4, 0, -4, DESERT_ROSE);
            s.mob("Верблюжонок", 3, 0, 3);
            s.mob("Караванщик", -3, 0, 3);
        }
        2 => {
            s.found(-5, -5, 5, 5, SAND);
            s.clear(-5, 0, -5, 5, 6, 5);
            for (cx, cz, c) in [(-2, 0, WOOL0 + 5), (3, 1, WOOL0)] {
                s.pyramid(cx - 3, cz - 3, cx + 3, cz + 3, 0, c, true);
                s.fill(cx - 3, 0, cz, cx - 3, 1, cz, AIR);
            }
            s.p(-2, 0, 0, BARREL);
            s.p(3, 0, 1, CRATE);
            s.p(0, 0, -4, CHEST);
            s.mob("Караванщик", -1, 0, -1);
            s.mob("Караванщик", 3, 0, -2);
        }
        3 => {
            s.found(-2, -2, 2, 2, SANDSTONE);
            s.clear(-2, 0, -2, 2, 5, 2);
            s.walls(-2, -4, -2, 2, 0, 2, SANDSTONE_BRICKS);
            s.fill(-1, -4, -1, 1, -1, 1, WATER);
            for (x, z) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                s.fill(x, 1, z, x, 3, z, SMOOTH_SANDSTONE);
            }
            s.pyramid(-2, -2, 2, 2, 4, CUT_RED_SANDSTONE, false);
            s.mob("Песчаный суслик", 3, 0, 1);
        }
        4 => {
            s.found(-6, -6, 6, 6, SMOOTH_SANDSTONE);
            s.clear(-6, 0, -6, 6, 9, 6);
            s.fill(-6, -1, -6, 6, -1, 6, SMOOTH_SANDSTONE);
            s.fill(-4, -1, -4, 4, -1, 4, GOLD_TILES);
            for (x, z) in [(-5, -5), (5, -5), (-5, 5), (5, 5), (0, -5), (0, 5), (-5, 0), (5, 0)] {
                s.fill(x, 0, z, x, 5, z, SANDSTONE_BRICKS);
            }
            s.fill(-6, 6, -6, 6, 6, 6, SMOOTH_SANDSTONE);
            s.fill(-3, 6, -3, 3, 6, 3, AIR);
            s.p(0, 0, 0, SUN_BLOCK);
            s.p(0, 1, 0, SUN_BLOCK);
            s.p(2, 0, 2, CHEST);
            s.mob("Жрица солнца", 2, 0, -2);
            s.mob("Страж храма", -2, 0, 2);
        }
        5 => {
            s.found(-2, -2, 2, 2, SANDSTONE);
            s.clear(-2, 0, -2, 2, 14, 2);
            s.fill(-2, 0, -2, 2, 0, 2, SANDSTONE_BRICKS);
            s.fill(-1, 1, -1, 1, 11, 1, CHISELED_SANDSTONE);
            s.p(0, 12, 0, GOLD_BLOCK);
            s.mob("Скорпион", 3, 0, 2);
            s.mob("Скорпион", -3, 0, -2);
        }
        6 => {
            s.found(-3, -3, 3, 3, ADOBE);
            s.clear(-3, 0, -3, 3, 6, 3);
            s.fill(-3, -1, -3, 3, -1, 3, ADOBE);
            s.walls(-3, 0, -3, 3, 3, 3, ADOBE);
            s.fill(-3, 4, -3, 3, 4, 3, TERRACOTTA0 + 1);
            s.fill(0, 0, -3, 0, 1, -3, AIR);
            s.p(3, 2, 0, GLASS);
            s.p(-3, 2, 0, GLASS);
            s.fill(-2, -1, -2, 2, -1, 2, MOSAIC);
            s.p(2, 0, 2, BARREL);
            s.mob("Караванщик", 0, 0, 0);
        }
        7 => {
            s.found(-4, -4, 4, 4, SANDSTONE_BRICKS);
            s.clear(-4, 0, -4, 4, 5, 4);
            s.walls(-4, -6, -4, 4, 2, 4, SANDSTONE_BRICKS);
            s.fill(-3, -5, -3, 3, -1, 3, AIR);
            s.fill(-4, 3, -4, 4, 3, 4, CHISELED_SANDSTONE);
            s.fill(0, -5, -4, 0, 2, -4, AIR);
            for y in -5..=0 {
                s.p(0, y, -3, LADDER);
            }
            s.p(-2, -5, 2, CHEST);
            s.p(2, -5, 2, CHEST);
            s.p(0, -5, 2, BONE_BLOCK);
            s.mob("Мумия", 0, -5, 0);
            s.mob("Мумия", 2, -5, -1);
        }
        8 => {
            s.found(-7, -4, 7, 4, SAND);
            s.clear(-7, 0, -4, 7, 5, 4);
            let cols = [WOOL0 + 5, WOOL0 + 7, WOOL0 + 12, WOOL0 + 9];
            for (i, x) in [-6, -2, 2, 6].iter().enumerate() {
                for z in [-3, 3] {
                    s.fill(x - 1, 0, z, x + 1, 0, z, CRATE);
                    s.fill(x - 1, 3, z - 1, x + 1, 3, z + 1, cols[i]);
                    s.fill(x - 1, 1, z - 1, x - 1, 2, z - 1, OAK_LOG);
                    s.fill(x + 1, 1, z - 1, x + 1, 2, z - 1, OAK_LOG);
                }
            }
            s.p(0, 0, 0, BARREL);
            s.mob("Караванщик", -4, 0, 0);
            s.mob("Караванщик", 4, 0, 0);
        }
        _ => {
            s.found(-5, -5, 5, 5, SAND);
            s.clear(-5, 0, -5, 5, 4, 5);
            s.fill(-3, 0, -1, 1, 0, 1, OAK_PLANKS);
            s.p(-3, 0, -2, BARREL);
            s.p(2, 0, 3, CRATE);
            s.p(-1, 1, 0, CRATE);
            s.p(3, 0, -3, BONE_BLOCK);
            s.p(-4, 0, 3, CHEST);
            s.p(1, 0, -3, DEAD_BUSH);
            s.mob("Скорпион", 2, 0, 1);
            s.mob("Гремучая змея", -2, 0, 3);
        }
    }
}

fn taiga(s: &mut SB, kind: usize) {
    match kind {
        0 => {
            s.found(-4, -3, 4, 3, COBBLE);
            s.clear(-5, 0, -4, 5, 8, 4);
            s.fill(-4, -1, -3, 4, -1, 3, SPRUCE_PLANKS_ID);
            s.walls(-4, 0, -3, 4, 3, 3, SPRUCE_LOG);
            s.fill(0, 0, -3, 0, 1, -3, AIR);
            s.p(-2, 2, -3, GLASS);
            s.p(2, 2, -3, GLASS);
            s.gable(-5, 5, -4, 4, 4, SPRUCE_PLANKS_ID);
            s.p(3, 0, 2, FURNACE);
            s.p(-3, 0, 2, CHEST);
            s.p(2, 0, 2, FUR_BLOCK);
            s.p(-3, 2, -2, LANTERN);
            s.mob("Лесник", 0, 0, 0);
        }
        1 => {
            s.found(-4, -4, 4, 4, SNOW_BLOCK);
            s.clear(-4, 0, -4, 4, 5, 4);
            s.dome(4.0, -1, SNOW_BRICKS);
            s.fill(0, 0, -4, 0, 1, -4, AIR);
            s.p(0, 3, 3, ICE);
            s.p(2, 0, 2, FUR_BLOCK);
            s.p(-2, 0, 2, CHEST);
            s.mob("Пингвинчик", 0, 0, 0);
            s.mob("Пингвинчик", 2, 0, -6);
        }
        2 => {
            s.found(-5, -5, 5, 5, PODZOL);
            s.clear(-5, 0, -5, 5, 5, 5);
            s.pyramid(-3, -3, 1, 1, 0, FUR_BLOCK, true);
            s.fill(-3, 0, -1, -3, 1, -1, AIR);
            s.p(3, -1, 2, MAGMA);
            for (x, z) in [(3, 0), (3, 4)] {
                s.fill(x, 0, z, x, 2, z, SPRUCE_LOG);
            }
            s.fill(3, 2, 1, 3, 2, 3, BEAM);
            s.p(-1, 0, 3, CHEST);
            s.p(1, 0, 4, BARREL);
            s.mob("Лесник", 1, 0, 2);
        }
        3 => {
            s.found(-2, -2, 2, 2, SPRUCE_LOG);
            s.clear(-3, 0, -3, 3, 13, 3);
            for (x, z) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                s.fill(x, 0, z, x, 9, z, SPRUCE_LOG);
            }
            s.fill(-3, 10, -3, 3, 10, 3, SPRUCE_PLANKS_ID);
            s.walls(-3, 11, -3, 3, 11, 3, LATTICE);
            s.fill(-1, 12, -1, 1, 12, 1, AIR);
            s.pyramid(-3, -3, 3, 3, 13, SPRUCE_PLANKS_ID, false);
            s.fill(0, 0, 1, 0, 10, 1, LADDER);
            s.p(0, 10, 1, AIR);
            s.p(2, 11, 0, CHEST);
            s.mob("Ледяной лучник", 1, 11, -1);
            s.mob("Ледяной лучник", 3, 0, 4);
        }
        4 => {
            s.found(-3, -3, 3, 3, PACKED_ICE);
            s.clear(-3, 0, -3, 3, 16, 3);
            for y in 0..15 {
                let r = 3.2 - y as f32 * 0.21;
                s.cyl(r.max(0.4), y, y, if y % 4 == 3 { ICE_BRICKS } else { PACKED_ICE }, false);
            }
            s.p(0, 15, 0, SEA_LANTERN);
            s.mob("Морозный дух", 4, 1, 0);
        }
        5 => {
            s.found(-4, -4, 4, 4, STONE);
            s.clear(-4, 0, -4, 4, 6, 4);
            s.dome(4.0, -1, MOSSY_COBBLE);
            s.fill(-1, 0, -4, 1, 1, -4, AIR);
            s.fill(-2, -1, -2, 2, -1, 2, PODZOL);
            s.p(2, 0, 2, BONE_BLOCK);
            s.p(-2, 0, 2, CHEST);
            s.mob("Шатун", 0, 0, 0);
        }
        6 => {
            s.found(-3, -3, 3, 3, MOSSY_COBBLE);
            s.clear(-3, 0, -3, 3, 7, 3);
            s.fill(-3, -1, -3, 3, -1, 3, MOSSY_STONE_BRICKS);
            s.walls(-3, 0, -3, 3, 3, 3, MOSSY_STONE_BRICKS);
            s.fill(0, 0, -3, 0, 1, -3, AIR);
            s.pyramid(-3, -3, 3, 3, 4, SPRUCE_PLANKS_ID, false);
            s.p(2, 0, 2, BOOKSHELF);
            s.p(1, 0, 2, BOOKSHELF);
            s.p(-2, 0, 2, CAULDRON);
            s.p(-2, 1, 2, CANDLE);
            s.p(2, 1, 2, CANDLE);
            s.p(-2, 0, -2, CHEST);
            s.mob("Отшельник", 0, 0, 0);
        }
        7 => {
            s.found(-5, -5, 5, 5, SNOW_BLOCK);
            s.clear(-5, 0, -5, 5, 6, 5);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                let (x, z) = ((a.cos() * 4.5).round() as i32, (a.sin() * 4.5).round() as i32);
                s.fill(x, 0, z, x, 3, z, FROST_STONE);
                s.p(x, 4, z, LAMP0 + 5);
            }
            s.p(0, 0, 0, SEA_LANTERN);
            s.p(0, 1, 0, SNOW_FLOWER);
            s.mob("Снежный барашек", 1, 0, 2);
            s.mob("Снежный барашек", -2, 0, -1);
            s.mob("Песец", 2, 0, -2);
        }
        8 => {
            s.found(-5, -5, 5, 5, PACKED_ICE);
            s.clear(-5, 0, -5, 5, 7, 5);
            s.walls(-5, 0, -5, 5, 4, 5, ICE_BRICKS);
            for y in 0..5 {
                for z in -5..=5 {
                    for x in -5..=5 {
                        if s.r((x + 6 + (y + 1) * 13 + (z + 6) * 71) as u32) < 0.3 + y as f32 * 0.1 {
                            s.p(x, y, z, AIR);
                        }
                    }
                }
            }
            s.fill(-4, -1, -4, 4, -1, 4, PACKED_ICE);
            s.p(2, 0, 2, BONE_BLOCK);
            s.p(-3, 0, 1, BONE_BLOCK);
            s.p(0, 0, 3, CHEST);
            s.mob("Снежный йети", 0, 0, 0);
        }
        _ => {
            s.found(-3, -6, 3, 6, SPRUCE_LOG);
            s.clear(-3, 0, -6, 3, 6, 6);
            s.fill(-3, -1, -6, 3, -1, 6, SPRUCE_PLANKS_ID);
            s.walls(-2, 0, -2, 2, 2, 2, SPRUCE_LOG);
            s.fill(0, 0, -2, 0, 1, -2, AIR);
            s.pyramid(-2, -2, 2, 2, 3, SPRUCE_PLANKS_ID, false);
            s.p(1, 0, 1, BARREL);
            s.p(-1, 0, 1, CHEST);
            for z in [-6, 6] {
                s.p(-3, 0, z, BEAM);
                s.p(3, 0, z, BEAM);
            }
            s.p(2, 0, 5, ROPE);
            s.mob("Лесник", 0, 0, -4);
            s.mob("Морж", 1, 0, 5);
        }
    }
}

fn heaven(s: &mut SB, kind: usize) {
    // в Раю постройки стоят на облачной платформе, если острова нет
    match kind {
        0 => {
            s.found(-7, -7, 7, 7, CLOUD);
            s.clear(-7, 0, -7, 7, 10, 7);
            s.fill(-7, -1, -7, 7, -1, 7, HEAVEN_BRICKS);
            s.fill(-5, -1, -5, 5, -1, 5, GOLD_TILES);
            for x in [-6, -3, 0, 3, 6] {
                for z in [-6, 6] {
                    s.fill(x, 0, z, x, 6, z, QUARTZ_PILLAR);
                    s.fill(z, 0, x, z, 6, x, QUARTZ_PILLAR);
                }
            }
            s.fill(-7, 7, -7, 7, 7, 7, HEAVEN_BRICKS);
            s.pyramid(-6, -6, 6, 6, 8, QUARTZ, false);
            s.p(0, 0, 0, SUN_BLOCK);
            s.p(3, 0, 3, CHEST);
            s.mob("Ангел-проводник", 1, 0, -2);
            s.mob("Хранитель сада", -3, 0, 2);
        }
        1 => {
            s.found(-3, -3, 3, 3, CLOUD);
            s.clear(-3, 0, -3, 3, 6, 3);
            s.fill(-3, -1, -3, 3, -1, 3, GOLD_TILES);
            for (x, z) in [(-3, -3), (3, -3), (-3, 3), (3, 3)] {
                s.fill(x, 0, z, x, 3, z, GOLD_BLOCK);
            }
            s.pyramid(-3, -3, 3, 3, 4, CELESTIAL_GLASS, false);
            s.walls(-3, 0, -3, 3, 0, 3, LATTICE);
            s.fill(0, 0, -3, 0, 0, -3, AIR);
            s.p(0, 0, 0, HEAVEN_LILY);
            s.mob("Херувимчик", 0, 2, 0);
            s.mob("Херувимчик", 2, 3, -5);
        }
        2 => {
            s.clear(-12, 0, -2, 12, 4, 2);
            s.fill(-12, -1, -1, 12, -1, 1, CLOUD);
            s.fill(-12, 0, -2, 12, 0, -2, LATTICE);
            s.fill(-12, 0, 2, 12, 0, 2, LATTICE);
            for x in [-12, -6, 0, 6, 12] {
                s.p(x, 1, -2, LAMP0 + 3);
                s.p(x, 1, 2, LAMP0 + 3);
            }
            s.mob("Облачная овечка", -4, 0, 0);
            s.mob("Облачная овечка", 5, 0, 0);
        }
        3 => {
            s.found(-6, -6, 6, 6, HEAVEN_DIRT);
            s.clear(-6, 0, -6, 6, 7, 6);
            s.fill(-6, -1, -6, 6, -1, 6, HEAVEN_GRASS);
            for z in -6..=6 {
                for x in -6..=6 {
                    if (x * x + z * z) % 5 == 0 {
                        s.p(x, 0, z, HEAVEN_LILY);
                    }
                }
            }
            for (x, z) in [(-4, -4), (4, 4)] {
                s.fill(x, 0, z, x, 3, z, LIGHT_LOG);
                s.fill(x - 1, 4, z - 1, x + 1, 5, z + 1, GOLDEN_LEAVES);
            }
            s.walls(-6, 0, -6, 6, 0, 6, LATTICE);
            s.fill(0, 0, -6, 0, 0, -6, AIR);
            s.mob("Хранитель сада", 0, 0, 0);
            s.mob("Лунный кролик", 2, 0, -2);
        }
        4 => {
            s.found(-5, -5, 5, 5, HEAVEN_STONE);
            s.clear(-5, 0, -5, 5, 7, 5);
            s.fill(-5, -1, -5, 5, -1, 5, HALO_STONE);
            s.dome(5.0, -1, CELESTIAL_GLASS);
            s.fill(0, 0, -5, 0, 1, -5, AIR);
            for (x, z) in [(-2, 0), (2, 0), (0, 2), (0, -2)] {
                s.fill(x, 0, z, x, 2, z, HEAVEN_CRYSTAL);
            }
            s.p(0, 0, 0, CRYSTAL_BLOCK);
            s.p(3, 0, 2, CHEST);
            s.mob("Светоносец", 1, 1, 1);
            s.mob("Светоносец", -1, 1, -1);
        }
        5 => {
            s.found(-4, -4, 4, 4, CLOUD);
            s.clear(-4, 0, -4, 4, 8, 4);
            s.fill(-4, -1, -4, 4, -1, 4, QUARTZ);
            s.walls(-4, 0, -4, 4, 5, 4, BOOKSHELF);
            for y in [2, 3] {
                s.fill(-4, y, 0, 4, y, 0, CELESTIAL_GLASS);
            }
            s.fill(0, 0, -4, 0, 1, -4, AIR);
            s.fill(-4, 6, -4, 4, 6, 4, QUARTZ);
            s.p(0, 0, 0, LAMP0);
            s.p(2, 0, 2, CHEST);
            s.mob("Ангел-проводник", 0, 0, 1);
        }
        6 => {
            s.found(-4, -4, 4, 4, CLOUD);
            s.clear(-4, 0, -4, 4, 6, 4);
            s.pyramid(-4, -4, 4, 4, -1, HALO_STONE, false);
            s.fill(-1, 3, -1, 1, 3, 1, SUN_BLOCK);
            s.p(0, 4, 0, SUN_BLOCK);
            s.p(2, 2, 0, CHEST);
            s.mob("Падший ангел", 3, 4, 3);
            s.mob("Колдун", -3, 4, -3);
            s.mob("Светоносец", 0, 6, 0);
        }
        7 => {
            s.found(-2, -2, 2, 2, QUARTZ);
            s.clear(-2, 0, -2, 2, 18, 2);
            s.cyl(2.0, 0, 14, QUARTZ, true);
            s.fill(0, 0, -2, 0, 1, -2, AIR);
            s.fill(0, 0, 1, 0, 14, 1, LADDER);
            s.cyl(2.5, 15, 15, QUARTZ, false);
            s.p(0, 16, 0, SUN_BLOCK);
            s.p(0, 17, 0, SUN_BLOCK);
            s.mob("Херувимчик", 2, 16, 2);
        }
        8 => {
            s.found(-5, -5, 5, 5, CLOUD);
            s.clear(-5, 0, -5, 5, 6, 5);
            s.cyl(5.0, -1, -1, QUARTZ, false);
            s.cyl(4.0, -1, -1, WATER, false);
            s.cyl(5.0, 0, 0, QUARTZ, true);
            s.fill(0, -1, 0, 0, 3, 0, QUARTZ_PILLAR);
            s.p(0, 4, 0, RAINBOW_BLOCK);
            s.mob("Райская птица", 2, 3, 2);
            s.mob("Райская птица", -2, 3, -2);
        }
        _ => {
            s.found(-5, -5, 5, 5, CLOUD);
            s.clear(-5, 0, -5, 5, 8, 5);
            s.fill(-5, -1, -5, 5, -1, 5, HEAVEN_BRICKS);
            s.cyl(5.0, 0, 2, HEAVEN_BRICKS, true);
            s.dome(5.0, 3, CELESTIAL_GLASS);
            s.fill(0, 0, -5, 0, 1, -5, AIR);
            for k in 0..4 {
                s.p(k - 1, 1 + k, 0, IRON_BARS);
            }
            s.p(3, 0, 2, CHEST);
            s.mob("Ангел-проводник", -2, 0, 2);
            s.mob("Золотой голем", 2, 0, -2);
        }
    }
}

const SPRUCE_PLANKS_ID: u16 = SPRUCE_LOG + 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Dim;

    #[test]
    fn structures_appear_with_inhabitants() {
        let mut spawns = 0;
        let mut chests = 0;
        for cz in -10..10 {
            for cx in -10..10 {
                let c = worldgen::generate(Dim::Overworld, 11, cx, cz);
                spawns += c.spawns.len();
                chests += c.blocks.iter().filter(|&&b| b == CHEST).count();
            }
        }
        assert!(spawns > 5, "spawns = {spawns}");
        assert!(chests > 3, "chests = {chests}");
        let mut hs = 0;
        for cz in -8..8 {
            for cx in -8..8 {
                hs += worldgen::generate(Dim::Heaven, 12, cx, cz).spawns.len();
            }
        }
        assert!(hs > 2, "heaven spawns = {hs}");
    }

    #[test]
    fn forty_structure_types() {
        assert_eq!(MEADOW.len() + DESERT.len() + TAIGA.len() + HEAVEN.len(), 40);
    }
}

#[cfg(test)]
mod probe {
    use super::*;
    #[test]
    #[ignore]
    fn print_sites() {
        let seed = crate::hash_seed("2024");
        for dim in [Dim::Overworld, Dim::Heaven] {
            for rz in -3..3 {
                for rx in -3..3 {
                    if let Some((x, y, z, b, k)) = region_site(dim, if dim == Dim::Heaven { seed.wrapping_add(17) } else { seed }, rx, rz) {
                        let name = match b { Biome::Desert => DESERT[k], Biome::Taiga => TAIGA[k], Biome::Islands => HEAVEN[k], _ => MEADOW[k] };
                        println!("{:?} {} {} {} {}", dim, x, y, z, name);
                    }
                }
            }
        }
    }
}
