//! Простой интерфейс «немедленного режима»: кнопки, ползунки, иконки предметов.

use crate::blocks::{self, Shape, tile_uv};
use crate::items::Item;
use macroquad::prelude::*;

pub const ACCENT: Color = Color::new(1.0, 0.72, 0.2, 1.0);
pub const PANEL: Color = Color::new(0.06, 0.07, 0.09, 0.82);

pub struct Ui {
    pub font: Font,
    pub atlas: Texture2D,
    pub s: f32,
    pub mouse: Vec2,
    pub pressed: bool,
    pub rpressed: bool,
    pub down: bool,
    drag: Option<u32>,
    tooltip: Option<(String, Option<String>)>,
}

impl Ui {
    pub fn new(font: Font, atlas: Texture2D) -> Self {
        Ui { font, atlas, s: 1.0, mouse: Vec2::ZERO, pressed: false, rpressed: false, down: false, drag: None, tooltip: None }
    }

    pub fn begin(&mut self, gui_scale: f32) {
        self.s = (screen_height() / 720.0).clamp(0.6, 3.0) * gui_scale;
        self.mouse = mouse_position().into();
        self.pressed = is_mouse_button_pressed(MouseButton::Left);
        self.rpressed = is_mouse_button_pressed(MouseButton::Right);
        self.down = is_mouse_button_down(MouseButton::Left);
        if !self.down {
            self.drag = None;
        }
        self.tooltip = None;
    }

    pub fn text(&self, t: &str, x: f32, y: f32, size: f32, color: Color) {
        let fs = (size * self.s).round().max(6.0) as u16;
        let p = TextParams { font: Some(&self.font), font_size: fs, color: Color::new(0.0, 0.0, 0.0, 0.5 * color.a), ..Default::default() };
        let off = (fs as f32 / 14.0).max(1.0);
        draw_text_ex(t, x + off, y + off, p.clone());
        draw_text_ex(t, x, y, TextParams { color, ..p });
    }

    pub fn measure(&self, t: &str, size: f32) -> f32 {
        let fs = (size * self.s).round().max(6.0) as u16;
        measure_text(t, Some(&self.font), fs, 1.0).width
    }

    pub fn text_center(&self, t: &str, cx: f32, y: f32, size: f32, color: Color) {
        let w = self.measure(t, size);
        self.text(t, cx - w / 2.0, y, size, color);
    }

    pub fn hover(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.mouse.x >= x && self.mouse.x < x + w && self.mouse.y >= y && self.mouse.y < y + h
    }

    pub fn panel(&self, x: f32, y: f32, w: f32, h: f32) {
        draw_rectangle(x, y, w, h, PANEL);
        draw_rectangle_lines(x, y, w, h, 2.0, Color::new(1.0, 1.0, 1.0, 0.08));
    }

    pub fn button(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str) -> bool {
        self.button_ex(x, y, w, h, label, true)
    }

    pub fn button_ex(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, enabled: bool) -> bool {
        self.button_full(x, y, w, h, label, enabled, 17.0)
    }

    pub fn button_small(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str) -> bool {
        self.button_full(x, y, w, h, label, true, 12.5)
    }

    fn button_full(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, enabled: bool, size: f32) -> bool {
        let hov = enabled && self.hover(x, y, w, h);
        let bg = if !enabled {
            Color::new(0.1, 0.1, 0.12, 0.6)
        } else if hov {
            Color::new(0.24, 0.26, 0.32, 0.95)
        } else {
            Color::new(0.13, 0.14, 0.18, 0.9)
        };
        draw_rectangle(x, y, w, h, bg);
        draw_rectangle(x, y + h - 3.0 * self.s, w, 3.0 * self.s, if hov { ACCENT } else { Color::new(0.0, 0.0, 0.0, 0.3) });
        draw_rectangle_lines(x, y, w, h, 2.0, if hov { ACCENT } else { Color::new(1.0, 1.0, 1.0, 0.1) });
        let col = if enabled { WHITE } else { GRAY };
        self.text_center(label, x + w / 2.0, y + h / 2.0 + size * self.s * 0.36, size, col);
        hov && self.pressed
    }

    pub fn slider(&mut self, id: u32, x: f32, y: f32, w: f32, h: f32, label: &str, value: f32, min: f32, max: f32) -> f32 {
        let hov = self.hover(x, y, w, h);
        if hov && self.pressed {
            self.drag = Some(id);
        }
        let mut v = value;
        if self.drag == Some(id) {
            let t = ((self.mouse.x - x) / w).clamp(0.0, 1.0);
            v = min + (max - min) * t;
        }
        let t = ((v - min) / (max - min)).clamp(0.0, 1.0);
        draw_rectangle(x, y, w, h, Color::new(0.13, 0.14, 0.18, 0.9));
        draw_rectangle(x, y, w * t, h, Color::new(1.0, 0.72, 0.2, 0.35));
        let kx = x + w * t;
        draw_rectangle(kx - 4.0 * self.s, y, 8.0 * self.s, h, if hov || self.drag == Some(id) { WHITE } else { ACCENT });
        draw_rectangle_lines(x, y, w, h, 2.0, Color::new(1.0, 1.0, 1.0, if hov { 0.3 } else { 0.1 }));
        let size = 16.0;
        self.text_center(label, x + w / 2.0, y + h / 2.0 + size * self.s * 0.36, size, WHITE);
        v
    }

    pub fn set_tooltip(&mut self, title: String, sub: Option<String>) {
        self.tooltip = Some((title, sub));
    }

    pub fn end(&mut self) {
        if let Some((t, sub)) = self.tooltip.take() {
            let w = self.measure(&t, 16.0).max(sub.as_ref().map_or(0.0, |s| self.measure(s, 13.0))) + 20.0 * self.s;
            let h = if sub.is_some() { 50.0 } else { 30.0 } * self.s;
            let (mut x, mut y) = (self.mouse.x + 14.0 * self.s, self.mouse.y - h - 4.0);
            x = x.min(screen_width() - w - 4.0);
            y = y.max(4.0);
            draw_rectangle(x, y, w, h, Color::new(0.04, 0.03, 0.08, 0.95));
            draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.5, 0.35, 0.9, 0.8));
            self.text(&t, x + 10.0 * self.s, y + 21.0 * self.s, 16.0, WHITE);
            if let Some(s) = sub {
                self.text(&s, x + 10.0 * self.s, y + 41.0 * self.s, 13.0, Color::new(0.6, 0.85, 1.0, 1.0));
            }
        }
    }

    fn tile_rect(tile: u16) -> Rect {
        let (u0, v0, u1, v1) = tile_uv(tile);
        let a = blocks::ATLAS as f32;
        Rect::new(u0 * a, v0 * a, (u1 - u0) * a, (v1 - v0) * a)
    }

    pub fn sprite(&self, tile: u16, x: f32, y: f32, size: f32, rot: f32, color: Color) {
        draw_texture_ex(
            &self.atlas,
            x,
            y,
            color,
            DrawTextureParams { dest_size: Some(vec2(size, size)), source: Some(Self::tile_rect(tile)), rotation: rot, ..Default::default() },
        );
    }

    /// Иконка предмета в квадрате (x, y, size).
    pub fn item_icon(&self, item: Item, x: f32, y: f32, size: f32) {
        match item {
            Item::Empty => {}
            Item::Block(b) => {
                let d = blocks::def(b);
                if d.shape == Shape::Cross {
                    self.sprite(d.tex[1], x + size * 0.1, y + size * 0.1, size * 0.8, 0.0, WHITE);
                } else {
                    iso_cube(&self.atlas, d.tex[0], d.tex[1], x + size / 2.0, y + size / 2.0, size * 0.86);
                }
            }
            other => {
                if let Some(i) = other.sprite_index() {
                    self.sprite(blocks::item_tile(i), x + size * 0.08, y + size * 0.08, size * 0.84, 0.0, WHITE);
                }
            }
        }
    }

    /// Ячейка инвентаря; возвращает (наведена, нажата ЛКМ, нажата ПКМ).
    pub fn slot(&mut self, x: f32, y: f32, size: f32, item: Item, selected: bool) -> (bool, bool, bool) {
        let hov = self.hover(x, y, size, size);
        draw_rectangle(x, y, size, size, if hov { Color::new(0.3, 0.3, 0.36, 0.9) } else { Color::new(0.12, 0.13, 0.16, 0.9) });
        if selected {
            draw_rectangle_lines(x - 2.0, y - 2.0, size + 4.0, size + 4.0, 4.0, ACCENT);
        } else {
            draw_rectangle_lines(x, y, size, size, 2.0, Color::new(1.0, 1.0, 1.0, 0.08));
        }
        self.item_icon(item, x, y, size);
        if hov && item != Item::Empty {
            self.set_tooltip(item.name(), item.description());
        }
        (hov, hov && self.pressed, hov && self.rpressed)
    }
}

/// Изометрический кубик блока для иконок.
pub fn iso_cube(atlas: &Texture2D, top: u16, side: u16, cx: f32, cy: f32, s: f32) {
    let (w, h) = (s * 0.45, s * 0.25);
    let t = vec2(cx, cy - s * 0.5);
    let r = vec2(cx + w, cy - s * 0.5 + h);
    let c = vec2(cx, cy - s * 0.5 + 2.0 * h);
    let l = vec2(cx - w, cy - s * 0.5 + h);
    let down = vec2(0.0, s * 0.5);
    let mut verts = Vec::with_capacity(12);
    let mut idx = Vec::with_capacity(18);
    let mut quad = |p: [Vec2; 4], tile: u16, k: f32| {
        let (u0, v0, u1, v1) = tile_uv(tile);
        let uv = [vec2(u0, v0), vec2(u1, v0), vec2(u1, v1), vec2(u0, v1)];
        let col = Color::new(k, k, k, 1.0);
        let b = verts.len() as u16;
        for i in 0..4 {
            verts.push(Vertex::new(p[i].x, p[i].y, 0.0, uv[i].x, uv[i].y, col));
        }
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    };
    quad([t, r, c, l], top, 1.0);
    quad([l, c, c + down, l + down], side, 0.78);
    quad([c, r, r + down, c + down], side, 0.6);
    draw_mesh(&Mesh { vertices: verts, indices: idx, texture: Some(atlas.clone()) });
}
