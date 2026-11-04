#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Расткрафт — воксельная песочница на Rust.

mod blocks;
mod worldgen;
mod items;
mod mesher;
mod mobs;
mod player;
mod render;
mod settings;
mod ui;
mod workers;
mod world;

use blocks::*;
use items::Item;
use macroquad::prelude::*;
use mobs::Mobs;
use player::{MAX_HEALTH, Player, break_time};
use render::{Frame, Loader, Renderer};
use settings::Settings;
use ui::{ACCENT, Ui};
use workers::Pool;
use world::{CS, Dim, World, chunk_of};

const REACH: f32 = 5.0;
const DAY_SECONDS: f32 = 720.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Расткрафт".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Title,
    NewWorld,
    Loading,
    Playing,
    Paused,
    Settings,
    Inventory,
    Dead,
}

struct Game {
    worlds: [World; 2],
    dim: Dim,
    seed: u32,
    player: Player,
    mobs: Mobs,
    creative: bool,
    time: f32,
    spawn: Vec3,
    portal_home: Option<IVec3>,
    pending_portals: Vec<(Dim, i32, i32)>,
    spawn_pending: bool,
    break_target: Option<IVec3>,
    break_progress: f32,
    use_cd: f32,
    attack_cd: f32,
    swing_t: f32,
    portal_cd: f32,
    label_t: f32,
    toast: Option<(String, f32)>,
}

impl Game {
    fn new(seed: u32, creative: bool) -> Self {
        let (sx, sz) = worldgen::find_spawn(seed);
        let spawn = vec3(sx as f32 + 0.5, 100.0, sz as f32 + 0.5);
        let mut player = Player::new(spawn);
        if creative {
            player.flying = false;
        }
        Game {
            worlds: [World::new(Dim::Overworld, seed), World::new(Dim::Heaven, seed.wrapping_add(17))],
            dim: Dim::Overworld,
            seed,
            player,
            mobs: Mobs::new(),
            creative,
            time: 0.04,
            spawn,
            portal_home: None,
            pending_portals: vec![(Dim::Overworld, sx + 5, sz - 1)],
            spawn_pending: true,
            break_target: None,
            break_progress: 0.0,
            use_cd: 0.0,
            attack_cd: 0.0,
            swing_t: 0.0,
            portal_cd: 2.0,
            label_t: 2.0,
            toast: Some(("Добро пожаловать в Расткрафт!".into(), 4.0)),
        }
    }

    fn world(&self) -> &World {
        &self.worlds[self.dim as usize]
    }

    fn world_mut(&mut self) -> &mut World {
        &mut self.worlds[self.dim as usize]
    }

    fn daylight(&self) -> f32 {
        if self.dim == Dim::Heaven {
            return 1.0;
        }
        let sunh = (self.time * std::f32::consts::TAU).sin();
        let d = ((sunh + 0.2) / 0.5).clamp(0.0, 1.0);
        0.1 + 0.9 * d * d * (3.0 - 2.0 * d)
    }

    fn sun_dir(&self) -> Vec3 {
        let a = self.time * std::f32::consts::TAU;
        vec3(a.cos(), a.sin(), 0.25).normalize()
    }
}

struct App {
    screen: Screen,
    settings_back: Screen,
    settings: Settings,
    ui: Ui,
    pool: Pool,
    loader: Loader,
    renderer: Renderer,
    game: Option<Game>,
    seed_input: String,
    new_creative: bool,
    inv_tab: usize,
    inv_scroll: f32,
    grabbed: bool,
    last_mouse: Vec2,
    show_debug: bool,
    hide_hud: bool,
    cracks: Vec<Texture2D>,
    time: f32,
    fps: f32,
}

fn hash_seed(s: &str) -> u32 {
    if let Ok(n) = s.trim().parse::<i64>() {
        return n as u32;
    }
    let mut h: u32 = 2_166_136_261;
    for b in s.bytes() {
        h = (h ^ b as u32).wrapping_mul(16_777_619);
    }
    h
}

fn crack_textures() -> Vec<Texture2D> {
    (1..=4)
        .map(|stage| {
            let mut px = vec![0u8; 16 * 16 * 4];
            for y in 0..16i32 {
                for x in 0..16i32 {
                    let mut on = false;
                    for k in 0..stage * 2 {
                        let (ox, oy) = ((hash(k, 1, 77) * 16.0) as i32, (hash(k, 2, 77) * 16.0) as i32);
                        let ang = hash(k, 3, 77) * 6.28;
                        let (dx, dy) = ((x - ox) as f32, (y - oy) as f32);
                        let along = dx * ang.cos() + dy * ang.sin();
                        let across = -dx * ang.sin() + dy * ang.cos();
                        if across.abs() < 0.7 && along.abs() < 2.0 + stage as f32 * 1.6 {
                            on = true;
                        }
                    }
                    if on {
                        let i = ((y * 16 + x) * 4) as usize;
                        px[i..i + 4].copy_from_slice(&[10, 10, 12, 200]);
                    }
                }
            }
            let t = Texture2D::from_rgba8(16, 16, &px);
            t.set_filter(FilterMode::Nearest);
            t
        })
        .collect()
}

fn set_grab(app: &mut App, grab: bool) {
    if app.grabbed != grab {
        app.grabbed = grab;
        set_cursor_grab(grab);
        show_mouse(!grab);
    }
}

/// Изменение блока с перестройкой геометрии.
fn edit_block(app: &mut App, p: IVec3, b: u8) {
    let smooth = app.settings.smooth_light;
    let Some(g) = app.game.as_mut() else { return };
    let w = g.world_mut();
    if !w.set(p.x, p.y, p.z, b) {
        return;
    }
    let (cx, cz) = chunk_of(p.x, p.z);
    for dz in -1..=1 {
        for dx in -1..=1 {
            if (dx, dz) != (0, 0) && w.chunks.contains_key(&(cx + dx, cz + dz)) {
                *w.versions.entry((cx + dx, cz + dz)).or_insert(0) += 1;
            }
        }
    }
    let ctx = unsafe { get_internal_gl() }.quad_context;
    let w = &app.game.as_ref().unwrap().worlds[app.game.as_ref().unwrap().dim as usize];
    app.loader.remesh_now(ctx, w, cx, cz, smooth);
}

fn build_portal(w: &mut World, x: i32, z: i32) -> Option<IVec3> {
    for dz in -2..=2 {
        for dx in -1..=5 {
            if !w.is_loaded(x + dx, z + dz) {
                return None;
            }
        }
    }
    let mut g = 0;
    for dx in 0..4 {
        g = g.max(w.surface_y(x + dx, z));
    }
    if g <= 0 {
        g = 60;
    }
    // расчищаем площадку и ставим рамку 4×5 с порталом 2×3 внутри
    for dz in -2..=2 {
        for dx in -1..=4 {
            for dy in 0..6 {
                w.set(x + dx, g + dy, z + dz, AIR);
            }
            if !is_solid(w.get(x + dx, g - 1, z + dz)) {
                w.set(x + dx, g - 1, z + dz, if w.dim == Dim::Heaven { HEAVEN_STONE } else { STONE_BRICKS });
            }
        }
    }
    for dx in 0..4 {
        for dy in 0..5 {
            let edge = dx == 0 || dx == 3 || dy == 0 || dy == 4;
            w.set(x + dx, g - 1 + dy, z, if edge { PORTAL_FRAME } else { PORTAL });
        }
    }
    let (cx, cz) = chunk_of(x, z);
    for ddz in -1..=1 {
        for ddx in -1..=2 {
            if w.chunks.contains_key(&(cx + ddx, cz + ddz)) {
                *w.versions.entry((cx + ddx, cz + ddz)).or_insert(0) += 1;
            }
        }
    }
    Some(ivec3(x, g, z))
}

/// Место на земле (не на кроне дерева) рядом с колонкой x, z.
fn ground_spot(w: &World, x: i32, z: i32) -> Option<Vec3> {
    let mut best: Option<(i32, Vec3)> = None;
    for dz in -10..=10 {
        for dx in -10..=10 {
            let (cx, cz) = (x + dx, z + dz);
            if !w.is_loaded(cx, cz) {
                continue;
            }
            let y = w.surface_y(cx, cz);
            if y <= 0 {
                continue;
            }
            let top = w.get(cx, y - 1, cz);
            let ok = matches!(top, GRASS | SAND | SNOW_GRASS | DIRT | HEAVEN_GRASS | GRAVEL | STONE | SNOW_BLOCK | RED_SAND)
                && w.get(cx, y, cz) != WATER
                && !blocks::is_solid(w.get(cx, y + 1, cz));
            let d = dx * dx + dz * dz;
            if ok && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                best = Some((d, vec3(cx as f32 + 0.5, y as f32 + 0.01, cz as f32 + 0.5)));
            }
        }
    }
    best.map(|b| b.1).or_else(|| {
        let y = w.surface_y(x, z);
        (y > 0).then(|| vec3(x as f32 + 0.5, y as f32 + 0.01, z as f32 + 0.5))
    })
}

fn switch_dimension(app: &mut App, to: Dim) {
    let ctx = unsafe { get_internal_gl() }.quad_context;
    app.loader.clear(ctx);
    let g = app.game.as_mut().unwrap();
    g.dim = to;
    g.mobs.list.clear();
    g.portal_cd = 4.0;
    g.break_target = None;
    match to {
        Dim::Heaven => {
            let (top, _) = worldgen::heaven_island(g.worlds[1].seed, 0, 0).unwrap_or((80, 70));
            g.player.body.pos = vec3(1.5, top as f32 + 1.2, 3.5);
            if !g.pending_portals.iter().any(|p| p.0 == Dim::Heaven) && g.worlds[1].chunks.is_empty() {
                g.pending_portals.push((Dim::Heaven, -1, 0));
            }
            g.toast = Some(("Вы попали в Рай".into(), 4.0));
        }
        Dim::Overworld => {
            let p = g.portal_home.map(|p| p.as_vec3() + vec3(1.5, 0.1, 2.5)).unwrap_or(g.spawn);
            g.player.body.pos = p;
            g.toast = Some(("Верхний мир".into(), 3.0));
        }
    }
    g.player.body.vel = Vec3::ZERO;
    g.spawn_pending = false;
    app.screen = Screen::Loading;
}

fn sky_colors(g: &Game) -> (Color, Color) {
    if g.dim == Dim::Heaven {
        return (Color::new(0.98, 0.78, 0.58, 1.0), Color::new(1.0, 0.94, 0.84, 1.0));
    }
    let d = g.daylight();
    let lerp = |a: Color, b: Color, t: f32| Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1.0);
    let top = lerp(Color::new(0.01, 0.02, 0.06, 1.0), Color::new(0.28, 0.52, 0.88, 1.0), d);
    let mut hor = lerp(Color::new(0.04, 0.06, 0.13, 1.0), Color::new(0.72, 0.85, 0.98, 1.0), d);
    let sunh = g.sun_dir().y;
    let dusk = (-(sunh / 0.2).powi(2)).exp();
    hor = lerp(hor, Color::new(0.98, 0.56, 0.32, 1.0), dusk * 0.7);
    (top, hor)
}

fn draw_sky(g: &Game, vp: &Mat4, top: Color, hor: Color) {
    let (w, h) = (screen_width(), screen_height());
    let proj = |dir: Vec3| -> Option<Vec2> {
        let c = *vp * (dir * 1000.0).extend(1.0);
        if c.w <= 0.0 {
            return None;
        }
        Some(vec2((c.x / c.w + 1.0) * 0.5 * w, (1.0 - c.y / c.w) * 0.5 * h))
    };
    let fwd = g.player.forward();
    let ahead = vec3(fwd.x, 0.0, fwd.z).normalize_or_zero();
    let horizon = proj(ahead).map_or(h * 0.5, |p| p.y);
    let top_y = horizon - h * 1.3;
    const BANDS: i32 = 40;
    for i in 0..BANDS {
        let t0 = i as f32 / BANDS as f32;
        let y0 = top_y + (horizon - top_y) * t0;
        let y1 = top_y + (horizon - top_y) * (i + 1) as f32 / BANDS as f32;
        let k = t0.powf(2.0);
        let c = Color::new(top.r + (hor.r - top.r) * k, top.g + (hor.g - top.g) * k, top.b + (hor.b - top.b) * k, 1.0);
        draw_rectangle(0.0, y0, w, y1 - y0 + 1.0, c);
    }
    if horizon < h {
        draw_rectangle(0.0, horizon, w, h - horizon, hor);
    }
    if top_y > 0.0 {
        draw_rectangle(0.0, 0.0, w, top_y, top);
    }
    if g.dim == Dim::Overworld {
        let d = g.daylight();
        if d < 0.6 {
            let a = (1.0 - d / 0.6).min(1.0);
            for i in 0..260 {
                let (u, v) = (hash(i, 1, 9) * std::f32::consts::TAU, hash(i, 2, 9) * 0.95 + 0.05);
                let dir = vec3(u.cos() * (1.0 - v * v).sqrt(), v, u.sin() * (1.0 - v * v).sqrt());
                if let Some(p) = proj(dir) {
                    let s = 1.0 + hash(i, 3, 9) * 1.5;
                    draw_rectangle(p.x, p.y, s, s, Color::new(1.0, 1.0, 1.0, a * (0.4 + hash(i, 4, 9) * 0.6)));
                }
            }
        }
        let sun = g.sun_dir();
        let r = h * 0.04;
        if let Some(p) = proj(sun) {
            for (k, a) in [(3.4, 0.07), (2.2, 0.13), (1.5, 0.25)] {
                draw_circle(p.x, p.y, r * k, Color::new(1.0, 0.95, 0.75, a));
            }
            draw_circle(p.x, p.y, r, Color::new(1.0, 0.98, 0.86, 1.0));
        }
        if let Some(p) = proj(-sun) {
            draw_circle(p.x, p.y, r * 1.8, Color::new(0.8, 0.85, 1.0, 0.08));
            draw_circle(p.x, p.y, r * 0.8, Color::new(0.88, 0.9, 0.98, 1.0));
            draw_circle(p.x + r * 0.25, p.y - r * 0.1, r * 0.2, Color::new(0.75, 0.77, 0.86, 1.0));
        }
    } else {
        // мягкое «солнце» Рая
        if let Some(p) = proj(vec3(0.3, 0.7, 0.5).normalize()) {
            for (k, a) in [(5.0, 0.06), (3.0, 0.12), (1.8, 0.25)] {
                draw_circle(p.x, p.y, h * 0.05 * k, Color::new(1.0, 1.0, 0.9, a));
            }
            draw_circle(p.x, p.y, h * 0.05, Color::new(1.0, 1.0, 0.95, 1.0));
        }
    }
}

// ---------------------------------------------------------------------------
// Игровая логика
// ---------------------------------------------------------------------------

fn update_game(app: &mut App, dt: f32, mouse_delta: Vec2) {
    let playing = app.screen == Screen::Playing;
    let sens = app.settings.sensitivity;
    let invert = if app.settings.invert_mouse { -1.0 } else { 1.0 };
    let difficulty = app.settings.difficulty;
    let mut edits: Vec<(IVec3, u8)> = Vec::new();
    let mut to_dim: Option<Dim> = None;
    let mut died = false;
    {
        let g = app.game.as_mut().unwrap();
        g.time = (g.time + dt / DAY_SECONDS).fract();
        if playing {
            g.player.look(mouse_delta.x, mouse_delta.y * invert, sens);
        }
        let creative = g.creative;
        let daylight = g.daylight();
        let dim = g.dim;
        let world = &g.worlds[dim as usize];
        g.player.update(world, dt, creative, playing);
        g.mobs.update(world, &mut g.player, dt, daylight, difficulty, creative);
        g.label_t -= dt;
        g.use_cd -= dt;
        g.attack_cd -= dt;
        g.swing_t = (g.swing_t - dt * 3.5).max(0.0);
        g.portal_cd -= dt;
        if let Some((_, t)) = &mut g.toast {
            *t -= dt;
        }
        if g.toast.as_ref().is_some_and(|(_, t)| *t <= 0.0) {
            g.toast = None;
        }

        // падение в пустоту
        if g.player.body.pos.y < -30.0 {
            if dim == Dim::Heaven {
                let (top, _) = worldgen::heaven_island(g.worlds[1].seed, 0, 0).unwrap_or((80, 70));
                g.player.body.pos = vec3(1.5, top as f32 + 1.2, 3.5);
                g.player.body.vel = Vec3::ZERO;
                g.player.hurt(4.0, Vec3::ZERO, true);
            } else if !creative {
                g.player.health = 0.0;
            } else {
                g.player.body.pos.y = 120.0;
            }
        }

        // порталы
        if g.portal_cd <= 0.0 && g.player.body.touching(&g.worlds[dim as usize], -0.05).contains(&PORTAL) {
            to_dim = Some(if dim == Dim::Overworld { Dim::Heaven } else { Dim::Overworld });
        }

        if playing {
            // выбор предмета
            let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8, KeyCode::Key9];
            for (i, k) in keys.iter().enumerate() {
                if is_key_pressed(*k) {
                    g.player.selected = i;
                    g.label_t = 2.0;
                }
            }
            let (_, wheel) = mouse_wheel();
            if wheel != 0.0 {
                g.player.selected = (g.player.selected as i32 - wheel.signum() as i32).rem_euclid(9) as usize;
                g.label_t = 2.0;
            }
            if is_key_pressed(KeyCode::F) && creative {
                g.player.flying = !g.player.flying;
            }

            let world = &g.worlds[dim as usize];
            let eye = g.player.eye();
            let dir = g.player.forward();
            let block_hit = world.raycast(eye, dir, REACH);
            let block_d = block_hit.map_or(f32::MAX, |h| h.2);
            let mut mob_hit: Option<(usize, f32)> = None;
            for (i, m) in g.mobs.list.iter().enumerate() {
                if let Some(t) = m.ray_hit(eye, dir) {
                    if t < 3.6 && t < block_d && mob_hit.is_none_or(|(_, bt)| t < bt) {
                        mob_hit = Some((i, t));
                    }
                }
            }
            let held = g.player.held();
            let lmb = is_mouse_button_down(MouseButton::Left);
            let lmb_p = is_mouse_button_pressed(MouseButton::Left);

            if lmb_p {
                g.swing_t = 1.0;
            }
            if let Some((mi, _)) = mob_hit {
                g.break_target = None;
                if lmb_p && g.attack_cd <= 0.0 {
                    let from = g.player.body.pos;
                    g.mobs.list[mi].damage(held.damage(), from);
                    g.attack_cd = 0.4;
                }
            } else if let Some((hit, prev, _)) = block_hit {
                let b = world.get(hit.x, hit.y, hit.z);
                if lmb {
                    if creative {
                        if (lmb_p || g.use_cd <= 0.0) && b != BEDROCK {
                            edits.push((hit, AIR));
                            g.use_cd = 0.22;
                        }
                    } else if let Some(bt) = break_time(b, held) {
                        if g.break_target != Some(hit) {
                            g.break_target = Some(hit);
                            g.break_progress = 0.0;
                        }
                        g.break_progress += if bt <= 0.0 { 1.0 } else { dt / bt };
                        g.swing_t = g.swing_t.max(0.6);
                        if g.break_progress >= 1.0 {
                            edits.push((hit, AIR));
                            g.break_target = None;
                            g.break_progress = 0.0;
                        }
                    }
                } else {
                    g.break_target = None;
                    g.break_progress = 0.0;
                }
                if is_mouse_button_down(MouseButton::Right) && (is_mouse_button_pressed(MouseButton::Right) || g.use_cd <= 0.0) {
                    match held {
                        Item::Block(nb) => {
                            let cur = world.get(prev.x, prev.y, prev.z);
                            let replace = cur == AIR || def(cur).shape == blocks::Shape::Liquid || def(cur).shape == blocks::Shape::Cross;
                            let blocked = is_solid(nb) && (g.player.body.overlaps_block(prev) || g.mobs.list.iter().any(|m| m.body.overlaps_block(prev)));
                            if replace && !blocked {
                                edits.push((prev, nb));
                                g.swing_t = 1.0;
                            }
                        }
                        Item::Armor(..) if is_mouse_button_pressed(MouseButton::Right) => {
                            g.player.equip(held);
                            g.toast = Some((format!("Надето: {}", held.name()), 2.0));
                        }
                        _ => {}
                    }
                    g.use_cd = 0.22;
                }
                if is_mouse_button_pressed(MouseButton::Middle) {
                    g.player.hotbar[g.player.selected] = Item::Block(b);
                    g.label_t = 2.0;
                }
            } else {
                g.break_target = None;
                if is_mouse_button_pressed(MouseButton::Right) {
                    if let Item::Armor(..) = held {
                        g.player.equip(held);
                        g.toast = Some((format!("Надето: {}", held.name()), 2.0));
                    }
                }
            }
        }
        if g.player.health <= 0.0 {
            died = true;
        }
    }

    for (p, b) in edits {
        edit_block(app, p, b);
        if b == AIR {
            // растения и факелы без опоры падают
            let mut y = p.y + 1;
            loop {
                let w = app.game.as_ref().unwrap().world();
                let above = w.get(p.x, y, p.z);
                if above != AIR && def(above).shape == blocks::Shape::Cross && above != COBWEB {
                    edit_block(app, ivec3(p.x, y, p.z), AIR);
                    y += 1;
                } else {
                    break;
                }
            }
        }
    }
    if let Some(d) = to_dim {
        switch_dimension(app, d);
    }
    if died {
        app.screen = Screen::Dead;
    }
}

fn place_pending(app: &mut App) {
    let g = app.game.as_mut().unwrap();
    let dim = g.dim;
    let mut i = 0;
    while i < g.pending_portals.len() {
        let (d, x, z) = g.pending_portals[i];
        if d == dim {
            if let Some(p) = build_portal(&mut g.worlds[dim as usize], x, z) {
                if d == Dim::Overworld {
                    g.portal_home = Some(p);
                }
                g.pending_portals.remove(i);
                continue;
            }
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Экраны
// ---------------------------------------------------------------------------

fn draw_title(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    // фон: небо, облака, полоса из блоков
    for i in 0..30 {
        let t = i as f32 / 30.0;
        let c = Color::new(0.25 + t * 0.45, 0.5 + t * 0.35, 0.86 + t * 0.12, 1.0);
        draw_rectangle(0.0, h * t * 0.8, w, h * 0.8 / 30.0 + 1.0, c);
    }
    for i in 0..9 {
        let x = ((app.time * (12.0 + i as f32 * 3.0) + i as f32 * 211.0) % (w + 400.0)) - 200.0;
        let y = h * (0.08 + hash(i, 0, 3) * 0.35);
        let cw = 120.0 + hash(i, 1, 3) * 160.0;
        draw_rectangle(x, y, cw * s, 18.0 * s, Color::new(1.0, 1.0, 1.0, 0.85));
        draw_rectangle(x + 24.0 * s, y - 12.0 * s, cw * 0.5 * s, 14.0 * s, Color::new(1.0, 1.0, 1.0, 0.85));
    }
    let cell = 56.0 * s;
    let row = [GRASS, GRASS, OAK_LOG, GRASS, SAND, SANDSTONE, SNOW_GRASS, SPRUCE_LOG, GRASS, STONE, RUBY_ORE, GRASS, CRYSTAL_ORE, HEAVEN_GRASS, GOLD_BLOCK];
    let ground = h * 0.8;
    draw_rectangle(0.0, ground, w, h - ground, Color::new(0.33, 0.24, 0.17, 1.0));
    let n = (w / cell) as i32 + 2;
    for i in 0..n {
        let b = row[(i as usize * 7 + 3) % row.len()];
        let d = def(b);
        ui::iso_cube(&app.ui.atlas, d.tex[0], d.tex[1], i as f32 * cell, ground + cell * 0.1, cell * 1.15);
    }
    // логотип
    let title = "РАСТКРАФТ";
    let size = 92.0;
    let tw = app.ui.measure(title, size);
    let ty = h * 0.25;
    for k in (1..=6).rev() {
        let o = k as f32 * 1.5 * s;
        app.ui.text(title, (w - tw) / 2.0 + o, ty + o, size, Color::new(0.35, 0.16, 0.05, 1.0));
    }
    app.ui.text(title, (w - tw) / 2.0, ty, size, ACCENT);
    let sub = "воксельная песочница на Rust";
    let pulse = 1.0 + (app.time * 3.0).sin() * 0.04;
    app.ui.text_center(sub, w / 2.0, ty + 48.0 * s, 20.0 * pulse, WHITE);

    let bw = 360.0 * s;
    let bh = 48.0 * s;
    let bx = (w - bw) / 2.0;
    let mut y = h * 0.42;
    let mut next = None;
    if app.ui.button(bx, y, bw, bh, "Новый мир") {
        next = Some(Screen::NewWorld);
    }
    y += bh + 12.0 * s;
    if app.ui.button(bx, y, bw, bh, "Настройки") {
        app.settings_back = Screen::Title;
        next = Some(Screen::Settings);
    }
    y += bh + 12.0 * s;
    if app.ui.button(bx, y, bw, bh, "Выход") {
        std::process::exit(0);
    }
    app.ui.text("Версия 0.2", 10.0 * s, h - 12.0 * s, 13.0, Color::new(1.0, 1.0, 1.0, 0.7));
    next
}

fn draw_new_world(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 1.0));
    app.ui.text_center("Создание мира", w / 2.0, h * 0.18, 36.0, WHITE);
    let bw = 440.0 * s;
    let bh = 48.0 * s;
    let bx = (w - bw) / 2.0;
    let mut y = h * 0.28;

    app.ui.text("Зерно мира (пусто — случайное):", bx, y - 8.0 * s, 15.0, Color::new(0.8, 0.8, 0.85, 1.0));
    draw_rectangle(bx, y, bw, bh, Color::new(0.03, 0.03, 0.05, 1.0));
    draw_rectangle_lines(bx, y, bw, bh, 2.0, ACCENT);
    while let Some(c) = get_char_pressed() {
        if !c.is_control() && app.seed_input.chars().count() < 32 {
            app.seed_input.push(c);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        app.seed_input.pop();
    }
    let caret = if (app.time * 2.0).fract() < 0.5 { "|" } else { "" };
    app.ui.text(&format!("{}{}", app.seed_input, caret), bx + 14.0 * s, y + bh * 0.66, 20.0, WHITE);
    y += bh + 24.0 * s;

    let mode = if app.new_creative { "Режим: Творчество" } else { "Режим: Выживание" };
    if app.ui.button(bx, y, bw, bh, mode) {
        app.new_creative = !app.new_creative;
    }
    y += bh + 6.0 * s;
    let hint = if app.new_creative {
        "Полёт (F), мгновенное разрушение, неуязвимость"
    } else {
        "Здоровье, броня, существа, добыча занимает время"
    };
    app.ui.text_center(hint, w / 2.0, y + 14.0 * s, 14.0, Color::new(0.75, 0.78, 0.85, 1.0));
    y += 34.0 * s;
    let diff = format!("Сложность: {}", settings::DIFFICULTY[app.settings.difficulty as usize]);
    if app.ui.button(bx, y, bw, bh, &diff) {
        app.settings.difficulty = (app.settings.difficulty + 1) % 4;
        app.settings.save();
    }
    y += bh + 30.0 * s;
    let mut next = None;
    if app.ui.button(bx, y, bw / 2.0 - 6.0 * s, bh, "Назад") {
        next = Some(Screen::Title);
    }
    if app.ui.button(bx + bw / 2.0 + 6.0 * s, y, bw / 2.0 - 6.0 * s, bh, "Создать мир") || is_key_pressed(KeyCode::Enter) {
        let seed = if app.seed_input.trim().is_empty() {
            (miniquad::date::now() * 1000.0) as u64 as u32
        } else {
            hash_seed(&app.seed_input)
        };
        start_world(app, seed, app.new_creative);
        next = Some(Screen::Loading);
    }
    next
}

fn start_world(app: &mut App, seed: u32, creative: bool) {
    let ctx = unsafe { get_internal_gl() }.quad_context;
    app.loader.clear(ctx);
    app.game = Some(Game::new(seed, creative));
}

fn settings_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    if app.game.is_none() {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 1.0));
    } else {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.55));
    }
    app.ui.text_center("Настройки", w / 2.0, h * 0.12, 34.0, WHITE);
    let cw = 330.0 * s;
    let bh = 44.0 * s;
    let gap = 12.0 * s;
    let x1 = w / 2.0 - cw - gap / 2.0;
    let x2 = w / 2.0 + gap / 2.0;
    let mut y = h * 0.18;
    let st = &mut app.settings;
    let old = st.clone();
    let ui = &mut app.ui;

    let rd = ui.slider(1, x1, y, cw, bh, &format!("Дальность прорисовки: {} чанков", st.render_dist), st.render_dist as f32, 2.0, 32.0);
    st.render_dist = rd.round() as i32;
    let fov = ui.slider(2, x2, y, cw, bh, &format!("Поле зрения: {:.0}°", st.fov), st.fov, 50.0, 110.0);
    st.fov = fov.round();
    y += bh + gap;
    st.sensitivity = ui.slider(3, x1, y, cw, bh, &format!("Чувствительность мыши: {:.0}%", st.sensitivity * 100.0), st.sensitivity, 0.1, 4.0);
    st.brightness = ui.slider(4, x2, y, cw, bh, &format!("Яркость: {:.0}%", st.brightness * 100.0), st.brightness, 0.0, 1.0);
    y += bh + gap;
    let gs = ui.slider(5, x1, y, cw, bh, &format!("Размер интерфейса: {:.0}%", st.gui_scale * 100.0), st.gui_scale, 0.75, 2.0);
    st.gui_scale = (gs * 20.0).round() / 20.0;
    if ui.button(x2, y, cw, bh, &format!("Сложность: {}", settings::DIFFICULTY[st.difficulty as usize])) {
        st.difficulty = (st.difficulty + 1) % 4;
    }
    y += bh + gap;
    let onoff = |b: bool| if b { "Вкл" } else { "Выкл" };
    if ui.button(x1, y, cw, bh, &format!("Туман: {}", onoff(st.fog))) {
        st.fog = !st.fog;
    }
    if ui.button(x2, y, cw, bh, &format!("Облака: {}", onoff(st.clouds))) {
        st.clouds = !st.clouds;
    }
    y += bh + gap;
    if ui.button(x1, y, cw, bh, &format!("Плавное освещение: {}", onoff(st.smooth_light))) {
        st.smooth_light = !st.smooth_light;
    }
    if ui.button(x2, y, cw, bh, &format!("Покачивание камеры: {}", onoff(st.view_bob))) {
        st.view_bob = !st.view_bob;
    }
    y += bh + gap;
    if ui.button(x1, y, cw, bh, &format!("Показывать FPS: {}", onoff(st.show_fps))) {
        st.show_fps = !st.show_fps;
    }
    if ui.button(x2, y, cw, bh, &format!("Инверсия мыши: {}", onoff(st.invert_mouse))) {
        st.invert_mouse = !st.invert_mouse;
    }
    y += bh + gap * 2.5;
    let done = ui.button((w - cw) / 2.0, y, cw, bh, "Готово") || is_key_pressed(KeyCode::Escape);
    st.clamp();
    if st.smooth_light != old.smooth_light {
        app.loader.epoch += 1;
    }
    if done {
        app.settings.save();
        return Some(app.settings_back);
    }
    None
}

fn pause_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.5));
    app.ui.text_center("Пауза", w / 2.0, h * 0.25, 36.0, WHITE);
    let bw = 380.0 * s;
    let bh = 46.0 * s;
    let bx = (w - bw) / 2.0;
    let mut y = h * 0.33;
    let mut next = None;
    if app.ui.button(bx, y, bw, bh, "Продолжить") {
        next = Some(Screen::Playing);
    }
    y += bh + 10.0 * s;
    let creative = app.game.as_ref().is_some_and(|g| g.creative);
    let label = if creative { "Режим: Творчество" } else { "Режим: Выживание" };
    if app.ui.button(bx, y, bw, bh, label) {
        if let Some(g) = app.game.as_mut() {
            g.creative = !g.creative;
            g.player.flying = false;
        }
    }
    y += bh + 10.0 * s;
    if app.ui.button(bx, y, bw, bh, "Настройки") {
        app.settings_back = Screen::Paused;
        next = Some(Screen::Settings);
    }
    y += bh + 10.0 * s;
    if app.ui.button(bx, y, bw, bh, "Выйти в главное меню") {
        let ctx = unsafe { get_internal_gl() }.quad_context;
        app.loader.clear(ctx);
        app.game = None;
        next = Some(Screen::Title);
    }
    y += bh + 10.0 * s;
    if app.ui.button(bx, y, bw, bh, "Выйти из игры") {
        std::process::exit(0);
    }
    next
}

fn dead_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.45, 0.0, 0.0, 0.45));
    app.ui.text_center("Вы погибли!", w / 2.0, h * 0.3, 48.0, WHITE);
    let bw = 360.0 * s;
    let bh = 46.0 * s;
    let bx = (w - bw) / 2.0;
    if app.ui.button(bx, h * 0.42, bw, bh, "Возродиться") {
        let in_heaven = app.game.as_ref().is_some_and(|g| g.dim == Dim::Heaven);
        if in_heaven {
            switch_dimension(app, Dim::Overworld);
        }
        let g = app.game.as_mut().unwrap();
        g.player.body.pos = g.spawn;
        g.player.body.vel = Vec3::ZERO;
        g.player.health = MAX_HEALTH;
        g.player.fall_start = None;
        g.spawn_pending = true;
        return Some(Screen::Loading);
    }
    if app.ui.button(bx, h * 0.42 + bh + 12.0 * s, bw, bh, "Главное меню") {
        let ctx = unsafe { get_internal_gl() }.quad_context;
        app.loader.clear(ctx);
        app.game = None;
        return Some(Screen::Title);
    }
    None
}

fn inventory_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.45));
    let slot = 48.0 * s;
    let gap = 6.0 * s;
    let cols = 10;
    let grid_w = cols as f32 * (slot + gap) - gap;
    let left_w = 170.0 * s;
    let pw = left_w + grid_w + 60.0 * s;
    let ph = (h * 0.86).min(640.0 * s);
    let px = (w - pw) / 2.0;
    let py = (h - ph) / 2.0;
    app.ui.panel(px, py, pw, ph);
    app.ui.text("Инвентарь", px + 20.0 * s, py + 34.0 * s, 22.0, WHITE);

    // броня
    let g = app.game.as_mut().unwrap();
    let ax = px + 20.0 * s;
    let mut ay = py + 70.0 * s;
    app.ui.text("Броня", ax, ay - 8.0 * s, 15.0, ACCENT);
    let names = ["Шлем", "Нагрудник", "Поножи", "Ботинки"];
    for i in 0..4 {
        let (_, click, rclick) = app.ui.slot(ax, ay, slot, g.player.armor[i], false);
        if g.player.armor[i] == Item::Empty {
            app.ui.text(names[i], ax + slot + 8.0 * s, ay + slot * 0.6, 13.0, GRAY);
        }
        if click || rclick {
            g.player.armor[i] = Item::Empty;
        }
        ay += slot + gap;
    }
    app.ui.text(&format!("Защита: {}", g.player.armor_points()), ax, ay + 20.0 * s, 15.0, WHITE);
    app.ui.text(&format!("Урон: {:.0}", g.player.held().damage()), ax, ay + 42.0 * s, 15.0, WHITE);

    // вкладки
    let gx = px + left_w + 30.0 * s;
    let tab_w = (grid_w - gap * 7.0) / 8.0;
    for (i, name) in items::TABS.iter().enumerate() {
        let x = gx + i as f32 * (tab_w + gap);
        let sel = app.inv_tab == i;
        if app.ui.button_small(x, py + 14.0 * s, tab_w, 34.0 * s, name) {
            app.inv_tab = i;
            app.inv_scroll = 0.0;
        }
        if sel {
            draw_rectangle(x, py + 14.0 * s + 31.0 * s, tab_w, 3.0 * s, WHITE);
        }
    }
    // каталог
    let cat = items::catalog(app.inv_tab);
    let grid_top = py + 62.0 * s;
    let hotbar_y = py + ph - slot - 20.0 * s;
    let grid_h = hotbar_y - grid_top - 30.0 * s;
    let rows = cat.len().div_ceil(cols);
    let visible_rows = (grid_h / (slot + gap)).floor().max(1.0) as usize;
    let max_scroll = rows.saturating_sub(visible_rows) as f32;
    let (_, wheel) = mouse_wheel();
    app.inv_scroll = (app.inv_scroll - wheel.signum()).clamp(0.0, max_scroll);
    let first = app.inv_scroll as usize * cols;
    for (k, item) in cat.iter().enumerate().skip(first).take(visible_rows * cols) {
        let i = k - first;
        let x = gx + (i % cols) as f32 * (slot + gap);
        let y = grid_top + (i / cols) as f32 * (slot + gap);
        let (_, click, _) = app.ui.slot(x, y, slot, *item, false);
        if click {
            match item {
                Item::Armor(..) => g.player.equip(*item),
                _ => g.player.hotbar[g.player.selected] = *item,
            }
        }
    }
    if max_scroll > 0.0 {
        app.ui.text(&format!("Колесо мыши — прокрутка ({}/{})", app.inv_scroll as usize + 1, max_scroll as usize + 1), gx, hotbar_y - 12.0 * s, 13.0, GRAY);
    }
    // панель быстрого доступа
    app.ui.text("Быстрый доступ: щелчок — выбрать ячейку, ПКМ — очистить", gx, hotbar_y - 32.0 * s + 14.0 * s, 13.0, Color::new(0.8, 0.8, 0.85, 1.0));
    for i in 0..9 {
        let x = gx + i as f32 * (slot + gap);
        let (_, click, rclick) = app.ui.slot(x, hotbar_y, slot, g.player.hotbar[i], g.player.selected == i);
        if click {
            g.player.selected = i;
        }
        if rclick {
            g.player.hotbar[i] = Item::Empty;
        }
    }
    if is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Escape) {
        return Some(Screen::Playing);
    }
    None
}

fn draw_hud(app: &mut App) {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    let g = app.game.as_ref().unwrap();

    // прицел
    let c = Color::new(1.0, 1.0, 1.0, 0.85);
    draw_rectangle(w / 2.0 - 10.0 * s, h / 2.0 - 1.0 * s, 20.0 * s, 2.0 * s, c);
    draw_rectangle(w / 2.0 - 1.0 * s, h / 2.0 - 10.0 * s, 2.0 * s, 20.0 * s, c);

    // панель быстрого доступа
    let slot = 48.0 * s;
    let gap = 5.0 * s;
    let total = 9.0 * (slot + gap) - gap;
    let x0 = (w - total) / 2.0;
    let y0 = h - slot - 14.0 * s;
    draw_rectangle(x0 - 8.0 * s, y0 - 8.0 * s, total + 16.0 * s, slot + 16.0 * s, Color::new(0.04, 0.05, 0.07, 0.55));
    for i in 0..9 {
        let x = x0 + i as f32 * (slot + gap);
        let sel = g.player.selected == i;
        draw_rectangle(x, y0, slot, slot, Color::new(0.1, 0.11, 0.13, 0.75));
        if sel {
            draw_rectangle_lines(x - 3.0 * s, y0 - 3.0 * s, slot + 6.0 * s, slot + 6.0 * s, 4.0 * s, ACCENT);
        }
        app.ui.item_icon(g.player.hotbar[i], x, y0, slot);
        app.ui.text(&(i + 1).to_string(), x + 3.0 * s, y0 + 13.0 * s, 11.0, Color::new(1.0, 1.0, 1.0, 0.6));
    }

    // здоровье и броня
    if !g.creative {
        let hs = 18.0 * s;
        let hy = y0 - hs - 14.0 * s;
        for i in 0..10 {
            let v = g.player.health - i as f32 * 2.0;
            let tile = if v >= 2.0 {
                items::HUD_HEART_FULL
            } else if v >= 1.0 {
                items::HUD_HEART_HALF
            } else {
                items::HUD_HEART_EMPTY
            };
            let shake = if g.player.health <= 4.0 { ((app.time * 30.0 + i as f32).sin() * 1.5 * s).round() } else { 0.0 };
            app.ui.sprite(blocks::item_tile(tile), x0 + i as f32 * (hs + 1.0), hy + shake, hs, 0.0, WHITE);
        }
        let ap = g.player.armor_points();
        if ap > 0 {
            let ay = hy - hs - 4.0 * s;
            for i in 0..10 {
                let v = ap - i * 2;
                let tile = if v >= 2 { items::HUD_ARMOR_FULL } else if v == 1 { items::HUD_ARMOR_HALF } else { items::HUD_ARMOR_EMPTY };
                app.ui.sprite(blocks::item_tile(tile), x0 + i as f32 * (hs + 1.0), ay, hs, 0.0, WHITE);
            }
        }
    }

    // название предмета
    if g.label_t > 0.0 {
        let item = g.player.held();
        if item != Item::Empty {
            let a = g.label_t.min(1.0);
            let off = if g.creative { 30.0 } else { 70.0 };
            app.ui.text_center(&item.name(), w / 2.0, y0 - off * s, 18.0, Color::new(1.0, 1.0, 1.0, a));
        }
    }
    if let Some((t, time)) = &g.toast {
        app.ui.text_center(t, w / 2.0, h * 0.2, 26.0, Color::new(1.0, 0.9, 0.6, time.min(1.0)));
    }

    // отладка / FPS
    if app.show_debug {
        let p = g.player.body.pos;
        let (cx, cz) = chunk_of(p.x.floor() as i32, p.z.floor() as i32);
        let biome = if g.dim == Dim::Heaven { worldgen::Biome::Islands } else { worldgen::column(g.seed, p.x as i32, p.z as i32).biome };
        let hours = (g.time * 24.0 + 6.0) % 24.0;
        let lines = [
            format!("Расткрафт 0.2   {:.0} к/с", app.fps),
            format!("XYZ: {:.1} / {:.1} / {:.1}", p.x, p.y, p.z),
            format!("Чанк: {cx} {cz}   Измерение: {}", g.dim.name()),
            format!("Биом: {}", biome.name()),
            format!("Чанков в памяти: {}   отрисовано: {}", g.world().chunks.len(), app.loader.gpu.len()),
            format!("Время: {:02}:{:02}   Существ: {}", hours as i32, ((hours.fract()) * 60.0) as i32, g.mobs.list.len()),
            format!("Зерно: {}", g.seed),
        ];
        draw_rectangle(6.0 * s, 6.0 * s, 430.0 * s, (lines.len() as f32 * 20.0 + 14.0) * s, Color::new(0.0, 0.0, 0.0, 0.45));
        for (i, l) in lines.iter().enumerate() {
            app.ui.text(l, 14.0 * s, (26.0 + i as f32 * 20.0) * s, 14.0, WHITE);
        }
    } else if app.settings.show_fps {
        app.ui.text(&format!("{:.0} к/с", app.fps), 12.0 * s, 24.0 * s, 14.0, Color::new(1.0, 1.0, 1.0, 0.8));
    }
}

fn draw_hand(app: &App, g: &Game) {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    let item = g.player.held();
    let bob = if app.settings.view_bob { g.player.walk_phase.sin() * 8.0 * s } else { 0.0 };
    let bob_y = if app.settings.view_bob { (g.player.walk_phase * 2.0).sin().abs() * 6.0 * s } else { 0.0 };
    let sw = (g.swing_t * std::f32::consts::PI).sin();
    let size = 170.0 * s;
    let x = w - size * 1.05 + bob - sw * 60.0 * s;
    let y = h - size * 0.85 + bob_y + sw * 40.0 * s;
    match item {
        Item::Empty => {
            draw_rectangle(x + size * 0.4, y + size * 0.3, size * 0.32, size * 0.8, Color::new(0.86, 0.66, 0.5, 1.0));
        }
        Item::Block(b) if def(b).shape != blocks::Shape::Cross => {
            let d = def(b);
            ui::iso_cube(&app.ui.atlas, d.tex[0], d.tex[1], x + size * 0.5, y + size * 0.55, size * 0.8);
        }
        _ => {
            let tile = match item {
                Item::Block(b) => def(b).tex[1],
                other => blocks::item_tile(other.sprite_index().unwrap_or(0)),
            };
            app.ui.sprite(tile, x, y, size, -0.3 - sw * 0.9, WHITE);
        }
    }
}

// ---------------------------------------------------------------------------

#[macroquad::main(window_conf)]
async fn main() {
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/DejaVuSans-Bold.ttf")).expect("шрифт");
    clear_background(Color::new(0.08, 0.09, 0.11, 1.0));
    let msg = "Расткрафт загружается…";
    let dim = measure_text(msg, Some(&font), 32, 1.0);
    draw_text_ex(msg, (screen_width() - dim.width) / 2.0, screen_height() / 2.0, TextParams { font: Some(&font), font_size: 32, color: WHITE, ..Default::default() });
    next_frame().await;

    let atlas = Texture2D::from_rgba8(ATLAS as u16, ATLAS as u16, atlas_pixels());
    atlas.set_filter(FilterMode::Nearest);
    let renderer = {
        let ctx = unsafe { get_internal_gl() }.quad_context;
        Renderer::new(ctx, atlas.raw_miniquad_id())
    };
    let mut app = App {
        screen: Screen::Title,
        settings_back: Screen::Title,
        settings: Settings::load(),
        ui: Ui::new(font, atlas),
        pool: Pool::new(),
        loader: Loader::new(),
        renderer,
        game: None,
        seed_input: String::new(),
        new_creative: false,
        inv_tab: 0,
        inv_scroll: 0.0,
        grabbed: false,
        last_mouse: mouse_position().into(),
        show_debug: false,
        hide_hud: false,
        cracks: crack_textures(),
        time: 0.0,
        fps: 60.0,
    };

    // режим для автоматических скриншотов
    let shot = std::env::var("RK_SHOT").ok();
    let mut shot_frames = 0;
    if let Ok(seed) = std::env::var("RK_SEED") {
        if let Ok(r) = std::env::var("RK_DIST") {
            app.settings.render_dist = r.parse().unwrap_or(8);
        }
        start_world(&mut app, hash_seed(&seed), std::env::var("RK_CREATIVE").is_ok());
        app.screen = Screen::Loading;
    }
    let shot_screen = std::env::var("RK_SCREEN").unwrap_or_default();
    if shot_screen == "title" {
        app.game = None;
        app.screen = Screen::Title;
    }

    loop {
        let raw_dt = get_frame_time();
        let dt = raw_dt.min(0.05);
        app.time += dt;
        app.fps += (1.0 / raw_dt.max(1e-4) - app.fps) * 0.05;
        app.ui.begin(app.settings.gui_scale);
        let mouse: Vec2 = mouse_position().into();
        let md = if app.grabbed { mouse - app.last_mouse } else { Vec2::ZERO };
        app.last_mouse = mouse;

        // --- клавиши ---
        match app.screen {
            Screen::Playing => {
                if is_key_pressed(KeyCode::Escape) {
                    app.screen = Screen::Paused;
                } else if is_key_pressed(KeyCode::E) {
                    app.screen = Screen::Inventory;
                }
                if is_key_pressed(KeyCode::F3) {
                    app.show_debug = !app.show_debug;
                }
                if is_key_pressed(KeyCode::F1) {
                    app.hide_hud = !app.hide_hud;
                }
            }
            Screen::Paused if is_key_pressed(KeyCode::Escape) => app.screen = Screen::Playing,
            _ => {}
        }
        let want_grab = app.screen == Screen::Playing && shot.is_none();
        set_grab(&mut app, want_grab);

        // --- мир ---
        if app.game.is_some() {
            let (center, dim) = {
                let g = app.game.as_ref().unwrap();
                let p = g.player.body.pos;
                (chunk_of(p.x.floor() as i32, p.z.floor() as i32), g.dim)
            };
            let _ = dim;
            {
                let ctx = unsafe { get_internal_gl() }.quad_context;
                let g = app.game.as_mut().unwrap();
                let w = &mut g.worlds[g.dim as usize];
                app.loader.update(ctx, &mut app.pool, w, center, app.settings.render_dist, app.settings.smooth_light);
            }
            place_pending(&mut app);

            if app.screen == Screen::Loading {
                let g = app.game.as_mut().unwrap();
                let p = g.player.body.pos;
                let (x, z) = (p.x.floor() as i32, p.z.floor() as i32);
                let mut ready = true;
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        if !app.loader.has_mesh(center.0 + dx, center.1 + dz) {
                            ready = false;
                        }
                    }
                }
                if ready && g.world().is_loaded(x, z) {
                    if g.spawn_pending || g.player.body.collides_at(g.world(), g.player.body.pos) {
                        if let Some(p) = ground_spot(g.world(), x, z) {
                            g.player.body.pos = p;
                        }
                        if g.spawn_pending {
                            g.spawn = g.player.body.pos;
                            g.spawn_pending = false;
                        }
                    }
                    if let Ok(pose) = std::env::var("RK_POSE") {
                        let v: Vec<f32> = pose.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                        if v.len() == 5 {
                            g.player.body.pos = vec3(v[0], v[1], v[2]);
                            g.player.yaw = v[3];
                            g.player.pitch = v[4];
                            g.creative = true;
                            g.player.flying = true;
                        }
                    }
                    if std::env::var("RK_MOBS").is_ok() {
                        let p = g.player.body.pos;
                        let f = g.player.forward().with_y(0.0).normalize_or_zero();
                        let side = vec3(-f.z, 0.0, f.x);
                        for (i, k) in [mobs::Kind::Shade, mobs::Kind::Hopper, mobs::Kind::Fluff].into_iter().enumerate() {
                            let at = p + f * 4.5 + side * (i as f32 - 1.0) * 1.8;
                            let y = g.world().surface_y(at.x.floor() as i32, at.z.floor() as i32) as f32;
                            g.mobs.list.push(mobs::Mob::new(k, vec3(at.x, y, at.z)));
                        }
                        g.creative = true;
                    }
                    if let Ok(t) = std::env::var("RK_TIME") {
                        g.time = t.parse().unwrap_or(g.time);
                    }
                    app.screen = Screen::Playing;
                    if std::env::var("RK_DIM").is_ok_and(|d| d == "heaven") && g.dim == Dim::Overworld {
                        switch_dimension(&mut app, Dim::Heaven);
                    }
                }
            }
            if matches!(app.screen, Screen::Playing | Screen::Inventory) {
                update_game(&mut app, dt, md);
            }
        }

        // --- отрисовка ---
        if let Some(g) = app.game.as_ref() {
            let (top, hor) = sky_colors(g);
            clear_background(hor);
            let eye_base = g.player.eye();
            let bob = if app.settings.view_bob && !g.player.flying {
                vec3(0.0, (g.player.walk_phase * 2.0).sin().abs() * 0.06, 0.0)
            } else {
                Vec3::ZERO
            };
            let eye = eye_base + bob;
            let dir = g.player.forward();
            let fovy = app.settings.fov.to_radians() * if is_key_down(KeyCode::C) && app.screen == Screen::Playing { 0.35 } else { 1.0 };
            let aspect = screen_width() / screen_height().max(1.0);
            let proj = Mat4::perspective_rh_gl(fovy, aspect, 0.05, 1500.0);
            let view = Mat4::look_to_rh(Vec3::ZERO, dir, Vec3::Y);
            let vp = proj * view;
            set_default_camera();
            draw_sky(g, &vp, top, hor);
            {
                let mut gl = unsafe { get_internal_gl() };
                gl.flush();
                let r = app.settings.render_dist as f32 * CS as f32;
                let (fog_color, fog_range) = if g.player.eye_in_water {
                    ([0.1, 0.25, 0.5, 1.0], [1.0, 26.0])
                } else if app.settings.fog {
                    ([hor.r, hor.g, hor.b, 1.0], [r * 0.5, r * 0.98])
                } else {
                    ([hor.r, hor.g, hor.b, 1.0], [1e5, 1e5 + 1.0])
                };
                let frame = Frame {
                    cam: eye,
                    view_proj: vp,
                    fog_color,
                    fog_range,
                    daylight: g.daylight(),
                    bright: app.settings.brightness,
                };
                let clouds = if app.settings.clouds { Some((g.dim, app.time)) } else { None };
                app.renderer.draw(gl.quad_context, &app.loader, &frame, clouds);
            }
            // существа, обводка, трещины
            set_camera(&Camera3D { position: eye, target: eye + dir, up: Vec3::Y, fovy, z_near: 0.05, z_far: 1500.0, ..Default::default() });
            g.mobs.draw(g.world(), g.daylight(), app.time);
            if app.screen == Screen::Playing && !app.hide_hud {
                if let Some((hit, _, _)) = g.world().raycast(g.player.eye(), dir, REACH) {
                    let c = hit.as_vec3() + Vec3::splat(0.5);
                    draw_cube_wires(c, Vec3::splat(1.006), Color::new(0.02, 0.02, 0.02, 0.85));
                    if g.break_target == Some(hit) && g.break_progress > 0.0 {
                        let stage = ((g.break_progress * 4.0) as usize).min(3);
                        draw_cube(c, Vec3::splat(1.01), Some(&app.cracks[stage]), WHITE);
                    }
                }
            }
            set_default_camera();
            if g.player.eye_in_water {
                draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.1, 0.3, 0.6, 0.35));
            }
            if g.player.hurt_t > 0.0 {
                draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.8, 0.0, 0.0, g.player.hurt_t * 0.6));
            }
            if !app.hide_hud && matches!(app.screen, Screen::Playing | Screen::Paused | Screen::Inventory) {
                draw_hand(&app, g);
                draw_hud(&mut app);
            }
        } else {
            clear_background(Color::new(0.08, 0.09, 0.11, 1.0));
        }

        // --- экраны поверх ---
        let next = match app.screen {
            Screen::Title => draw_title(&mut app),
            Screen::NewWorld => draw_new_world(&mut app),
            Screen::Settings => settings_screen(&mut app),
            Screen::Paused => pause_screen(&mut app),
            Screen::Dead => dead_screen(&mut app),
            Screen::Inventory => inventory_screen(&mut app),
            Screen::Loading => {
                let (w, h) = (screen_width(), screen_height());
                draw_rectangle(0.0, 0.0, w, h, Color::new(0.07, 0.08, 0.1, 1.0));
                let label = if app.game.as_ref().is_some_and(|g| g.dim == Dim::Heaven) { "Путь в Рай…" } else { "Строим мир…" };
                app.ui.text_center(label, w / 2.0, h / 2.0 - 10.0, 32.0, WHITE);
                let n = app.loader.gpu.len() as f32;
                let t = (n / 9.0).min(1.0);
                let bw = 360.0 * app.ui.s;
                draw_rectangle((w - bw) / 2.0, h / 2.0 + 20.0, bw, 8.0, Color::new(1.0, 1.0, 1.0, 0.15));
                draw_rectangle((w - bw) / 2.0, h / 2.0 + 20.0, bw * t, 8.0, ACCENT);
                None
            }
            Screen::Playing => None,
        };
        if let Some(n) = next {
            app.screen = n;
        }
        app.ui.end();

        if let Some(path) = &shot {
            let ready = match shot_screen.as_str() {
                "title" => true,
                _ => app.game.is_some() && app.screen != Screen::Loading,
            };
            if ready {
                shot_frames += 1;
                if shot_screen == "inventory" && shot_frames == 2 {
                    app.screen = Screen::Inventory;
                    app.inv_tab = std::env::var("RK_TAB").ok().and_then(|t| t.parse().ok()).unwrap_or(0);
                    if let Some(g) = app.game.as_mut() {
                        for it in items::catalog(7).into_iter().skip(12).take(4) {
                            g.player.equip(it);
                        }
                    }
                }
                if shot_screen == "settings" && shot_frames == 2 {
                    app.settings_back = Screen::Playing;
                    app.screen = Screen::Settings;
                }
                let wait: i32 = std::env::var("RK_WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
                if shot_frames >= wait {
                    get_screen_data().export_png(path);
                    std::process::exit(0);
                }
            }
        }
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_from_text_is_stable() {
        assert_eq!(hash_seed("123"), 123);
        assert_eq!(hash_seed("рай"), hash_seed("рай"));
        assert_ne!(hash_seed("a"), hash_seed("b"));
    }

    #[test]
    fn heaven_portal_fits_in_loaded_world() {
        let mut w = World::new(Dim::Heaven, 3);
        for cz in -2..=2 {
            for cx in -2..=2 {
                w.chunks.insert((cx, cz), std::sync::Arc::new(worldgen::generate(Dim::Heaven, 3, cx, cz)));
            }
        }
        let p = build_portal(&mut w, -1, 0).expect("портал");
        assert_eq!(w.get(p.x + 1, p.y, p.z), PORTAL);
        assert_eq!(w.get(p.x, p.y, p.z), PORTAL_FRAME);
    }
}
