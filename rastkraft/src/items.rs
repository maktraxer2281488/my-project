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
}

pub const TOOL_KINDS: [ToolKind; 7] = [ToolKind::Sword, ToolKind::Pickaxe, ToolKind::Axe, ToolKind::Shovel, ToolKind::Dagger, ToolKind::Spear, ToolKind::Mace];

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
    Block(u8),
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
    for mob in MOBS.iter() {
        let kind = match mob.drop_kind {
            DropKind::Mat => TKind::Mat,
            DropKind::Food(h, e, s) => TKind::Food(h, e, s),
        };
        let c = mob.drop_col;
        let dark = ((c.0 as f32 * 0.6) as u8, (c.1 as f32 * 0.6) as u8, (c.2 as f32 * 0.6) as u8);
        t(mob.drop_name, kind, mob.drop_icon, c, dark);
    }
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
pub const ARMOR_SPRITES: usize = 70;
pub const HUD_HEART_FULL: usize = 110;
pub const HUD_HEART_HALF: usize = 111;
pub const HUD_HEART_EMPTY: usize = 112;
pub const HUD_ARMOR_FULL: usize = 113;
pub const HUD_ARMOR_HALF: usize = 114;
pub const HUD_ARMOR_EMPTY: usize = 115;
pub const HUD_FOOD_FULL: usize = 116;
pub const HUD_FOOD_HALF: usize = 117;
pub const HUD_FOOD_EMPTY: usize = 118;
pub const THING_SPRITES: usize = 119;

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
                }
            }
            _ => 1.0,
        }
    }

    /// Пауза между ударами.
    pub fn cooldown(self) -> f32 {
        match self {
            Item::Tool(ToolKind::Dagger, _) => 0.22,
            Item::Tool(ToolKind::Mace, _) => 0.8,
            Item::Tool(ToolKind::Spear, _) => 0.55,
            Item::Tool(ToolKind::Axe, _) => 0.6,
            _ => 0.4,
        }
    }

    /// Дальность удара.
    pub fn reach(self) -> f32 {
        match self {
            Item::Tool(ToolKind::Spear, _) => 5.0,
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
            (Item::Tool(ToolKind::Sword, _), _) => 1.3,
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
                ToolKind::Sword | ToolKind::Dagger | ToolKind::Spear | ToolKind::Mace => {
                    format!("Урон: {:.0} · удар раз в {:.2} с · дальность {:.1}", self.damage(), self.cooldown(), self.reach())
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
                    if t >= MOB_DROP0 {
                        Some(format!("Трофей: {}", MOBS[(t - MOB_DROP0) as usize].name))
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
            1 if lo < blocks::BLOCK_COUNT => Item::Block(lo as u8),
            2 if mid < 7 && lo < 12 => Item::Tool(TOOL_KINDS[mid], ALL_MATS[lo]),
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
    let by_cat = |c: Cat| -> Vec<Item> { (1..blocks::BLOCK_COUNT as u8).filter(|&b| blocks::def(b).cat == c).map(Item::Block).collect() };
    let n = things().len() as u16;
    match tab {
        0 => by_cat(Cat::Building),
        1 => by_cat(Cat::Nature),
        2 => by_cat(Cat::Colored),
        3 => by_cat(Cat::Ores),
        4 => by_cat(Cat::Decor),
        5 => by_cat(Cat::Heaven),
        6 => (0..MOB_DROP0).filter(|&t| matches!(thing(t).kind, TKind::Food(..))).map(Item::Thing).collect(),
        7 => (0..MOB_DROP0).filter(|&t| thing(t).kind == TKind::Mat).map(Item::Thing).collect(),
        8 => (MOB_DROP0..n).map(Item::Thing).collect(),
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
        let blocks = (1..crate::blocks::BLOCK_COUNT as u8).filter(|&b| crate::blocks::def(b).cat != crate::blocks::Cat::Hidden).count();
        println!("blocks={} things={} total_kinds={} recipes={}", blocks, super::things().len(), super::total_kinds(), crate::crafting::recipes().len());
    }
}
