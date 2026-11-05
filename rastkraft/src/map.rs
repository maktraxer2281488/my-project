//! Карта мира: рельеф и биомы рисуются по генератору в фоне, поверх — игроки,
//! точка возрождения и постройки. Колесо — масштаб, ЛКМ — перетаскивание.

use crate::structures::{self, REGION};
use crate::world::Dim;
use crate::worldgen::{self, Biome, SEA};
use macroquad::prelude::*;
use std::sync::mpsc::{Receiver, channel};

/// Размер картинки карты в пикселях.
pub const N: usize = 384;
/// Масштабы: блоков на пиксель.
pub const SCALES: [f32; 6] = [0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

pub struct MapView {
    pub center: Vec2,
    pub scale_i: usize,
    tex: Option<(Texture2D, Vec2, f32, Dim)>,
    pending: Option<(Receiver<Vec<u8>>, Vec2, f32, Dim)>,
    drag: Option<(Vec2, Vec2)>,
}

fn shade(c: [f32; 3], k: f32) -> [u8; 4] {
    [(c[0] * k).clamp(0.0, 255.0) as u8, (c[1] * k).clamp(0.0, 255.0) as u8, (c[2] * k).clamp(0.0, 255.0) as u8, 255]
}

/// Нарисовать участок мира: каждая точка — колонка генератора.
fn render(seed: u32, dim: Dim, center: Vec2, scale: f32) -> Vec<u8> {
    let mut px = vec![0u8; N * N * 4];
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
    let rows = N.div_ceil(threads);
    std::thread::scope(|sc| {
        for (ci, part) in px.chunks_mut(rows * N * 4).enumerate() {
            sc.spawn(move || {
                for (ly, line) in part.chunks_mut(N * 4).enumerate() {
                    let j = ci * rows + ly;
                    let wz = (center.y + (j as f32 - N as f32 / 2.0) * scale).floor() as i32;
                    let mut prev_h: Option<i32> = None;
                    for i in 0..N {
                        let wx = (center.x + (i as f32 - N as f32 / 2.0) * scale).floor() as i32;
                        let c = match dim {
                            Dim::Overworld => {
                                let col = worldgen::column(seed, wx, wz);
                                let h = col.height;
                                let relief = prev_h.map_or(1.0, |p| 1.0 + ((h - p) as f32 * 0.06 / scale.max(1.0)).clamp(-0.25, 0.25));
                                prev_h = Some(h);
                                let height_k = 0.85 + ((h - SEA) as f32 / 60.0).clamp(-0.2, 0.35);
                                if h < SEA {
                                    let depth = (SEA - h) as f32;
                                    shade([50.0, 110.0, 200.0], (1.05 - depth * 0.025).max(0.45))
                                } else {
                                    let base = match col.biome {
                                        Biome::Desert => [222.0, 200.0, 140.0],
                                        Biome::Taiga => if h > 82 { [236.0, 240.0, 246.0] } else { [200.0, 214.0, 220.0] },
                                        _ => if h <= SEA + 1 { [214.0, 200.0, 150.0] } else { [96.0, 160.0, 72.0] },
                                    };
                                    shade(base, height_k * relief)
                                }
                            }
                            Dim::Abyss => {
                                // Бездна: лава и пепел (по шуму высоты дна)
                                let gnd = 34.0 + worldgen::fbm2(wx as f32 * 0.02, wz as f32 * 0.02, seed.wrapping_add(701), 3) * 26.0;
                                if gnd <= 31.0 { [230, 90, 30, 255] } else { shade([90.0, 70.0, 70.0], 0.7 + (gnd - 30.0) / 60.0) }
                            }
                            Dim::Heaven => match worldgen::heaven_island(seed, wx, wz) {
                                Some((top, _)) => {
                                    let relief = prev_h.map_or(1.0, |p| 1.0 + ((top - p) as f32 * 0.05).clamp(-0.2, 0.2));
                                    prev_h = Some(top);
                                    shade([236.0, 226.0, 170.0], relief * (0.9 + (top as f32 - 60.0) * 0.004))
                                }
                                None => {
                                    prev_h = None;
                                    [150, 190, 235, 255]
                                }
                            },
                        };
                        line[i * 4..i * 4 + 4].copy_from_slice(&c);
                    }
                }
            });
        }
    });
    px
}

pub struct Marker {
    pub pos: Vec2,
    pub label: String,
    pub color: Color,
    pub yaw: Option<f32>,
}

impl MapView {
    pub fn new(center: Vec2) -> Self {
        MapView { center, scale_i: 2, tex: None, pending: None, drag: None }
    }

    pub fn scale(&self) -> f32 {
        SCALES[self.scale_i]
    }

    /// Обновить и нарисовать карту. Возвращает true, если карту надо закрыть.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&mut self, ui: &mut crate::ui::Ui, seed: u32, dim: Dim, me: Vec2, my_yaw: f32, markers: &[Marker], just_opened: bool) -> bool {
        let (w, h) = (screen_width(), screen_height());
        let s = ui.s;
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.02, 0.03, 0.05, 0.92));
        let size = (h - 110.0 * s).min(w - 40.0 * s);
        let (x0, y0) = ((w - size) / 2.0, 60.0 * s);
        let ppu = size / N as f32; // экранных пикселей на пиксель карты
        let scale = self.scale();
        let to_screen = |p: Vec2, center: Vec2| vec2(x0 + size / 2.0 + (p.x - center.x) / scale * ppu, y0 + size / 2.0 + (p.y - center.y) / scale * ppu);
        let to_world = |q: Vec2, center: Vec2| vec2(center.x + (q.x - x0 - size / 2.0) / ppu * scale, center.y + (q.y - y0 - size / 2.0) / ppu * scale);

        // ввод: масштаб, перетаскивание, центр на игроке
        let mouse = ui.mouse;
        let inside = mouse.x >= x0 && mouse.x < x0 + size && mouse.y >= y0 && mouse.y < y0 + size;
        let (_, wheel) = mouse_wheel();
        if inside && wheel != 0.0 {
            let before = to_world(mouse, self.center);
            self.scale_i = if wheel > 0.0 { self.scale_i.saturating_sub(1) } else { (self.scale_i + 1).min(SCALES.len() - 1) };
            let after = to_world(mouse, self.center);
            self.center += before - after;
        }
        if ui.pressed && inside {
            self.drag = Some((mouse, self.center));
        }
        if !ui.down {
            self.drag = None;
        }
        if let Some((m0, c0)) = self.drag {
            self.center = c0 - (mouse - m0) / ppu * self.scale();
        }
        if is_key_pressed(KeyCode::Space) {
            self.center = me;
        }

        // фоновая перерисовка, когда ушли далеко или сменился масштаб
        let scale = self.scale();
        if let Some((rx, c, sc, d)) = &self.pending {
            if let Ok(px) = rx.try_recv() {
                let t = Texture2D::from_rgba8(N as u16, N as u16, &px);
                t.set_filter(FilterMode::Nearest);
                self.tex = Some((t, *c, *sc, *d));
                self.pending = None;
            }
        }
        let need = match &self.tex {
            None => true,
            Some((_, c, sc, d)) => *sc != scale || *d != dim || (*c - self.center).abs().max_element() > N as f32 * scale * 0.2,
        };
        let busy = self.pending.as_ref().is_some_and(|(_, _, sc, d)| *sc == scale && *d == dim);
        if need && !busy {
            let (tx, rx) = channel();
            let (c, sc) = (self.center, scale);
            std::thread::spawn(move || {
                let _ = tx.send(render(seed, dim, c, sc));
            });
            self.pending = Some((rx, self.center, scale, dim));
        }

        // картинка (сдвинутая, пока готовится новая)
        draw_rectangle(x0, y0, size, size, Color::new(0.1, 0.12, 0.16, 1.0));
        if let Some((t, c, sc, _)) = &self.tex {
            let k = *sc / scale;
            let tsz = size * k;
            let tl = to_screen(*c, self.center) - vec2(tsz, tsz) / 2.0;
            // обрезаем по рамке карты
            let (cx0, cy0) = (tl.x.max(x0), tl.y.max(y0));
            let (cx1, cy1) = ((tl.x + tsz).min(x0 + size), (tl.y + tsz).min(y0 + size));
            if cx1 > cx0 && cy1 > cy0 {
                let sx = (cx0 - tl.x) / tsz * N as f32;
                let sy = (cy0 - tl.y) / tsz * N as f32;
                let sw = (cx1 - cx0) / tsz * N as f32;
                let sh = (cy1 - cy0) / tsz * N as f32;
                draw_texture_ex(t, cx0, cy0, WHITE, DrawTextureParams { dest_size: Some(vec2(cx1 - cx0, cy1 - cy0)), source: Some(Rect::new(sx, sy, sw, sh)), ..Default::default() });
            }
        }
        draw_rectangle_lines(x0, y0, size, size, 3.0 * s, Color::new(0.85, 0.7, 0.35, 1.0));

        // постройки
        let half = N as f32 / 2.0 * scale;
        let (r0x, r1x) = (((self.center.x - half) / REGION as f32).floor() as i32 - 1, ((self.center.x + half) / REGION as f32).floor() as i32 + 1);
        let (r0z, r1z) = (((self.center.y - half) / REGION as f32).floor() as i32 - 1, ((self.center.y + half) / REGION as f32).floor() as i32 + 1);
        let mut hover: Option<String> = None;
        if (r1x - r0x) * (r1z - r0z) < 2500 {
            for rz in r0z..=r1z {
                for rx in r0x..=r1x {
                    let Some((sx, _, sz, biome, kind)) = structures::region_site(dim, seed, rx, rz) else { continue };
                    let q = to_screen(vec2(sx as f32, sz as f32), self.center);
                    if q.x < x0 || q.y < y0 || q.x > x0 + size || q.y > y0 + size {
                        continue;
                    }
                    let r = (4.0 * s).max(2.0);
                    draw_rectangle(q.x - r, q.y - r, r * 2.0, r * 2.0, Color::new(0.35, 0.2, 0.1, 1.0));
                    draw_rectangle(q.x - r + 1.5, q.y - r + 1.5, r * 2.0 - 3.0, r * 2.0 - 3.0, Color::new(0.95, 0.75, 0.3, 1.0));
                    if (mouse - q).length() < r * 2.0 {
                        let names = match biome {
                            Biome::Desert => &structures::DESERT,
                            Biome::Taiga => &structures::TAIGA,
                            Biome::Islands => &structures::HEAVEN,
                            _ => &structures::MEADOW,
                        };
                        hover = Some(names[kind % 10].to_string());
                    }
                }
            }
        }
        // метки: игроки, возрождение
        for m in markers {
            let q = to_screen(m.pos, self.center);
            if q.x < x0 || q.y < y0 || q.x > x0 + size || q.y > y0 + size {
                continue;
            }
            draw_circle(q.x, q.y, 5.0 * s, Color::new(0.0, 0.0, 0.0, 0.6));
            draw_circle(q.x, q.y, 3.8 * s, m.color);
            ui.text(&m.label, q.x + 8.0 * s, q.y + 4.0 * s, 12.0, WHITE);
        }
        // я — стрелка по направлению взгляда
        let q = to_screen(me, self.center);
        if q.x >= x0 && q.y >= y0 && q.x <= x0 + size && q.y <= y0 + size {
            let f = vec2(my_yaw.cos(), my_yaw.sin());
            let side = vec2(-f.y, f.x);
            let r = 9.0 * s;
            let (a, b, c) = (q + f * r, q - f * r * 0.6 + side * r * 0.6, q - f * r * 0.6 - side * r * 0.6);
            draw_triangle(a + vec2(1.5, 1.5), b + vec2(1.5, 1.5), c + vec2(1.5, 1.5), Color::new(0.0, 0.0, 0.0, 0.6));
            draw_triangle(a, b, c, Color::new(1.0, 0.3, 0.25, 1.0));
        }

        // подписи
        ui.text_center(match dim { Dim::Heaven => "Карта Рая", Dim::Abyss => "Карта Бездны", _ => "Карта мира" }, w / 2.0, 40.0 * s, 24.0, WHITE);
        let wpos = to_world(mouse, self.center);
        let info = if inside {
            let place = match dim {
                Dim::Overworld => worldgen::column(seed, wpos.x.floor() as i32, wpos.y.floor() as i32).biome.name(),
                Dim::Heaven => "Рай",
                Dim::Abyss => "Бездна",
            };
            format!("X {:.0}  Z {:.0} · {}", wpos.x, wpos.y, place)
        } else {
            format!("Вы: X {:.0}  Z {:.0}", me.x, me.y)
        };
        let by = y0 + size + 24.0 * s;
        ui.text(&info, x0, by, 14.0, WHITE);
        let help = format!("Колесо — масштаб (1 пикс. = {} бл.) · ЛКМ — двигать · Пробел — к себе · M или Esc — закрыть", scale);
        ui.text(&help, x0, by + 20.0 * s, 12.0, GRAY);
        if let Some(name) = hover {
            ui.set_tooltip(name, Some("Постройка".into()));
        }
        if self.pending.is_some() {
            ui.text("Рисую карту…", x0 + size - ui.measure("Рисую карту…", 12.0), y0 - 6.0 * s, 12.0, GRAY);
        }
        !just_opened && (is_key_pressed(KeyCode::M) || is_key_pressed(KeyCode::Escape))
    }
}
