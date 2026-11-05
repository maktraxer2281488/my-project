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
    /// Подпись под кнопкой.
    pub fn label(self) -> &'static str {
        match self {
            Act::Jump => "Прыжок",
            Act::Attack => "Ломать",
            Act::Use => "Ставить",
            Act::Sneak => "Присесть",
            Act::Sprint => "Бег",
            Act::Inventory => "Сумка",
            Act::Pause => "Меню",
            Act::Map => "Карта",
            Act::Book => "Рецепты",
            Act::Chat => "Чат",
            Act::Fly => "Полёт",
            Act::Camera => "Камера",
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
    /// Короткое касание экрана: по существу — удар, иначе — поставить/использовать.
    pub tap: bool,
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
    /// Подписи под кнопками.
    pub labels: bool,
    /// Касание экрана — поставить/использовать, удержание — ломать/бить.
    pub tap: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { pos: ACTS.iter().map(|a| (*a, a.default_pos())).collect(), opacity: 0.6, scale: 1.0, labels: true, tap: true }
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
        v.push(format!("labels:{}:0:0", self.labels as u8));
        v.push(format!("tap:{}:0:0", self.tap as u8));
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
                "labels" => l.labels = f[1] != "0",
                "tap" => l.tap = f[1] != "0",
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

/// Палец, которым водят по экрану: обзор, касание и удержание.
#[derive(Clone, Copy)]
struct Look {
    id: u64,
    last: Vec2,
    start: Vec2,
    t0: f64,
    /// Насколько далеко палец уходил от места касания.
    moved: f32,
    /// Палец держат на месте — ломаем/бьём.
    holding: bool,
}

/// Сколько держать палец, чтобы начать ломать (с).
const HOLD_TIME: f64 = 0.3;

#[derive(Default)]
struct State {
    out: Out,
    /// Палец на джойстике: (номер касания, центр).
    stick: Option<(u64, Vec2)>,
    /// Палец обзора.
    look: Option<Look>,
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
        let tap_slop = h * 0.025;
        let now = get_time();
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
                        st.look = Some(Look { id: t.id, last: p, start: p, t0: now, moved: 0.0, holding: false });
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
                    if let Some(mut l) = st.look.filter(|l| l.id == t.id) {
                        st.out.look += p - l.last;
                        l.last = p;
                        l.moved = l.moved.max((p - l.start).length());
                        // держат палец на месте — начинаем ломать
                        if layout.tap && !l.holding && l.moved < tap_slop && now - l.t0 > HOLD_TIME {
                            l.holding = true;
                            st.out.pressed[Act::Attack as usize] = true;
                        }
                        if l.holding {
                            st.out.down[Act::Attack as usize] = true;
                        }
                        st.look = Some(l);
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
                    if let Some(l) = st.look.filter(|l| l.id == t.id) {
                        // короткое касание на месте — поставить блок/использовать
                        if layout.tap && t.phase == TouchPhase::Ended && !l.holding && l.moved < tap_slop && now - l.t0 < HOLD_TIME {
                            st.out.tap = true;
                        }
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

fn quad(a: Vec2, b: Vec2, c: Vec2, d: Vec2, col: Color) {
    draw_triangle(a, b, c, col);
    draw_triangle(a, c, d, col);
}

fn line(a: Vec2, b: Vec2, w: f32, col: Color) {
    draw_line(a.x, a.y, b.x, b.y, w, col);
}

/// Нарисовать значок действия в круге радиуса `r` с центром `c`.
pub fn draw_icon(act: Act, c: Vec2, r: f32, col: Color) {
    let u = r * 0.55;
    let lw = (r * 0.11).max(2.0);
    let v = |x: f32, y: f32| c + vec2(x * u, y * u);
    let dim = Color::new(col.r * 0.75, col.g * 0.75, col.b * 0.75, col.a);
    let dark = Color::new(col.r * 0.5, col.g * 0.5, col.b * 0.5, col.a);
    let bg = Color::new(0.05, 0.06, 0.08, col.a);
    match act {
        Act::Jump => {
            draw_triangle(v(0.0, -0.95), v(-0.85, 0.05), v(0.85, 0.05), col);
            quad(v(-0.3, 0.0), v(0.3, 0.0), v(0.3, 0.85), v(-0.3, 0.85), col);
        }
        Act::Attack => {
            // кирка
            let (a, b) = (v(-0.75, 0.8), v(0.35, -0.3));
            line(a, b, lw * 1.3, Color::new(0.75, 0.55, 0.3, col.a));
            let head = [v(-0.45, -0.55), v(0.0, -0.75), v(0.45, -0.6), v(0.75, -0.2), v(0.65, 0.3)];
            for k in 0..4 {
                line(head[k], head[k + 1], lw * 1.8, col);
            }
        }
        Act::Use => {
            // блок
            let (t, rr, m, l) = (v(0.0, -0.85), v(0.8, -0.42), v(0.0, 0.0), v(-0.8, -0.42));
            let d = vec2(0.0, 0.85 * u);
            quad(t, rr, m, l, col);
            quad(l, m, m + d, l + d, dim);
            quad(m, rr, rr + d, m + d, dark);
        }
        Act::Sneak => {
            for k in 0..2 {
                let y = -0.55 + k as f32 * 0.65;
                line(v(-0.7, y), v(0.0, y + 0.5), lw * 1.3, col);
                line(v(0.0, y + 0.5), v(0.7, y), lw * 1.3, col);
            }
        }
        Act::Sprint => {
            for k in 0..2 {
                let x = -0.6 + k as f32 * 0.65;
                line(v(x, -0.7), v(x + 0.5, 0.0), lw * 1.3, col);
                line(v(x + 0.5, 0.0), v(x, 0.7), lw * 1.3, col);
            }
        }
        Act::Inventory => {
            // рюкзак: ручка, скруглённый верх, карман
            draw_circle_lines(c.x, c.y - 0.85 * u, 0.2 * u, lw * 0.8, col);
            draw_circle(c.x, c.y - 0.2 * u, 0.68 * u, col);
            quad(v(-0.68, -0.2), v(0.68, -0.2), v(0.68, 0.85), v(-0.68, 0.85), col);
            quad(v(-0.42, 0.2), v(0.42, 0.2), v(0.42, 0.68), v(-0.42, 0.68), dark);
            line(v(-0.42, 0.2), v(0.42, 0.2), lw * 0.7, bg);
        }
        Act::Pause => {
            // меню: три полоски
            for k in 0..3 {
                let y = -0.55 + k as f32 * 0.55;
                quad(v(-0.75, y - 0.12), v(0.75, y - 0.12), v(0.75, y + 0.12), v(-0.75, y + 0.12), col);
            }
        }
        Act::Map => {
            let xs = [-0.85, -0.28, 0.28, 0.85];
            let top = [-0.6, -0.8, -0.6, -0.8];
            for k in 0..3 {
                let cc = if k % 2 == 0 { col } else { dim };
                quad(v(xs[k], top[k]), v(xs[k + 1], top[k + 1]), v(xs[k + 1], top[k + 1] + 1.4), v(xs[k], top[k] + 1.4), cc);
            }
            draw_circle(c.x + 0.1 * u, c.y - 0.05 * u, 0.2 * u, Color::new(0.9, 0.2, 0.15, col.a));
        }
        Act::Book => {
            quad(v(-0.9, -0.6), v(-0.05, -0.45), v(-0.05, 0.75), v(-0.9, 0.6), col);
            quad(v(0.05, -0.45), v(0.9, -0.6), v(0.9, 0.6), v(0.05, 0.75), dim);
            for k in 0..3 {
                let y = -0.2 + k as f32 * 0.3;
                line(v(-0.7, y), v(-0.25, y + 0.07), lw * 0.6, bg);
                line(v(0.25, y + 0.07), v(0.7, y), lw * 0.6, bg);
            }
        }
        Act::Chat => {
            quad(v(-0.85, -0.65), v(0.85, -0.65), v(0.85, 0.35), v(-0.85, 0.35), col);
            draw_triangle(v(-0.45, 0.3), v(-0.05, 0.3), v(-0.55, 0.85), col);
            for k in 0..3 {
                draw_circle(c.x + (k as f32 - 1.0) * 0.45 * u, c.y - 0.15 * u, 0.12 * u, bg);
            }
        }
        Act::Fly => {
            // крылья
            for sx in [-1.0f32, 1.0] {
                draw_triangle(v(0.1 * sx, 0.1), v(0.95 * sx, -0.7), v(0.75 * sx, 0.15), col);
                draw_triangle(v(0.1 * sx, 0.1), v(0.75 * sx, 0.15), v(0.45 * sx, 0.6), dim);
            }
            draw_circle(c.x, c.y + 0.05 * u, 0.2 * u, col);
        }
        Act::Camera => {
            // глаз
            draw_ellipse(c.x, c.y, 0.9 * u, 0.5 * u, 0.0, col);
            draw_circle(c.x, c.y, 0.36 * u, bg);
            draw_circle(c.x, c.y, 0.18 * u, col);
        }
        Act::Drop => {
            quad(v(-0.22, -0.85), v(0.22, -0.85), v(0.22, 0.0), v(-0.22, 0.0), col);
            draw_triangle(v(-0.6, -0.05), v(0.6, -0.05), v(0.0, 0.55), col);
            line(v(-0.8, 0.8), v(0.8, 0.8), lw * 1.2, col);
        }
        Act::Players => {
            for (k, dx) in [(1, 0.35f32), (0, -0.3)] {
                let cc = if k == 1 { dim } else { col };
                draw_circle(c.x + dx * u, c.y - 0.4 * u, 0.28 * u, cc);
                draw_circle(c.x + dx * u, c.y + 0.45 * u, 0.42 * u, cc);
                draw_rectangle(c.x + (dx - 0.42) * u, c.y + 0.45 * u, 0.84 * u, 0.4 * u, cc);
            }
        }
    }
}

/// Нарисовать одну кнопку: круг, значок и подпись.
pub fn draw_button(act: Act, c: Vec2, r: f32, on: bool, a: f32, labels: bool, ui: &crate::ui::Ui) {
    let fill = if on { Color::new(1.0, 0.72, 0.2, (a * 0.85).min(1.0)) } else { Color::new(0.05, 0.06, 0.08, a * 0.6) };
    draw_circle(c.x, c.y, r, fill);
    draw_circle_lines(c.x, c.y, r, 2.0, Color::new(1.0, 1.0, 1.0, a * 0.7));
    let ic = if on { Color::new(0.1, 0.08, 0.02, 1.0) } else { Color::new(1.0, 1.0, 1.0, (a + 0.3).min(1.0)) };
    draw_icon(act, c, r, ic);
    if labels {
        let size = 11.0;
        let px = size * ui.s;
        let h = screen_height();
        // подпись под кнопкой, у нижнего края — над ней
        let y = if c.y + r + px * 1.2 > h { c.y - r - px * 0.35 } else { c.y + r + px * 0.95 };
        ui.text_center(act.label(), c.x + 1.0, y + 1.0, size, Color::new(0.0, 0.0, 0.0, (a + 0.2).min(0.9)));
        ui.text_center(act.label(), c.x, y, size, Color::new(1.0, 1.0, 1.0, (a + 0.35).min(1.0)));
    }
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
            draw_button(act, vec2(x * w, y * h), r * h * layout.scale, st.out.down[act as usize], a, layout.labels, ui);
        }
    });
}
