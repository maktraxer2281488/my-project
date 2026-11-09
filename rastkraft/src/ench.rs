//! Зачарования: у предмета одно зачарование уровня 1–3, хранится в стопке (`Stack::ench`).

use crate::items::{Item, ToolKind};

pub const SHARP: u8 = 1;
pub const FIRE: u8 = 2;
pub const KNOCK: u8 = 3;
pub const VAMP: u8 = 4;
pub const EFFIC: u8 = 5;
pub const PROT: u8 = 6;
pub const POWER: u8 = 7;
pub const THRIFT: u8 = 8;

pub fn pack(kind: u8, level: u8) -> u8 {
    kind * 4 + level.clamp(1, 3)
}

pub fn kind(e: u8) -> u8 {
    e / 4
}

pub fn level(e: u8) -> u8 {
    e % 4
}

pub fn kind_name(k: u8) -> &'static str {
    match k {
        SHARP => "Острота",
        FIRE => "Огненный клинок",
        KNOCK => "Отбрасывание",
        VAMP => "Вампиризм",
        EFFIC => "Эффективность",
        PROT => "Защита",
        POWER => "Сила выстрела",
        THRIFT => "Бережливость",
        _ => "",
    }
}

pub fn name(e: u8) -> Option<String> {
    if e == 0 {
        return None;
    }
    Some(format!("{} {}", kind_name(kind(e)), ["", "I", "II", "III"][level(e) as usize]))
}

/// Какие зачарования подходят предмету.
pub fn options(item: Item) -> Vec<u8> {
    match item {
        Item::Tool(ToolKind::Bow | ToolKind::Crossbow, _) => vec![POWER, THRIFT, FIRE],
        Item::Tool(ToolKind::Shield, _) => vec![PROT],
        Item::Tool(k, _) if k.is_weapon() => vec![SHARP, FIRE, KNOCK, VAMP],
        Item::Tool(ToolKind::Axe, _) => vec![EFFIC, SHARP],
        Item::Tool(..) => vec![EFFIC],
        Item::Armor(..) => vec![PROT],
        _ => vec![],
    }
}

/// Множитель урона в ближнем бою.
pub fn melee_mult(e: u8) -> f32 {
    match kind(e) {
        SHARP => 1.0 + 0.2 * level(e) as f32,
        FIRE => 1.0 + 0.12 * level(e) as f32,
        _ => 1.0,
    }
}

/// Множитель урона стрел.
pub fn shot_mult(e: u8) -> f32 {
    if kind(e) == POWER { 1.0 + 0.25 * level(e) as f32 } else { 1.0 }
}

/// Множитель скорости добычи.
pub fn mine_mult(e: u8) -> f32 {
    if kind(e) == EFFIC { 1.0 + 0.35 * level(e) as f32 } else { 1.0 }
}

/// Снижение урона от брони с «Защитой» (доля).
pub fn protection(armor: &[u8; 4]) -> f32 {
    let lv: u32 = armor.iter().filter(|e| kind(**e) == PROT).map(|e| level(*e) as u32).sum();
    (lv as f32 * 0.04).min(0.48)
}

/// Цена в уровнях опыта.
pub fn cost(level: u8) -> u32 {
    [0, 3, 7, 12][level as usize]
}
