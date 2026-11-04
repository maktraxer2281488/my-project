//! Реестр блоков и процедурный атлас текстур.
//! Каждая плитка 16×16 рисуется кодом — внешних картинок в игре нет.

use std::sync::OnceLock;

pub const TILE: usize = 16;
pub const ATLAS_TILES: usize = 32;
pub const ATLAS: usize = TILE * ATLAS_TILES;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Air,
    Cube,
    Cutout,
    Liquid,
    Cross,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolNeed {
    Any,
    Pickaxe,
    Axe,
    Shovel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cat {
    Building,
    Nature,
    Colored,
    Ores,
    Decor,
    Heaven,
    Hidden,
}

#[derive(Clone, Debug)]
pub struct BlockDef {
    pub name: &'static str,
    /// Плитки: верх, бок, низ.
    pub tex: [u16; 3],
    pub shape: Shape,
    pub solid: bool,
    pub light: u8,
    /// Секунд ломать рукой.
    pub hardness: f32,
    pub tool: ToolNeed,
    pub cat: Cat,
}

// ---------- идентификаторы ----------
pub const AIR: u8 = 0;
pub const GRASS: u8 = 1;
pub const DIRT: u8 = 2;
pub const STONE: u8 = 3;
pub const COBBLE: u8 = 4;
pub const SAND: u8 = 5;
pub const GRAVEL: u8 = 6;
pub const CLAY: u8 = 7;
pub const SANDSTONE: u8 = 8;
pub const RED_SAND: u8 = 9;
pub const RED_SANDSTONE: u8 = 10;
pub const SNOW_GRASS: u8 = 11;
pub const SNOW_BLOCK: u8 = 12;
pub const ICE: u8 = 13;
pub const PACKED_ICE: u8 = 14;
pub const WATER: u8 = 15;
pub const LAVA: u8 = 16;
pub const BEDROCK: u8 = 17;
pub const MOSSY_COBBLE: u8 = 18;
pub const GRANITE: u8 = 19;
pub const MARBLE: u8 = 20;
pub const BASALT: u8 = 21;
pub const SLATE: u8 = 22;
pub const TUFF: u8 = 23;
pub const OBSIDIAN: u8 = 24;
pub const COAL_ORE: u8 = 25;
pub const IRON_ORE: u8 = 26;
pub const COPPER_ORE: u8 = 27;
pub const GOLD_ORE: u8 = 28;
pub const RUBY_ORE: u8 = 29;
pub const CRYSTAL_ORE: u8 = 30;
pub const SAPPHIRE_ORE: u8 = 31;
pub const MALACHITE_ORE: u8 = 32;
pub const COAL_BLOCK: u8 = 33;
pub const IRON_BLOCK: u8 = 34;
pub const COPPER_BLOCK: u8 = 35;
pub const GOLD_BLOCK: u8 = 36;
pub const RUBY_BLOCK: u8 = 37;
pub const CRYSTAL_BLOCK: u8 = 38;
pub const SAPPHIRE_BLOCK: u8 = 39;
pub const MALACHITE_BLOCK: u8 = 40;
pub const OAK_LOG: u8 = 41;
pub const OAK_PLANKS: u8 = 42;
pub const OAK_LEAVES: u8 = 43;
pub const SPRUCE_LOG: u8 = 44;
pub const SPRUCE_PLANKS: u8 = 45;
pub const SPRUCE_LEAVES: u8 = 46;
pub const BIRCH_LOG: u8 = 47;
pub const BIRCH_PLANKS: u8 = 48;
pub const BIRCH_LEAVES: u8 = 49;
pub const SAKURA_LOG: u8 = 50;
pub const SAKURA_PLANKS: u8 = 51;
pub const SAKURA_LEAVES: u8 = 52;
pub const PALM_LOG: u8 = 53;
pub const PALM_PLANKS: u8 = 54;
pub const PALM_LEAVES: u8 = 55;
pub const TALL_GRASS: u8 = 56;
pub const FERN: u8 = 57;
pub const DAISY: u8 = 58;
pub const POPPY: u8 = 59;
pub const CORNFLOWER: u8 = 60;
pub const DANDELION: u8 = 61;
pub const DEAD_BUSH: u8 = 62;
pub const RED_MUSHROOM: u8 = 63;
pub const BROWN_MUSHROOM: u8 = 64;
pub const REEDS: u8 = 65;
pub const CACTUS: u8 = 66;
pub const WOOL0: u8 = 67;
pub const CONCRETE0: u8 = 83;
pub const GLASS: u8 = 99;
pub const GLASS_RED: u8 = 100;
pub const GLASS_BLUE: u8 = 101;
pub const GLASS_GREEN: u8 = 102;
pub const GLASS_YELLOW: u8 = 103;
pub const BRICKS: u8 = 104;
pub const STONE_BRICKS: u8 = 105;
pub const MOSSY_STONE_BRICKS: u8 = 106;
pub const SANDSTONE_BRICKS: u8 = 107;
pub const DARK_BRICKS: u8 = 108;
pub const BOOKSHELF: u8 = 109;
pub const WORKBENCH: u8 = 110;
pub const FURNACE: u8 = 111;
pub const CHEST: u8 = 112;
pub const PUMPKIN: u8 = 113;
pub const MELON: u8 = 114;
pub const HAY: u8 = 115;
pub const GLOWSTONE: u8 = 116;
pub const LANTERN: u8 = 117;
pub const SPONGE: u8 = 118;
pub const BEAM: u8 = 119;
pub const COBWEB: u8 = 120;
pub const TORCH: u8 = 121;
pub const CLOUD: u8 = 122;
pub const HEAVEN_GRASS: u8 = 123;
pub const HEAVEN_DIRT: u8 = 124;
pub const GOLDEN_LEAVES: u8 = 125;
pub const LIGHT_LOG: u8 = 126;
pub const LIGHT_PLANKS: u8 = 127;
pub const HEAVEN_STONE: u8 = 128;
pub const HEAVEN_CRYSTAL: u8 = 129;
pub const PORTAL: u8 = 130;
pub const PORTAL_FRAME: u8 = 131;
pub const POLISHED_GRANITE: u8 = 132;
pub const POLISHED_MARBLE: u8 = 133;
pub const POLISHED_BASALT: u8 = 134;
pub const SLATE_BRICKS: u8 = 135;
pub const SLATE_TILES: u8 = 136;
pub const TUFF_BRICKS: u8 = 137;
pub const CHISELED_STONE: u8 = 138;
pub const CHISELED_SANDSTONE: u8 = 139;
pub const SMOOTH_SANDSTONE: u8 = 140;
pub const CUT_RED_SANDSTONE: u8 = 141;
pub const MARBLE_PILLAR: u8 = 142;
pub const OBSIDIAN_BRICKS: u8 = 143;
pub const CRACKED_STONE_BRICKS: u8 = 144;
pub const ICE_BRICKS: u8 = 145;
pub const SNOW_BRICKS: u8 = 146;
pub const HEAVEN_BRICKS: u8 = 147;
pub const GOLD_TILES: u8 = 148;
pub const QUARTZ: u8 = 149;
pub const QUARTZ_PILLAR: u8 = 150;
pub const TERRACOTTA0: u8 = 151;
pub const GLASS_EXTRA0: u8 = 167;
pub const LAMP0: u8 = 179;
pub const TULIP_RED: u8 = 187;
pub const TULIP_ORANGE: u8 = 188;
pub const TULIP_PINK: u8 = 189;
pub const LAVENDER: u8 = 190;
pub const BLUEBELL: u8 = 191;
pub const SUNFLOWER: u8 = 192;
pub const ROSE_BUSH: u8 = 193;
pub const BERRY_BUSH: u8 = 194;
pub const WHEAT_CROP: u8 = 195;
pub const CARROT_CROP: u8 = 196;
pub const SNOW_FLOWER: u8 = 197;
pub const DESERT_ROSE: u8 = 198;
pub const HEAVEN_LILY: u8 = 199;
pub const GLOW_MUSHROOM: u8 = 200;
pub const MUD: u8 = 201;
pub const PEAT: u8 = 202;
pub const MOSS_BLOCK: u8 = 203;
pub const PODZOL: u8 = 204;
pub const COARSE_DIRT: u8 = 205;
pub const FARMLAND: u8 = 206;
pub const SALT_BLOCK: u8 = 207;
pub const AMBER_BLOCK: u8 = 208;
pub const CORAL_RED: u8 = 209;
pub const CORAL_BLUE: u8 = 210;
pub const CORAL_YELLOW: u8 = 211;
pub const SEA_STONE: u8 = 212;
pub const SEA_LANTERN: u8 = 213;
pub const MAGMA: u8 = 214;
pub const PUMICE: u8 = 215;
pub const ASH_BLOCK: u8 = 216;
pub const RED_MUSHROOM_BLOCK: u8 = 217;
pub const BROWN_MUSHROOM_BLOCK: u8 = 218;
pub const MUSHROOM_STEM: u8 = 219;
pub const BAMBOO_BLOCK: u8 = 220;
pub const CRATE: u8 = 221;
pub const BARREL: u8 = 222;
pub const BEEHIVE: u8 = 223;
pub const HONEY_BLOCK: u8 = 224;
pub const JELLY_BLOCK: u8 = 225;
pub const ANVIL: u8 = 226;
pub const CAULDRON: u8 = 227;
pub const CANDLE: u8 = 228;
pub const IRON_BARS: u8 = 229;
pub const LATTICE: u8 = 230;
pub const LADDER: u8 = 231;
pub const ROPE: u8 = 232;
pub const HALO_STONE: u8 = 233;
pub const SUN_BLOCK: u8 = 234;
pub const CELESTIAL_GLASS: u8 = 235;
pub const STARDUST_BLOCK: u8 = 236;
pub const FEATHER_BLOCK: u8 = 237;
pub const RAINBOW_BLOCK: u8 = 238;
pub const ADOBE: u8 = 239;
pub const MOSAIC: u8 = 240;
pub const TURQUOISE_BLOCK: u8 = 241;
pub const FROST_STONE: u8 = 242;
pub const CONE_BLOCK: u8 = 243;
pub const FUR_BLOCK: u8 = 244;
pub const THATCH: u8 = 245;
pub const PAVING: u8 = 246;
pub const WHITEWASH: u8 = 247;
pub const TIMBER_FRAME: u8 = 248;
pub const BONE_BLOCK: u8 = 249;
pub const BLOCK_COUNT: usize = 250;

pub fn is_climbable(b: u8) -> bool {
    b == LADDER || b == ROPE
}

// ---------- доступ к реестру ----------
pub struct Registry {
    pub defs: Vec<BlockDef>,
    pub opaque: [bool; 256],
    pub solid: [bool; 256],
}

static ALL: OnceLock<(Registry, Vec<u8>, Vec<u16>)> = OnceLock::new();

fn all() -> &'static (Registry, Vec<u8>, Vec<u16>) {
    ALL.get_or_init(build_all)
}

pub fn reg() -> &'static Registry {
    &all().0
}

#[inline]
pub fn def(id: u8) -> &'static BlockDef {
    &reg().defs[id as usize]
}

#[inline]
pub fn is_opaque(id: u8) -> bool {
    reg().opaque[id as usize]
}

#[inline]
pub fn is_solid(id: u8) -> bool {
    reg().solid[id as usize]
}

/// Пиксели атласа (RGBA8, ATLAS×ATLAS).
pub fn atlas_pixels() -> &'static [u8] {
    &all().1
}

/// Плитка спрайта предмета/интерфейса (индексы см. items.rs).
pub fn item_tile(i: usize) -> u16 {
    all().2[i]
}

pub fn tile_uv(tile: u16) -> (f32, f32, f32, f32) {
    let (tx, ty) = ((tile as usize % ATLAS_TILES) as f32, (tile as usize / ATLAS_TILES) as f32);
    let s = 1.0 / ATLAS_TILES as f32;
    let e = 0.01 / ATLAS as f32;
    (tx * s + e, ty * s + e, (tx + 1.0) * s - e, (ty + 1.0) * s - e)
}

// ---------- пиксельная кухня ----------
pub fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_215.0
}

type Px = [u8; 4];
const CLEAR: Px = [0, 0, 0, 0];

const fn rgb(r: u8, g: u8, b: u8) -> Px {
    [r, g, b, 255]
}

fn shade(c: Px, k: f32) -> Px {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

fn mix(a: Px, b: Px, t: f32) -> Px {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).clamp(0.0, 255.0) as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])]
}

fn blotch(x: i32, y: i32, seed: u32) -> f32 {
    0.6 * hash(x.div_euclid(2), y.div_euclid(2), seed) + 0.4 * hash(x, y, seed ^ 0x9e37)
}

struct Atlas {
    px: Vec<u8>,
    next: u16,
}

impl Atlas {
    fn add(&mut self, f: impl Fn(i32, i32) -> Px) -> u16 {
        let t = self.next;
        assert!((t as usize) < ATLAS_TILES * ATLAS_TILES, "атлас переполнен");
        self.next += 1;
        let (tx, ty) = (t as usize % ATLAS_TILES, t as usize / ATLAS_TILES);
        for y in 0..TILE {
            for x in 0..TILE {
                let c = f(x as i32, y as i32);
                let i = ((ty * TILE + y) * ATLAS + tx * TILE + x) * 4;
                self.px[i..i + 4].copy_from_slice(&c);
            }
        }
        t
    }
}

// --- стили текстур ---
fn t_noise(base: Px, var: f32, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| shade(base, 1.0 - var + blotch(x, y, seed) * var * 2.0)
}

fn t_speck(base: Px, var: f32, sp: Px, chance: f32, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        if hash(x, y, seed + 1) > 1.0 - chance {
            sp
        } else {
            shade(base, 1.0 - var + blotch(x, y, seed) * var * 2.0)
        }
    }
}

fn t_stone(base: Px, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let mut c = shade(base, 0.82 + blotch(x, y, seed) * 0.3);
        let crack = ((x * 3 + y * 5 + (hash(y, 0, seed) * 6.0) as i32) % 13 == 0) && hash(x, y, seed + 1) > 0.4;
        if crack {
            c = shade(c, 0.74);
        }
        c
    }
}

fn t_layers(base: Px, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let band = (y + (hash(x / 4, 0, seed) * 2.0) as i32) / 3;
        shade(base, 0.85 + hash(band, 0, seed) * 0.2 + hash(x, y, seed) * 0.06)
    }
}

fn cells(x: i32, y: i32, seed: u32, n: i32) -> (f32, f32, i32) {
    let s = (TILE as i32 / n) as f32;
    let (cx, cy) = (x / (TILE as i32 / n), y / (TILE as i32 / n));
    let (mut d1, mut d2, mut id) = (f32::MAX, f32::MAX, 0);
    for oy in -1..=1 {
        for ox in -1..=1 {
            let (gx, gy) = (cx + ox, cy + oy);
            let (wx, wy) = (gx.rem_euclid(n), gy.rem_euclid(n));
            let px = gx as f32 * s + 0.6 + hash(wx, wy, seed) * (s - 1.2);
            let py = gy as f32 * s + 0.6 + hash(wx, wy, seed + 7) * (s - 1.2);
            let d = ((x as f32 + 0.5 - px).powi(2) + (y as f32 + 0.5 - py).powi(2)).sqrt();
            if d < d1 {
                d2 = d1;
                d1 = d;
                id = wy * n + wx;
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (d1, d2, id)
}

fn t_cobble(base: Px, moss: Option<Px>, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let (d1, d2, id) = cells(x, y, seed, 4);
        if d2 - d1 < 0.85 {
            shade(base, 0.55 + hash(x, y, seed) * 0.08)
        } else {
            let c = shade(base, 0.85 + hash(id, 0, seed + 2) * 0.28 - d1 * 0.04 + hash(x, y, seed) * 0.05);
            match moss {
                Some(m) if blotch(x, y, seed + 9) > 0.62 => shade(m, 0.85 + hash(x, y, 3) * 0.2),
                _ => c,
            }
        }
    }
}

fn t_ore(stone: Px, ore: Px, seed: u32) -> impl Fn(i32, i32) -> Px {
    let st = t_stone(stone, 31);
    move |x, y| {
        let (d1, _, id) = cells(x, y, seed, 3);
        if hash(id, 1, seed) > 0.35 && d1 < 1.7 {
            let hl = if d1 < 0.7 { 1.25 } else { 1.0 };
            shade(ore, hl * (0.9 + hash(x, y, seed) * 0.15))
        } else {
            st(x, y)
        }
    }
}

fn t_storage(c: Px, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let edge = x == 0 || y == 0 || x == 15 || y == 15;
        let bevel = x == 1 || y == 1;
        let inner = (x == 4 || x == 11 || y == 4 || y == 11) && (3..=12).contains(&x) && (3..=12).contains(&y);
        let k = if edge {
            0.6
        } else if bevel {
            1.3
        } else if inner {
            0.8
        } else {
            1.0 + hash(x, y, seed) * 0.08
        };
        shade(c, k)
    }
}

fn t_planks(base: Px, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let board = y / 4;
        let mut c = shade(base, 0.84 + hash(x / 3, board, seed) * 0.16 + hash(x, y, seed) * 0.06);
        let seam = (board * 5 + 3) % 16;
        if y % 4 == 3 || x == seam {
            c = shade(base, 0.6);
        } else if (x == (seam + 2) % 16 || x == (seam + 14) % 16) && y % 4 == 1 {
            c = shade(base, 0.45);
        }
        c
    }
}

fn t_log(bark: Px, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let stripe = hash(x, 0, seed);
        let mut c = shade(bark, 0.7 + stripe * 0.38 + hash(x, y, seed) * 0.08);
        if (x == 4 || x == 11) && (y % 7 == 2) {
            c = shade(bark, 0.55);
        }
        c
    }
}

fn t_birch(x: i32, y: i32) -> Px {
    let mut c = shade(rgb(226, 222, 210), 0.92 + hash(x, y, 47) * 0.1);
    if hash(x / 3, y, 48) > 0.84 && y % 5 != 0 {
        c = rgb(52, 48, 44);
    }
    c
}

fn t_logtop(bark: Px, wood: Px) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let d = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
        if d > 6.8 {
            shade(bark, 0.8 + hash(x, y, 3) * 0.2)
        } else {
            shade(wood, 0.86 + 0.14 * (d * 2.3).sin() + hash(x, y, 4) * 0.05)
        }
    }
}

fn t_leaves(c: Px, dot: Option<Px>, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        if hash(x, y, seed) < 0.16 {
            CLEAR
        } else {
            let k = 0.62 + hash(x.div_euclid(2), y.div_euclid(2), seed + 1) * 0.45;
            match dot {
                Some(d) if hash(x, y, seed + 2) > 0.95 => d,
                _ => shade(c, k),
            }
        }
    }
}

fn t_bricks(brick: Px, mortar: Px, bw: i32, bh: i32, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let row = y / bh;
        let off = if row % 2 == 1 { bw / 2 } else { 0 };
        if y % bh == bh - 1 || (x + off) % bw == bw - 1 {
            shade(mortar, 0.92 + hash(x, y, seed) * 0.08)
        } else {
            let tone = hash((x + off) / bw, row, seed + 1);
            shade(brick, 0.84 + tone * 0.2 + hash(x, y, seed) * 0.07)
        }
    }
}

fn t_stone_bricks(base: Px, moss: bool) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let row = y / 8;
        let off = if row % 2 == 1 { 4 } else { 0 };
        let mortar = y % 8 == 7 || (x + off) % 8 == 7;
        let mut c = if mortar {
            shade(base, 0.6)
        } else {
            let edge = y % 8 == 0 || (x + off) % 8 == 0;
            shade(base, if edge { 1.12 } else { 0.92 + hash(x, y, 105) * 0.1 })
        };
        if moss && blotch(x, y, 106) > 0.63 {
            c = shade(rgb(86, 128, 62), 0.85 + hash(x, y, 107) * 0.25);
        }
        c
    }
}

fn t_wool(c: Px) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let knit = if (x + y) % 4 < 2 { 1.04 } else { 0.94 };
        shade(c, knit * (0.94 + hash(x, y, 67) * 0.1))
    }
}

fn t_concrete(c: Px) -> impl Fn(i32, i32) -> Px {
    move |x, y| shade(c, 0.96 + hash(x, y, 83) * 0.06)
}

fn t_glass(tint: Option<Px>) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let edge = x == 0 || y == 0 || x == 15 || y == 15;
        let glint = (x + y == 9 || x + y == 10) && (3..=7).contains(&x);
        match tint {
            None if edge => rgb(196, 226, 238),
            None if glint => [250, 254, 255, 255],
            None => CLEAR,
            Some(t) if edge || glint => shade(t, if glint { 1.3 } else { 0.8 }),
            Some(t) => {
                let k = if hash(x, y, 99) > 0.5 { 1.0 } else { 0.94 };
                shade(t, k)
            }
        }
    }
}

fn t_grass_side(top: Px, under: impl Fn(i32, i32) -> Px, seed: u32) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let depth = 3 + (hash(x, 0, seed) * 3.0) as i32;
        if y < depth {
            shade(top, 0.82 + blotch(x, y, seed) * 0.3)
        } else if y == depth && hash(x, y, seed + 1) > 0.45 {
            shade(top, 0.66)
        } else {
            under(x, y)
        }
    }
}

/// Спрайт по маске: каждая буква — цвет из палитры, '.' — прозрачно.
fn t_mask(mask: [&'static str; 16], pal: Vec<(char, Px)>) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let ch = mask[y as usize].as_bytes().get(x as usize).copied().unwrap_or(b'.') as char;
        pal.iter().find(|(c, _)| *c == ch).map(|(_, p)| *p).unwrap_or(CLEAR)
    }
}

fn t_flower(petal: Px, center: Px) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let stem = rgb(70, 130, 50);
        let (cx, cy) = (7.5f32, 5.0f32);
        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
        if d < 1.3 {
            center
        } else if d < 3.4 && hash(x, y, 58) > 0.12 {
            shade(petal, 0.85 + hash(x, y, 59) * 0.25)
        } else if (x == 7 || x == 8) && y > 7 {
            stem
        } else if y == 12 && (5..=6).contains(&x) || y == 11 && (9..=10).contains(&x) {
            shade(stem, 1.15)
        } else {
            CLEAR
        }
    }
}

fn t_tuft(c: Px, seed: u32, dry: bool) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        // несколько травинок от низа к верху
        for b in 0..6 {
            let bx = 1 + b * 3 - (b / 3) + (hash(b, 0, seed) * 2.0) as i32;
            let h = 6 + (hash(b, 1, seed) * 9.0) as i32;
            let lean = if hash(b, 2, seed) > 0.5 { 1 } else { -1 };
            let top = 16 - h;
            if y >= top {
                let t = (y - top) as f32 / h as f32;
                let xx = bx + if t < 0.35 { lean } else { 0 };
                if x == xx || (dry && x == xx + lean && t < 0.2) {
                    return shade(c, 0.75 + t * 0.4);
                }
            }
        }
        CLEAR
    }
}

pub const COLOR_NAMES_F: [&str; 16] = [
    "Белая", "Светло-серая", "Серая", "Чёрная", "Коричневая", "Красная", "Оранжевая", "Жёлтая",
    "Лаймовая", "Зелёная", "Бирюзовая", "Голубая", "Синяя", "Фиолетовая", "Пурпурная", "Розовая",
];
pub const COLOR_NAMES_N: [&str; 16] = [
    "Белое", "Светло-серое", "Серое", "Чёрное", "Коричневое", "Красное", "Оранжевое", "Жёлтое",
    "Лаймовое", "Зелёное", "Бирюзовое", "Голубое", "Синее", "Фиолетовое", "Пурпурное", "Розовое",
];
pub const COLOR_NAMES_M: [&str; 16] = [
    "Белый", "Светло-серый", "Серый", "Чёрный", "Коричневый", "Красный", "Оранжевый", "Жёлтый",
    "Лаймовый", "Зелёный", "Бирюзовый", "Голубой", "Синий", "Фиолетовый", "Пурпурный", "Розовый",
];
const PALETTE16: [Px; 16] = [
    rgb(236, 238, 240), rgb(158, 162, 166), rgb(84, 90, 96), rgb(30, 31, 36),
    rgb(112, 74, 46), rgb(176, 46, 42), rgb(232, 120, 32), rgb(244, 200, 48),
    rgb(126, 196, 46), rgb(70, 112, 40), rgb(32, 138, 140), rgb(84, 172, 222),
    rgb(52, 62, 160), rgb(116, 50, 168), rgb(176, 58, 166), rgb(236, 140, 172),
];

pub fn palette16(i: usize) -> [u8; 4] {
    PALETTE16[i]
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn build_all() -> (Registry, Vec<u8>, Vec<u16>) {
    let mut a = Atlas { px: vec![0u8; ATLAS * ATLAS * 4], next: 0 };
    let mut defs: Vec<Option<BlockDef>> = vec![None; BLOCK_COUNT];
    let mut put = |id: u8, name: &'static str, tex: [u16; 3], shape: Shape, hardness: f32, tool: ToolNeed, cat: Cat, light: u8| {
        let solid = matches!(shape, Shape::Cube | Shape::Cutout);
        defs[id as usize] = Some(BlockDef { name, tex, shape, solid, light, hardness, tool, cat });
    };
    use Cat::*;
    use Shape::*;
    use ToolNeed::*;

    let dirt_c = rgb(134, 94, 62);
    let dirt_f = move |x: i32, y: i32| {
        if hash(x, y, 12) > 0.94 { rgb(168, 142, 112) } else { shade(dirt_c, 0.78 + blotch(x, y, 11) * 0.34) }
    };
    let grass_c = rgb(104, 172, 74);
    let stone_c = rgb(124, 130, 140);

    put(AIR, "Воздух", [0; 3], Air, 0.0, Any, Hidden, 0);

    let t_dirt = a.add(dirt_f);
    let t_grass_top = a.add(move |x, y| {
        let f = hash(x, y, 999);
        if f > 0.985 {
            if hash(x, y, 5) > 0.5 { rgb(252, 222, 92) } else { rgb(214, 206, 238) }
        } else {
            shade(grass_c, 0.8 + blotch(x, y, 21) * 0.32)
        }
    });
    let t_gside = a.add(t_grass_side(grass_c, dirt_f, 77));
    put(GRASS, "Луговая трава", [t_grass_top, t_gside, t_dirt], Cube, 0.6, Shovel, Nature, 0);
    put(DIRT, "Земля", [t_dirt; 3], Cube, 0.5, Shovel, Nature, 0);
    let t = a.add(t_stone(stone_c, 31));
    put(STONE, "Камень", [t; 3], Cube, 1.5, Pickaxe, Nature, 0);
    let t = a.add(t_cobble(rgb(140, 144, 152), None, 121));
    put(COBBLE, "Булыжник", [t; 3], Cube, 2.0, Pickaxe, Building, 0);
    let t = a.add(t_speck(rgb(230, 210, 156), 0.06, rgb(196, 170, 120), 0.07, 41));
    put(SAND, "Песок", [t; 3], Cube, 0.5, Shovel, Nature, 0);
    let t = a.add(move |x, y| {
        let (d1, d2, id) = cells(x, y, 61, 5);
        if d2 - d1 < 0.6 { rgb(96, 90, 88) } else { shade(rgb(150, 142, 138), 0.75 + hash(id, 3, 62) * 0.4 - d1 * 0.03) }
    });
    put(GRAVEL, "Гравий", [t; 3], Cube, 0.6, Shovel, Nature, 0);
    let t = a.add(t_noise(rgb(160, 166, 178), 0.06, 71));
    put(CLAY, "Глина", [t; 3], Cube, 0.6, Shovel, Nature, 0);
    let ss = rgb(222, 204, 150);
    let t_ss_top = a.add(t_noise(ss, 0.05, 81));
    let t_ss_side = a.add(t_layers(ss, 82));
    let t_ss_bot = a.add(t_noise(shade(ss, 0.92), 0.08, 83));
    put(SANDSTONE, "Песчаник", [t_ss_top, t_ss_side, t_ss_bot], Cube, 1.2, Pickaxe, Building, 0);
    let rs = rgb(196, 104, 52);
    let t = a.add(t_speck(rs, 0.08, shade(rs, 0.7), 0.06, 91));
    put(RED_SAND, "Красный песок", [t; 3], Cube, 0.5, Shovel, Nature, 0);
    let t_rss_top = a.add(t_noise(rgb(184, 98, 50), 0.05, 92));
    let t_rss_side = a.add(t_layers(rgb(184, 98, 50), 93));
    put(RED_SANDSTONE, "Красный песчаник", [t_rss_top, t_rss_side, t_rss_top], Cube, 1.2, Pickaxe, Building, 0);
    let snow_c = rgb(234, 242, 250);
    let t_snow = a.add(move |x, y| if hash(x, y, 111) > 0.97 { rgb(255, 255, 255) } else { shade(snow_c, 0.95 + hash(x, y, 112) * 0.05) });
    let t_snow_side = a.add(t_grass_side(snow_c, dirt_f, 113));
    put(SNOW_GRASS, "Заснеженная трава", [t_snow, t_snow_side, t_dirt], Cube, 0.6, Shovel, Nature, 0);
    put(SNOW_BLOCK, "Снег", [t_snow; 3], Cube, 0.3, Shovel, Nature, 0);
    let t = a.add(move |x, y| {
        let c = shade(rgb(150, 196, 240), 0.92 + hash(x, y, 131) * 0.06);
        if (x - y).rem_euclid(9) == 0 && hash(x, y, 132) > 0.3 { shade(c, 1.2) } else { c }
    });
    put(ICE, "Лёд", [t; 3], Cube, 0.5, Pickaxe, Nature, 0);
    let t = a.add(t_noise(rgb(124, 164, 224), 0.07, 141));
    put(PACKED_ICE, "Плотный лёд", [t; 3], Cube, 0.6, Pickaxe, Nature, 0);
    let t = a.add(move |x, y| {
        let wave = ((x + (y / 4) * 5) % 8 < 2) && y % 4 == 1;
        let c = shade(rgb(54, 122, 204), 0.9 + hash(x, y, 151) * 0.1);
        if wave { mix(c, rgb(170, 214, 255), 0.6) } else { c }
    });
    put(WATER, "Вода", [t; 3], Liquid, 0.0, Any, Hidden, 0);
    let t = a.add(move |x, y| {
        let v = blotch(x, y, 161);
        mix(rgb(208, 72, 16), rgb(255, 196, 64), (v * 1.4 - 0.3).clamp(0.0, 1.0))
    });
    put(LAVA, "Лава", [t; 3], Liquid, 0.0, Any, Hidden, 15);
    let t = a.add(t_cobble(rgb(70, 70, 76), None, 171));
    put(BEDROCK, "Бездонник", [t; 3], Cube, -1.0, Any, Hidden, 0);
    let t = a.add(t_cobble(rgb(140, 144, 152), Some(rgb(86, 128, 62)), 181));
    put(MOSSY_COBBLE, "Замшелый булыжник", [t; 3], Cube, 2.0, Pickaxe, Building, 0);
    let t = a.add(t_speck(rgb(172, 112, 96), 0.1, rgb(214, 170, 156), 0.12, 191));
    put(GRANITE, "Гранит", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(move |x, y| {
        let vein = ((x as f32 * 0.7 + (y as f32 * 0.5).sin() * 2.5) as i32 + y / 3) % 7 == 0;
        if vein { rgb(170, 170, 178) } else { shade(rgb(232, 230, 226), 0.95 + hash(x, y, 201) * 0.06) }
    });
    put(MARBLE, "Мрамор", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(move |x, y| {
        let col = x / 4;
        let c = shade(rgb(64, 64, 72), 0.85 + hash(col, 0, 211) * 0.25 + hash(x, y, 212) * 0.05);
        if x % 4 == 3 { shade(c, 0.7) } else { c }
    });
    put(BASALT, "Базальт", [t; 3], Cube, 1.6, Pickaxe, Building, 0);
    let t = a.add(t_layers(rgb(74, 78, 90), 221));
    put(SLATE, "Сланец", [t; 3], Cube, 2.2, Pickaxe, Nature, 0);
    let t = a.add(t_speck(rgb(112, 114, 100), 0.1, rgb(146, 148, 128), 0.1, 231));
    put(TUFF, "Туф", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(move |x, y| {
        let c = shade(rgb(30, 22, 44), 0.85 + blotch(x, y, 241) * 0.3);
        if hash(x, y, 242) > 0.95 { rgb(96, 70, 140) } else { c }
    });
    put(OBSIDIAN, "Обсидиан", [t; 3], Cube, 8.0, Pickaxe, Building, 0);

    let ores: [(u8, u8, &str, &str, Px); 8] = [
        (COAL_ORE, COAL_BLOCK, "Угольная руда", "Угольный блок", rgb(40, 40, 44)),
        (IRON_ORE, IRON_BLOCK, "Железная руда", "Железный блок", rgb(214, 180, 150)),
        (COPPER_ORE, COPPER_BLOCK, "Медная руда", "Медный блок", rgb(214, 116, 70)),
        (GOLD_ORE, GOLD_BLOCK, "Золотая руда", "Золотой блок", rgb(250, 212, 60)),
        (RUBY_ORE, RUBY_BLOCK, "Рубиновая руда", "Рубиновый блок", rgb(222, 40, 72)),
        (CRYSTAL_ORE, CRYSTAL_BLOCK, "Кристаллическая руда", "Кристаллический блок", rgb(150, 100, 240)),
        (SAPPHIRE_ORE, SAPPHIRE_BLOCK, "Сапфировая руда", "Сапфировый блок", rgb(48, 96, 230)),
        (MALACHITE_ORE, MALACHITE_BLOCK, "Малахитовая руда", "Малахитовый блок", rgb(40, 180, 120)),
    ];
    for (i, (ore, blk, n1, n2, c)) in ores.iter().enumerate() {
        let t = a.add(t_ore(stone_c, *c, 300 + i as u32 * 7));
        put(*ore, n1, [t; 3], Cube, 2.5 + i as f32 * 0.2, Pickaxe, Ores, 0);
        let st = if *blk == COAL_BLOCK { shade(*c, 1.4) } else { *c };
        let t = a.add(t_storage(st, 400 + i as u32));
        put(*blk, n2, [t; 3], Cube, 3.0, Pickaxe, Ores, 0);
    }

    // породы деревьев: (бревно, доски, листва, имя, кора, древесина, листья, ягоды)
    let woods: [(u8, &str, &str, &str, Px, Px, Px, Option<Px>); 5] = [
        (OAK_LOG, "Дубовое бревно", "Дубовые доски", "Дубовая листва", rgb(112, 80, 50), rgb(196, 152, 96), rgb(64, 140, 70), Some(rgb(232, 120, 96))),
        (SPRUCE_LOG, "Еловое бревно", "Еловые доски", "Хвоя", rgb(70, 50, 34), rgb(140, 100, 62), rgb(60, 122, 84), None),
        (BIRCH_LOG, "Берёзовое бревно", "Берёзовые доски", "Берёзовая листва", rgb(226, 222, 210), rgb(222, 204, 150), rgb(110, 168, 72), None),
        (SAKURA_LOG, "Бревно сакуры", "Доски сакуры", "Цветы сакуры", rgb(88, 54, 58), rgb(232, 182, 176), rgb(242, 160, 196), Some(rgb(255, 230, 240))),
        (PALM_LOG, "Пальмовое бревно", "Пальмовые доски", "Пальмовая листва", rgb(150, 120, 80), rgb(214, 180, 120), rgb(84, 172, 60), None),
    ];
    for (i, (log, nl, np, nv, bark, wood, leaf, dot)) in woods.iter().enumerate() {
        let side = if *log == BIRCH_LOG { a.add(t_birch) } else { a.add(t_log(*bark, 500 + i as u32)) };
        let top = a.add(t_logtop(*bark, *wood));
        put(*log, nl, [top, side, top], Cube, 1.4, Axe, Nature, 0);
        let t = a.add(t_planks(*wood, 510 + i as u32));
        put(*log + 1, np, [t; 3], Cube, 1.2, Axe, Building, 0);
        let t = a.add(t_leaves(*leaf, *dot, 520 + i as u32));
        put(*log + 2, nv, [t; 3], Cutout, 0.2, Any, Nature, 0);
    }

    let t = a.add(t_tuft(rgb(96, 170, 70), 561, false));
    put(TALL_GRASS, "Высокая трава", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(move |x, y| {
        let mid = 7.5 + ((y as f32) * 0.5).sin();
        let d = (x as f32 - mid).abs();
        let w = (16 - y) as f32 * 0.42;
        if y > 1 && d < w && (d < 0.8 || (y + x) % 3 != 0) { shade(rgb(64, 140, 74), 0.8 + hash(x, y, 571) * 0.3) } else { CLEAR }
    });
    put(FERN, "Папоротник", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_flower(rgb(250, 250, 250), rgb(250, 210, 60)));
    put(DAISY, "Ромашка", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_flower(rgb(220, 40, 40), rgb(30, 30, 30)));
    put(POPPY, "Мак", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_flower(rgb(80, 120, 240), rgb(240, 240, 255)));
    put(CORNFLOWER, "Василёк", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_flower(rgb(250, 214, 40), rgb(230, 150, 20)));
    put(DANDELION, "Одуванчик", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_tuft(rgb(150, 110, 60), 621, true));
    put(DEAD_BUSH, "Сухой куст", [t; 3], Cross, 0.0, Any, Nature, 0);
    let mush = |cap: Px, dots: bool| {
        move |x: i32, y: i32| {
            let d = ((x as f32 - 7.5).powi(2) / 30.0 + (y as f32 - 8.0).powi(2) / 9.0).sqrt();
            if y <= 8 && d < 1.0 {
                if dots && hash(x, y, 631) > 0.8 { rgb(250, 250, 250) } else { shade(cap, 0.85 + (8 - y) as f32 * 0.03) }
            } else if y > 8 && (6..=9).contains(&x) {
                rgb(226, 218, 196)
            } else {
                CLEAR
            }
        }
    };
    let t = a.add(mush(rgb(200, 40, 36), true));
    put(RED_MUSHROOM, "Красный гриб", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(mush(rgb(150, 110, 80), false));
    put(BROWN_MUSHROOM, "Бурый гриб", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(move |x, y| {
        let stalk = [2, 7, 12].iter().any(|&s| x == s || x == s + 1);
        if stalk {
            let c = shade(rgb(150, 196, 96), 0.85 + hash(x, 0, 651) * 0.2);
            if y % 5 == 0 { shade(c, 0.75) } else { c }
        } else {
            CLEAR
        }
    });
    put(REEDS, "Тростник", [t; 3], Cross, 0.0, Any, Nature, 0);
    let cac = rgb(70, 150, 64);
    let t_cs = a.add(move |x, y| {
        if x == 0 || x == 15 { CLEAR } else if (x == 3 || x == 12) && y % 4 == 1 { rgb(230, 230, 200) } else { shade(cac, if x % 4 == 2 { 0.8 } else { 1.0 } * (0.92 + hash(x, y, 661) * 0.1)) }
    });
    let t_ct = a.add(move |x, y| {
        let r = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
        if r > 7.2 { shade(cac, 0.7) } else { shade(cac, 1.0 + (r * 1.6).sin() * 0.08) }
    });
    put(CACTUS, "Кактус", [t_ct, t_cs, t_ct], Cutout, 0.5, Any, Nature, 0);

    for i in 0..16u8 {
        let t = a.add(t_wool(PALETTE16[i as usize]));
        put(WOOL0 + i, leak(format!("{} шерсть", COLOR_NAMES_F[i as usize])), [t; 3], Cube, 0.8, Any, Colored, 0);
    }
    for i in 0..16u8 {
        let t = a.add(t_concrete(PALETTE16[i as usize]));
        put(CONCRETE0 + i, leak(format!("{} бетон", COLOR_NAMES_M[i as usize])), [t; 3], Cube, 1.8, Pickaxe, Colored, 0);
    }
    let t = a.add(t_glass(None));
    put(GLASS, "Стекло", [t; 3], Cutout, 0.3, Any, Building, 0);
    let tints = [
        (GLASS_RED, "Красное стекло", rgb(200, 60, 60)),
        (GLASS_BLUE, "Синее стекло", rgb(60, 90, 210)),
        (GLASS_GREEN, "Зелёное стекло", rgb(70, 170, 80)),
        (GLASS_YELLOW, "Жёлтое стекло", rgb(230, 200, 60)),
    ];
    for (id, n, c) in tints {
        let t = a.add(t_glass(Some(c)));
        put(id, n, [t; 3], Cube, 0.3, Any, Colored, 0);
    }

    let t = a.add(t_bricks(rgb(176, 78, 56), rgb(206, 198, 186), 8, 4, 104));
    put(BRICKS, "Кирпич", [t; 3], Cube, 2.0, Pickaxe, Building, 0);
    let t = a.add(t_stone_bricks(rgb(132, 136, 144), false));
    put(STONE_BRICKS, "Каменный кирпич", [t; 3], Cube, 1.8, Pickaxe, Building, 0);
    let t = a.add(t_stone_bricks(rgb(132, 136, 144), true));
    put(MOSSY_STONE_BRICKS, "Замшелый каменный кирпич", [t; 3], Cube, 1.8, Pickaxe, Building, 0);
    let t = a.add(t_bricks(ss, shade(ss, 0.7), 8, 8, 107));
    put(SANDSTONE_BRICKS, "Песчаниковый кирпич", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(t_bricks(rgb(70, 40, 48), rgb(40, 30, 34), 8, 4, 108));
    put(DARK_BRICKS, "Тёмный кирпич", [t; 3], Cube, 2.0, Pickaxe, Building, 0);

    let oak_pl = t_planks(rgb(196, 152, 96), 510);
    let t_planks_oak = a.add(t_planks(rgb(196, 152, 96), 510));
    let t = a.add(move |x, y| {
        if y < 2 || y > 13 || y == 7 || y == 8 {
            oak_pl(x, y)
        } else {
            let book = x / 2;
            let cols = [rgb(150, 40, 40), rgb(40, 80, 150), rgb(50, 120, 60), rgb(160, 120, 40), rgb(100, 50, 120)];
            let c = cols[(hash(book, y / 7, 109) * 5.0) as usize % 5];
            if x % 2 == 1 { shade(c, 0.7) } else { shade(c, 0.95 + hash(x, y, 110) * 0.1) }
        }
    });
    put(BOOKSHELF, "Книжная полка", [t_planks_oak, t, t_planks_oak], Cube, 1.2, Axe, Decor, 0);
    let wb_top = a.add(move |x, y| {
        let grid = x % 5 == 0 || y % 5 == 0;
        if grid { rgb(110, 76, 46) } else { shade(rgb(176, 130, 80), 0.9 + hash(x, y, 111) * 0.12) }
    });
    let wb_side = a.add(move |x, y| {
        let tool = (y == 6 && (3..=7).contains(&x)) || (x == 5 && (6..=12).contains(&y)) || (y + x == 20 && (9..=13).contains(&x));
        if tool { rgb(120, 120, 128) } else if y < 3 { rgb(150, 104, 62) } else { shade(rgb(176, 130, 80), 0.85 + hash(x / 4, y / 4, 112) * 0.12) }
    });
    put(WORKBENCH, "Верстак", [wb_top, wb_side, t_planks_oak], Cube, 1.5, Axe, Decor, 0);
    let fur = a.add(move |x, y| {
        let hole = (4..=11).contains(&x) && (8..=13).contains(&y);
        if hole {
            if y > 11 { mix(rgb(250, 140, 40), rgb(255, 220, 90), hash(x, y, 113)) } else { rgb(28, 28, 30) }
        } else if (4..=11).contains(&x) && y == 3 {
            rgb(70, 70, 74)
        } else {
            t_stone(rgb(130, 132, 138), 114)(x, y)
        }
    });
    let fur_top = a.add(t_stone(rgb(140, 142, 148), 115));
    put(FURNACE, "Печь", [fur_top, fur, fur_top], Cube, 2.5, Pickaxe, Decor, 0);
    let chest = a.add(move |x, y| {
        let edge = x == 0 || x == 15 || y == 0 || y == 15 || y == 5;
        let lock = (7..=8).contains(&x) && (4..=7).contains(&y);
        if lock { rgb(220, 200, 120) } else if edge { rgb(90, 60, 30) } else { shade(rgb(176, 120, 60), 0.88 + hash(x, y / 3, 116) * 0.14) }
    });
    put(CHEST, "Сундук", [chest, chest, chest], Cube, 1.5, Axe, Decor, 0);
    let pk = rgb(226, 132, 30);
    let pk_side = a.add(move |x, y| shade(pk, if x % 4 == 0 { 0.78 } else { 0.95 + hash(x, y, 117) * 0.08 }));
    let pk_top = a.add(move |x, y| {
        let r = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
        if r < 1.8 { rgb(100, 140, 50) } else { shade(pk, 0.82 + (r * 0.9).cos() * 0.1) }
    });
    put(PUMPKIN, "Тыква", [pk_top, pk_side, pk_top], Cube, 1.0, Axe, Nature, 0);
    let ml = rgb(96, 160, 48);
    let ml_side = a.add(move |x, y| shade(ml, if (x + (y / 4) % 2) % 5 < 2 { 0.7 } else { 1.0 + hash(x, y, 118) * 0.06 }));
    let ml_top = a.add(t_noise(rgb(120, 170, 60), 0.08, 119));
    put(MELON, "Арбуз", [ml_top, ml_side, ml_top], Cube, 1.0, Axe, Nature, 0);
    let hay = rgb(214, 180, 60);
    let hay_side = a.add(move |x, y| {
        if y == 3 || y == 12 { rgb(150, 60, 40) } else { shade(hay, 0.8 + hash(x, 0, 120) * 0.25 + hash(x, y, 121) * 0.05) }
    });
    let hay_top = a.add(t_speck(hay, 0.12, shade(hay, 0.7), 0.15, 122));
    put(HAY, "Сноп сена", [hay_top, hay_side, hay_top], Cube, 0.5, Any, Decor, 0);
    let t = a.add(move |x, y| {
        let (d1, d2, _) = cells(x, y, 123, 4);
        if d2 - d1 < 0.8 { rgb(150, 110, 50) } else { mix(rgb(250, 210, 120), rgb(255, 250, 200), (1.0 - d1 / 3.0).clamp(0.0, 1.0)) }
    });
    put(GLOWSTONE, "Светокамень", [t; 3], Cube, 0.4, Any, Decor, 15);
    let t = a.add(move |x, y| {
        let frame = x < 2 || x > 13 || y < 2 || y > 13 || x == 7 || x == 8;
        if frame { rgb(60, 60, 66) } else { mix(rgb(255, 190, 80), rgb(255, 245, 200), 1.0 - ((x as f32 - 7.5).abs() + (y as f32 - 7.5).abs()) / 14.0) }
    });
    put(LANTERN, "Фонарь", [t; 3], Cube, 0.8, Pickaxe, Decor, 15);
    let t = a.add(move |x, y| {
        if hash(x, y, 124) > 0.8 || (x % 5 == 2 && y % 4 == 1) { rgb(160, 150, 40) } else { shade(rgb(214, 204, 72), 0.9 + hash(x, y, 125) * 0.12) }
    });
    put(SPONGE, "Губка", [t; 3], Cube, 0.4, Any, Decor, 0);
    let beam_side = a.add(move |x, y| {
        let c = shade(rgb(150, 110, 66), 0.85 + hash(x, 0, 126) * 0.2 + hash(x, y, 127) * 0.05);
        if x == 0 || x == 15 { shade(c, 0.7) } else { c }
    });
    let beam_top = a.add(t_logtop(rgb(110, 80, 48), rgb(150, 110, 66)));
    put(BEAM, "Опорная балка", [beam_top, beam_side, beam_top], Cube, 1.2, Axe, Building, 0);
    let t = a.add(move |x, y| {
        let (fx, fy) = (x as f32 - 7.5, y as f32 - 7.5);
        let r = (fx * fx + fy * fy).sqrt();
        let ang = fy.atan2(fx);
        let spoke = (ang * 8.0 / std::f32::consts::TAU).fract() < 0.12;
        let ring = (r % 3.2) < 0.5;
        if (spoke || ring) && r < 8.0 { [236, 236, 240, 255] } else { CLEAR }
    });
    put(COBWEB, "Паутина", [t; 3], Cross, 0.3, Any, Decor, 0);
    let t = a.add(move |x, y| {
        if (7..=8).contains(&x) {
            if y < 3 { rgb(255, 240, 160) } else if y < 5 { rgb(255, 170, 40) } else if y >= 6 { shade(rgb(120, 86, 50), 0.9 + hash(x, y, 128) * 0.15) } else { rgb(255, 120, 30) }
        } else if (y == 2 || y == 3) && (x == 6 || x == 9) {
            [255, 200, 90, 255]
        } else {
            CLEAR
        }
    });
    put(TORCH, "Факел", [t; 3], Cross, 0.0, Any, Decor, 14);

    // ---- Рай ----
    let t = a.add(move |x, y| shade(rgb(250, 252, 255), 0.94 + blotch(x, y, 129) * 0.08));
    put(CLOUD, "Облачный блок", [t; 3], Cube, 0.3, Any, Heaven, 0);
    let hg = rgb(150, 230, 170);
    let hd = rgb(214, 196, 160);
    let hdirt = move |x: i32, y: i32| shade(hd, 0.85 + blotch(x, y, 130) * 0.2);
    let t_hd = a.add(hdirt);
    let t_hg_top = a.add(move |x, y| if hash(x, y, 131) > 0.98 { rgb(255, 240, 150) } else { shade(hg, 0.85 + blotch(x, y, 132) * 0.25) });
    let t_hg_side = a.add(t_grass_side(hg, hdirt, 133));
    put(HEAVEN_GRASS, "Райская трава", [t_hg_top, t_hg_side, t_hd], Cube, 0.6, Shovel, Heaven, 0);
    put(HEAVEN_DIRT, "Райская земля", [t_hd; 3], Cube, 0.5, Shovel, Heaven, 0);
    let t = a.add(t_leaves(rgb(250, 210, 90), Some(rgb(255, 250, 210)), 134));
    put(GOLDEN_LEAVES, "Золотая листва", [t; 3], Cutout, 0.2, Any, Heaven, 0);
    let ls = a.add(t_log(rgb(236, 230, 214), 135));
    let lt = a.add(t_logtop(rgb(236, 230, 214), rgb(250, 236, 200)));
    put(LIGHT_LOG, "Светлое бревно", [lt, ls, lt], Cube, 1.4, Axe, Heaven, 0);
    let t = a.add(t_planks(rgb(246, 232, 200), 136));
    put(LIGHT_PLANKS, "Светлые доски", [t; 3], Cube, 1.2, Axe, Heaven, 0);
    let t = a.add(t_stone(rgb(206, 210, 226), 137));
    put(HEAVEN_STONE, "Райский камень", [t; 3], Cube, 1.5, Pickaxe, Heaven, 0);
    let t = a.add(t_ore(rgb(206, 210, 226), rgb(120, 240, 250), 138));
    put(HEAVEN_CRYSTAL, "Райский кристалл", [t; 3], Cube, 2.5, Pickaxe, Heaven, 10);
    let t = a.add(move |x, y| {
        let (fx, fy) = (x as f32 - 7.5, y as f32 - 7.5);
        let r = (fx * fx + fy * fy).sqrt();
        let a = fy.atan2(fx) + r * 0.6;
        let v = (a * 3.0).sin() * 0.5 + 0.5;
        mix(rgb(255, 214, 110), rgb(255, 250, 230), v * (1.0 - r / 12.0).max(0.0))
    });
    put(PORTAL, "Портал в Рай", [t; 3], Cutout, -1.0, Any, Heaven, 11);
    let t = a.add(t_storage(rgb(232, 196, 90), 139));
    put(PORTAL_FRAME, "Каркас портала", [t; 3], Cube, 5.0, Pickaxe, Heaven, 0);


    // ======== расширение: ещё 118 блоков ========
    let t = a.add(t_noise(rgb(178, 118, 100), 0.05, 701));
    put(POLISHED_GRANITE, "Полированный гранит", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(move |x, y| { let v = (x * 2 + y) % 11 == 0; if v { rgb(196, 196, 204) } else { shade(rgb(240, 238, 234), 0.97 + hash(x, y, 702) * 0.04) } });
    put(POLISHED_MARBLE, "Полированный мрамор", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(t_noise(rgb(70, 70, 80), 0.05, 703));
    put(POLISHED_BASALT, "Полированный базальт", [t; 3], Cube, 1.6, Pickaxe, Building, 0);
    let t = a.add(t_bricks(rgb(70, 74, 86), rgb(40, 42, 50), 8, 4, 704));
    put(SLATE_BRICKS, "Сланцевый кирпич", [t; 3], Cube, 2.0, Pickaxe, Building, 0);
    let t = a.add(t_bricks(rgb(74, 78, 90), rgb(44, 46, 54), 4, 4, 705));
    put(SLATE_TILES, "Сланцевая плитка", [t; 3], Cube, 2.0, Pickaxe, Building, 0);
    let t = a.add(t_bricks(rgb(112, 114, 100), rgb(70, 72, 62), 8, 8, 706));
    put(TUFF_BRICKS, "Туфовый кирпич", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(move |x, y| { let ring = (x == 3 || x == 12 || y == 3 || y == 12) && (3..=12).contains(&x) && (3..=12).contains(&y); let edge = x == 0 || y == 0 || x == 15 || y == 15; shade(rgb(132, 136, 144), if edge { 0.7 } else if ring { 0.8 } else { 1.0 + hash(x, y, 707) * 0.06 }) });
    put(CHISELED_STONE, "Резной камень", [t; 3], Cube, 1.8, Pickaxe, Building, 0);
    let ss2 = rgb(222, 204, 150);
    let t = a.add(move |x, y| { let glyph = (x % 5 == 2 && (3..12).contains(&y)) || (y % 6 == 3 && (x % 5) < 4 && x > 1 && x < 14) || ((x / 5 + y / 6) % 2 == 0 && x % 5 == 1 && y % 6 == 1); if glyph { shade(ss2, 0.62) } else { shade(ss2, 0.95 + hash(x, y, 708) * 0.06) } });
    put(CHISELED_SANDSTONE, "Песчаник с письменами", [t_ss_top, t, t_ss_top], Cube, 1.2, Pickaxe, Building, 0);
    let t = a.add(t_noise(rgb(226, 210, 160), 0.03, 709));
    put(SMOOTH_SANDSTONE, "Гладкий песчаник", [t; 3], Cube, 1.2, Pickaxe, Building, 0);
    let t = a.add(t_bricks(rgb(186, 100, 52), rgb(150, 76, 40), 16, 8, 710));
    put(CUT_RED_SANDSTONE, "Тёсаный красный песчаник", [t; 3], Cube, 1.2, Pickaxe, Building, 0);
    let mp_top = a.add(t_logtop(rgb(220, 218, 214), rgb(240, 238, 234)));
    let t = a.add(move |x, y| shade(rgb(238, 236, 232), if x % 4 == 0 { 0.82 } else { 0.98 + hash(x, y, 711) * 0.04 }));
    put(MARBLE_PILLAR, "Мраморная колонна", [mp_top, t, mp_top], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(t_bricks(rgb(40, 28, 58), rgb(16, 10, 24), 8, 4, 712));
    put(OBSIDIAN_BRICKS, "Обсидиановый кирпич", [t; 3], Cube, 8.0, Pickaxe, Building, 0);
    let sb = t_stone_bricks(rgb(132, 136, 144), false);
    let t = a.add(move |x, y| if (x * 3 + y * 2) % 17 == 0 && hash(x, y, 713) > 0.3 { rgb(60, 62, 66) } else { sb(x, y) });
    put(CRACKED_STONE_BRICKS, "Треснувший каменный кирпич", [t; 3], Cube, 1.8, Pickaxe, Building, 0);
    let t = a.add(t_bricks(rgb(160, 204, 244), rgb(110, 160, 220), 8, 4, 714));
    put(ICE_BRICKS, "Ледяной кирпич", [t; 3], Cube, 0.8, Pickaxe, Building, 0);
    let t = a.add(t_bricks(rgb(240, 244, 250), rgb(196, 206, 224), 8, 8, 715));
    put(SNOW_BRICKS, "Снежный кирпич", [t; 3], Cube, 0.4, Shovel, Building, 0);
    let t = a.add(t_bricks(rgb(232, 230, 244), rgb(240, 210, 120), 8, 4, 716));
    put(HEAVEN_BRICKS, "Райский кирпич", [t; 3], Cube, 1.5, Pickaxe, Heaven, 0);
    let t = a.add(t_bricks(rgb(250, 210, 70), rgb(190, 140, 30), 4, 4, 717));
    put(GOLD_TILES, "Золотая плитка", [t; 3], Cube, 2.0, Pickaxe, Building, 0);
    let t = a.add(t_noise(rgb(240, 236, 236), 0.03, 718));
    put(QUARTZ, "Белый кварц", [t; 3], Cube, 1.2, Pickaxe, Building, 0);
    let q_top = a.add(t_logtop(rgb(226, 222, 222), rgb(244, 242, 242)));
    let t = a.add(move |x, y| shade(rgb(244, 240, 240), if x == 1 || x == 14 { 0.85 } else { 0.98 + hash(x, y, 719) * 0.03 }));
    put(QUARTZ_PILLAR, "Кварцевая колонна", [q_top, t, q_top], Cube, 1.2, Pickaxe, Building, 0);
    for i in 0..16u8 {
        let c = mix(PALETTE16[i as usize], rgb(160, 90, 60), 0.35);
        let t = a.add(t_noise(c, 0.05, 720 + i as u32));
        put(TERRACOTTA0 + i, leak(format!("{} терракота", COLOR_NAMES_F[i as usize])), [t; 3], Cube, 1.25, Pickaxe, Colored, 0);
    }
    let mut gi = 0u8;
    for i in 0..16usize {
        if matches!(i, 5 | 7 | 9 | 12) {
            continue;
        }
        let t = a.add(t_glass(Some(PALETTE16[i])));
        put(GLASS_EXTRA0 + gi, leak(format!("{} стекло", COLOR_NAMES_N[i])), [t; 3], Cube, 0.3, Any, Colored, 0);
        gi += 1;
    }
    let lamp_cols = [0usize, 5, 6, 7, 8, 11, 13, 15];
    for (k, &i) in lamp_cols.iter().enumerate() {
        let c = PALETTE16[i];
        let t = a.add(move |x, y| {
            let frame = x == 0 || y == 0 || x == 15 || y == 15 || (x + y) % 8 == 0 && (x == 1 || x == 14);
            if frame { rgb(70, 60, 50) } else { mix(c, rgb(255, 255, 240), 0.35 + 0.3 * (1.0 - ((x as f32 - 7.5).abs() + (y as f32 - 7.5).abs()) / 15.0)) }
        });
        put(LAMP0 + k as u8, leak(format!("{} лампа", COLOR_NAMES_F[i])), [t; 3], Cube, 0.4, Any, Decor, 15);
    }
    let flowers: [(u8, &str, Px, Px); 4] = [
        (TULIP_RED, "Красный тюльпан", rgb(220, 40, 50), rgb(250, 120, 120)),
        (TULIP_ORANGE, "Оранжевый тюльпан", rgb(240, 130, 30), rgb(255, 200, 100)),
        (TULIP_PINK, "Розовый тюльпан", rgb(240, 140, 180), rgb(255, 210, 230)),
        (BLUEBELL, "Колокольчик", rgb(90, 110, 230), rgb(180, 190, 255)),
    ];
    for (id, n, c, c2) in flowers {
        let t = a.add(move |x, y| {
            let cup = y >= 2 && y <= 7 && (x - 8).abs() <= 3 - (y == 2) as i32 && !(y == 2 && x == 8);
            if cup { if x == 6 || y == 3 { c2 } else { c } } else if (x == 7 || x == 8) && y > 7 { rgb(70, 130, 50) } else if y == 11 && (4..=6).contains(&x) { rgb(90, 160, 60) } else { CLEAR }
        });
        put(id, n, [t; 3], Cross, 0.0, Any, Nature, 0);
    }
    let t = a.add(move |x, y| if (x == 7 || x == 8) && y > 5 { rgb(70, 120, 50) } else if y <= 6 && (x - 7).abs() <= 1 + (y % 2) && hash(x, y, 731) > 0.15 { mix(rgb(150, 110, 220), rgb(200, 170, 250), hash(x, y, 732)) } else { CLEAR });
    put(LAVENDER, "Лаванда", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_flower(rgb(250, 200, 30), rgb(110, 70, 30)));
    put(SUNFLOWER, "Подсолнух", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_leaves(rgb(60, 130, 60), Some(rgb(220, 40, 60)), 733));
    put(ROSE_BUSH, "Розовый куст", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_leaves(rgb(70, 140, 60), Some(rgb(200, 30, 90)), 734));
    put(BERRY_BUSH, "Ягодный куст", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(move |x, y| { let stalk = x % 3 == 1; if stalk && y > 3 { rgb(200, 170, 70) } else if stalk && y <= 3 && y > 0 { rgb(226, 196, 90) } else { CLEAR } });
    put(WHEAT_CROP, "Пшеница", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(move |x, y| if y > 11 && (x - 5).abs() < 2 || y > 11 && (x - 11).abs() < 2 { rgb(240, 120, 30) } else if y <= 11 && (x % 6 == 5 || x % 6 == 4) && y > 4 { rgb(80, 160, 60) } else { CLEAR });
    put(CARROT_CROP, "Морковная грядка", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_flower(rgb(220, 240, 255), rgb(120, 200, 255)));
    put(SNOW_FLOWER, "Снежноцвет", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_flower(rgb(230, 90, 120), rgb(250, 220, 120)));
    put(DESERT_ROSE, "Пустынная роза", [t; 3], Cross, 0.0, Any, Nature, 0);
    let t = a.add(t_flower(rgb(255, 250, 230), rgb(255, 210, 80)));
    put(HEAVEN_LILY, "Райская лилия", [t; 3], Cross, 0.0, Any, Heaven, 6);
    let gm = mush(rgb(80, 220, 210), true);
    let t = a.add(gm);
    put(GLOW_MUSHROOM, "Светящийся гриб", [t; 3], Cross, 0.0, Any, Nature, 8);
    let t = a.add(t_speck(rgb(96, 70, 50), 0.08, rgb(70, 50, 36), 0.1, 741));
    put(MUD, "Грязь", [t; 3], Cube, 0.5, Shovel, Nature, 0);
    let t = a.add(t_noise(rgb(60, 42, 30), 0.12, 742));
    put(PEAT, "Торф", [t; 3], Cube, 0.5, Shovel, Nature, 0);
    let t = a.add(t_noise(rgb(90, 140, 60), 0.12, 743));
    put(MOSS_BLOCK, "Мох", [t; 3], Cube, 0.3, Any, Nature, 0);
    let pz_top = a.add(t_speck(rgb(110, 76, 44), 0.1, rgb(150, 110, 60), 0.2, 744));
    let pz_side = a.add(t_grass_side(rgb(110, 76, 44), dirt_f, 745));
    put(PODZOL, "Подзол", [pz_top, pz_side, t_dirt], Cube, 0.6, Shovel, Nature, 0);
    let t = a.add(t_speck(rgb(126, 90, 60), 0.12, rgb(110, 110, 110), 0.12, 746));
    put(COARSE_DIRT, "Каменистая земля", [t; 3], Cube, 0.6, Shovel, Nature, 0);
    let t = a.add(move |x, y| if y % 4 == 0 { rgb(70, 46, 30) } else { shade(rgb(100, 68, 44), 0.9 + hash(x, y, 747) * 0.15) });
    put(FARMLAND, "Пашня", [t, t_dirt, t_dirt], Cube, 0.6, Shovel, Nature, 0);
    let t = a.add(t_speck(rgb(236, 232, 226), 0.04, rgb(255, 255, 255), 0.15, 748));
    put(SALT_BLOCK, "Соляной блок", [t; 3], Cube, 1.0, Pickaxe, Nature, 0);
    let t = a.add(move |x, y| mix(rgb(220, 130, 30), rgb(255, 200, 80), blotch(x, y, 749)));
    put(AMBER_BLOCK, "Янтарь", [t; 3], Cube, 1.0, Pickaxe, Decor, 4);
    for (k, (id, n, c)) in [(CORAL_RED, "Красный коралл", rgb(220, 70, 80)), (CORAL_BLUE, "Синий коралл", rgb(70, 110, 220)), (CORAL_YELLOW, "Жёлтый коралл", rgb(230, 200, 60))].into_iter().enumerate() {
        let t = a.add(move |x, y| { let (d1, _, _) = cells(x, y, 750 + k as u32, 4); shade(c, 0.7 + (1.0 - d1 / 3.0).clamp(0.0, 1.0) * 0.5) });
        put(id, n, [t; 3], Cube, 0.6, Any, Nature, 0);
    }
    let t = a.add(move |x, y| { let (d1, d2, _) = cells(x, y, 753, 4); if d2 - d1 < 0.8 { rgb(40, 110, 100) } else { shade(rgb(80, 170, 150), 0.85 + hash(x, y, 754) * 0.2) } });
    put(SEA_STONE, "Морской камень", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(move |x, y| { let edge = x < 2 || y < 2 || x > 13 || y > 13; if edge { rgb(120, 200, 190) } else { mix(rgb(200, 250, 240), rgb(255, 255, 255), hash(x / 2, y / 2, 755)) } });
    put(SEA_LANTERN, "Морской фонарь", [t; 3], Cube, 0.4, Any, Decor, 15);
    let t = a.add(move |x, y| { let (d1, d2, _) = cells(x, y, 756, 4); if d2 - d1 < 0.9 { rgb(255, 140, 30) } else { shade(rgb(90, 30, 20), 0.8 + hash(x, y, 757) * 0.3) } });
    put(MAGMA, "Магма", [t; 3], Cube, 0.8, Pickaxe, Nature, 6);
    let t = a.add(t_speck(rgb(200, 196, 180), 0.08, rgb(150, 146, 130), 0.25, 758));
    put(PUMICE, "Пемза", [t; 3], Cube, 0.8, Pickaxe, Nature, 0);
    let t = a.add(t_noise(rgb(90, 88, 92), 0.1, 759));
    put(ASH_BLOCK, "Пепел", [t; 3], Cube, 0.4, Shovel, Nature, 0);
    let t = a.add(move |x, y| if hash(x / 3, y / 3, 760) > 0.75 { rgb(250, 250, 240) } else { shade(rgb(190, 40, 36), 0.9 + hash(x, y, 761) * 0.12) });
    put(RED_MUSHROOM_BLOCK, "Шляпка красного гриба", [t; 3], Cube, 0.3, Axe, Nature, 0);
    let t = a.add(t_noise(rgb(150, 110, 80), 0.08, 762));
    put(BROWN_MUSHROOM_BLOCK, "Шляпка бурого гриба", [t; 3], Cube, 0.3, Axe, Nature, 0);
    let t = a.add(t_noise(rgb(220, 214, 196), 0.05, 763));
    put(MUSHROOM_STEM, "Ножка гриба", [t; 3], Cube, 0.3, Axe, Nature, 0);
    let bt = a.add(t_logtop(rgb(150, 170, 60), rgb(200, 210, 110)));
    let t = a.add(move |x, y| shade(rgb(160, 186, 70), if x % 4 == 0 || y % 8 == 0 { 0.75 } else { 0.95 + hash(x, y, 764) * 0.1 }));
    put(BAMBOO_BLOCK, "Бамбуковый блок", [bt, t, bt], Cube, 1.0, Axe, Building, 0);
    let t = a.add(move |x, y| { let edge = x < 2 || y < 2 || x > 13 || y > 13 || x == y || x == 15 - y; shade(rgb(170, 128, 76), if edge { 0.72 } else { 0.95 + hash(x / 3, y, 765) * 0.12 }) });
    put(CRATE, "Ящик", [t; 3], Cube, 1.0, Axe, Decor, 0);
    let br_top = a.add(t_logtop(rgb(90, 60, 36), rgb(150, 106, 62)));
    let t = a.add(move |x, y| if y == 3 || y == 12 { rgb(70, 70, 76) } else { shade(rgb(140, 98, 56), if x % 4 == 0 { 0.75 } else { 0.95 + hash(x, y, 766) * 0.1 }) });
    put(BARREL, "Бочка", [br_top, t, br_top], Cube, 1.2, Axe, Decor, 0);
    let t = a.add(move |x, y| if y % 4 == 2 { rgb(170, 120, 50) } else if x % 5 == 2 && y % 4 == 0 { rgb(60, 40, 20) } else { shade(rgb(220, 180, 90), 0.9 + hash(x, y, 767) * 0.12) });
    put(BEEHIVE, "Улей", [t; 3], Cube, 0.8, Axe, Decor, 0);
    let t = a.add(move |x, y| mix(rgb(230, 150, 30), rgb(255, 210, 90), blotch(x, y, 768) * 0.8 + if x < 2 || y < 2 { 0.3 } else { 0.0 }));
    put(HONEY_BLOCK, "Медовый блок", [t; 3], Cube, 0.3, Any, Decor, 0);
    let t = a.add(move |x, y| { let edge = x < 2 || y < 2 || x > 13 || y > 13; let c = rgb(120, 220, 110); if edge { c } else { mix(c, rgb(200, 255, 190), hash(x / 3, y / 3, 769) * 0.4) } });
    put(JELLY_BLOCK, "Желейный блок (пружинит)", [t; 3], Cube, 0.2, Any, Decor, 0);
    let t = a.add(move |x, y| shade(rgb(70, 72, 78), if y < 4 { 1.1 } else if (4..12).contains(&x) { 0.95 } else { 0.7 } * (0.95 + hash(x, y, 770) * 0.08)));
    put(ANVIL, "Наковальня", [t; 3], Cube, 4.0, Pickaxe, Decor, 0);
    let ct = a.add(move |x, y| if x < 2 || y < 2 || x > 13 || y > 13 { rgb(60, 60, 66) } else { mix(rgb(60, 110, 200), rgb(110, 170, 240), hash(x, y, 771)) });
    let cs = a.add(move |x, y| shade(rgb(64, 64, 70), if y > 12 || x < 2 || x > 13 { 0.7 } else { 0.95 + hash(x, y, 772) * 0.08 }));
    put(CAULDRON, "Котёл", [ct, cs, cs], Cube, 3.0, Pickaxe, Decor, 0);
    let t = a.add(move |x, y| if (7..=8).contains(&x) && y > 7 { rgb(240, 230, 210) } else if (7..=8).contains(&x) && (4..=7).contains(&y) { rgb(255, 200, 80) } else if x == 8 && y == 3 { rgb(255, 250, 200) } else { CLEAR });
    put(CANDLE, "Свеча", [t; 3], Cross, 0.0, Any, Decor, 10);
    let t = a.add(move |x, y| if x % 5 == 1 || y == 1 || y == 14 { shade(rgb(110, 112, 120), 0.9 + hash(x, y, 773) * 0.15) } else { CLEAR });
    put(IRON_BARS, "Железная решётка", [t; 3], Cutout, 2.0, Pickaxe, Decor, 0);
    let t = a.add(move |x, y| if (x + y) % 6 == 0 || (x - y).rem_euclid(6) == 0 { shade(rgb(170, 128, 76), 0.9 + hash(x, y, 774) * 0.12) } else { CLEAR });
    put(LATTICE, "Деревянная решётка", [t; 3], Cutout, 0.8, Axe, Decor, 0);
    let t = a.add(move |x, y| if x == 2 || x == 3 || x == 12 || x == 13 || y % 4 == 1 { shade(rgb(160, 118, 70), 0.9 + hash(x, y, 775) * 0.12) } else { CLEAR });
    put(LADDER, "Лестница", [t; 3], Cutout, 0.4, Axe, Decor, 0);
    let t = a.add(move |x, y| if (x == 7 || x == 8) && (y + x) % 3 != 0 { rgb(180, 150, 100) } else if x == 7 || x == 8 { rgb(140, 110, 70) } else { CLEAR });
    put(ROPE, "Верёвка", [t; 3], Cross, 0.0, Any, Decor, 0);
    let t = a.add(move |x, y| { let r = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt(); if (r - 5.0).abs() < 1.2 { rgb(255, 240, 170) } else { shade(rgb(214, 214, 230), 0.95 + hash(x, y, 776) * 0.05) } });
    put(HALO_STONE, "Нимбовый камень", [t; 3], Cube, 1.5, Pickaxe, Heaven, 9);
    let t = a.add(move |x, y| { let r = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt(); mix(rgb(255, 255, 220), rgb(255, 190, 60), (r / 10.0).min(1.0)) });
    put(SUN_BLOCK, "Солнечный блок", [t; 3], Cube, 0.8, Any, Heaven, 15);
    let t = a.add(t_glass(Some(rgb(250, 230, 160))));
    put(CELESTIAL_GLASS, "Небесное стекло", [t; 3], Cube, 0.3, Any, Heaven, 0);
    let t = a.add(move |x, y| if hash(x, y, 777) > 0.9 { rgb(255, 255, 255) } else { shade(rgb(60, 50, 120), 0.8 + hash(x, y, 778) * 0.3) });
    put(STARDUST_BLOCK, "Звёздная пыль", [t; 3], Cube, 0.5, Shovel, Heaven, 5);
    let t = a.add(move |x, y| shade(rgb(250, 248, 240), if (x + y / 2) % 5 == 0 { 0.85 } else { 0.98 + hash(x, y, 779) * 0.03 }));
    put(FEATHER_BLOCK, "Перьевой блок", [t; 3], Cube, 0.2, Any, Heaven, 0);
    let t = a.add(move |x, y| { let k = ((x + y) / 3) % 6; [rgb(240, 80, 80), rgb(250, 170, 60), rgb(250, 230, 80), rgb(100, 210, 100), rgb(80, 150, 240), rgb(160, 100, 230)][k as usize] });
    put(RAINBOW_BLOCK, "Радужный блок", [t; 3], Cube, 0.8, Any, Heaven, 4);
    let t = a.add(t_bricks(rgb(196, 150, 104), rgb(160, 120, 80), 8, 8, 780));
    put(ADOBE, "Саманный кирпич", [t; 3], Cube, 1.0, Pickaxe, Building, 0);
    let t = a.add(move |x, y| { let k = (x / 2 + y / 2) % 4; let c = [rgb(60, 140, 180), rgb(230, 210, 150), rgb(190, 80, 50), rgb(250, 240, 220)][k as usize]; if x % 2 == 1 && y % 2 == 1 { shade(c, 0.8) } else { c } });
    put(MOSAIC, "Мозаика", [t; 3], Cube, 1.0, Pickaxe, Building, 0);
    let t = a.add(t_ore(rgb(196, 170, 120), rgb(60, 200, 190), 781));
    put(TURQUOISE_BLOCK, "Бирюзовая порода", [t; 3], Cube, 1.5, Pickaxe, Ores, 0);
    let t = a.add(move |x, y| if (x * 2 + y * 3) % 13 == 0 { rgb(220, 240, 255) } else { shade(rgb(130, 150, 170), 0.85 + blotch(x, y, 782) * 0.25) });
    put(FROST_STONE, "Морозный камень", [t; 3], Cube, 1.6, Pickaxe, Nature, 0);
    let t = a.add(move |x, y| { let (d1, d2, _) = cells(x, y, 783, 4); if d2 - d1 < 0.7 { rgb(80, 50, 30) } else { shade(rgb(150, 100, 60), 0.85 + (1.0 - d1 / 3.0).max(0.0) * 0.3) } });
    put(CONE_BLOCK, "Шишечный блок", [t; 3], Cube, 0.6, Axe, Decor, 0);
    let t = a.add(move |x, y| shade(rgb(140, 110, 80), 0.75 + hash(x, y / 2, 784) * 0.35));
    put(FUR_BLOCK, "Меховой блок", [t; 3], Cube, 0.4, Any, Decor, 0);
    let t = a.add(move |x, y| { let stripe = (y + (x % 3)) % 4 == 0; shade(rgb(214, 180, 90), if stripe { 0.7 } else { 0.9 + hash(x, y, 785) * 0.15 }) });
    put(THATCH, "Соломенная кровля", [t; 3], Cube, 0.5, Any, Building, 0);
    let t = a.add(move |x, y| { let (d1, d2, id) = cells(x, y, 786, 3); if d2 - d1 < 0.7 { rgb(90, 80, 70) } else { shade(rgb(170, 160, 146), 0.85 + hash(id, 0, 787) * 0.25 - d1 * 0.03) } });
    put(PAVING, "Мощёная дорожка", [t; 3], Cube, 1.5, Pickaxe, Building, 0);
    let t = a.add(move |x, y| shade(rgb(242, 238, 228), 0.94 + blotch(x, y, 788) * 0.06));
    put(WHITEWASH, "Побелённая стена", [t; 3], Cube, 1.0, Pickaxe, Building, 0);
    let t = a.add(move |x, y| { let beam = x < 2 || x > 13 || y < 2 || y > 13 || (x - y).abs() < 2; if beam { shade(rgb(100, 66, 40), 0.9 + hash(x, y, 789) * 0.1) } else { shade(rgb(240, 234, 220), 0.95 + hash(x, y, 790) * 0.05) } });
    put(TIMBER_FRAME, "Фахверк", [t; 3], Cube, 1.2, Axe, Building, 0);
    let bt2 = a.add(t_logtop(rgb(220, 214, 196), rgb(236, 230, 214)));
    let t = a.add(move |x, y| shade(rgb(232, 226, 206), if x % 5 == 0 { 0.85 } else { 0.97 + hash(x, y, 791) * 0.04 }));
    put(BONE_BLOCK, "Костяной блок", [bt2, t, bt2], Cube, 1.5, Pickaxe, Decor, 0);

    // портал проходим насквозь
    let portal = defs[PORTAL as usize].as_mut().unwrap();
    portal.solid = false;
    for id in [COBWEB, LADDER, IRON_BARS, LATTICE] {
        if id == LADDER || id == COBWEB {
            defs[id as usize].as_mut().unwrap().solid = false;
        }
    }

    let defs: Vec<BlockDef> = defs.into_iter().enumerate().map(|(i, d)| d.unwrap_or_else(|| panic!("блок {i} не описан"))).collect();
    let mut opaque = [false; 256];
    let mut solid = [false; 256];
    for (i, d) in defs.iter().enumerate() {
        opaque[i] = d.shape == Shape::Cube;
        solid[i] = d.solid;
    }

    let items = crate::items::item_sprites(&mut |f| a.add(f));
    (Registry { defs, opaque, solid }, a.px, items)
}

/// Генератор плитки для предметов (используется из items.rs).
pub fn mask_tile(mask: [&'static str; 16], pal: Vec<(char, [u8; 4])>) -> impl Fn(i32, i32) -> [u8; 4] {
    t_mask(mask, pal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_over_100_blocks() {
        let r = reg();
        let real = r.defs.iter().filter(|d| d.cat != Cat::Hidden).count();
        assert!(real >= 100, "real = {real}");
        assert!(is_opaque(STONE));
        assert!(!is_opaque(GLASS));
        assert!(!is_solid(WATER));
        assert_eq!(atlas_pixels().len(), ATLAS * ATLAS * 4);
    }
}
