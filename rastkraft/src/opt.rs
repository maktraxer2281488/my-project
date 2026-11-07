//! Уровень оптимизации из настроек: что упрощать ради FPS.

use std::sync::atomic::{AtomicU8, Ordering};

pub const NAMES: [&str; 4] = ["Выкл", "Лёгкая", "Сильная", "Максимальная"];
pub const HINTS: [&str; 4] = [
    "Всё в полном качестве",
    "Дальние чанки упрощены, меньше частиц",
    "+ быстрая листва, мобы и трава ближе",
    "+ всё упрощено с 4 чанков, без теней",
];

#[derive(Clone, Copy, Debug)]
pub struct Opt {
    /// С какого расстояния (в блоках) чанки рисуются упрощённой копией; f32::MAX — никогда.
    pub lod_dist: f32,
    /// Листва сплошными блоками.
    pub fast_leaves: bool,
    /// Трава и папоротники рисуются ближе этого.
    pub detail_dist: f32,
    /// Тёмные грани пещер рисуются ближе этого (если игрок не под землёй).
    pub dark_dist: f32,
    /// Мобы рисуются ближе этого.
    pub mob_dist: f32,
    /// Мобы дальше этого думают реже.
    pub mob_far: f32,
    /// Множитель числа частиц.
    pub particles: f32,
    /// Тени под мобами и предметами.
    pub shadows: bool,
}

static LEVEL: AtomicU8 = AtomicU8::new(1);
/// Авто-качество включило быструю листву.
static AUTO_FAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_level(l: u8) {
    LEVEL.store(l.min(3), Ordering::Relaxed);
    let fast = get().fast_leaves || AUTO_FAST.load(Ordering::Relaxed);
    crate::mesher::FAST_LEAVES.store(fast, Ordering::Relaxed);
}

/// Включить/выключить быструю листву от авто-качества.
/// Возвращает true, если вид листвы поменялся (нужно перестроить чанки).
pub fn set_auto_fast(on: bool) -> bool {
    let before = crate::mesher::FAST_LEAVES.load(Ordering::Relaxed);
    AUTO_FAST.store(on, Ordering::Relaxed);
    set_level(level());
    before != crate::mesher::FAST_LEAVES.load(Ordering::Relaxed)
}

pub fn auto_fast() -> bool {
    AUTO_FAST.load(Ordering::Relaxed)
}

pub fn level() -> u8 {
    LEVEL.load(Ordering::Relaxed)
}

pub fn get() -> Opt {
    match level() {
        0 => Opt { lod_dist: f32::MAX, fast_leaves: false, detail_dist: 56.0, dark_dist: 40.0, mob_dist: 72.0, mob_far: 44.0, particles: 1.0, shadows: true },
        1 => Opt { lod_dist: 128.0, fast_leaves: false, detail_dist: 48.0, dark_dist: 36.0, mob_dist: 64.0, mob_far: 40.0, particles: 0.7, shadows: true },
        2 => Opt { lod_dist: 96.0, fast_leaves: true, detail_dist: 36.0, dark_dist: 28.0, mob_dist: 52.0, mob_far: 32.0, particles: 0.5, shadows: true },
        _ => Opt { lod_dist: 64.0, fast_leaves: true, detail_dist: 24.0, dark_dist: 20.0, mob_dist: 40.0, mob_far: 26.0, particles: 0.3, shadows: false },
    }
}
