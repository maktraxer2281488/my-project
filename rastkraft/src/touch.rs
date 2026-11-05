//! Сенсорное управление для телефонов и планшетов: джойстик движения слева,
//! обзор — проведением пальцем справа, экранные кнопки. Расположение, размер и
//! прозрачность кнопок настраиваются и сохраняются в настройках.

use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Act {
    Jump,
    Attack,
    Use,
    Sneak,
    Sprint,
    Inventory,
    Pause,
    Map,
    Book,
    Chat,
    Fly,
    Camera,
    Drop,
    Players,
}

pub const ACTS: [Act; 14] = [
    Act::Jump, Act::Attack, Act::Use, Act::Sneak, Act::Sprint, Act::Inventory, Act::Pause,
    Act::Map, Act::Book, Act::Chat, Act::Fly, Act::Camera, Act::Drop, Act::Players,
];

impl Act {
    pub fn label(self) -> &'static str {
        match self {
            Act::Jump => "▲",
            Act::Attack => "Удар",
            Act::Use => "Исп.",
            Act::Sneak => "Присед",
            Act::Sprint => "Бег",
            Act::Inventory => "Сумка",
            Act::Pause => "II",
            Act::Map => "Карта",
            Act::Book => "Книга",
            Act::Chat => "Чат",
            Act::Fly => "Полёт",
            Act::Camera => "Вид",
            Act::Drop => "Бросить",
            Act::Players => "Игроки",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Act::Jump => "jump",
            Act::Attack => "attack",
            Act::Use => "use",
            Act::Sneak => "sneak",
            Act::Sprint => "sprint",
            Act::Inventory => "inv",
            Act::Pause => "pause",
            Act::Map => "map",
            Act::Book => "book",
            Act::Chat => "chat",
            Act::Fly => "fly",
            Act::Camera => "camera",
            Act::Drop => "drop",
            Act::Players => "players",
        }
    }
    /// Положение по умолчанию: (x, y) в долях экрана, радиус в долях высоты.
    pub fn default_pos(self) -> (f32, f32, f32) {
        match self {
            Act::Jump => (0.90, 0.78, 0.085),
            Act::Attack => (0.90, 0.52, 0.075),
            Act::Use => (0.78, 0.64, 0.065),
            Act::Sneak => (0.27, 0.86, 0.055),
            Act::Sprint => (0.27, 0.66, 0.05),
            Act::Inventory => (0.72, 0.93, 0.045),
            Act::Pause => (0.035, 0.07, 0.04),
            Act::Map => (0.11, 0.07, 0.04),
            Act::Book => (0.18, 0.07, 0.04),
            Act::Chat => (0.25, 0.07, 0.04),
            Act::Fly => (0.78, 0.40, 0.045),
            Act::Camera => (0.32, 0.07, 0.04),
            Act::Drop => (0.66, 0.80, 0.04),
            Act::Players => (0.39, 0.07, 0.04),
        }
    }
}

/// Что нажато в этом кадре (для игры).
#[derive(Clone, Copy, Default, Debug)]
pub struct Out {
    /// Сенсорный режим включён.
    pub active: bool,
    /// Движение: x — вправо, y — вперёд (−1..1).
    pub mv: Vec2,
    /// Поворот камеры в пикселях за кадр.
    pub look: Vec2,
    pub down: [bool; 14],
    pub pressed: [bool; 14],
    /// Нажатая ячейка панели быстрого доступа.
    pub hotbar: Option<usize>,
}

impl Out {
    pub fn down(&self, a: Act) -> bool {
        self.down[a as usize]
    }
    pub fn pressed(&self, a: Act) -> bool {
        self.pressed[a as usize]
    }
}

/// Раскладка: положение и размер каждой кнопки, общая прозрачность и чувствительность обзора.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub pos: HashMap<Act, (f32, f32, f32)>,
    pub opacity: f32,
    pub scale: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { pos: ACTS.iter().map(|a| (*a, a.default_pos())).collect(), opacity: 0.55, scale: 1.0 }
    }
}

impl Layout {
    pub fn get(&self, a: Act) -> (f32, f32, f32) {
        *self.pos.get(&a).unwrap_or(&a.default_pos())
    }
    /// Строка для файла настроек.
    pub fn to_text(&self) -> String {
        let mut v: Vec<String> = ACTS.iter().map(|a| {
            let (x, y, r) = self.get(*a);
            format!("{}:{:.3}:{:.3}:{:.3}", a.key(), x, y, r)
        }).collect();
        v.push(format!("opacity:{:.2}:0:0", self.opacity));
        v.push(format!("scale:{:.2}:0:0", self.scale));
        v.join(";")
    }
    pub fn from_text(t: &str) -> Layout {
        let mut l = Layout::default();
        for part in t.split(';') {
            let f: Vec<&str> = part.split(':').collect();
            if f.len() != 4 {
                continue;
            }
            let num = |i: usize| f[i].parse::<f32>().ok();
            match f[0] {
                "opacity" => l.opacity = num(1).unwrap_or(l.opacity).clamp(0.1, 1.0),
                "scale" => l.scale = num(1).unwrap_or(l.scale).clamp(0.6, 1.8),
                k => {
                    if let (Some(a), Some(x), Some(y), Some(r)) = (ACTS.iter().find(|a| a.key() == k), num(1), num(2), num(3)) {
                        l.pos.insert(*a, (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0), r.clamp(0.02, 0.2)));
                    }
                }
            }
        }
        l
    }
}

#[derive(Default)]
struct State {
    out: Out,
    /// Палец на джойстике: (номер касания, центр).
    stick: Option<(u64, Vec2)>,
    /// Палец обзора: (номер касания, прошлое положение).
    look: Option<(u64, Vec2)>,
    /// Пальцы на кнопках.
    buttons: HashMap<u64, Act>,
    /// Где джойстик рисовать (центр, отклонение).
    stick_draw: Option<(Vec2, Vec2)>,
    /// Залипающие кнопки (бег, присед): включены ли.
    toggled: [bool; 14],
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State::default());
}

/// Текущее состояние для игры.
pub fn out() -> Out {
    ST.with(|s| s.borrow().out)
}

/// Устройство с сенсорным экраном (на ПК включается в настройках).
pub fn is_mobile() -> bool {
    cfg!(target_os = "android") || cfg!(target_os = "ios")
}

/// Опросить касания. `playing` — идёт игра (в меню касания работают как мышь).
/// `hotbar` — прямоугольник панели быстрого доступа (x, y, ширина ячейки, шаг, 9 ячеек).
pub fn update(layout: &Layout, enabled: bool, playing: bool, hotbar: (f32, f32, f32, f32)) {
    let (w, h) = (screen_width(), screen_height());
    ST.with(|st| {
        let mut st = st.borrow_mut();
        let prev_down = st.out.down;
        let toggled = st.toggled;
        st.out = Out { active: enabled, ..Default::default() };
        if !enabled || !playing {
            st.stick = None;
            st.look = None;
            st.buttons.clear();
            st.stick_draw = None;
            return;
        }
        let r_of = |a: Act| layout.get(a).2 * h * layout.scale;
        let center = |a: Act| {
            let (x, y, _) = layout.get(a);
            vec2(x * w, y * h)
        };
        let stick_r = 0.13 * h * layout.scale;
        for t in touches() {
            let p = t.position;
            match t.phase {
                TouchPhase::Started => {
                    // кнопка?
                    if let Some(a) = ACTS.iter().copied().filter(|a| (p - center(*a)).length() < r_of(*a) * 1.15).min_by(|a, b| (p - center(*a)).length().total_cmp(&(p - center(*b)).length())) {
                        st.buttons.insert(t.id, a);
                        st.out.pressed[a as usize] = true;
                        if matches!(a, Act::Sneak | Act::Sprint) {
                            st.toggled[a as usize] = !toggled[a as usize];
                        }
                        continue;
                    }
                    // панель быстрого доступа
                    let (hx, hy, cell, step) = hotbar;
                    if p.y >= hy && p.y <= hy + cell && p.x >= hx && p.x <= hx + step * 9.0 {
                        st.out.hotbar = Some((((p.x - hx) / step) as usize).min(8));
                        continue;
                    }
                    if p.x < w * 0.42 && p.y > h * 0.3 && st.stick.is_none() {
                        st.stick = Some((t.id, p));
                    } else if st.look.is_none() {
                        st.look = Some((t.id, p));
                    }
                }
                TouchPhase::Moved | TouchPhase::Stationary => {
                    if let Some((id, c)) = st.stick {
                        if id == t.id {
                            let d = p - c;
                            let k = (d.length() / stick_r).min(1.0);
                            let v = d.normalize_or_zero() * k;
                            st.out.mv = vec2(v.x, -v.y);
                            st.stick_draw = Some((c, v * stick_r));
                            // далеко отклонили вперёд — бег
                            if k > 0.95 && v.y < -0.85 {
                                st.out.down[Act::Sprint as usize] = true;
                            }
                        }
                    }
                    if let Some((id, last)) = st.look {
                        if id == t.id {
                            st.out.look += p - last;
                            st.look = Some((id, p));
                        }
                    }
                    if let Some(a) = st.buttons.get(&t.id).copied() {
                        st.out.down[a as usize] = true;
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if st.stick.is_some_and(|(id, _)| id == t.id) {
                        st.stick = None;
                        st.stick_draw = None;
                    }
                    if st.look.is_some_and(|(id, _)| id == t.id) {
                        st.look = None;
                    }
                    st.buttons.remove(&t.id);
                }
            }
        }
        // кнопки держатся, пока палец на них (в том числе в кадре нажатия)
        let held: Vec<Act> = st.buttons.values().copied().collect();
        for a in held {
            st.out.down[a as usize] = true;
        }
        for a in [Act::Sneak, Act::Sprint] {
            if st.toggled[a as usize] {
                st.out.down[a as usize] = true;
            }
        }
        let _ = prev_down;
    });
}

/// Нарисовать джойстик и кнопки поверх игры.
pub fn draw(layout: &Layout, ui: &crate::ui::Ui, creative: bool) {
    let (w, h) = (screen_width(), screen_height());
    let a = layout.opacity;
    ST.with(|st| {
        let st = st.borrow();
        let stick_r = 0.13 * h * layout.scale;
        let (c, off) = st.stick_draw.unwrap_or((vec2(w * 0.15, h * 0.72), Vec2::ZERO));
        draw_circle(c.x, c.y, stick_r, Color::new(0.0, 0.0, 0.0, a * 0.35));
        draw_circle_lines(c.x, c.y, stick_r, 2.0, Color::new(1.0, 1.0, 1.0, a * 0.6));
        draw_circle(c.x + off.x, c.y + off.y, stick_r * 0.42, Color::new(1.0, 1.0, 1.0, a * 0.55));
        for act in ACTS {
            if act == Act::Fly && !creative {
                continue;
            }
            let (x, y, r) = layout.get(act);
            let (x, y, r) = (x * w, y * h, r * h * layout.scale);
            let on = st.out.down[act as usize];
            draw_circle(x, y, r, if on { Color::new(1.0, 0.75, 0.25, a * 0.7) } else { Color::new(0.05, 0.06, 0.08, a * 0.6) });
            draw_circle_lines(x, y, r, 2.0, Color::new(1.0, 1.0, 1.0, a * 0.7));
            let size = (r / ui.s * 0.42).clamp(9.0, 22.0);
            ui.text_center(act.label(), x, y + size * ui.s * 0.36, size, Color::new(1.0, 1.0, 1.0, (a + 0.25).min(1.0)));
        }
    });
}
