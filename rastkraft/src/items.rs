//! Предметы: блоки, оружие и инструменты, броня, еда, материалы, зелья, трофеи.

use crate::blocks::{self, mask_tile};
use crate::icons::{self, Icon};
use crate::mobdefs::{DropKind, MOBS};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mat {
    Wood,
    Leather,
    Fur,
    Stone,
    Copper,
    Iron,
    Gold,
    Ruby,
    Sapphire,
    Malachite,
    Crystal,
    Heaven,
}

pub const ALL_MATS: [Mat; 12] = [Mat::Wood, Mat::Leather, Mat::Fur, Mat::Stone, Mat::Copper, Mat::Iron, Mat::Gold, Mat::Ruby, Mat::Sapphire, Mat::Malachite, Mat::Crystal, Mat::Heaven];
pub const TOOL_MATS: [Mat; 10] = [Mat::Wood, Mat::Stone, Mat::Copper, Mat::Iron, Mat::Gold, Mat::Ruby, Mat::Sapphire, Mat::Malachite, Mat::Crystal, Mat::Heaven];
pub const ARMOR_MATS: [Mat; 10] = [Mat::Leather, Mat::Fur, Mat::Copper, Mat::Iron, Mat::Gold, Mat::Ruby, Mat::Sapphire, Mat::Malachite, Mat::Crystal, Mat::Heaven];

impl Mat {
    pub fn adj(self) -> [&'static str; 3] {
        match self {
            Mat::Wood => ["Деревянный", "Деревянная", "Деревянные"],
            Mat::Leather => ["Кожаный", "Кожаная", "Кожаные"],
            Mat::Fur => ["Меховой", "Меховая", "Меховые"],
            Mat::Stone => ["Каменный", "Каменная", "Каменные"],
            Mat::Copper => ["Медный", "Медная", "Медные"],
            Mat::Iron => ["Железный", "Железная", "Железные"],
            Mat::Gold => ["Золотой", "Золотая", "Золотые"],
            Mat::Ruby => ["Рубиновый", "Рубиновая", "Рубиновые"],
            Mat::Sapphire => ["Сапфировый", "Сапфировая", "Сапфировые"],
            Mat::Malachite => ["Малахитовый", "Малахитовая", "Малахитовые"],
            Mat::Crystal => ["Кристаллический", "Кристаллическая", "Кристаллические"],
            Mat::Heaven => ["Небесный", "Небесная", "Небесные"],
        }
    }
    /// (основной, тёмный, светлый)
    pub fn colors(self) -> ([u8; 4], [u8; 4], [u8; 4]) {
        let t = |a: (u8, u8, u8), b: (u8, u8, u8), c: (u8, u8, u8)| ([a.0, a.1, a.2, 255], [b.0, b.1, b.2, 255], [c.0, c.1, c.2, 255]);
        match self {
            Mat::Wood => t((170, 124, 72), (92, 64, 36), (210, 168, 110)),
            Mat::Leather => t((150, 92, 52), (84, 50, 28), (190, 130, 84)),
            Mat::Fur => t((190, 176, 156), (110, 96, 80), (236, 226, 210)),
            Mat::Stone => t((140, 142, 148), (72, 74, 80), (188, 190, 196)),
            Mat::Copper => t((214, 120, 70), (120, 60, 30), (250, 170, 120)),
            Mat::Iron => t((214, 216, 222), (110, 112, 120), (250, 250, 252)),
            Mat::Gold => t((248, 206, 60), (160, 110, 20), (255, 244, 160)),
            Mat::Ruby => t((220, 40, 70), (110, 14, 36), (255, 140, 160)),
            Mat::Sapphire => t((50, 100, 230), (20, 40, 120), (140, 180, 255)),
            Mat::Malachite => t((40, 180, 120), (14, 90, 60), (140, 240, 190)),
            Mat::Crystal => t((150, 110, 244), (70, 40, 150), (130, 230, 250)),
            Mat::Heaven => t((255, 236, 180), (200, 150, 60), (255, 255, 240)),
        }
    }
    pub fn speed(self) -> f32 {
        match self {
            Mat::Wood | Mat::Leather | Mat::Fur => 2.0,
            Mat::Stone => 3.5,
            Mat::Copper => 4.2,
            Mat::Iron => 5.0,
            Mat::Gold => 7.0,
            Mat::Malachite => 6.0,
            Mat::Sapphire => 7.5,
            Mat::Ruby => 8.0,
            Mat::Crystal => 10.0,
            Mat::Heaven => 12.0,
        }
    }
    pub fn tier(self) -> f32 {
        match self {
            Mat::Wood | Mat::Leather | Mat::Fur => 0.0,
            Mat::Stone => 1.0,
            Mat::Copper => 1.4,
            Mat::Gold => 1.5,
            Mat::Iron => 2.0,
            Mat::Malachite => 2.4,
            Mat::Sapphire => 2.7,
            Mat::Ruby => 3.0,
            Mat::Crystal => 4.0,
            Mat::Heaven => 5.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ToolKind {
    Sword,
    Pickaxe,
    Axe,
    Shovel,
    Dagger,
    Spear,
    Mace,
    Katana,
    Battleaxe,
    Warhammer,
    Scythe,
    Halberd,
    Rapier,
    Hoe,
}

pub const TOOL_KINDS: [ToolKind; 14] = [
    ToolKind::Sword, ToolKind::Pickaxe, ToolKind::Axe, ToolKind::Shovel, ToolKind::Dagger, ToolKind::Spear, ToolKind::Mace,
    ToolKind::Katana, ToolKind::Battleaxe, ToolKind::Warhammer, ToolKind::Scythe, ToolKind::Halberd, ToolKind::Rapier,
    ToolKind::Hoe,
];

impl ToolKind {
    /// Оружие (а не инструмент для добычи).
    pub fn is_weapon(self) -> bool {
        !matches!(self, ToolKind::Pickaxe | ToolKind::Axe | ToolKind::Shovel | ToolKind::Hoe)
    }
    /// Широкий взмах задевает существ рядом с целью.
    pub fn sweeps(self) -> bool {
        matches!(self, ToolKind::Battleaxe | ToolKind::Scythe | ToolKind::Halberd)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Slot {
    Helmet,
    Chest,
    Legs,
    Boots,
}

pub const SLOTS: [Slot; 4] = [Slot::Helmet, Slot::Chest, Slot::Legs, Slot::Boots];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Eff {
    None,
    Heal,
    Regen,
    Speed,
    Jump,
    Strength,
    Night,
    FireRes,
    SlowFall,
    Saturation,
    Haste,
}

impl Eff {
    pub fn name(self) -> &'static str {
        match self {
            Eff::None => "",
            Eff::Heal => "Исцеление",
            Eff::Regen => "Регенерация",
            Eff::Speed => "Скорость",
            Eff::Jump => "Прыгучесть",
            Eff::Strength => "Сила",
            Eff::Night => "Ночное зрение",
            Eff::FireRes => "Огнестойкость",
            Eff::SlowFall => "Плавное падение",
            Eff::Saturation => "Сытость",
            Eff::Haste => "Спешка",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TKind {
    Mat,
    Food(u8, Eff, f32),
    Potion(Eff, f32),
}

#[derive(Clone, Debug)]
pub struct ThingDef {
    pub name: &'static str,
    pub kind: TKind,
    pub icon: Icon,
    pub c1: [u8; 4],
    pub c2: [u8; 4],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Item {
    Empty,
    Block(u16),
    Tool(ToolKind, Mat),
    Armor(Slot, Mat),
    Thing(u16),
}

// ---- вещи (индексы) ----
pub const STICK: u16 = 0;
pub const COAL: u16 = 1;
pub const IRON_INGOT: u16 = 2;
pub const COPPER_INGOT: u16 = 3;
pub const GOLD_INGOT: u16 = 4;
pub const RUBY: u16 = 5;
pub const CRYSTAL: u16 = 6;
pub const SAPPHIRE: u16 = 7;
pub const MALACHITE: u16 = 8;
pub const HEAVEN_DUST: u16 = 9;
pub const HEAVEN_INGOT: u16 = 10;
pub const THREAD: u16 = 11;
pub const LEATHER: u16 = 12;
pub const FUR: u16 = 13;
pub const BONE: u16 = 14;
pub const FEATHER: u16 = 15;
pub const SLIME: u16 = 16;
pub const WHEAT: u16 = 17;
pub const FLOUR: u16 = 18;
pub const SUGAR: u16 = 19;
pub const CLAY_BALL: u16 = 20;
pub const BRICK: u16 = 21;
pub const BOTTLE: u16 = 22;
pub const BOWL: u16 = 23;
pub const PAPER: u16 = 24;
pub const BOOK: u16 = 25;
pub const SALT: u16 = 26;
pub const AMBER: u16 = 27;
pub const HONEYCOMB: u16 = 28;
pub const WAX: u16 = 29;
pub const RESIN: u16 = 30;
pub const FLINT: u16 = 31;
pub const REED: u16 = 32;
pub const EGG: u16 = 33;
pub const MILK: u16 = 34;
pub const COCOA: u16 = 35;
pub const DYE0: u16 = 36;
pub const APPLE: u16 = 52;
pub const GOLDEN_APPLE: u16 = 53;
pub const BREAD: u16 = 54;
pub const CARROT: u16 = 55;
pub const BAKED_CARROT: u16 = 56;
pub const POTATO: u16 = 57;
pub const BAKED_POTATO: u16 = 58;
pub const BERRIES: u16 = 59;
pub const HONEY_JAR: u16 = 60;
pub const PUMPKIN_PIE: u16 = 61;
pub const COOKIE: u16 = 62;
pub const CAKE: u16 = 63;
pub const CHEESE: u16 = 64;
pub const MUSHROOM_SOUP: u16 = 65;
pub const STEW: u16 = 66;
pub const RAW_BEEF: u16 = 67;
pub const STEAK: u16 = 68;
pub const RAW_CHICKEN: u16 = 69;
pub const ROAST_CHICKEN: u16 = 70;
pub const RAW_MUTTON: u16 = 71;
pub const ROAST_MUTTON: u16 = 72;
pub const RAW_FISH: u16 = 73;
pub const COOKED_FISH: u16 = 74;
pub const SALMON: u16 = 75;
pub const SUSHI: u16 = 76;
pub const CACTUS_FRUIT: u16 = 77;
pub const DATES: u16 = 78;
pub const COCONUT: u16 = 79;
pub const MELON_SLICE: u16 = 80;
pub const JAM: u16 = 81;
pub const SWEET_ROLL: u16 = 82;
pub const KEBAB: u16 = 83;
pub const PANCAKES: u16 = 84;
pub const PORRIDGE: u16 = 85;
pub const NECTAR: u16 = 86;
pub const AMBROSIA: u16 = 87;
pub const STARFRUIT: u16 = 88;
pub const ICE_CREAM: u16 = 89;
pub const MORS: u16 = 90;
pub const GINGERBREAD: u16 = 91;
pub const POTION0: u16 = 92;
pub const MOB_DROP0: u16 = 102;
/// Вещи версии 0.9 идут после трофеев.
pub const EXTRA0: u16 = MOB_DROP0 + 100;
pub const STEEL_INGOT: u16 = EXTRA0 + 0;
pub const BRONZE_INGOT: u16 = EXTRA0 + 1;
pub const PEARL: u16 = EXTRA0 + 2;
pub const SILK: u16 = EXTRA0 + 3;
pub const CLOTH: u16 = EXTRA0 + 4;
pub const STRAP: u16 = EXTRA0 + 5;
pub const GEAR: u16 = EXTRA0 + 6;
pub const SCROLL: u16 = EXTRA0 + 7;
pub const INK: u16 = EXTRA0 + 8;
pub const GUNPOWDER: u16 = EXTRA0 + 9;
pub const CHARCOAL: u16 = EXTRA0 + 10;
pub const OBSIDIAN_SHARD: u16 = EXTRA0 + 11;
pub const CORAL_SHARD: u16 = EXTRA0 + 12;
pub const GEM_DUST: u16 = EXTRA0 + 13;
pub const MAPLE_SYRUP: u16 = EXTRA0 + 14;
pub const PIZZA: u16 = EXTRA0 + 15;
pub const DONUT: u16 = EXTRA0 + 16;
pub const BURGER: u16 = EXTRA0 + 17;
pub const CHOCOLATE: u16 = EXTRA0 + 18;
pub const CANDY: u16 = EXTRA0 + 19;
pub const PEAR: u16 = EXTRA0 + 20;
pub const GRAPES: u16 = EXTRA0 + 21;
pub const CHERRIES: u16 = EXTRA0 + 22;
pub const PLUM: u16 = EXTRA0 + 23;
pub const ORANGE: u16 = EXTRA0 + 24;
pub const LEMON: u16 = EXTRA0 + 25;
pub const NUTS: u16 = EXTRA0 + 26;
pub const CARAMEL_APPLE: u16 = EXTRA0 + 27;
pub const BORSCHT: u16 = EXTRA0 + 28;
pub const SHCHI: u16 = EXTRA0 + 29;
pub const PELMENI: u16 = EXTRA0 + 30;
pub const PILAF: u16 = EXTRA0 + 31;
pub const OMELETTE: u16 = EXTRA0 + 32;
pub const BLINI_JAM: u16 = EXTRA0 + 33;
pub const SYRNIKI: u16 = EXTRA0 + 34;
pub const SHAWARMA: u16 = EXTRA0 + 35;
pub const FRIED_MUSHROOMS: u16 = EXTRA0 + 36;
pub const KOMPOT: u16 = EXTRA0 + 37;
pub const KVASS: u16 = EXTRA0 + 38;
pub const TEA: u16 = EXTRA0 + 39;
pub const COCOA_DRINK: u16 = EXTRA0 + 40;
pub const HONEY_CAKE: u16 = EXTRA0 + 41;
pub const HEAVEN_PIE: u16 = EXTRA0 + 42;
pub const STAR_COOKIE: u16 = EXTRA0 + 43;
pub const RECIPE_BOOK: u16 = EXTRA0 + 44;
pub const EXTRA_COUNT: u16 = 45;
/// Трофеи существ с номером 100 и больше (v0.11) идут после вещей v0.9.
pub const MOB_DROP2: u16 = EXTRA0 + EXTRA_COUNT;
/// Предметы призыва боссов (по одному на босса).
pub const SUMMON0: u16 = MOB_DROP2 + (crate::mobdefs::MOBS.len() as u16 - 100);
pub const SUMMON_COUNT: u16 = 6;
/// Вёдра (v0.15): пустое, с водой, с лавой.
pub const BUCKET: u16 = SUMMON0 + SUMMON_COUNT;
pub const WATER_BUCKET: u16 = BUCKET + 1;
pub const LAVA_BUCKET: u16 = BUCKET + 2;
pub const COIN: u16 = BUCKET + 3;
pub const WHEAT_SEEDS: u16 = BUCKET + 4;

/// Трофей существа.
pub fn mob_drop(def: usize) -> u16 {
    if def < 100 { MOB_DROP0 + def as u16 } else { MOB_DROP2 + (def - 100) as u16 }
}

/// Существо, чей это трофей.
pub fn drop_owner(t: u16) -> Option<usize> {
    if (MOB_DROP0..EXTRA0).contains(&t) {
        Some((t - MOB_DROP0) as usize)
    } else if (MOB_DROP2..SUMMON0).contains(&t) {
        Some((t - MOB_DROP2) as usize + 100)
    } else {
        None
    }
}

/// Какого босса призывает предмет.
pub fn summon_boss(t: u16) -> Option<usize> {
    (SUMMON0..SUMMON0 + SUMMON_COUNT).contains(&t).then(|| crate::mobdefs::BOSS0 + (t - SUMMON0) as usize)
}

pub const POTIONS: [(Eff, f32, &str, (u8, u8, u8)); 10] = [
    (Eff::Heal, 0.0, "Зелье исцеления", (230, 50, 60)),
    (Eff::Regen, 30.0, "Зелье регенерации", (240, 120, 180)),
    (Eff::Speed, 90.0, "Зелье скорости", (120, 200, 250)),
    (Eff::Jump, 90.0, "Зелье прыгучести", (120, 240, 120)),
    (Eff::Strength, 90.0, "Зелье силы", (180, 40, 40)),
    (Eff::Night, 180.0, "Зелье ночного зрения", (60, 60, 200)),
    (Eff::FireRes, 180.0, "Зелье огнестойкости", (250, 150, 40)),
    (Eff::Saturation, 0.0, "Зелье сытости", (240, 200, 90)),
    (Eff::SlowFall, 90.0, "Зелье плавного падения", (230, 230, 250)),
    (Eff::Haste, 90.0, "Зелье спешки", (250, 230, 80)),
];

fn rgba(c: (u8, u8, u8)) -> [u8; 4] {
    [c.0, c.1, c.2, 255]
}

fn build_things() -> Vec<ThingDef> {
    use Icon::*;
    let mut v: Vec<ThingDef> = Vec::new();
    let mut t = |name: &'static str, kind: TKind, icon: Icon, c1: (u8, u8, u8), c2: (u8, u8, u8)| {
        v.push(ThingDef { name, kind, icon, c1: rgba(c1), c2: rgba(c2) });
    };
    let m = TKind::Mat;
    let f = |h: u8| TKind::Food(h, Eff::None, 0.0);
    let fe = |h: u8, e: Eff, s: f32| TKind::Food(h, e, s);
    t("Палка", m, Stick, (150, 110, 66), (0, 0, 0));
    t("Уголь", m, Lump, (44, 44, 48), (90, 90, 96));
    t("Железный слиток", m, Ingot, (214, 216, 222), (0, 0, 0));
    t("Медный слиток", m, Ingot, (214, 120, 70), (0, 0, 0));
    t("Золотой слиток", m, Ingot, (248, 206, 60), (0, 0, 0));
    t("Рубин", m, Gem, (220, 40, 70), (150, 20, 40));
    t("Кристалл", m, Gem, (150, 110, 244), (90, 220, 250));
    t("Сапфир", m, Gem, (50, 100, 230), (30, 60, 150));
    t("Малахит", m, Gem, (40, 180, 120), (20, 110, 70));
    t("Райская пыль", m, Dust, (255, 220, 120), (0, 0, 0));
    t("Небесный слиток", m, Ingot, (255, 240, 190), (0, 0, 0));
    t("Нить", m, Coil, (240, 240, 236), (200, 200, 196));
    t("Кожа", m, Hide, (150, 92, 52), (120, 70, 40));
    t("Мех", m, Hide, (190, 176, 156), (150, 136, 116));
    t("Кость", m, Bone, (236, 232, 216), (0, 0, 0));
    t("Перо", m, Feather, (246, 246, 246), (200, 200, 200));
    t("Слизь", m, Blob, (110, 210, 90), (0, 0, 0));
    t("Пшеница", m, Stalks, (226, 196, 90), (190, 160, 70));
    t("Мука", m, Sack, (246, 242, 230), (150, 110, 70));
    t("Сахар", m, Dust, (250, 250, 250), (0, 0, 0));
    t("Ком глины", m, Blob, (160, 166, 178), (0, 0, 0));
    t("Кирпичик", m, Brick, (176, 78, 56), (0, 0, 0));
    t("Стеклянная бутылка", m, Bottle, (200, 220, 240), (150, 110, 70));
    t("Миска", m, Bowl, (170, 124, 72), (130, 90, 50));
    t("Бумага", m, Paper, (246, 244, 236), (180, 180, 190));
    t("Книга", m, Book, (150, 60, 50), (100, 70, 40));
    t("Соль", m, Dust, (240, 240, 244), (0, 0, 0));
    t("Кусочек янтаря", m, Gem, (240, 160, 40), (200, 110, 20));
    t("Соты", m, Comb, (240, 180, 40), (200, 130, 20));
    t("Воск", m, Blob, (240, 220, 140), (0, 0, 0));
    t("Смола", m, Drop, (200, 120, 30), (0, 0, 0));
    t("Кремень", m, Shard, (70, 70, 76), (0, 0, 0));
    t("Тростинка", m, Stick, (150, 196, 96), (0, 0, 0));
    t("Яйцо", fe(2, Eff::None, 0.0), Egg, (240, 226, 200), (200, 180, 150));
    t("Бутыль молока", fe(3, Eff::None, 0.0), Bottle, (250, 250, 245), (150, 110, 70));
    t("Какао-бобы", m, Berries, (120, 70, 40), (70, 120, 50));
    for i in 0..16 {
        let c = crate::blocks::palette16(i);
        let name: &'static str = Box::leak(format!("{} краситель", crate::blocks::COLOR_NAMES_M[i]).into_boxed_str());
        t(name, m, Dust, (c[0], c[1], c[2]), (0, 0, 0));
    }
    // еда
    t("Яблоко", f(4), Apple, (220, 40, 40), (90, 140, 50));
    t("Золотое яблоко", fe(8, Eff::Regen, 20.0), Apple, (250, 210, 60), (90, 140, 50));
    t("Хлеб", f(5), Bread, (200, 140, 70), (150, 100, 50));
    t("Морковь", f(3), Carrot, (240, 130, 30), (90, 170, 60));
    t("Печёная морковь", f(5), Carrot, (200, 100, 30), (90, 140, 50));
    t("Картофель", f(1), Potato, (200, 170, 110), (150, 120, 70));
    t("Печёный картофель", f(5), Potato, (230, 190, 100), (170, 120, 60));
    t("Ягоды", f(2), Berries, (200, 30, 80), (70, 130, 50));
    t("Банка мёда", fe(6, Eff::Regen, 5.0), Jar, (240, 170, 30), (160, 110, 60));
    t("Тыквенный пирог", f(8), Pie, (230, 130, 40), (200, 150, 90));
    t("Печенье", f(2), Cookie, (210, 150, 80), (90, 50, 30));
    t("Кусок торта", f(7), Cake, (250, 230, 220), (220, 60, 80));
    t("Сыр", f(5), Cheese, (250, 210, 80), (220, 170, 50));
    t("Грибной суп", f(6), Soup, (170, 120, 80), (150, 104, 60));
    t("Рагу", f(9), Soup, (220, 120, 50), (150, 104, 60));
    t("Сырое мясо", f(3), Meat, (220, 90, 90), (240, 236, 220));
    t("Стейк", f(8), Meat, (150, 80, 40), (240, 236, 220));
    t("Сырая птица", f(2), Drumstick, (240, 180, 160), (240, 236, 220));
    t("Жареная птица", f(6), Drumstick, (200, 130, 60), (240, 236, 220));
    t("Сырая баранина", f(2), Meat, (230, 110, 110), (240, 236, 220));
    t("Жареная баранина", f(6), Meat, (170, 90, 50), (240, 236, 220));
    t("Сырая рыба", f(2), Fish, (150, 170, 190), (40, 40, 50));
    t("Жареная рыба", f(5), Fish, (210, 160, 90), (40, 40, 50));
    t("Лосось", f(3), Fish, (240, 130, 110), (40, 40, 50));
    t("Суши", f(7), Sushi, (240, 130, 110), (40, 60, 40));
    t("Плод кактуса", f(3), Apple, (230, 90, 140), (70, 150, 64));
    t("Финики", f(3), Berries, (130, 70, 40), (90, 140, 50));
    t("Кокос", f(4), Coconut, (120, 80, 50), (60, 40, 20));
    t("Долька арбуза", f(2), Melon, (230, 60, 70), (80, 150, 50));
    t("Ягодный джем", f(6), Jar, (180, 30, 60), (200, 200, 210));
    t("Сладкий рулет", f(6), Roll, (230, 180, 120), (180, 80, 60));
    t("Шашлык", fe(9, Eff::Strength, 20.0), Skewer, (170, 90, 50), (150, 110, 70));
    t("Оладьи", f(6), Pancake, (230, 180, 100), (240, 170, 40));
    t("Каша", f(6), Soup, (236, 220, 180), (150, 104, 60));
    t("Райский нектар", fe(6, Eff::Regen, 30.0), Bottle, (255, 210, 90), (200, 160, 80));
    t("Амброзия", fe(10, Eff::Strength, 60.0), Jar, (255, 236, 160), (250, 210, 80));
    t("Звёздный фрукт", fe(5, Eff::Night, 90.0), Star, (250, 230, 80), (0, 0, 0));
    t("Пломбир", fe(4, Eff::FireRes, 30.0), IceCream, (250, 246, 236), (210, 150, 80));
    t("Клюквенный морс", fe(3, Eff::Speed, 30.0), Mug, (200, 40, 70), (170, 124, 72));
    t("Пряник", f(5), Cookie, (170, 100, 50), (250, 250, 250));
    for (e, s, name, c) in POTIONS {
        t(name, TKind::Potion(e, s), Bottle, c, (150, 110, 70));
    }
    // трофеи существ
    for mob in MOBS[..100].iter() {
        let kind = match mob.drop_kind {
            DropKind::Mat => TKind::Mat,
            DropKind::Food(h, e, s) => TKind::Food(h, e, s),
        };
        let c = mob.drop_col;
        let dark = ((c.0 as f32 * 0.6) as u8, (c.1 as f32 * 0.6) as u8, (c.2 as f32 * 0.6) as u8);
        t(mob.drop_name, kind, mob.drop_icon, c, dark);
    }
    // v0.9
    t("Стальной слиток", m, Ingot, (150, 160, 176), (0, 0, 0));
    t("Бронзовый слиток", m, Ingot, (196, 140, 70), (0, 0, 0));
    t("Жемчужина", m, Pearl, (240, 236, 246), (0, 0, 0));
    t("Шёлк", m, Coil, (250, 240, 250), (230, 200, 230));
    t("Отрез ткани", m, Paper, (200, 70, 70), (150, 40, 40));
    t("Кожаный ремень", m, Coil, (150, 92, 52), (110, 64, 34));
    t("Шестерёнка", m, Gear, (200, 204, 214), (0, 0, 0));
    t("Свиток", m, Scroll, (236, 222, 180), (90, 60, 40));
    t("Чернила", m, Bottle, (30, 30, 50), (150, 110, 70));
    t("Порох", m, Dust, (80, 80, 86), (0, 0, 0));
    t("Древесный уголь", m, Lump, (60, 50, 46), (110, 90, 80));
    t("Осколок обсидиана", m, Shard, (70, 50, 110), (0, 0, 0));
    t("Осколок коралла", m, Shard, (240, 120, 170), (0, 0, 0));
    t("Самоцветная пыль", m, Dust, (180, 140, 250), (0, 0, 0));
    t("Кленовый сироп", fe(5, Eff::Regen, 5.0), Jar, (200, 110, 40), (160, 110, 60));
    t("Пицца", f(9), Pizza, (240, 200, 90), (200, 40, 40));
    t("Пончик", f(4), Donut, (210, 150, 80), (240, 120, 170));
    t("Бургер", f(10), Burger, (210, 150, 70), (70, 150, 50));
    t("Плитка шоколада", fe(4, Eff::Speed, 20.0), Chocolate, (110, 60, 34), (80, 40, 22));
    t("Леденец", fe(2, Eff::Haste, 15.0), Candy, (230, 60, 90), (250, 250, 250));
    t("Груша", f(4), Pear, (200, 210, 80), (110, 80, 40));
    t("Виноград", f(3), Grapes, (130, 60, 150), (90, 140, 50));
    t("Вишня", f(2), Berries, (170, 20, 40), (70, 130, 50));
    t("Слива", f(3), Apple, (110, 60, 140), (90, 140, 50));
    t("Апельсин", f(4), Apple, (250, 150, 30), (80, 140, 50));
    t("Лимон", fe(2, Eff::Speed, 10.0), Egg, (250, 230, 70), (220, 200, 40));
    t("Орехи", f(3), Berries, (170, 120, 70), (120, 80, 40));
    t("Карамельное яблоко", f(6), Apple, (220, 130, 40), (150, 100, 60));
    t("Борщ", fe(9, Eff::Regen, 8.0), Soup, (190, 40, 60), (150, 104, 60));
    t("Щи", f(8), Soup, (150, 170, 80), (150, 104, 60));
    t("Пельмени", f(9), Soup, (240, 236, 220), (150, 104, 60));
    t("Плов", f(9), Soup, (230, 170, 80), (150, 104, 60));
    t("Омлет", f(6), Pancake, (250, 220, 90), (250, 250, 240));
    t("Блины с вареньем", f(7), Pancake, (230, 180, 100), (180, 30, 60));
    t("Сырники", f(6), Cookie, (240, 220, 170), (250, 250, 250));
    t("Шаурма", fe(10, Eff::Strength, 20.0), Roll, (220, 180, 120), (170, 90, 50));
    t("Жареные грибы", f(5), Skewer, (150, 110, 80), (150, 110, 70));
    t("Компот", fe(3, Eff::Regen, 5.0), Mug, (200, 90, 120), (170, 124, 72));
    t("Квас", fe(3, Eff::Speed, 15.0), Mug, (140, 80, 30), (170, 124, 72));
    t("Горячий чай", fe(2, Eff::FireRes, 20.0), Mug, (170, 90, 40), (240, 240, 240));
    t("Какао с зефиром", fe(5, Eff::Saturation, 0.0), Mug, (120, 70, 40), (250, 250, 250));
    t("Медовик", f(8), Cake, (230, 170, 80), (150, 90, 40));
    t("Небесный пирог", fe(12, Eff::Regen, 30.0), Pie, (255, 230, 160), (250, 210, 80));
    t("Звёздное печенье", fe(4, Eff::Night, 60.0), Cookie, (250, 230, 120), (130, 110, 240));
    t("Книга рецептов", m, Book, (60, 110, 70), (230, 190, 70));
    // v0.11: трофеи новых существ и боссов
    for mob in MOBS[100..].iter() {
        let kind = match mob.drop_kind {
            DropKind::Mat => TKind::Mat,
            DropKind::Food(h, e, s) => TKind::Food(h, e, s),
        };
        let c = mob.drop_col;
        let dark = ((c.0 as f32 * 0.6) as u8, (c.1 as f32 * 0.6) as u8, (c.2 as f32 * 0.6) as u8);
        t(mob.drop_name, kind, mob.drop_icon, c, dark);
    }
    // предметы призыва боссов
    t("Слизневый тотем", m, Blob, (90, 210, 80), (250, 220, 60));
    t("Руна титана", m, Gem, (255, 140, 40), (90, 90, 100));
    t("Паучье яйцо", m, Egg, (60, 40, 60), (220, 40, 80));
    t("Проклятый анкх", m, Horn, (250, 200, 60), (40, 90, 160));
    t("Кристалл вечной зимы", m, Gem, (140, 220, 255), (60, 120, 255));
    t("Чёрный нимб", m, Orb, (60, 50, 70), (255, 60, 60));
    // v0.15
    t("Ведро", m, Bucket, (190, 194, 204), (60, 60, 66));
    t("Ведро воды", m, Bucket, (190, 194, 204), (60, 120, 220));
    t("Ведро лавы", m, Bucket, (190, 194, 204), (250, 120, 30));
    t("Монета", m, Orb, (250, 210, 70), (180, 130, 30));
    t("Семена пшеницы", m, Berries, (200, 180, 100), (150, 130, 70));
    v
}

static THINGS: OnceLock<Vec<ThingDef>> = OnceLock::new();

pub fn things() -> &'static Vec<ThingDef> {
    THINGS.get_or_init(build_things)
}

pub fn thing(id: u16) -> &'static ThingDef {
    &things()[id as usize]
}

pub const TOOL_SPRITES: usize = 0;
pub const ARMOR_SPRITES: usize = TOOL_KINDS.len() * 10;
const HUD0: usize = ARMOR_SPRITES + 40;
pub const HUD_HEART_FULL: usize = HUD0;
pub const HUD_HEART_HALF: usize = HUD0 + 1;
pub const HUD_HEART_EMPTY: usize = HUD0 + 2;
pub const HUD_ARMOR_FULL: usize = HUD0 + 3;
pub const HUD_ARMOR_HALF: usize = HUD0 + 4;
pub const HUD_ARMOR_EMPTY: usize = HUD0 + 5;
pub const HUD_FOOD_FULL: usize = HUD0 + 6;
pub const HUD_FOOD_HALF: usize = HUD0 + 7;
pub const HUD_FOOD_EMPTY: usize = HUD0 + 8;
pub const THING_SPRITES: usize = HUD0 + 9;

impl Item {
    pub fn name(self) -> String {
        match self {
            Item::Empty => String::new(),
            Item::Block(b) => blocks::def(b).name.to_string(),
            Item::Tool(k, m) => {
                let a = m.adj();
                match k {
                    ToolKind::Sword => format!("{} меч", a[0]),
                    ToolKind::Pickaxe => format!("{} кирка", a[1]),
                    ToolKind::Axe => format!("{} топор", a[0]),
                    ToolKind::Shovel => format!("{} лопата", a[1]),
                    ToolKind::Dagger => format!("{} кинжал", a[0]),
                    ToolKind::Spear => {
                        let base: String = a[0].chars().take(a[0].chars().count() - 2).collect();
                        format!("{base}ое копьё")
                    }
                    ToolKind::Mace => format!("{} булава", a[1]),
                    ToolKind::Katana => format!("{} катана", a[1]),
                    ToolKind::Battleaxe => format!("{} секира", a[1]),
                    ToolKind::Warhammer => format!("{} боевой молот", a[0]),
                    ToolKind::Scythe => format!("{} коса", a[1]),
                    ToolKind::Halberd => format!("{} алебарда", a[1]),
                    ToolKind::Rapier => format!("{} рапира", a[1]),
                    ToolKind::Hoe => format!("{} мотыга", a[1]),
                }
            }
            Item::Armor(s, m) => {
                let a = m.adj();
                match s {
                    Slot::Helmet => format!("{} шлем", a[0]),
                    Slot::Chest => format!("{} нагрудник", a[0]),
                    Slot::Legs => format!("{} поножи", a[2]),
                    Slot::Boots => format!("{} ботинки", a[2]),
                }
            }
            Item::Thing(t) => thing(t).name.to_string(),
        }
    }

    pub fn max_stack(self) -> u16 {
        match self {
            Item::Tool(..) | Item::Armor(..) => 1,
            Item::Thing(RECIPE_BOOK) => 1,
            Item::Thing(t) if summon_boss(t).is_some() => 1,
            Item::Thing(WATER_BUCKET | LAVA_BUCKET) => 1,
            Item::Thing(BUCKET) => 16,
            Item::Thing(t) if matches!(thing(t).kind, TKind::Potion(..)) => 8,
            Item::Empty => 0,
            _ => 64,
        }
    }

    /// Урон по существам и игрокам.
    pub fn damage(self) -> f32 {
        match self {
            Item::Tool(k, m) => {
                let base = 4.0 + m.tier() * 1.5;
                base * match k {
                    ToolKind::Sword => 1.0,
                    ToolKind::Dagger => 0.65,
                    ToolKind::Spear => 0.9,
                    ToolKind::Mace => 1.5,
                    ToolKind::Axe => 0.9,
                    ToolKind::Pickaxe => 0.55,
                    ToolKind::Shovel => 0.45,
                    ToolKind::Katana => 1.15,
                    ToolKind::Battleaxe => 1.7,
                    ToolKind::Warhammer => 1.85,
                    ToolKind::Scythe => 1.3,
                    ToolKind::Halberd => 1.45,
                    ToolKind::Rapier => 0.85,
                    ToolKind::Hoe => 0.5,
                }
            }
            _ => 1.0,
        }
    }

    /// Время полной зарядки удара (как в новых версиях классических блочных игр).
    pub fn cooldown(self) -> f32 {
        match self {
            Item::Tool(ToolKind::Dagger, _) => 0.4,
            Item::Tool(ToolKind::Sword, _) => 0.625,
            Item::Tool(ToolKind::Spear, _) => 0.8,
            Item::Tool(ToolKind::Pickaxe, _) => 0.83,
            Item::Tool(ToolKind::Axe, _) => 1.0,
            Item::Tool(ToolKind::Shovel, _) => 1.0,
            Item::Tool(ToolKind::Mace, _) => 1.25,
            Item::Tool(ToolKind::Katana, _) => 0.55,
            Item::Tool(ToolKind::Rapier, _) => 0.45,
            Item::Tool(ToolKind::Battleaxe, _) => 1.3,
            Item::Tool(ToolKind::Warhammer, _) => 1.4,
            Item::Tool(ToolKind::Scythe, _) => 0.9,
            Item::Tool(ToolKind::Halberd, _) => 1.1,
            _ => 0.25,
        }
    }

    /// Дальность удара.
    pub fn reach(self) -> f32 {
        match self {
            Item::Tool(ToolKind::Spear, _) | Item::Tool(ToolKind::Halberd, _) => 5.0,
            Item::Tool(ToolKind::Scythe, _) => 4.2,
            Item::Tool(ToolKind::Rapier, _) => 4.0,
            Item::Tool(ToolKind::Katana, _) => 3.8,
            Item::Tool(ToolKind::Dagger, _) => 3.0,
            _ => 3.6,
        }
    }

    pub fn armor_points(self) -> i32 {
        let Item::Armor(slot, m) = self else { return 0 };
        let base: [i32; 4] = match m {
            Mat::Leather => [1, 3, 2, 1],
            Mat::Fur => [1, 3, 3, 1],
            Mat::Copper => [2, 4, 3, 1],
            Mat::Gold => [2, 5, 3, 1],
            Mat::Iron => [2, 6, 5, 2],
            Mat::Malachite => [2, 6, 5, 2],
            Mat::Sapphire => [3, 7, 5, 2],
            Mat::Ruby => [3, 7, 6, 3],
            Mat::Crystal => [3, 8, 6, 3],
            _ => [4, 8, 7, 3],
        };
        base[slot as usize]
    }

    pub fn mining_speed(self, need: blocks::ToolNeed) -> f32 {
        use blocks::ToolNeed as N;
        match (self, need) {
            (Item::Tool(ToolKind::Pickaxe, m), N::Pickaxe) => m.speed(),
            (Item::Tool(ToolKind::Axe, m), N::Axe) => m.speed(),
            (Item::Tool(ToolKind::Shovel, m), N::Shovel) => m.speed(),
            (Item::Tool(ToolKind::Battleaxe, m), N::Axe) => m.speed() * 0.8,
            (Item::Tool(ToolKind::Warhammer, m), N::Pickaxe) => m.speed() * 0.6,
            (Item::Tool(ToolKind::Sword | ToolKind::Katana, _), _) => 1.3,
            _ => 1.0,
        }
    }

    pub fn food(self) -> Option<(u8, Eff, f32)> {
        match self {
            Item::Thing(t) => match thing(t).kind {
                TKind::Food(h, e, s) => Some((h, e, s)),
                TKind::Potion(e, s) => Some((if e == Eff::Saturation { 12 } else { 0 }, e, s)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn description(self) -> Option<String> {
        match self {
            Item::Tool(k, m) => Some(match k {
                k if k.is_weapon() => {
                    let sweep = if k.sweeps() { " · задевает соседей" } else { "" };
                    format!("Урон: {:.0} · полная зарядка {:.2} с · дальность {:.1}{sweep}", self.damage(), self.cooldown(), self.reach())
                }
                _ => format!("Урон: {:.0} · скорость добычи ×{:.1}", self.damage(), m.speed()),
            }),
            Item::Armor(..) => Some(format!("Защита: +{}", self.armor_points())),
            Item::Thing(t) => match thing(t).kind {
                TKind::Food(h, e, s) => Some(if e == Eff::None {
                    format!("Сытость: +{h}")
                } else if s > 0.0 {
                    format!("Сытость: +{h} · {} {:.0} с", e.name(), s)
                } else {
                    format!("Сытость: +{h} · {}", e.name())
                }),
                TKind::Potion(e, s) => Some(if s > 0.0 { format!("{} на {:.0} с", e.name(), s) } else { e.name().to_string() }),
                TKind::Mat => {
                    if let Some(b) = summon_boss(t) {
                        Some(format!("ПКМ — призвать босса: {}", MOBS[b].name))
                    } else if t == RECIPE_BOOK {
                        Some("ПКМ или B — открыть. Все рецепты игры".into())
                    } else if let Some(o) = drop_owner(t) {
                        let boss = if MOBS[o].flags & crate::mobdefs::BOSS != 0 { "Трофей босса" } else { "Трофей" };
                        Some(format!("{boss}: {}", MOBS[o].name))
                    } else {
                        Some("Материал для крафта".into())
                    }
                }
            },
            _ => None,
        }
    }

    pub fn sprite_index(self) -> Option<usize> {
        match self {
            Item::Tool(k, m) => Some(TOOL_SPRITES + k as usize * 10 + TOOL_MATS.iter().position(|x| *x == m)?),
            Item::Armor(s, m) => Some(ARMOR_SPRITES + s as usize * 10 + ARMOR_MATS.iter().position(|x| *x == m)?),
            Item::Thing(t) => Some(THING_SPRITES + t as usize),
            _ => None,
        }
    }

    /// Компактный код для сохранений и сети.
    pub fn code(self) -> u32 {
        match self {
            Item::Empty => 0,
            Item::Block(b) => 0x0100_0000 | b as u32,
            Item::Tool(k, m) => 0x0200_0000 | (k as u32) << 8 | ALL_MATS.iter().position(|x| *x == m).unwrap() as u32,
            Item::Armor(s, m) => 0x0300_0000 | (s as u32) << 8 | ALL_MATS.iter().position(|x| *x == m).unwrap() as u32,
            Item::Thing(t) => 0x0400_0000 | t as u32,
        }
    }

    pub fn from_code(c: u32) -> Item {
        let lo = (c & 0xff) as usize;
        let mid = ((c >> 8) & 0xff) as usize;
        match c >> 24 {
            1 if ((c & 0xffff) as usize) < blocks::BLOCK_COUNT => Item::Block((c & 0xffff) as u16),
            2 if mid < TOOL_KINDS.len() && lo < 12 => Item::Tool(TOOL_KINDS[mid], ALL_MATS[lo]),
            3 if mid < 4 && lo < 12 => Item::Armor(SLOTS[mid], ALL_MATS[lo]),
            4 if ((c & 0xffff) as usize) < things().len() => Item::Thing((c & 0xffff) as u16),
            _ => Item::Empty,
        }
    }
}

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

const SHOVEL: [&str; 16] = [
    "............DDD.",
    "...........DHMMD",
    "..........DHMMMD",
    "..........DMMMD.",
    "..........DMMD..",
    ".........DwDD...",
    "........DwD.....",
    ".......DwD......",
    "......DwD.......",
    ".....DwD........",
    "....DwD.........",
    "...DwD..........",
    "..DwD...........",
    ".DwD............",
    "DDD.............",
    "................",
];
const DAGGER: [&str; 16] = [
    "................",
    "................",
    "................",
    "...........DD...",
    "..........DHD...",
    ".........DHMD...",
    "........DHMD....",
    ".......DHMD.....",
    "...DD.DHMD......",
    "....DDHMD.......",
    "....DwMD........",
    "...DwDDD........",
    "..DwD...........",
    "..DD............",
    "................",
    "................",
];
const SPEAR: [&str; 16] = [
    ".............DD.",
    "............DHMD",
    "...........DHMMD",
    "...........DMMD.",
    "..........DwDD..",
    ".........DwD....",
    "........DwD.....",
    ".......DwD......",
    "......DwD.......",
    ".....DwD........",
    "....DwD.........",
    "...DwD..........",
    "..DwD...........",
    ".DwD............",
    "DwD.............",
    "DD..............",
];
const MACE: [&str; 16] = [
    "..........D.D...",
    ".........DMDMD..",
    "........DHHMMMD.",
    ".......DDHMMMMDD",
    "........DMMMMMD.",
    "........DMMMMD..",
    ".......DwDMDD...",
    "......DwD..D....",
    ".....DwD........",
    "....DwD.........",
    "...DwD..........",
    "..DwD...........",
    ".DwD............",
    "DwD.............",
    "DD..............",
    "................",
];
const DRUMSTICK_HUD: [&str; 16] = [
    "................",
    "......DDDD......",
    ".....DMMMMD.....",
    "....DHMMMMMD....",
    "....DHMMMMMD....",
    "....DMMMMMMD....",
    ".....DMMMMD.....",
    "......DMMD......",
    ".......DwD......",
    "........DwD.....",
    ".......DwwwD....",
    ".......DwDwD....",
    "........D.D.....",
    "................",
    "................",
    "................",
];

/// Класс пикселя оружия: 0 пусто, 1 металл, 2 блик, 3 рукоять, 4 обмотка, 5 гарда/украшение.
/// Координаты повёрнуты: `a` — вдоль рукояти (снизу-слева вверх-вправо), `b` — поперёк.
fn weapon_px(k: ToolKind, x: i32, y: i32) -> u8 {
    let (a, b) = ((x - y) as f32, (x + y - 15) as f32);
    let handle = |from: f32, to: f32| a >= from && a <= to && b.abs() <= 1.0;
    match k {
        ToolKind::Katana => {
            let c = 0.012 * (a + 5.0).powi(2);
            let d = b - c;
            let tip = if a > 11.0 { 14.0 - a } else { 2.0 };
            if a >= -5.0 && a <= 14.0 && d >= -1.0 && d <= tip.min(2.0) {
                if d > 0.5 { 2 } else { 1 }
            } else if a >= -7.0 && a < -5.0 && b.abs() <= 3.0 {
                5
            } else if handle(-15.0, -7.0) {
                if (a as i32).rem_euclid(3) == 0 { 4 } else { 3 }
            } else {
                0
            }
        }
        ToolKind::Rapier => {
            if a >= -4.0 && a <= 15.0 && b.abs() <= 1.0 {
                if b < 0.0 { 2 } else { 1 }
            } else if (a >= -6.0 && a < -4.0 && b.abs() <= 3.0) || (b.abs() >= 3.0 && b.abs() <= 4.0 && a >= -11.0 && a <= -4.0 && b > 0.0) || (a >= -15.0 && a <= -13.0 && b.abs() <= 2.0) {
                5
            } else if handle(-13.0, -6.0) {
                if (a as i32).rem_euclid(2) == 0 { 4 } else { 3 }
            } else {
                0
            }
        }
        ToolKind::Scythe => {
            let t = -b;
            let ac = 11.0 - 0.05 * t * t;
            let w = 1.6 * (1.0 - t / 15.0) + 0.6;
            if t >= 0.0 && t <= 13.0 && (a - ac).abs() <= w {
                if a < ac - 0.3 { 2 } else { 1 }
            } else if handle(-15.0, 13.0) {
                if a > 9.0 && a < 13.0 { 5 } else { 3 }
            } else {
                0
            }
        }
        ToolKind::Battleaxe => {
            let wa = 1.5 + b.abs() * 0.45;
            if b.abs() >= 1.0 && b.abs() <= 7.0 && (a - 6.5).abs() <= wa {
                if b.abs() >= 6.0 { 2 } else if (a - 6.5).abs() > wa - 1.0 { 2 } else { 1 }
            } else if a > 11.0 && a <= 14.0 && b.abs() <= 1.0 {
                1
            } else if handle(-15.0, 11.0) {
                if (a - 4.0).abs() < 1.0 || (a - 9.0).abs() < 1.0 { 5 } else { 3 }
            } else {
                0
            }
        }
        ToolKind::Halberd => {
            if a >= 9.0 && a <= 15.0 && b.abs() <= (15.0 - a) * 0.4 + 1.0 {
                if b < 0.0 { 2 } else { 1 }
            } else if a >= 2.0 && a <= 9.0 && b >= 1.0 && b <= 7.0 && (a - 5.5).abs() <= 1.4 + b * 0.32 {
                if b >= 6.0 { 2 } else { 1 }
            } else if a >= 4.0 && a <= 7.0 && b <= -1.0 && b >= -4.0 && (a - 5.5).abs() <= 1.0 + (b + 4.0) * 0.2 {
                1
            } else if handle(-15.0, 9.0) {
                if (a - 1.0).abs() < 1.0 { 5 } else { 3 }
            } else {
                0
            }
        }
        ToolKind::Hoe => {
            if a >= 9.0 && a <= 12.0 && b <= 0.0 && b >= -5.0 {
                if b > -1.0 { 1 } else { 2 }
            } else if handle(-15.0, 12.0) {
                3
            } else {
                0
            }
        }
        _ => {
            // боевой молот
            if a >= 5.0 && a <= 11.0 && b.abs() <= 6.0 {
                if (a - 8.0).abs() < 0.6 { 5 } else if b.abs() >= 5.0 || a >= 10.5 { 2 } else { 1 }
            } else if handle(-15.0, 5.0) {
                if (a as i32).rem_euclid(4) == 0 { 4 } else { 3 }
            } else {
                0
            }
        }
    }
}

fn weapon_sprite(k: ToolKind, m: Mat) -> impl Fn(i32, i32) -> [u8; 4] {
    let (main, dark, light) = m.colors();
    let wood = [120, 82, 44, 255];
    let wrap = [58, 40, 34, 255];
    let gold = if m == Mat::Gold { [200, 120, 40, 255] } else { [232, 190, 64, 255] };
    let sh = |c: [u8; 4], f: f32| [(c[0] as f32 * f).min(255.0) as u8, (c[1] as f32 * f).min(255.0) as u8, (c[2] as f32 * f).min(255.0) as u8, 255];
    move |x, y| {
        let p = weapon_px(k, x, y);
        let n = crate::blocks::hash(x, y, 4242) * 0.1 + 0.95;
        match p {
            1 => sh(main, n),
            2 => light,
            3 => sh(wood, n),
            4 => wrap,
            5 => sh(gold, n),
            _ => {
                let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    (0..16).contains(&nx) && (0..16).contains(&ny) && weapon_px(k, nx, ny) != 0
                });
                if near { dark } else { [0, 0, 0, 0] }
            }
        }
    }
}

/// Рисует все спрайты предметов и интерфейса; возвращает их плитки по порядку.
pub fn item_sprites(add: &mut dyn FnMut(Box<dyn Fn(i32, i32) -> [u8; 4]>) -> u16) -> Vec<u16> {
    let wood = [120, 82, 44, 255];
    let mut out = Vec::new();
    for mask in [SWORD, PICKAXE, AXE, SHOVEL, DAGGER, SPEAR, MACE] {
        for m in TOOL_MATS {
            let (main, dark, light) = m.colors();
            out.push(add(Box::new(mask_tile(mask, vec![('M', main), ('D', dark), ('H', light), ('w', wood)]))));
        }
    }
    for k in &TOOL_KINDS[7..] {
        for m in TOOL_MATS {
            out.push(add(Box::new(weapon_sprite(*k, m))));
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
    let meat = [200, 120, 60, 255];
    let md = [70, 36, 20, 255];
    let bone = [240, 236, 220, 255];
    let me = [70, 50, 40, 255];
    out.push(add(Box::new(mask_tile(DRUMSTICK_HUD, vec![('M', meat), ('D', md), ('H', [240, 170, 110, 255]), ('w', bone)]))));
    let fh = mask_tile(DRUMSTICK_HUD, vec![('M', meat), ('D', md), ('H', [240, 170, 110, 255]), ('w', bone)]);
    let fe = mask_tile(DRUMSTICK_HUD, vec![('M', me), ('D', md), ('H', me), ('w', me)]);
    out.push(add(Box::new(move |x, y| if x < 8 { fh(x, y) } else { fe(x, y) })));
    out.push(add(Box::new(mask_tile(DRUMSTICK_HUD, vec![('M', me), ('D', md), ('H', me), ('w', me)]))));
    assert_eq!(out.len(), THING_SPRITES);
    for t in things() {
        out.push(add(Box::new(icons::draw(t.icon, t.c1, t.c2))));
    }
    out
}

pub const TABS: [&str; 12] = ["Стройка", "Природа", "Цветные", "Руды", "Декор", "Рай", "Еда", "Материалы", "Трофеи", "Оружие", "Броня", "Зелья"];

/// Каталог предметов для творческого инвентаря, по вкладкам.
pub fn catalog(tab: usize) -> Vec<Item> {
    use blocks::Cat;
    let by_cat = |c: Cat| -> Vec<Item> { (1..blocks::BLOCK_COUNT as u16).filter(|&b| blocks::def(b).cat == c).map(Item::Block).collect() };
    let n = things().len() as u16;
    match tab {
        0 => by_cat(Cat::Building),
        1 => by_cat(Cat::Nature),
        2 => by_cat(Cat::Colored),
        3 => by_cat(Cat::Ores),
        4 => by_cat(Cat::Decor),
        5 => by_cat(Cat::Heaven),
        6 => (0..MOB_DROP0).chain(EXTRA0..MOB_DROP2).filter(|&t| matches!(thing(t).kind, TKind::Food(..))).map(Item::Thing).collect(),
        7 => (0..MOB_DROP0).chain(EXTRA0..MOB_DROP2).chain(SUMMON0..n).filter(|&t| thing(t).kind == TKind::Mat).map(Item::Thing).collect(),
        8 => (MOB_DROP0..EXTRA0).chain(MOB_DROP2..SUMMON0).map(Item::Thing).collect(),
        9 => TOOL_KINDS.iter().flat_map(|&k| TOOL_MATS.iter().map(move |&m| Item::Tool(k, m))).collect(),
        10 => ARMOR_MATS.iter().flat_map(|&m| SLOTS.iter().map(move |&s| Item::Armor(s, m))).collect(),
        _ => (0..10).map(|i| Item::Thing(POTION0 + i)).collect(),
    }
}

/// Сколько всего разных предметов (для README/тестов).
pub fn total_kinds() -> usize {
    (0..TABS.len()).map(|t| catalog(t).len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_roundtrip_and_counts() {
        for tab in 0..TABS.len() {
            for it in catalog(tab) {
                assert_eq!(Item::from_code(it.code()), it, "{:?}", it);
                assert!(!it.name().is_empty());
            }
        }
        assert_eq!(thing(MOB_DROP0 - 1).name, "Зелье спешки");
        assert!(total_kinds() >= 500, "всего {}", total_kinds());
        assert_eq!(Item::Tool(ToolKind::Spear, Mat::Iron).name(), "Железное копьё");
    }
}

#[cfg(test)]
mod counts {
    #[test]
    #[ignore]
    fn print_counts() {
        let blocks = (1..crate::blocks::BLOCK_COUNT as u16).filter(|&b| crate::blocks::def(b).cat != crate::blocks::Cat::Hidden).count();
        println!("blocks={} things={} total_kinds={} recipes={}", blocks, super::things().len(), super::total_kinds(), crate::crafting::recipes().len());
    }
}
