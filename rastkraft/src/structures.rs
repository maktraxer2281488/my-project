//! Постройки: по 10 видов на луга, пустыню, тайгу и Рай. Каждая строится
//! детерминированно из зерна, частями — по чанкам, и заселяется своими жителями.

use crate::blocks::*;
use crate::mobdefs::by_name;
use crate::world::{CH, Dim};
use crate::worldgen::{self, Biome, Gen, SEA};

pub const REGION: i32 = 64;
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
            if near_big(seed, ox, oz) {
                return None;
            }
            Some((ox, c.height + 1, oz, c.biome, kind))
        }
        Dim::Heaven => {
            let y = worldgen::heaven_island(seed, ox, oz).map_or(70, |(t, _)| t + 1);
            Some((ox, y, oz, Biome::Islands, kind))
        }
        Dim::Abyss => None,
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
            match biome.family() {
                Biome::Meadow | Biome::Ocean => meadow(&mut sb, kind),
                Biome::Desert => desert(&mut sb, kind),
                Biome::Taiga => taiga(&mut sb, kind),
                Biome::Islands => heaven(&mut sb, kind),
                _ => meadow(&mut sb, kind),
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

// =====================================================================
// Деревни (своя на каждый биом) и большие постройки
// =====================================================================

/// Размер участка для деревни или большой постройки.
pub const BIG_REGION: i32 = 320;
/// Насколько далеко от центра они могут тянуться.
const BIG_REACH: i32 = 44;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Big {
    Village,
    Fortress,
    Tower,
    Ziggurat,
}

pub fn big_name(kind: Big, biome: Biome) -> String {
    match kind {
        Big::Village => format!("Деревня: {}", village_style(biome).name),
        Big::Fortress => "Каменная крепость".into(),
        Big::Tower => "Башня чародея".into(),
        Big::Ziggurat => "Древний зиккурат".into(),
    }
}

/// Деревня или большая постройка на участке: (x, y, z, биом, вид).
pub fn big_site(seed: u32, rx: i32, rz: i32) -> Option<(i32, i32, i32, Biome, Big)> {
    let s = seed.wrapping_add(9090);
    let roll = hash(rx, rz, s);
    if roll > 0.8 {
        return None;
    }
    let ox = rx * BIG_REGION + 70 + (hash(rx, rz, s + 1) * (BIG_REGION - 140) as f32) as i32;
    let oz = rz * BIG_REGION + 70 + (hash(rx, rz, s + 2) * (BIG_REGION - 140) as f32) as i32;
    let c = worldgen::column(seed, ox, oz);
    if c.biome == Biome::Ocean || c.height <= SEA || c.height > 100 {
        return None;
    }
    let kind = if roll < 0.44 {
        Big::Village
    } else {
        match c.biome {
            Biome::Desert | Biome::Canyon | Biome::Jungle | Biome::Swamp | Biome::Savanna => if roll < 0.66 { Big::Ziggurat } else { Big::Tower },
            _ => if roll < 0.66 { Big::Fortress } else { Big::Tower },
        }
    };
    Some((ox, c.height + 1, oz, c.biome, kind))
}

/// Поляны под деревни и большие постройки рядом с чанком: (x, z, радиус).
/// На них не растут деревья.
pub fn big_clearings(seed: u32, x0: i32, z0: i32) -> Vec<(i32, i32, i32)> {
    let mut v = Vec::new();
    let (r0x, r1x) = ((x0 - BIG_REACH).div_euclid(BIG_REGION), (x0 + 16 + BIG_REACH).div_euclid(BIG_REGION));
    let (r0z, r1z) = ((z0 - BIG_REACH).div_euclid(BIG_REGION), (z0 + 16 + BIG_REACH).div_euclid(BIG_REGION));
    for rz in r0z..=r1z {
        for rx in r0x..=r1x {
            if let Some((x, _, z, _, k)) = big_site(seed, rx, rz) {
                let r = match k {
                    Big::Village => 38,
                    Big::Fortress => 26,
                    Big::Tower => 11,
                    Big::Ziggurat => 24,
                };
                v.push((x, z, r));
            }
        }
    }
    v
}

fn near_big(seed: u32, x: i32, z: i32) -> bool {
    let (rx, rz) = (x.div_euclid(BIG_REGION), z.div_euclid(BIG_REGION));
    for dz in -1..=1 {
        for dx in -1..=1 {
            if let Some((bx, _, bz, _, _)) = big_site(seed, rx + dx, rz + dz) {
                if (bx - x).abs() < BIG_REACH + REACH && (bz - z).abs() < BIG_REACH + REACH {
                    return true;
                }
            }
        }
    }
    false
}

/// Стиль деревни под биом.
struct Style {
    name: &'static str,
    wall: u16,
    corner: u16,
    floor: u16,
    roof: u16,
    /// 0 — двускатная, 1 — плоская, 2 — шатровая.
    roof_kind: u8,
    window: u16,
    path: u16,
    base: u16,
    /// Дома на сваях (болото).
    stilts: bool,
    farms: bool,
    people: [&'static str; 3],
}

fn village_style(b: Biome) -> Style {
    let st = |name, wall, corner, floor, roof, roof_kind, window, path, base, stilts, farms, people| Style { name, wall, corner, floor, roof, roof_kind, window, path, base, stilts, farms, people };
    match b {
        Biome::Desert => st("Песчаный оазис", SMOOTH_SANDSTONE, CHISELED_SANDSTONE, SANDSTONE, SANDSTONE_BRICKS, 1, GLASS_YELLOW, SANDSTONE, SANDSTONE, false, true, ["Караванщик", "Селянин", "Жрица солнца"]),
        Biome::Taiga => st("Северный посёлок", SPRUCE_PLANKS, SPRUCE_LOG, SPRUCE_PLANKS, SNOW_BRICKS, 0, GLASS, GRAVEL, COBBLE, false, true, ["Лесник", "Селянин", "Кузнец"]),
        Biome::Swamp => st("Деревня на сваях", MANGROVE_PLANKS, MANGROVE_LOG, MANGROVE_PLANKS, MOSS_BLOCK, 0, GLASS_GREEN, MANGROVE_PLANKS, MANGROVE_LOG, true, false, ["Отшельник", "Селянин", "Селянин"]),
        Biome::Jungle => st("Бамбуковое селение", BAMBOO_BLOCK, EBONY_LOG, MANGROVE_PLANKS, THATCH, 2, GLASS_GREEN, COARSE_DIRT, MOSSY_COBBLE, false, true, ["Лесник", "Селянин", "Отшельник"]),
        Biome::Savanna => st("Деревня саванны", ACACIA_PLANKS, ACACIA_LOG, ACACIA_PLANKS, THATCH, 0, GLASS, COARSE_DIRT, COBBLE, false, true, ["Селянин", "Караванщик", "Мельник"]),
        Biome::Canyon => st("Город в каньоне", ADOBE, CUT_RED_SANDSTONE, RED_SANDSTONE, RED_SANDSTONE, 1, GLASS_RED, RED_SAND, RED_SANDSTONE, false, false, ["Караванщик", "Кузнец", "Селянин"]),
        Biome::Cherry => st("Сакуровая деревня", WHITEWASH, SAKURA_LOG, SAKURA_PLANKS, SAKURA_PLANKS, 2, PAPER_LANTERN, PAVING, STONE_BRICKS, false, true, ["Селянин", "Жрица солнца", "Мельник"]),
        Biome::IceSpikes => st("Ледяное стойбище", SNOW_BRICKS, PACKED_ICE, SPRUCE_PLANKS, ICE_BRICKS, 2, ICE, SNOW_PACKED, SNOW_PACKED, false, false, ["Лесник", "Селянин", "Отшельник"]),
        Biome::Volcano => st("Кузнечный посёлок", BASALT_BRICKS, POLISHED_BASALT, POLISHED_BASALT, OBSIDIAN_TILES, 2, GLASS_RED, BLACK_SAND, BASALT, false, false, ["Кузнец", "Кузнец", "Селянин"]),
        _ => st("Деревня у реки", TIMBER_FRAME, OAK_LOG, OAK_PLANKS, THATCH, 0, GLASS, PAVING, COBBLE, false, true, ["Селянин", "Мельник", "Кузнец"]),
    }
}

/// Строить деревни и большие постройки, задевающие этот чанк.
pub fn place_big(g: &mut Gen, seed: u32) {
    let (x0, z0) = (g.x0, g.z0);
    let r0x = (x0 - BIG_REACH).div_euclid(BIG_REGION);
    let r1x = (x0 + 16 + BIG_REACH).div_euclid(BIG_REGION);
    let r0z = (z0 - BIG_REACH).div_euclid(BIG_REGION);
    let r1z = (z0 + 16 + BIG_REACH).div_euclid(BIG_REGION);
    for rz in r0z..=r1z {
        for rx in r0x..=r1x {
            let Some((ox, oy, oz, biome, kind)) = big_site(seed, rx, rz) else { continue };
            if ox + BIG_REACH < x0 || ox - BIG_REACH >= x0 + 16 || oz + BIG_REACH < z0 || oz - BIG_REACH >= z0 + 16 {
                continue;
            }
            let rs = seed ^ (rx as u32).wrapping_mul(7919) ^ (rz as u32).wrapping_mul(104729) ^ 0xB16;
            match kind {
                Big::Village => village(g, seed, rs, ox, oz, biome),
                Big::Fortress => fortress(&mut SB { g, ox, oy, oz, seed: rs }),
                Big::Tower => tower(&mut SB { g, ox, oy, oz, seed: rs }),
                Big::Ziggurat => ziggurat(&mut SB { g, ox, oy, oz, seed: rs }, biome),
            }
        }
    }
}

fn ground(seed: u32, x: i32, z: i32) -> i32 {
    worldgen::column(seed, x, z).height
}

fn village(g: &mut Gen, seed: u32, rs: u32, cx: i32, cz: i32, biome: Biome) {
    let st = village_style(biome);
    let cy = ground(seed, cx, cz).max(SEA) + 1;
    // площадь с колодцем
    {
        let mut s = SB { g: &mut *g, ox: cx, oy: cy, oz: cz, seed: rs };
        s.found(-5, -5, 5, 5, st.base);
        s.clear(-5, 0, -5, 5, 6, 5);
        s.fill(-5, -1, -5, 5, -1, 5, st.path);
        s.walls(-2, 0, -2, 2, 0, 2, st.base);
        s.fill(-1, -1, -1, 1, -1, 1, WATER);
        s.fill(-1, -3, -1, 1, -2, 1, WATER);
        for (x, z) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
            s.fill(x, 1, z, x, 3, z, st.corner);
        }
        s.fill(-2, 4, -2, 2, 4, 2, st.roof);
        s.p(0, 3, 0, LANTERN);
    }
    // дома по кругу; первый — большой дом старосты
    let n = 6 + (hash(cx, cz, rs) * 3.0) as i32;
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU + hash(i, 7, rs) * 0.4;
        let dist = 17.0 + hash(i, 9, rs) * 9.0;
        let (hx, hz) = (cx + (a.cos() * dist) as i32, cz + (a.sin() * dist) as i32);
        let gh = ground(seed, hx, hz);
        if gh < SEA - 1 || (gh <= SEA && !st.stilts) || (gh - cy).abs() > 12 {
            continue;
        }
        let big = i == 0;
        let (hw, hd) = if big { (5, 4) } else if hash(i, 11, rs) > 0.5 { (4, 3) } else { (3, 3) };
        // дверь — к площади
        let (dx, dz) = (cx - hx, cz - hz);
        let side = if dx.abs() > dz.abs() { if dx > 0 { 0 } else { 1 } } else if dz > 0 { 2 } else { 3 };
        let lift = if st.stilts { 3 } else { 0 };
        let oy = gh.max(SEA) + 1 + lift;
        {
            let mut s = SB { g: &mut *g, ox: hx, oy, oz: hz, seed: rs.wrapping_add(i as u32 * 31) };
            house(&mut s, &st, hw, hd, side, big, lift, i as usize);
        }
        // дорожка от двери к площади
        let (doorx, doorz) = match side {
            0 => (hx + hw + 1, hz),
            1 => (hx - hw - 1, hz),
            2 => (hx, hz + hd + 1),
            _ => (hx, hz - hd - 1),
        };
        path(g, seed, &st, doorx, doorz, cx, cz);
    }
    // огороды
    if st.farms {
        for k in 0..2 {
            let a = hash(k, 21, rs) * std::f32::consts::TAU;
            let (fx, fz) = (cx + (a.cos() * 30.0) as i32, cz + (a.sin() * 30.0) as i32);
            let fy = ground(seed, fx, fz);
            if fy <= SEA || (fy - cy).abs() > 8 {
                continue;
            }
            let mut s = SB { g: &mut *g, ox: fx, oy: fy, oz: fz, seed: rs };
            s.found(-4, -3, 4, 3, DIRT);
            s.clear(-4, 1, -3, 4, 3, 3);
            s.walls(-4, 0, -3, 4, 0, 3, st.corner);
            for z in -2..=2 {
                for x in -3..=3 {
                    if z == 0 {
                        s.p(x, 0, z, WATER);
                    } else {
                        s.p(x, 0, z, FARMLAND);
                        s.p(x, 1, z, crop_block(((x + z + 9) % 3) as u8, 3));
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn house(s: &mut SB, st: &Style, hw: i32, hd: i32, side: u8, big: bool, lift: i32, idx: usize) {
    let h = if big { 5 } else { 4 };
    if lift > 0 {
        // сваи до земли и лесенка
        for (x, z) in [(-hw, -hd), (hw, -hd), (-hw, hd), (hw, hd)] {
            for y in -lift - 6..0 {
                if !is_opaque(s.get(x, y, z)) || s.get(x, y, z) == WATER {
                    s.p(x, y, z, st.corner);
                }
            }
        }
        s.fill(-hw, -1, -hd, hw, -1, hd, st.floor);
    } else {
        s.found(-hw, -hd, hw, hd, st.base);
        s.fill(-hw, -1, -hd, hw, -1, hd, st.base);
    }
    s.clear(-hw - 1, 0, -hd - 1, hw + 1, h + hw.max(hd) + 2, hd + 1);
    s.walls(-hw, 0, -hd, hw, h - 1, hd, st.wall);
    for (x, z) in [(-hw, -hd), (hw, -hd), (-hw, hd), (hw, hd)] {
        s.fill(x, 0, z, x, h - 1, z, st.corner);
    }
    s.fill(-hw + 1, 0, -hd + 1, hw - 1, 0, hd - 1, st.floor);
    // окна
    for x in [-hw + 2, hw - 2] {
        s.p(x, 2, -hd, st.window);
        s.p(x, 2, hd, st.window);
    }
    s.p(-hw, 2, 0, st.window);
    s.p(hw, 2, 0, st.window);
    // дверной проём
    match side {
        0 => s.fill(hw, 1, 0, hw, 2, 0, AIR),
        1 => s.fill(-hw, 1, 0, -hw, 2, 0, AIR),
        2 => s.fill(0, 1, hd, 0, 2, hd, AIR),
        _ => s.fill(0, 1, -hd, 0, 2, -hd, AIR),
    }
    // крыша
    match st.roof_kind {
        0 => s.gable(-hw - 1, hw + 1, -hd - 1, hd + 1, h, st.roof),
        1 => {
            s.fill(-hw, h, -hd, hw, h, hd, st.roof);
            for x in -hw..=hw {
                if x % 2 == 0 {
                    s.p(x, h + 1, -hd, st.corner);
                    s.p(x, h + 1, hd, st.corner);
                }
            }
        }
        _ => s.pyramid(-hw - 1, -hd - 1, hw + 1, hd + 1, h, st.roof, false),
    }
    // обстановка
    s.p(-hw + 1, 1, -hd + 1, CHEST);
    s.p(hw - 1, 1, -hd + 1, if idx % 3 == 0 { FURNACE } else { WORKBENCH });
    s.p(-hw + 1, 1, hd - 1, if idx % 2 == 0 { BARREL } else { BOOKSHELF });
    s.p(0, h - 1, 0, LANTERN);
    if big {
        s.p(hw - 1, 1, hd - 1, BOOKSHELF);
        s.p(hw - 2, 1, hd - 1, BOOKSHELF);
        s.p(-hw + 2, 1, hd - 1, CHEST);
    }
    let who = st.people[idx % 3];
    s.mob(who, 0, 1, 0);
    if big || idx % 2 == 0 {
        s.mob(st.people[(idx + 1) % 3], 1, 1, 1);
    }
}

/// Дорожка от (x0, z0) к (x1, z1) по рельефу; через воду — мостки.
fn path(g: &mut Gen, seed: u32, st: &Style, x0: i32, z0: i32, x1: i32, z1: i32) {
    let steps = (x1 - x0).abs().max((z1 - z0).abs()).max(1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = x0 + ((x1 - x0) as f32 * t).round() as i32;
        let z = z0 + ((z1 - z0) as f32 * t).round() as i32;
        for (ox, oz) in [(0, 0), (1, 0), (0, 1)] {
            let (px, pz) = (x + ox, z + oz);
            if !g.inside(px, 0, pz) {
                continue;
            }
            let y = ground(seed, px, pz);
            if y < SEA {
                g.set(px, SEA, pz, st.floor);
            } else if !matches!(g.get(px, y + 1, pz), AIR) && !is_opaque(g.get(px, y + 1, pz)) {
                g.set(px, y + 1, pz, AIR);
                g.set(px, y, pz, st.path);
            } else if g.get(px, y + 1, pz) == AIR {
                g.set(px, y, pz, st.path);
            }
        }
        // фонари вдоль дорожки
        if i % 9 == 4 {
            let (lx, lz) = (x - 1, z - 1);
            if g.inside(lx, 0, lz) {
                let y = ground(seed, lx, lz);
                if y >= SEA && g.get(lx, y + 1, lz) == AIR {
                    g.set(lx, y + 1, lz, st.corner);
                    g.set(lx, y + 2, lz, st.corner);
                    g.set(lx, y + 3, lz, LANTERN);
                }
            }
        }
    }
}

/// Кольцо/столб радиуса r с центром (cx, cz).
fn ring(s: &mut SB, cx: i32, cz: i32, r: f32, y0: i32, y1: i32, b: u16, hollow: bool) {
    let ri = r.ceil() as i32;
    for z in -ri..=ri {
        for x in -ri..=ri {
            let d = ((x * x + z * z) as f32).sqrt();
            if d <= r && (!hollow || d > r - 1.0) {
                s.fill(cx + x, y0, cz + z, cx + x, y1, cz + z, b);
            }
        }
    }
}

fn fortress(s: &mut SB) {
    const R: i32 = 16;
    s.found(-R, -R, R, R, STONE_BRICKS);
    s.clear(-R - 4, 0, -R - 4, R + 4, 22, R + 4);
    s.fill(-R, -1, -R, R, -1, R, PAVING);
    // стены с зубцами
    s.walls(-R, 0, -R, R, 6, R, STONE_BRICKS);
    for i in -R..=R {
        if i % 2 == 0 {
            for (x, z) in [(i, -R), (i, R), (-R, i), (R, i)] {
                s.p(x, 7, z, STONE_BRICKS);
            }
        }
        if s.r(i as u32 + 40) < 0.2 {
            s.p(i, 3, -R, MOSSY_STONE_BRICKS);
            s.p(R, 2, i, CRACKED_STONE_BRICKS);
        }
    }
    // боевой ход по стене
    s.walls(-R + 1, 5, -R + 1, R - 1, 5, R - 1, OAK_PLANKS);
    // угловые башни
    for (x, z) in [(-R, -R), (R, -R), (-R, R), (R, R)] {
        ring(s, x, z, 3.5, -1, 11, STONE_BRICKS, false);
        ring(s, x, z, 2.5, 0, 10, AIR, false);
        ring(s, x, z, 3.5, 12, 12, DARK_BRICKS, true);
        s.p(x, 11, z, LANTERN);
        s.p(x, 0, z, CHEST);
    }
    // ворота
    s.fill(-1, 0, -R, 1, 3, -R, AIR);
    s.fill(-2, 4, -R, 2, 4, -R, DARK_BRICKS);
    s.fill(-1, 3, -R, 1, 3, -R, IRON_BARS);
    // донжон
    s.walls(-5, 0, -5, 5, 14, 5, DARK_BRICKS);
    for y in [4, 9] {
        s.fill(-4, y, -4, 4, y, 4, OAK_PLANKS);
        s.p(-4, y, -4, AIR);
    }
    s.fill(-4, 0, -4, -4, 13, -4, LADDER);
    s.fill(-5, 15, -5, 5, 15, 5, DARK_BRICKS);
    for i in -5..=5 {
        if i % 2 == 0 {
            for (x, z) in [(i, -5), (i, 5), (-5, i), (5, i)] {
                s.p(x, 16, z, DARK_BRICKS);
            }
        }
    }
    s.fill(0, 1, -5, 0, 2, -5, AIR);
    for (y, k) in [(1, 0), (5, 1), (10, 2)] {
        s.p(3, y, 3, CHEST);
        s.p(-3 + k, y, 3, LANTERN);
        s.p(3, y, -3, if k == 2 { ANVIL } else { BARREL });
    }
    s.p(0, 10, 0, WEAPON_RACK);
    s.mob("Рыцарь-скелет", 0, 1, 2);
    s.mob("Рыцарь-скелет", 0, 10, 2);
    s.mob("Разбойник", 8, 0, 8);
    s.mob("Разбойник", -8, 0, 8);
    s.mob("Разбойник", 8, 0, -8);
}

fn tower(s: &mut SB) {
    const H: i32 = 36;
    s.found(-6, -6, 6, 6, COBBLE);
    s.clear(-7, 0, -7, 7, H + 8, 7);
    ring(s, 0, 0, 5.5, -1, -1, COBBLE, false);
    ring(s, 0, 0, 5.5, 0, H, POLISHED_SLATE, true);
    // пояса из другого камня
    for y in (0..H).step_by(6) {
        ring(s, 0, 0, 5.5, y, y, CHISELED_SLATE, true);
    }
    // этажи, лестница, окна
    for (k, y) in (6..H).step_by(6).enumerate() {
        ring(s, 0, 0, 4.6, y, y, SPRUCE_PLANKS, false);
        s.p(3, y, 0, AIR);
        for (x, z) in [(5, 0), (-5, 0), (0, 5), (0, -5)] {
            if !(x == 0 && z == -5 && y < 6) {
                s.p(x, y + 3, z, GLASS_BLUE);
            }
        }
        s.p(-3, y + 1, -2, if k % 2 == 0 { CHEST } else { BOOKSHELF });
        s.p(-3, y + 1, 2, BOOKSHELF);
        s.p(0, y + 5, 0, LANTERN);
        if k % 2 == 1 {
            s.mob("Колдун", 0, y + 1, 1);
        }
    }
    s.fill(3, 0, 0, 3, H - 1, 0, LADDER);
    s.fill(0, 1, -5, 0, 2, -5, AIR);
    s.p(0, 1, 2, CHEST);
    // крыша-купол и кабинет чародея наверху
    s.dome(6.5, H, OBSIDIAN_TILES);
    s.p(0, H + 1, 0, ENCHANT_TABLE);
    s.p(2, H + 1, 2, CHEST);
    s.p(-2, H + 1, 2, BOOKSHELF);
    s.p(-2, H + 1, -2, BOOKSHELF);
    s.p(0, H + 5, 0, GLOWSTONE);
    s.mob("Колдун", 1, H + 1, 0);
}

fn ziggurat(s: &mut SB, biome: Biome) {
    let (main, alt, trim) = match biome {
        Biome::Jungle | Biome::Swamp => (MOSSY_STONE_BRICKS, STONE_BRICKS, CHISELED_STONE),
        Biome::Canyon => (CUT_RED_SANDSTONE, RED_SANDSTONE, ADOBE),
        _ => (SANDSTONE_BRICKS, SMOOTH_SANDSTONE, CHISELED_SANDSTONE),
    };
    const B: i32 = 17;
    s.found(-B, -B, B, B, main);
    s.clear(-B - 2, 0, -B - 2, B + 2, 26, B + 2);
    // ступени
    for k in 0..7 {
        let r = B - k * 2;
        for y in k * 3..k * 3 + 3 {
            for z in -r..=r {
                for x in -r..=r {
                    let b = if hash(x, y * 7 + z, s.seed) < 0.25 { alt } else { main };
                    s.p(x, y, z, b);
                }
            }
        }
        // кромка
        s.walls(-r, k * 3 + 2, -r, r, k * 3 + 2, r, trim);
    }
    // лестница по центру южной стороны
    for k in 0..21 {
        let z = -B + k * 2 / 3;
        s.fill(-1, k, -B - 1, 1, k, z, alt);
    }
    // святилище на вершине
    s.fill(-2, 21, -2, 2, 21, 2, GOLD_TILES);
    for (x, z) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
        s.fill(x, 22, z, x, 24, z, trim);
    }
    s.fill(-2, 25, -2, 2, 25, 2, main);
    s.p(0, 22, 0, SUN_BLOCK);
    // потайной зал внутри
    s.clear(-6, 1, -6, 6, 7, 6);
    s.fill(-6, 0, -6, 6, 0, 6, GOLD_TILES);
    s.clear(-1, 1, -B, 1, 3, -6);
    for (x, z) in [(-5, -5), (5, -5), (-5, 5), (5, 5)] {
        s.fill(x, 1, z, x, 7, z, trim);
        s.p(x, 4, z, LANTERN);
    }
    for (x, z) in [(-4, 5), (0, 5), (4, 5)] {
        s.p(x, 1, z, CHEST);
    }
    s.p(0, 1, 2, GOLD_BLOCK);
    s.mob("Страж храма", 0, 1, 0);
    s.mob("Культист", -3, 1, 2);
    s.mob("Культист", 3, 1, 2);
}

#[cfg(test)]
mod probe {
    use super::*;
    #[test]
    #[ignore]
    fn print_big_sites() {
        let seed: u32 = std::env::var("RK_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(7);
        for rz in -4..4 {
            for rx in -4..4 {
                if let Some((x, y, z, b, k)) = big_site(seed, rx, rz) {
                    println!("BIG {} {} {} {:?} {:?} {}", x, y, z, b, k, big_name(k, b));
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn print_sites() {
        let seed = crate::hash_seed("2024");
        for dim in [Dim::Overworld, Dim::Heaven] {
            for rz in -3..3 {
                for rx in -3..3 {
                    if let Some((x, y, z, b, k)) = region_site(dim, if dim == Dim::Heaven { seed.wrapping_add(17) } else { seed }, rx, rz) {
                        let name = match b.family() { Biome::Desert => DESERT[k], Biome::Taiga => TAIGA[k], Biome::Islands => HEAVEN[k], _ => MEADOW[k] };
                        println!("{:?} {} {} {} {}", dim, x, y, z, name);
                    }
                }
            }
        }
    }
}
