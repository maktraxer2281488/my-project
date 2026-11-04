//! Предметы: блоки, оружие и инструменты, броня.

use crate::blocks::{self, mask_tile};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mat {
    Wood,
    Leather,
    Stone,
    Iron,
    Gold,
    Ruby,
    Crystal,
}

impl Mat {
    pub fn name_m(self) -> &'static str {
        match self {
            Mat::Wood => "Деревянный",
            Mat::Leather => "Кожаный",
            Mat::Stone => "Каменный",
            Mat::Iron => "Железный",
            Mat::Gold => "Золотой",
            Mat::Ruby => "Рубиновый",
            Mat::Crystal => "Кристаллический",
        }
    }
    pub fn name_f(self) -> &'static str {
        match self {
            Mat::Wood => "Деревянная",
            Mat::Leather => "Кожаная",
            Mat::Stone => "Каменная",
            Mat::Iron => "Железная",
            Mat::Gold => "Золотая",
            Mat::Ruby => "Рубиновая",
            Mat::Crystal => "Кристаллическая",
        }
    }
    pub fn name_n(self) -> &'static str {
        match self {
            Mat::Wood => "Деревянные",
            Mat::Leather => "Кожаные",
            Mat::Stone => "Каменные",
            Mat::Iron => "Железные",
            Mat::Gold => "Золотые",
            Mat::Ruby => "Рубиновые",
            Mat::Crystal => "Кристаллические",
        }
    }
    /// (основной, тёмный, светлый)
    fn colors(self) -> ([u8; 4], [u8; 4], [u8; 4]) {
        match self {
            Mat::Wood => ([170, 124, 72, 255], [92, 64, 36, 255], [210, 168, 110, 255]),
            Mat::Leather => ([150, 92, 52, 255], [84, 50, 28, 255], [190, 130, 84, 255]),
            Mat::Stone => ([140, 142, 148, 255], [72, 74, 80, 255], [188, 190, 196, 255]),
            Mat::Iron => ([214, 216, 222, 255], [110, 112, 120, 255], [250, 250, 252, 255]),
            Mat::Gold => ([248, 206, 60, 255], [160, 110, 20, 255], [255, 244, 160, 255]),
            Mat::Ruby => ([220, 40, 70, 255], [110, 14, 36, 255], [255, 140, 160, 255]),
            Mat::Crystal => ([150, 110, 244, 255], [70, 40, 150, 255], [130, 230, 250, 255]),
        }
    }
    /// Скорость добычи инструментом.
    pub fn speed(self) -> f32 {
        match self {
            Mat::Wood | Mat::Leather => 2.0,
            Mat::Stone => 3.5,
            Mat::Iron => 5.0,
            Mat::Gold => 7.0,
            Mat::Ruby => 8.0,
            Mat::Crystal => 10.0,
        }
    }
    fn tier(self) -> f32 {
        match self {
            Mat::Wood | Mat::Leather => 0.0,
            Mat::Stone => 1.0,
            Mat::Iron => 2.0,
            Mat::Gold => 1.5,
            Mat::Ruby => 3.0,
            Mat::Crystal => 4.0,
        }
    }
}

pub const TOOL_MATS: [Mat; 6] = [Mat::Wood, Mat::Stone, Mat::Iron, Mat::Gold, Mat::Ruby, Mat::Crystal];
pub const ARMOR_MATS: [Mat; 5] = [Mat::Leather, Mat::Iron, Mat::Gold, Mat::Ruby, Mat::Crystal];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolKind {
    Sword,
    Pickaxe,
    Axe,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Helmet,
    Chest,
    Legs,
    Boots,
}

pub const SLOTS: [Slot; 4] = [Slot::Helmet, Slot::Chest, Slot::Legs, Slot::Boots];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Empty,
    Block(u8),
    Tool(ToolKind, Mat),
    Armor(Slot, Mat),
}

impl Item {
    pub fn name(self) -> String {
        match self {
            Item::Empty => String::new(),
            Item::Block(b) => blocks::def(b).name.to_string(),
            Item::Tool(ToolKind::Sword, m) => format!("{} меч", m.name_m()),
            Item::Tool(ToolKind::Pickaxe, m) => format!("{} кирка", m.name_f()),
            Item::Tool(ToolKind::Axe, m) => format!("{} топор", m.name_m()),
            Item::Armor(Slot::Helmet, m) => format!("{} шлем", m.name_m()),
            Item::Armor(Slot::Chest, m) => format!("{} нагрудник", m.name_m()),
            Item::Armor(Slot::Legs, m) => format!("{} поножи", m.name_n()),
            Item::Armor(Slot::Boots, m) => format!("{} ботинки", m.name_n()),
        }
    }

    /// Урон по существам.
    pub fn damage(self) -> f32 {
        match self {
            Item::Tool(ToolKind::Sword, m) => 4.0 + m.tier() * 1.5,
            Item::Tool(ToolKind::Axe, m) => 3.0 + m.tier() * 1.2,
            Item::Tool(ToolKind::Pickaxe, m) => 2.0 + m.tier() * 0.6,
            _ => 1.0,
        }
    }

    /// Очки брони предмета.
    pub fn armor_points(self) -> i32 {
        let Item::Armor(slot, m) = self else { return 0 };
        let base = match m {
            Mat::Leather => [1, 3, 2, 1],
            Mat::Iron => [2, 6, 5, 2],
            Mat::Gold => [2, 5, 3, 1],
            Mat::Ruby => [3, 7, 6, 3],
            _ => [3, 8, 6, 3],
        };
        base[slot as usize]
    }

    /// Множитель скорости добычи блока.
    pub fn mining_speed(self, need: blocks::ToolNeed) -> f32 {
        match (self, need) {
            (Item::Tool(ToolKind::Pickaxe, m), blocks::ToolNeed::Pickaxe) => m.speed(),
            (Item::Tool(ToolKind::Axe, m), blocks::ToolNeed::Axe) => m.speed(),
            (Item::Tool(ToolKind::Sword, _), _) => 1.3,
            _ => 1.0,
        }
    }

    pub fn description(self) -> Option<String> {
        match self {
            Item::Tool(k, m) => Some(match k {
                ToolKind::Sword => format!("Урон: {:.0}", self.damage()),
                _ => format!("Урон: {:.0} · скорость добычи ×{:.1}", self.damage(), m.speed()),
            }),
            Item::Armor(..) => Some(format!("Защита: +{}", self.armor_points())),
            _ => None,
        }
    }

    /// Индекс плитки в атласе для иконки предмета (не блока).
    pub fn sprite_index(self) -> Option<usize> {
        match self {
            Item::Tool(k, m) => {
                let mi = TOOL_MATS.iter().position(|x| *x == m)?;
                Some(k as usize * TOOL_MATS.len() + mi)
            }
            Item::Armor(s, m) => {
                let mi = ARMOR_MATS.iter().position(|x| *x == m)?;
                Some(18 + s as usize * ARMOR_MATS.len() + mi)
            }
            _ => None,
        }
    }
}

pub const HUD_HEART_FULL: usize = 38;
pub const HUD_HEART_HALF: usize = 39;
pub const HUD_HEART_EMPTY: usize = 40;
pub const HUD_ARMOR_FULL: usize = 41;
pub const HUD_ARMOR_HALF: usize = 42;
pub const HUD_ARMOR_EMPTY: usize = 43;

const SWORD: [&str; 16] = [
    "..............DD",
    ".............DHD",
    "............DHMD",
    "...........DHMD.",
    "..........DHMD..",
    ".........DHMD...",
    "........DHMD....",
    "...DD..DHMD.....",
    "...DMDDHMD......",
    "....DMHMD.......",
    "....DwMD........",
    "...DwDDMD.......",
    "..DwD..DD.......",
    ".DwD............",
    "DDD.............",
    "................",
];
const PICKAXE: [&str; 16] = [
    "................",
    "....DDDDDD......",
    "..DDHHHHMMDD....",
    ".DHMMDDDDMMMD...",
    ".DMD....DwDMMD..",
    "DMD....DwD.DMMD.",
    "DD....DwD...DMD.",
    "D....DwD.....DMD",
    "....DwD......DD.",
    "...DwD..........",
    "..DwD...........",
    ".DwD............",
    "DwD.............",
    "DD..............",
    "................",
    "................",
];
const AXE: [&str; 16] = [
    "................",
    ".......DDD......",
    "......DHHMD.....",
    ".....DHMMMMD....",
    ".....DMMMDwD....",
    "......DMDwD.....",
    ".......DwD......",
    "......DwD.......",
    ".....DwD........",
    "....DwD.........",
    "...DwD..........",
    "..DwD...........",
    ".DwD............",
    "DwD.............",
    "DD..............",
    "................",
];
const HELMET: [&str; 16] = [
    "................",
    "................",
    "................",
    "....DDDDDDDD....",
    "...DHHHHMMMMD...",
    "..DHMMMMMMMMMD..",
    "..DMMMMMMMMMMD..",
    "..DMMDDDDDDMMD..",
    "..DMD......DMD..",
    "..DMD......DMD..",
    "..DDD......DDD..",
    "................",
    "................",
    "................",
    "................",
    "................",
];
const CHEST: [&str; 16] = [
    "................",
    "..DDDD....DDDD..",
    ".DHMMD....DMMMD.",
    ".DMMMDDDDDDMMMD.",
    ".DMMMMMMMMMMMMD.",
    ".DDDMMMMMMMMDDD.",
    "...DHMMMMMMMD...",
    "...DMMMMMMMMD...",
    "...DMMMMMMMMD...",
    "...DMMMMMMMMD...",
    "...DMMMMMMMMD...",
    "...DMMMMMMMMD...",
    "...DDDDDDDDDD...",
    "................",
    "................",
    "................",
];
const LEGS: [&str; 16] = [
    "................",
    "...DDDDDDDDDD...",
    "...DHMMMMMMMD...",
    "...DMMMMMMMMD...",
    "...DMMMDDMMMD...",
    "...DMMMD.DMMD...",
    "...DMMD..DMMD...",
    "...DMMD..DMMD...",
    "...DMMD..DMMD...",
    "...DMMD..DMMD...",
    "...DMMD..DMMD...",
    "...DMMD..DMMD...",
    "...DDDD..DDDD...",
    "................",
    "................",
    "................",
];
const BOOTS: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "..DDDD....DDDD..",
    "..DHMD....DHMD..",
    "..DMMD....DMMD..",
    "..DMMD....DMMD..",
    "..DMMD....DMMD..",
    ".DMMMD....DMMMD.",
    "DMMMMD....DMMMMD",
    "DDDDDD....DDDDDD",
    "................",
    "................",
    "................",
];
const HEART: [&str; 16] = [
    "................",
    "................",
    "..DDDD...DDDD...",
    ".DHHRRD.DRRRRD..",
    "DHHRRRRDRRRRRRD.",
    "DHRRRRRRRRRRRRD.",
    "DRRRRRRRRRRRRRD.",
    "DRRRRRRRRRRRRRD.",
    ".DRRRRRRRRRRRD..",
    "..DRRRRRRRRRD...",
    "...DRRRRRRRD....",
    "....DRRRRRD.....",
    ".....DRRRD......",
    "......DRD.......",
    ".......D........",
    "................",
];
const SHIELD: [&str; 16] = [
    "................",
    "..DDDDDDDDDDDD..",
    ".DHHHMMMMMMMMMD.",
    ".DHMMMMMMMMMMMD.",
    ".DMMMMMMMMMMMMD.",
    ".DMMMMMMMMMMMMD.",
    ".DMMMMMMMMMMMMD.",
    ".DMMMMMMMMMMMMD.",
    "..DMMMMMMMMMMD..",
    "..DMMMMMMMMMMD..",
    "...DMMMMMMMMD...",
    "....DMMMMMMD....",
    ".....DMMMMD.....",
    "......DDDD......",
    "................",
    "................",
];

/// Рисует все спрайты предметов и интерфейса; возвращает их плитки по порядку.
pub fn item_sprites(add: &mut dyn FnMut(Box<dyn Fn(i32, i32) -> [u8; 4]>) -> u16) -> Vec<u16> {
    let wood = [120, 82, 44, 255];
    let mut out = Vec::new();
    for mask in [SWORD, PICKAXE, AXE] {
        for m in TOOL_MATS {
            let (main, dark, light) = m.colors();
            out.push(add(Box::new(mask_tile(mask, vec![('M', main), ('D', dark), ('H', light), ('w', wood)]))));
        }
    }
    for mask in [HELMET, CHEST, LEGS, BOOTS] {
        for m in ARMOR_MATS {
            let (main, dark, light) = m.colors();
            out.push(add(Box::new(mask_tile(mask, vec![('M', main), ('D', dark), ('H', light)]))));
        }
    }
    let red = [220, 30, 40, 255];
    let dark = [40, 10, 14, 255];
    let hl = [255, 170, 170, 255];
    let empty = [60, 30, 34, 255];
    out.push(add(Box::new(mask_tile(HEART, vec![('R', red), ('D', dark), ('H', hl)]))));
    let half = mask_tile(HEART, vec![('R', red), ('D', dark), ('H', hl)]);
    let half_e = mask_tile(HEART, vec![('R', empty), ('D', dark), ('H', empty)]);
    out.push(add(Box::new(move |x, y| if x < 8 { half(x, y) } else { half_e(x, y) })));
    out.push(add(Box::new(mask_tile(HEART, vec![('R', empty), ('D', dark), ('H', empty)]))));
    let st = [200, 204, 214, 255];
    let sd = [60, 64, 74, 255];
    let se = [70, 74, 84, 255];
    out.push(add(Box::new(mask_tile(SHIELD, vec![('M', st), ('D', sd), ('H', [250, 250, 255, 255])]))));
    let sh = mask_tile(SHIELD, vec![('M', st), ('D', sd), ('H', [250, 250, 255, 255])]);
    let sh_e = mask_tile(SHIELD, vec![('M', se), ('D', sd), ('H', se)]);
    out.push(add(Box::new(move |x, y| if x < 8 { sh(x, y) } else { sh_e(x, y) })));
    out.push(add(Box::new(mask_tile(SHIELD, vec![('M', se), ('D', sd), ('H', se)]))));
    out
}

/// Каталог предметов для инвентаря, по вкладкам.
pub fn catalog(tab: usize) -> Vec<Item> {
    use blocks::Cat;
    let by_cat = |c: Cat| -> Vec<Item> {
        (1..blocks::BLOCK_COUNT as u8).filter(|&b| blocks::def(b).cat == c).map(Item::Block).collect()
    };
    match tab {
        0 => by_cat(Cat::Building),
        1 => by_cat(Cat::Nature),
        2 => by_cat(Cat::Colored),
        3 => by_cat(Cat::Ores),
        4 => by_cat(Cat::Decor),
        5 => by_cat(Cat::Heaven),
        6 => {
            let mut v = Vec::new();
            for k in [ToolKind::Sword, ToolKind::Pickaxe, ToolKind::Axe] {
                for m in TOOL_MATS {
                    v.push(Item::Tool(k, m));
                }
            }
            v
        }
        _ => {
            let mut v = Vec::new();
            for m in ARMOR_MATS {
                for s in SLOTS {
                    v.push(Item::Armor(s, m));
                }
            }
            v
        }
    }
}

pub const TABS: [&str; 8] = ["Стройка", "Природа", "Цветные", "Руды", "Декор", "Рай", "Оружие", "Броня"];
