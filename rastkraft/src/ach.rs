//! Достижения: проверяются по состоянию игры и по событиям, всплывают с фанфарами.

pub struct Ach {
    pub name: &'static str,
    pub hint: &'static str,
}

const fn a(name: &'static str, hint: &'static str) -> Ach {
    Ach { name, hint }
}

pub const LIST: [Ach; 24] = [
    a("Первый инструмент", "Сделайте любой инструмент или оружие"),
    a("Железный век", "Получите железный слиток"),
    a("Самоцветы", "Добудьте рубин, сапфир или малахит"),
    a("Кристальная мечта", "Добудьте кристалл"),
    a("Повар", "Съешьте приготовленное блюдо"),
    a("Фермер", "Посадите семена на пашню"),
    a("Друг", "Приручите питомца"),
    a("Животновод", "Разведите животных"),
    a("Торговец", "Совершите сделку с жителем"),
    a("Спокойной ночи", "Проспите ночь в кровати"),
    a("Стрелок", "Попадите стрелой в существо"),
    a("Чародей", "Зачаруйте предмет"),
    a("Опытный", "Достигните 10 уровня"),
    a("Мастер", "Достигните 30 уровня"),
    a("На небесах", "Попадите в Рай"),
    a("В глубины", "Спуститесь в Бездну"),
    a("Охотник на чудищ", "Победите босса"),
    a("Легенда", "Победите всех шесть боссов"),
    a("Инженер", "Включите лампу сигналом"),
    a("Мореход", "Прокатитесь на лодке"),
    a("Путеец", "Прокатитесь на вагонетке"),
    a("Картограф", "Поставьте метку на карте"),
    a("Строитель", "Поставьте дверь"),
    a("Исследователь подземелий", "Откройте сундук в подземелье"),
];

pub const COUNT: usize = LIST.len();

pub const FIRST_TOOL: u8 = 0;
pub const IRON: u8 = 1;
pub const GEMS: u8 = 2;
pub const CRYSTAL: u8 = 3;
pub const COOK: u8 = 4;
pub const FARMER: u8 = 5;
pub const PET: u8 = 6;
pub const BREED: u8 = 7;
pub const TRADE: u8 = 8;
pub const SLEEP: u8 = 9;
pub const ARCHER: u8 = 10;
pub const ENCHANT: u8 = 11;
pub const LVL10: u8 = 12;
pub const LVL30: u8 = 13;
pub const HEAVEN: u8 = 14;
pub const ABYSS: u8 = 15;
pub const BOSS: u8 = 16;
pub const ALL_BOSSES: u8 = 17;
pub const ENGINEER: u8 = 18;
pub const BOAT: u8 = 19;
pub const CART: u8 = 20;
pub const MARKER: u8 = 21;
pub const DOOR: u8 = 22;
pub const DUNGEON: u8 = 23;
