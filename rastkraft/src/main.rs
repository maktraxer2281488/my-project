#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Расткрафт — воксельная песочница на Rust.

mod blocks;
mod crafting;
mod ench;
mod entities;
mod icons;
mod inventory;
mod items;
mod mesher;
mod mobdefs;
mod mobs;
mod model;
mod ach;
mod audio;
mod music;
mod map;
mod trade;
mod mech;
mod net;
mod opt;
mod player;
mod render;
mod save;
mod settings;
mod shapes;
mod skins;
mod structures;
mod touch;
mod packs;
mod ui;
mod workers;
mod world;
mod worldgen;

use blocks::*;
use entities::Entities;
use inventory::Stack;
use items::Item;
use macroquad::prelude::*;
use mobs::Mobs;
use net::{Msg, Net, PState, Remote, Remotes};
use player::{MAX_HEALTH, MAX_HUNGER, Player, break_time};
use render::{Frame, Loader, Renderer};
use settings::Settings;
use std::collections::HashSet;
use ui::{ACCENT, Ui};
use workers::Pool;
use world::{CS, Dim, World, chunk_of};

const DAY_SECONDS: f32 = 720.0;
const AUTOSAVE: f32 = 120.0;

/// VSync, с которым запущено окно (меняется только перезапуском).
static VSYNC_AT_START: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

fn window_conf() -> macroquad::conf::Conf {
    // VSync выключен по умолчанию: со слабой видеокартой он сбрасывает
    // 40–55 кадров ровно до 30 (кадр ждёт следующего обновления экрана)
    let vsync = *VSYNC_AT_START.get_or_init(|| Settings::load().vsync);
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Расткрафт".to_owned(),
            window_width: std::env::var("RK_W").ok().and_then(|v| v.parse().ok()).unwrap_or(1280),
            window_height: std::env::var("RK_H").ok().and_then(|v| v.parse().ok()).unwrap_or(720),
            window_resizable: true,
            fullscreen: cfg!(target_os = "android"),
            high_dpi: cfg!(target_os = "android"),
            platform: macroquad::miniquad::conf::Platform { swap_interval: Some(vsync as i32), ..Default::default() },
            ..Default::default()
        },
        draw_call_vertex_capacity: 40_000,
        draw_call_index_capacity: 120_000,
        ..Default::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Title,
    Worlds,
    NewWorld,
    Multiplayer,
    Loading,
    Playing,
    Paused,
    Settings,
    Inventory,
    Dead,
    Chat,
    RecipeBook,
    Map,
    Trade,
    Enchant,
    Achievements,
    Look,
    Textures,
    Relay,
    Controls,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    None,
    Name,
    Seed,
    Ip,
    Nick,
}

struct Game {
    worlds: [World; 3],
    /// Где в верхнем мире стоит портал в Бездну.
    abyss_home: Option<Vec3>,
    last_dim: Dim,
    /// Механизмы и жидкости.
    mech: mech::Mech,
    /// Посаженные игроком посевы: (место, секунд до следующей стадии).
    crops: Vec<(IVec3, f32)>,
    /// Погода: 0 ясно, 1 дождь/снег/буря, 2 гроза; плавная сила и вспышка молнии.
    weather: u8,
    weather_t: f32,
    rain_k: f32,
    flash_t: f32,
    thunder_t: f32,
    /// Сон в кровати: затемнение до утра.
    sleep_t: f32,
    pending_edits: Vec<(IVec3, u16)>,
    /// Натяжение лука / зарядка арбалета (0..1) и заряжен ли арбалет.
    draw_t: f32,
    cross_loaded: bool,
    /// Достижения (биты), побеждённые боссы (биты), всплывающие окна, звуки из игры.
    ach: u32,
    bosses: u8,
    ach_pop: Vec<(u8, f32)>,
    sfx_q: Vec<(music::Sfx, f32)>,
    ach_check_t: f32,
    ach_pending_archer: bool,
    ach_later: Vec<u8>,
    /// Метки на карте: (измерение, место, название).
    markers: Vec<(u8, Vec2, String)>,
    /// На какой лодке/вагонетке едет игрок.
    riding: Option<usize>,
    dim: Dim,
    seed: u32,
    name: String,
    player: Player,
    mobs: Mobs,
    ents: Entities,
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
    attack_max: f32,
    crit_t: f32,
    swing_t: f32,
    /// Прогресс взмаха руки 0..1 (1 — покой).
    swing_p: f32,
    /// Опущенность руки при смене предмета.
    equip: f32,
    last_held: Item,
    /// Направление взгляда «с запаздыванием» — для инерции руки.
    lag_look: Vec2,
    portal_cd: f32,
    label_t: f32,
    toast: Option<(String, f32)>,
    opened: HashSet<[i32; 3]>,
    spawned: HashSet<[i32; 3]>,
    autosave_t: f32,
    net: Option<Net>,
    remotes: Remotes,
    chat: Vec<(String, f32)>,
    state_t: f32,
    mob_t: f32,
    third_person: bool,
    rng: u32,
}

impl Game {
    fn new(seed: u32, creative: bool, name: &str) -> Self {
        let (sx, sz) = worldgen::find_spawn(seed);
        let spawn = vec3(sx as f32 + 0.5, 100.0, sz as f32 + 0.5);
        let mut player = Player::new(spawn);
        player.starter_kit(creative);
        Game {
            worlds: [World::new(Dim::Overworld, seed), World::new(Dim::Heaven, seed.wrapping_add(17)), World::new(Dim::Abyss, seed.wrapping_add(33))],
            abyss_home: None,
            last_dim: Dim::Overworld,
            mech: mech::Mech::default(),
            crops: Vec::new(),
            weather: 0,
            weather_t: 240.0,
            rain_k: 0.0,
            flash_t: 0.0,
            thunder_t: 10.0,
            sleep_t: 0.0,
            pending_edits: Vec::new(),
            draw_t: 0.0,
            cross_loaded: false,
            ach: 0,
            bosses: 0,
            ach_pop: Vec::new(),
            sfx_q: Vec::new(),
            ach_check_t: 0.0,
            ach_pending_archer: false,
            ach_later: Vec::new(),
            markers: Vec::new(),
            riding: None,
            dim: Dim::Overworld,
            seed,
            name: name.to_string(),
            player,
            mobs: Mobs::new(),
            ents: Entities::default(),
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
            attack_max: 0.25,
            crit_t: 0.0,
            swing_t: 0.0,
            swing_p: 1.0,
            equip: 0.0,
            last_held: Item::Empty,
            lag_look: Vec2::ZERO,
            portal_cd: 2.0,
            label_t: 2.0,
            toast: Some(("Добро пожаловать в Расткрафт!".into(), 4.0)),
            opened: HashSet::new(),
            spawned: HashSet::new(),
            autosave_t: AUTOSAVE,
            net: None,
            remotes: Remotes::new(),
            chat: Vec::new(),
            state_t: 0.0,
            mob_t: 0.0,
            third_person: false,
            rng: seed | 1,
        }
    }

    fn world(&self) -> &World {
        &self.worlds[self.dim as usize]
    }

    fn world_mut(&mut self) -> &mut World {
        &mut self.worlds[self.dim as usize]
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0xffffff) as f32 / 16_777_215.0
    }

    fn daylight(&self) -> f32 {
        if self.dim == Dim::Heaven {
            return 1.0;
        }
        let sunh = (self.time * std::f32::consts::TAU).sin();
        let d = ((sunh + 0.2) / 0.5).clamp(0.0, 1.0);
        (0.1 + 0.9 * d * d * (3.0 - 2.0 * d)) * (1.0 - 0.32 * self.rain_k)
    }

    fn sun_dir(&self) -> Vec3 {
        let a = self.time * std::f32::consts::TAU;
        vec3(a.cos(), a.sin(), 0.25).normalize()
    }

    fn say(&mut self, text: String) {
        self.chat.push((text, 10.0));
        if self.chat.len() > 8 {
            self.chat.remove(0);
        }
    }

    /// Открыть достижение (один раз, с фанфарами).
    fn unlock(&mut self, id: u8) {
        if self.creative || self.ach & (1 << id) != 0 {
            return;
        }
        self.ach |= 1 << id;
        self.ach_pop.push((id, 5.0));
        self.sfx_q.push((music::Sfx::Crit, 0.8));
    }

    /// Проверки по состоянию (раз в секунду).
    fn check_achievements(&mut self) {
        let inv = &self.player.inv;
        let has = |it: Item| inv.count(it) > 0;
        let mut got: Vec<u8> = Vec::new();
        if inv.slots.iter().any(|s| matches!(s.item, Item::Tool(..))) {
            got.push(ach::FIRST_TOOL);
        }
        if has(Item::Thing(items::IRON_INGOT)) {
            got.push(ach::IRON);
        }
        if has(Item::Thing(items::RUBY)) || has(Item::Thing(items::SAPPHIRE)) || has(Item::Thing(items::MALACHITE)) {
            got.push(ach::GEMS);
        }
        if has(Item::Thing(items::CRYSTAL)) {
            got.push(ach::CRYSTAL);
        }
        for id in got {
            self.unlock(id);
        }
        let (lvl, _) = self.player.level();
        if lvl >= 10 {
            self.unlock(ach::LVL10);
        }
        if lvl >= 30 {
            self.unlock(ach::LVL30);
        }
        match self.dim {
            Dim::Heaven => self.unlock(ach::HEAVEN),
            Dim::Abyss => self.unlock(ach::ABYSS),
            _ => {}
        }
        let p = self.player.body.pos;
        if self.mobs.list.iter().any(|m| m.baby > 178.0 && (m.body.pos - p).length() < 24.0) {
            self.unlock(ach::BREED);
        }
    }

    fn is_client(&self) -> bool {
        self.net.as_ref().is_some_and(|n| !n.host)
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
    pending_net: Option<Net>,
    seed_input: String,
    name_input: String,
    ip_input: String,
    focus: Field,
    new_creative: bool,
    inv_tab: usize,
    inv_scroll: f32,
    craft_scroll: f32,
    craft_all: bool,
    cursor: Stack,
    grabbed: bool,
    last_mouse: Vec2,
    show_debug: bool,
    hide_hud: bool,
    cracks: Vec<Texture2D>,
    time: f32,
    fps: f32,
    status: Option<(String, f32)>,
    worlds_list: Vec<save::WorldInfo>,
    /// Экран открыт в этом кадре — не закрывать его той же клавишей.
    just_opened: bool,
    finder: Option<net::Finder>,
    chat_input: String,
    audio: audio::Audio,
    map: Option<map::MapView>,
    minimap: map::MiniMap,
    /// Раскладка сенсорных кнопок.
    touch_layout: touch::Layout,
    /// Редактор раскладки: какая кнопка выбрана и перетаскивается ли.
    touch_edit: Option<touch::Act>,
    keyboard_shown: bool,
    /// Открытая торговля: (вид жителя, его номер).
    trade: Option<(usize, u32)>,
    /// Разрешение текстур, с которым игра запущена.
    tex_res_active: u32,
    /// Стиль текстур и пак, с которыми запущена игра.
    style_active: u8,
    pack_active: String,
    /// Найденные текстур-паки (для экрана «Текстуры и небо»).
    pack_list: Vec<String>,
    /// Звуки, накопленные за кадр (проигрываются после обновления мира).
    sfx: Vec<(music::Sfx, f32)>,
    export_rx: Option<std::sync::mpsc::Receiver<String>>,
    book_tab: usize,
    book_scroll: f32,
    book_sel: Option<usize>,
    book_back: Vec<usize>,
    book_search: String,
    /// Текстура пониженного разрешения для 3D (масштаб рендера).
    rt: Option<(RenderTarget, u32, u32)>,
    /// Действующая дальность (при авто-настройке может быть меньше выбранной).
    eff_dist: i32,
    auto_t: f32,
}

fn set_grab(app: &mut App, grab: bool) {
    if app.grabbed != grab {
        app.grabbed = grab;
        set_cursor_grab(grab);
        show_mouse(!grab);
    }
}

fn dim_code(d: Dim) -> u8 {
    match d {
        Dim::Overworld => 0,
        Dim::Heaven => 1,
        Dim::Abyss => 2,
    }
}

fn dim_from(c: u8) -> Dim {
    match c {
        1 => Dim::Heaven,
        2 => Dim::Abyss,
        _ => Dim::Overworld,
    }
}

/// Изменение блока: мир, перестройка геометрии и (по сети) другим игрокам.
fn edit_block(app: &mut App, dim: Dim, p: IVec3, b: u16, share: bool) {
    let smooth = app.settings.smooth_light;
    let Some(g) = app.game.as_mut() else { return };
    let w = &mut g.worlds[dim as usize];
    if !w.set(p.x, p.y, p.z, b) {
        return;
    }
    if share && std::env::var("RK_NETLOG").is_ok() {
        eprintln!("NETLOG local_edit at=({},{},{}) block={b} net={}", p.x, p.y, p.z, g.net.is_some());
    }
    let (cx, cz) = chunk_of(p.x, p.z);
    for dz in -1..=1 {
        for dx in -1..=1 {
            if (dx, dz) != (0, 0) && w.chunks.contains_key(&(cx + dx, cz + dz)) {
                *w.versions.entry((cx + dx, cz + dz)).or_insert(0) += 1;
            }
        }
    }
    if share {
        if let Some(n) = &g.net {
            n.broadcast(&Msg::Edit { dim: dim_code(dim), x: p.x, y: p.y, z: p.z, b }, None);
        }
    }
    if dim == g.dim {
        let g = app.game.as_ref().unwrap();
        let w = &g.worlds[dim as usize];
        // сначала сам чанк, затем соседи — всё в фоновых потоках вне очереди
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1), (0, 0)] {
            if w.chunks.contains_key(&(cx + dx, cz + dz)) && app.loader.has_mesh(cx + dx, cz + dz) {
                app.loader.remesh_soon(&mut app.pool, w, cx + dx, cz + dz, smooth);
            }
        }
    }
}

fn switch_dimension(app: &mut App, to: Dim) {
    let ctx = unsafe { get_internal_gl() }.quad_context;
    app.loader.clear(ctx);
    let g = app.game.as_mut().unwrap();
    g.last_dim = g.dim;
    g.dim = to;
    if std::env::var("RK_NETLOG").is_ok() {
        eprintln!("NETLOG switch_dimension to {:?}", to);
    }
    g.mobs.list.clear();
    g.ents = Entities::default();
    g.portal_cd = 4.0;
    g.break_target = None;
    match to {
        Dim::Heaven => {
            let (top, _) = worldgen::heaven_island(g.worlds[1].seed, 0, 0).unwrap_or((80, 70));
            g.player.body.pos = vec3(1.5, top as f32 + 1.2, 3.5);
            if !g.pending_portals.iter().any(|p| p.0 == Dim::Heaven) && g.worlds[1].chunks.is_empty() && g.worlds[1].stored.is_empty() {
                g.pending_portals.push((Dim::Heaven, -1, 0));
            }
            g.toast = Some(("Вы попали в Рай".into(), 4.0));
        }
        Dim::Overworld => {
            let from_abyss = g.abyss_home.is_some() && g.last_dim == Dim::Abyss;
            let p = if from_abyss { g.abyss_home.unwrap() + vec3(0.0, 0.1, 1.6) } else { g.portal_home.map(|p| p.as_vec3() + vec3(1.5, 0.1, 2.5)).unwrap_or(g.spawn) };
            g.player.body.pos = p;
            g.toast = Some(("Верхний мир".into(), 3.0));
        }
        Dim::Abyss => {
            g.player.body.pos = vec3(0.5, 60.1, 0.5);
            g.toast = Some(("Вы спустились в Бездну. Портал назад — рядом".into(), 4.0));
        }
    }
    g.player.body.vel = Vec3::ZERO;
    g.spawn_pending = false;
    app.screen = Screen::Loading;
}

fn place_pending(app: &mut App) {
    let g = app.game.as_mut().unwrap();
    if g.is_client() {
        g.pending_portals.clear();
        return;
    }
    let dim = g.dim;
    let mut i = 0;
    let mut built = Vec::new();
    while i < g.pending_portals.len() {
        let (d, x, z) = g.pending_portals[i];
        if d == dim {
            if let Some(p) = build_portal(&mut g.worlds[dim as usize], x, z) {
                if d == Dim::Overworld {
                    g.portal_home = Some(p);
                }
                g.pending_portals.remove(i);
                built.push(p);
                continue;
            }
        }
        i += 1;
    }
    // сообщим клиентам о новом портале (отправим все блоки рамки)
    if let Some(n) = &g.net {
        for p in built {
            for dz in -2..=2 {
                for dx in -1..=4 {
                    for dy in -1..6 {
                        let (x, y, z) = (p.x + dx, p.y + dy, p.z + dz);
                        let b = g.worlds[dim as usize].get(x, y, z);
                        n.broadcast(&Msg::Edit { dim: dim_code(dim), x, y, z, b }, None);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Сохранение и загрузка
// ---------------------------------------------------------------------------

fn save_game(app: &mut App) -> bool {
    let Some(g) = app.game.as_ref() else { return false };
    if g.is_client() || g.name.is_empty() {
        return false;
    }
    let dir = save::world_dir(&g.name);
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let mut kv = save::Kv::default();
    let p = &g.player;
    let v3 = |v: Vec3| format!("{},{},{}", v.x, v.y, v.z);
    kv.set("name", &g.name);
    kv.set("seed", g.seed);
    kv.set("creative", g.creative as u8);
    kv.set("time", g.time);
    kv.set("dim", dim_code(g.dim));
    if let Some(h) = g.abyss_home {
        kv.set("abyss_home", format!("{},{},{}", h.x, h.y, h.z));
    }
    kv.set("pos", v3(p.body.pos));
    kv.set("look", format!("{},{}", p.yaw, p.pitch));
    kv.set("health", p.health);
    kv.set("hunger", format!("{},{}", p.hunger, p.saturation));
    kv.set("spawn", v3(g.spawn));
    if let Some(pp) = g.portal_home {
        kv.set("portal", format!("{},{},{}", pp.x, pp.y, pp.z));
    }
    kv.set("pending", g.pending_portals.iter().map(|(d, x, z)| format!("{},{},{}", dim_code(*d), x, z)).collect::<Vec<_>>().join(","));
    kv.set("inv", p.inv.slots.iter().map(|s| format!("{},{}", s.item.code(), s.count)).collect::<Vec<_>>().join(","));
    kv.set("armor", p.armor.iter().map(|a| a.code().to_string()).collect::<Vec<_>>().join(","));
    kv.set("selected", p.selected);
    kv.set("book", 1);
    kv.set("ach", g.ach);
    kv.set("markers", g.markers.iter().map(|(d, p, n)| format!("{d}|{:.0}|{:.0}|{n}", p.x, p.y)).collect::<Vec<_>>().join(";"));
    kv.set("bosses", g.bosses);
    kv.set("xp", p.xp);
    kv.set("inv_ench", p.inv.slots.iter().map(|s| s.ench.to_string()).collect::<Vec<_>>().join(","));
    kv.set("armor_ench", p.armor_ench.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(","));
    kv.set("crops", g.crops.iter().map(|(p, t)| format!("{},{},{},{:.0}", p.x, p.y, p.z, t)).collect::<Vec<_>>().join(","));
    let pets: Vec<String> = g.mobs.list.iter().filter(|m| m.owner == Some(mobs::OWNER_LOCAL) && m.alive()).map(|m| format!("{},{:.1},{:.1},{:.1},{}", m.def, m.body.pos.x, m.body.pos.y, m.body.pos.z, m.sitting as u8)).collect();
    kv.set("pets", pets.join(";"));
    kv.set("weather", g.weather);
    let set3 = |s: &HashSet<[i32; 3]>| s.iter().map(|v| format!("{},{},{}", v[0], v[1], v[2])).collect::<Vec<_>>().join(",");
    kv.set("opened", set3(&g.opened));
    kv.set("spawned", set3(&g.spawned));
    if std::fs::write(dir.join("level.txt"), kv.to_text()).is_err() {
        return false;
    }
    for (i, tag) in ["ow", "hv", "ab"].iter().enumerate() {
        let w = &g.worlds[i];
        let list: Vec<((i32, i32), &world::Chunk)> = w.chunks.iter().filter(|(k, _)| w.versions.contains_key(k)).map(|(k, c)| (*k, c.as_ref())).collect();
        if save::save_chunks(&dir, tag, &list).is_err() {
            return false;
        }
    }
    true
}

fn load_game(app: &mut App, name: &str) -> bool {
    let dir = save::world_dir(name);
    let Ok(text) = std::fs::read_to_string(dir.join("level.txt")) else { return false };
    let kv = save::Kv::parse(&text);
    let ctx = unsafe { get_internal_gl() }.quad_context;
    app.loader.clear(ctx);
    let mut g = Game::new(kv.u("seed", 1), kv.get("creative") == Some("1"), name);
    g.name = kv.get("name").unwrap_or(name).to_string();
    g.time = kv.f("time", 0.1);
    g.dim = dim_from(kv.u("dim", 0) as u8);
    g.abyss_home = { let v = kv.floats("abyss_home"); (v.len() == 3).then(|| vec3(v[0], v[1], v[2])) };
    let v3 = |v: Vec<f32>| (v.len() == 3).then(|| vec3(v[0], v[1], v[2]));
    if let Some(p) = v3(kv.floats("pos")) {
        g.player.body.pos = p;
    }
    let look = kv.floats("look");
    if look.len() == 2 {
        g.player.yaw = look[0];
        g.player.pitch = look[1];
    }
    g.player.health = kv.f("health", MAX_HEALTH);
    let h = kv.floats("hunger");
    if h.len() == 2 {
        g.player.hunger = h[0];
        g.player.saturation = h[1];
    }
    if let Some(s) = v3(kv.floats("spawn")) {
        g.spawn = s;
    }
    let pp = kv.ints("portal");
    g.portal_home = (pp.len() == 3).then(|| ivec3(pp[0] as i32, pp[1] as i32, pp[2] as i32));
    let pend = kv.ints("pending");
    g.pending_portals = pend.chunks_exact(3).map(|c| (dim_from(c[0] as u8), c[1] as i32, c[2] as i32)).collect();
    let inv = kv.ints("inv");
    g.player.inv = inventory::Inventory::new();
    for (i, c) in inv.chunks_exact(2).enumerate().take(inventory::SLOTS) {
        g.player.inv.slots[i] = Stack::new(Item::from_code(c[0] as u32), c[1] as u16);
    }
    for (i, c) in kv.ints("armor").iter().enumerate().take(4) {
        g.player.armor[i] = Item::from_code(*c as u32);
    }
    g.player.selected = (kv.u("selected", 0) as usize).min(8);
    g.player.xp = kv.u("xp", 0) as u32;
    g.ach = kv.u("ach", 0) as u32;
    if let Some(ms) = kv.get("markers") {
        for e in ms.split(';') {
            let f: Vec<&str> = e.splitn(4, '|').collect();
            if f.len() == 4 {
                if let (Ok(d), Ok(x), Ok(z)) = (f[0].parse::<u8>(), f[1].parse::<f32>(), f[2].parse::<f32>()) {
                    g.markers.push((d, vec2(x, z), f[3].to_string()));
                }
            }
        }
    }
    g.bosses = kv.u("bosses", 0) as u8;
    for (i, e) in kv.ints("inv_ench").iter().enumerate().take(inventory::SLOTS) {
        g.player.inv.slots[i].ench = *e as u8;
    }
    for (i, e) in kv.ints("armor_ench").iter().enumerate().take(4) {
        g.player.armor_ench[i] = *e as u8;
    }
    g.crops = kv.ints("crops").chunks_exact(4).map(|c| (ivec3(c[0] as i32, c[1] as i32, c[2] as i32), c[3] as f32)).collect();
    if let Some(p) = kv.get("pets") {
        for e in p.split(';') {
            let v: Vec<f32> = e.split(',').filter_map(|x| x.parse().ok()).collect();
            if v.len() == 5 && (v[0] as usize) < mobdefs::MOBS.len() {
                let mut m = mobs::Mob::new(v[0] as usize, vec3(v[1], v[2], v[3]));
                m.owner = Some(mobs::OWNER_LOCAL);
                m.sitting = v[4] > 0.5;
                g.mobs.list.push(m);
            }
        }
    }
    g.weather = kv.u("weather", 0).min(2) as u8;
    // миры до версии 0.10: выдать книгу рецептов один раз
    if !g.creative && kv.get("book").is_none() && g.player.inv.count(Item::Thing(items::RECIPE_BOOK)) == 0 {
        g.player.inv.add(Item::Thing(items::RECIPE_BOOK), 1);
    }
    let to_set = |v: Vec<i64>| v.chunks_exact(3).map(|c| [c[0] as i32, c[1] as i32, c[2] as i32]).collect::<HashSet<_>>();
    g.opened = to_set(kv.ints("opened"));
    g.spawned = to_set(kv.ints("spawned"));
    g.worlds[0].stored = save::load_chunks(&dir, "ow");
    g.worlds[1].stored = save::load_chunks(&dir, "hv");
    g.worlds[2].stored = save::load_chunks(&dir, "ab");
    g.spawn_pending = false;
    g.toast = Some((format!("Мир «{}» загружен", g.name), 3.0));
    app.game = Some(g);
    app.screen = Screen::Loading;
    true
}

fn start_world(app: &mut App, seed: u32, creative: bool, name: &str) {
    let ctx = unsafe { get_internal_gl() }.quad_context;
    app.loader.clear(ctx);
    let mut name = save::sanitize(name);
    // не перезаписываем существующий мир
    let base = name.clone();
    let mut k = 2;
    while save::world_dir(&name).join("level.txt").exists() {
        name = format!("{base} {k}");
        k += 1;
    }
    app.game = Some(Game::new(seed, creative, &name));
}

// ---------------------------------------------------------------------------
// Сеть
// ---------------------------------------------------------------------------

fn welcome_for(g: &Game, id: u8) -> Msg {
    let mut chunks = Vec::new();
    for (i, w) in g.worlds.iter().enumerate() {
        for (k, c) in w.chunks.iter() {
            if w.versions.contains_key(k) {
                chunks.push((i as u8, k.0, k.1, save::rle_encode(&c.blocks)));
            }
        }
        for (k, c) in w.stored.iter() {
            chunks.push((i as u8, k.0, k.1, save::rle_encode(&c.blocks)));
        }
    }
    Msg::Welcome { id, seed: g.seed, creative: g.creative, time: g.time, chunks }
}

fn process_net(app: &mut App) {
    // ожидание ответа сервера после подключения
    if let Some(n) = app.pending_net.take() {
        let mut keep = Some(n);
        for (_, m) in keep.as_ref().unwrap().poll() {
            match m {
                Some(Msg::Welcome { id, seed, creative, time, chunks }) => {
                    let mut net = keep.take().unwrap();
                    net.my_id = id;
                    let ctx = unsafe { get_internal_gl() }.quad_context;
                    app.loader.clear(ctx);
                    let mut g = Game::new(seed, creative, "");
                    g.time = time;
                    for (d, cx, cz, data) in chunks {
                        if let Some(blocks) = save::rle_decode(&data) {
                            g.worlds[(d as usize).min(1)].stored.insert((cx, cz), world::Chunk { blocks, spawns: Vec::new() });
                        }
                    }
                    g.pending_portals.clear();
                    g.toast = Some((net.info.clone(), 4.0));
                    g.net = Some(net);
                    app.game = Some(g);
                    app.screen = Screen::Loading;
                    break;
                }
                None => {
                    app.status = Some(("Сервер закрыл соединение".into(), 4.0));
                    keep = None;
                    app.screen = Screen::Multiplayer;
                    break;
                }
                _ => {}
            }
        }
        app.pending_net = keep;
        return;
    }

    let Some(g) = app.game.as_mut() else { return };
    let Some(n) = g.net.take() else { return };
    let msgs = n.poll();
    let mut edits: Vec<(Dim, IVec3, u16)> = Vec::new();
    let mut lost = false;
    for (from, m) in msgs {
        match m {
            Some(Msg::Hello { name }) if n.host => {
                n.send_to(from, &welcome_for(g, from));
                let t = format!("{name} присоединился к игре");
                n.broadcast(&Msg::Chat { text: t.clone() }, Some(from));
                g.say(t);
            }
            Some(Msg::State(st)) => {
                if st.id == n.my_id {
                    continue;
                }
                if n.host {
                    n.broadcast(&Msg::State(st.clone()), Some(from));
                }
                let e = g.remotes.entry(st.id).or_insert(Remote { shown: st.pos, st: st.clone(), hurt_t: 0.0 });
                e.st = st;
            }
            Some(Msg::Edit { dim, x, y, z, b }) => {
                if std::env::var("RK_NETLOG").is_ok() {
                    eprintln!("NETLOG edit dim={dim} at=({x},{y},{z}) block={b}");
                }
                if n.host {
                    n.broadcast(&Msg::Edit { dim, x, y, z, b }, Some(from));
                }
                edits.push((dim_from(dim), ivec3(x, y, z), b));
            }
            Some(Msg::Hit { target, from: attacker, dmg, knock }) => {
                if target == n.my_id && attacker == net::FROM_MOB {
                    if !g.creative && g.player.health > 0.0 {
                        g.player.hurt(dmg, Vec3::from(knock), false);
                    }
                } else if target == n.my_id {
                    if app.settings.pvp && !g.creative && g.player.health > 0.0 {
                        g.player.hurt(dmg, Vec3::from(knock), false);
                        if g.player.health <= 0.0 {
                            let killer = g.remotes.get(&attacker).map_or("?".to_string(), |r| r.st.name.clone());
                            let t = format!("{} пал от руки {}", n.name, killer);
                            n.broadcast(&Msg::Chat { text: t.clone() }, None);
                            g.say(t);
                        }
                    }
                } else if n.host {
                    n.send_to(target, &Msg::Hit { target, from: attacker, dmg, knock });
                }
                if let Some(r) = g.remotes.get_mut(&target) {
                    r.hurt_t = 0.3;
                }
            }
            Some(Msg::Chat { text }) => {
                if std::env::var("RK_NETLOG").is_ok() {
                    eprintln!("NETLOG chat {text}");
                }
                if n.host {
                    n.broadcast(&Msg::Chat { text: text.clone() }, Some(from));
                }
                g.say(text);
            }
            Some(Msg::MobHit { uid, dmg }) if n.host => {
                let from_pos = g.remotes.get(&from).map_or(Vec3::ZERO, |r| Vec3::from(r.st.pos));
                if let Some(m) = g.mobs.list.iter_mut().find(|m| m.uid == uid) {
                    m.damage(dmg, from_pos);
                    if !m.alive() {
                        m.killer = Some(from);
                    }
                }
            }
            Some(Msg::Mobs { time, weather, list }) if !n.host => {
                g.time = time;
                g.weather = weather;
                let host_dim = g.remotes.get(&0).map(|r| r.st.dim);
                if host_dim == Some(dim_code(g.dim)) {
                    g.mobs.apply_snapshot(&list);
                    if std::env::var("RK_NETLOG").is_ok() {
                        let boss = g.mobs.list.iter().find(|m| m.is_boss()).map(|m| (m.d().name, m.health_frac()));
                        eprintln!("NETLOG mobs_from_host={} host_dim={:?} boss={:?}", list.len(), host_dim, boss);
                    }
                }
            }
            Some(Msg::Give { item, count }) => {
                let it = Item::from_code(item);
                let left = g.player.inv.add(it, count);
                if left > 0 {
                    let p = g.player.body.pos + vec3(0.0, 1.0, 0.0);
                    g.ents.spawn_drop(it, left, p, 0.4);
                }
            }
            Some(Msg::Leave { id }) => {
                if let Some(r) = g.remotes.remove(&id) {
                    g.say(format!("{} покинул игру", r.st.name));
                }
            }
            None => {
                if n.host {
                    if let Some(r) = g.remotes.remove(&from) {
                        g.say(format!("{} покинул игру", r.st.name));
                    }
                    n.broadcast(&Msg::Leave { id: from }, None);
                } else {
                    lost = true;
                }
            }
            _ => {}
        }
    }
    // хост: существа и время для каждого клиента ~10 раз в секунду
    if n.host {
        g.mob_t -= get_frame_time();
        if g.mob_t <= 0.0 {
            g.mob_t = 0.1;
            for (id, r) in g.remotes.iter() {
                let list = if r.st.dim == dim_code(g.dim) { g.mobs.snapshot(Vec3::from(r.st.pos), 96.0) } else { Vec::new() };
                n.send_to(*id, &Msg::Mobs { time: g.time, weather: g.weather, list });
            }
            if let Some(b) = &n.beacon {
                let world = if g.name.is_empty() { "мир".to_string() } else { g.name.clone() };
                *b.lock().unwrap() = format!("{} · {} · игроков: {}", n.name, world, g.remotes.len() + 1);
            }
        }
    }
    // своё состояние ~20 раз в секунду
    g.state_t -= get_frame_time();
    if g.state_t <= 0.0 && (n.host || n.my_id != 255) {
        g.state_t = 0.05;
        let p = &g.player;
        let st = PState {
            id: n.my_id,
            name: n.name.clone(),
            dim: dim_code(g.dim),
            pos: p.body.pos.to_array(),
            yaw: p.yaw,
            pitch: p.pitch,
            held: p.held().code(),
            armor: [p.armor[0].code(), p.armor[1].code(), p.armor[2].code(), p.armor[3].code()],
            health: p.health,
            walk: p.walk_phase,
            speed: vec2(p.body.vel.x, p.body.vel.z).length() / 4.0,
            swing: g.swing_t,
            look: u32::from_le_bytes(app.settings.look),
        };
        n.broadcast(&Msg::State(st), None);
    }
    if lost {
        g.say("Связь с сервером потеряна".into());
        g.toast = Some(("Связь с сервером потеряна".into(), 5.0));
    } else {
        g.net = Some(n);
    }
    let host_side = app.game.as_ref().is_some_and(|g| g.net.as_ref().is_some_and(|n| n.host));
    for (d, p, b) in edits {
        let old = app.game.as_ref().map_or(AIR, |g| g.worlds[d as usize].get(p.x, p.y, p.z));
        edit_block(app, d, p, b, false);
        // хозяин мира считает механизмы и воду и за правки гостей
        if host_side {
            after_edit(app, d, p, old, b);
        }
    }
}

/// После правки блока: пересчитать сигналы и разбудить жидкости рядом.
fn after_edit(app: &mut App, dim: Dim, p: IVec3, old: u16, new: u16) {
    let near = |f: &dyn Fn(u16) -> bool| {
        app.game.as_ref().is_some_and(|g| {
            let w = &g.worlds[dim as usize];
            [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z].iter().any(|d| {
                let q = p + *d;
                f(w.get(q.x, q.y, q.z))
            })
        })
    };
    let wake_fluid = fluid(old).is_some() || fluid(new).is_some() || new == AIR && near(&|b| fluid(b).is_some());
    let wake_mech = mech::is_mech(old) || mech::is_mech(new) || near(&mech::is_mech);
    if wake_fluid {
        if let Some(g) = app.game.as_mut() {
            g.mech.schedule(p);
        }
    }
    if wake_mech {
        mech::update_signals(app, dim, p);
    }
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

/// Небо: 0 — обычное голубое, 1 — розово-фиолетовое (из настроек).
static SKY_STYLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

fn sky_colors(g: &Game) -> (Color, Color) {
    if g.dim == Dim::Heaven {
        return (Color::new(0.98, 0.78, 0.58, 1.0), Color::new(1.0, 0.94, 0.84, 1.0));
    }
    if g.dim == Dim::Abyss {
        return (Color::new(0.12, 0.02, 0.02, 1.0), Color::new(0.32, 0.07, 0.04, 1.0));
    }
    let d = g.daylight();
    let lerp = |a: Color, b: Color, t: f32| Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1.0);
    let pink = SKY_STYLE.load(std::sync::atomic::Ordering::Relaxed) == 1;
    let (top, mut hor, dusk_c) = if pink {
        // розово-фиолетовое небо: сиреневый верх, розовый горизонт, ночью — тёмно-фиолетовое
        (
            lerp(Color::new(0.06, 0.02, 0.12, 1.0), Color::new(0.62, 0.46, 0.93, 1.0), d),
            lerp(Color::new(0.13, 0.05, 0.2, 1.0), Color::new(0.96, 0.72, 0.93, 1.0), d),
            Color::new(1.0, 0.52, 0.66, 1.0),
        )
    } else {
        (
            lerp(Color::new(0.01, 0.02, 0.06, 1.0), Color::new(0.28, 0.52, 0.88, 1.0), d),
            lerp(Color::new(0.04, 0.06, 0.13, 1.0), Color::new(0.72, 0.85, 0.98, 1.0), d),
            Color::new(0.98, 0.56, 0.32, 1.0),
        )
    };
    let sunh = g.sun_dir().y;
    let dusk = (-(sunh / 0.2).powi(2)).exp();
    hor = lerp(hor, dusk_c, dusk * 0.7);
    // непогода: серое небо, в пустыне — песчаная мгла
    let k = g.rain_k;
    if k > 0.0 {
        let p = g.player.body.pos;
        let desert = worldgen::column(g.seed, p.x as i32, p.z as i32).biome == worldgen::Biome::Desert;
        let gray = if desert { Color::new(0.78 * d + 0.1, 0.66 * d + 0.08, 0.45 * d + 0.06, 1.0) } else { Color::new(0.45 * d + 0.05, 0.47 * d + 0.05, 0.5 * d + 0.06, 1.0) };
        return (lerp(top, gray, k * 0.85), lerp(hor, gray, k * 0.9));
    }
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

/// Станции крафта рядом с игроком.
fn stations_near(g: &Game) -> Vec<crafting::Station> {
    use crafting::Station::*;
    let mut out = vec![Hand];
    let p = g.player.body.pos.floor().as_ivec3();
    let w = g.world();
    for st in [Workbench, Furnace, Cauldron, Anvil] {
        let b = st.block().unwrap();
        'f: for dy in -2..=3 {
            for dz in -4..=4 {
                for dx in -4..=4 {
                    if w.get(p.x + dx, p.y + dy, p.z + dz) == b {
                        out.push(st);
                        break 'f;
                    }
                }
            }
        }
    }
    if g.creative {
        return vec![Hand, Workbench, Furnace, Cauldron, Anvil];
    }
    out
}

fn xr(r: &mut u32) -> f32 {
    *r ^= *r << 13;
    *r ^= *r >> 17;
    *r ^= *r << 5;
    (*r & 0xffffff) as f32 / 16_777_215.0
}

fn update_game(app: &mut App, dt: f32, mouse_delta: Vec2) {
    let playing = app.screen == Screen::Playing;
    let sens = app.settings.sensitivity;
    let invert = if app.settings.invert_mouse { -1.0 } else { 1.0 };
    let difficulty = app.settings.difficulty;
    let pvp = app.settings.pvp;
    let mut edits: Vec<(IVec3, u16)> = Vec::new();
    let mut to_dim: Option<Dim> = None;
    let mut died = false;
    let mut open_inv = false;
    let mut open_book = false;
    let mut open_trade: Option<(usize, u32)> = None;
    let mut open_enchant = false;
    let mut sfx: Vec<(music::Sfx, f32)> = Vec::new();
    {
        let g = app.game.as_mut().unwrap();
        let hp_before = g.player.health;
        let hunger_before = g.player.hunger;
        g.time = (g.time + dt / DAY_SECONDS).fract();
        for id in std::mem::take(&mut g.ach_later) {
            g.unlock(id);
        }
        g.ach_check_t -= dt;
        if g.ach_check_t <= 0.0 {
            g.ach_check_t = 1.0;
            g.check_achievements();
        }
        for p in g.ach_pop.iter_mut() {
            p.1 -= dt;
        }
        g.ach_pop.retain(|p| p.1 > 0.0);
        sfx.append(&mut g.sfx_q);
        // погода (решает хозяин мира), посевы, сон
        if !g.is_client() {
            g.weather_t -= dt;
            if g.weather_t <= 0.0 {
                let r = xr(&mut g.rng);
                g.weather = if g.dim == Dim::Heaven { 0 } else if r < 0.6 { 0 } else if r < 0.9 { 1 } else { 2 };
                g.weather_t = 150.0 + xr(&mut g.rng) * 300.0;
            }
            if let Ok(w) = std::env::var("RK_WEATHER") {
                g.weather = w.parse().unwrap_or(0);
            }
            // посевы растут по стадиям
            let mut grown: Vec<(IVec3, u16)> = Vec::new();
            let w = &g.worlds[g.dim as usize];
            let rain_bonus = 1.0 + g.rain_k;
            g.crops.retain_mut(|(p, t)| {
                let b = w.get(p.x, p.y, p.z);
                let Some((kind, st)) = crop(b) else { return false };
                if st >= 3 {
                    return false;
                }
                *t -= dt * rain_bonus;
                if *t <= 0.0 {
                    grown.push((*p, crop_block(kind, st + 1)));
                    *t = 45.0 + (p.x.wrapping_mul(31) ^ p.z) as f32 % 30.0;
                }
                true
            });
            for (p, b) in grown {
                g.pending_edits.push((p, b));
            }
        }
        let target_k = if g.weather > 0 && g.dim == Dim::Overworld { 1.0 } else { 0.0 };
        g.rain_k += (target_k - g.rain_k) * (dt * 0.25).min(1.0);
        g.flash_t = (g.flash_t - dt).max(0.0);
        if g.weather == 2 && g.rain_k > 0.6 {
            g.thunder_t -= dt;
            if g.thunder_t <= 0.0 {
                g.thunder_t = 7.0 + xr(&mut g.rng) * 14.0;
                g.flash_t = 0.35;
                sfx.push((music::Sfx::Thunder, 0.9));
            }
        }
        if g.sleep_t > 0.0 {
            g.sleep_t -= dt;
            if g.sleep_t <= 0.0 {
                g.time = 0.02;
                g.weather = 0;
                g.ach_later.push(ach::SLEEP);
                g.toast = Some(("Доброе утро!".into(), 3.0));
            }
        }
        if playing {
            g.player.look(mouse_delta.x, mouse_delta.y * invert, sens);
        }
        let creative = g.creative;
        let daylight = g.daylight();
        let dim = g.dim;
        // отладка: RK_BOSS=N призывает босса через несколько секунд после входа
        {
            use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
            static DONE: AtomicBool = AtomicBool::new(false);
            static FRAMES: AtomicU32 = AtomicU32::new(0);
            if !DONE.load(Ordering::Relaxed) {
                if let Some(b) = std::env::var("RK_BOSS").ok().and_then(|v| v.parse::<usize>().ok()) {
                    if FRAMES.fetch_add(1, Ordering::Relaxed) > 60 {
                        DONE.store(true, Ordering::Relaxed);
                        let (p, d) = (g.player.body.pos, g.player.forward());
                        let _ = g.mobs.summon_boss(&g.worlds[dim as usize], mobdefs::BOSS0 + b, p, d);
                    }
                }
            }
        }
        {
            let world = &g.worlds[dim as usize];
            g.player.update(world, dt, creative, playing, difficulty);
            // лодки и вагонетки
            let fw = if playing && g.riding.is_some() { (is_key_down(KeyCode::W) as i32 - is_key_down(KeyCode::S) as i32) as f32 } else { 0.0 };
            g.ents.update_vehicles(world, g.riding, (fw, g.player.forward()), dt);
            if let Some(i) = g.riding {
                if i >= g.ents.vehicles.len() || (playing && (is_key_pressed(KeyCode::LeftShift) || touch::out().pressed(touch::Act::Sneak))) {
                    if let Some(v) = g.ents.vehicles.get(i) {
                        g.player.body.pos = v.pos + vec3(0.0, 1.0, 0.0);
                    }
                    g.riding = None;
                } else {
                    let v = &g.ents.vehicles[i];
                    g.player.body.pos = v.pos + vec3(0.0, if v.boat { 0.25 } else { 0.35 }, 0.0);
                    g.player.body.vel = Vec3::ZERO;
                    g.player.fall_start = None;
                }
            }
            // кто управляет существами: хост (или одиночная игра) — симулирует; клиент — показывает снимки хоста
            let host_dim = g.remotes.get(&0).map(|r| r.st.dim);
            let client_mode = g.net.as_ref().is_some_and(|n| !n.host) && host_dim == Some(dim_code(dim));
            if client_mode {
                g.mobs.client_tick(world, dt, g.player.body.pos);
            } else {
                let is_host = g.net.as_ref().is_some_and(|n| n.host);
                let others: Vec<(u8, Vec3)> = if is_host {
                    g.remotes.iter().filter(|(_, r)| r.st.dim == dim_code(dim) && r.st.health > 0.0).map(|(id, r)| (*id, Vec3::from(r.st.pos))).collect()
                } else {
                    Vec::new()
                };
                let ev = g.mobs.update(world, &mut g.player, &mut g.ents, dt, daylight, difficulty, creative, &others);
                for (p, v) in &ev.xp {
                    g.ents.spawn_xp(*p, *v);
                }
                for text in &ev.say {
                    if text.contains("повержен") {
                        g.ach_later.push(ach::BOSS);
                        for k in 0..6 {
                            if text.contains(mobdefs::MOBS[mobdefs::BOSS0 + k].name) {
                                g.bosses |= 1 << k;
                            }
                        }
                        if g.bosses == 0b111111 {
                            g.ach_later.push(ach::ALL_BOSSES);
                        }
                    }
                    if text.contains("пробу") {
                        sfx.push((music::Sfx::Boss, 1.0));
                    }
                    g.chat.push((text.clone(), 10.0));
                    if g.chat.len() > 8 {
                        g.chat.remove(0);
                    }
                    g.toast = Some((text.clone(), 3.5));
                    if let Some(n) = &g.net {
                        n.broadcast(&Msg::Chat { text: text.clone() }, None);
                    }
                }
                if let Some(n) = &g.net {
                    for (id, dmg, knock) in ev.hurts {
                        n.send_to(id, &Msg::Hit { target: id, from: net::FROM_MOB, dmg, knock: knock.to_array() });
                    }
                    for (id, it, cnt) in ev.gives {
                        n.send_to(id, &Msg::Give { item: it.code(), count: cnt });
                    }
                }
            }
            let picked = g.ents.update(world, g.player.body.pos, dt, g.player.health > 0.0);
            if !picked.is_empty() {
                sfx.push((music::Sfx::Pickup, 0.5));
            }
            for (it, n) in picked {
                let left = g.player.inv.add(it, n);
                if left > 0 {
                    let p = g.player.body.pos + vec3(0.0, 1.0, 0.0);
                    g.ents.spawn_drop(it, left, p, 0.5);
                }
            }
            g.ents.update_particles(world, dt);
            for (dmg, dir) in g.ents.update_projectiles(world, g.player.body.pos, dt) {
                if !creative {
                    g.player.hurt(dmg, dir * 4.0 + vec3(0.0, 3.0, 0.0), false);
                }
            }
            // стрелы игрока попадают в существ
            let client_mobs = g.net.as_ref().is_some_and(|n| !n.host);
            for pr in g.ents.projectiles.iter_mut().filter(|p| p.from_player && p.age < 6.0) {
                for m in g.mobs.list.iter_mut() {
                    if !m.alive() {
                        continue;
                    }
                    let c = m.body.pos + vec3(0.0, m.body.h * 0.5, 0.0);
                    let half = vec3(m.body.hw + 0.15, m.body.h * 0.5 + 0.15, m.body.hw + 0.15);
                    if (pr.pos - c).abs().cmple(half).all() {
                        if client_mobs {
                            if let Some(n) = &g.net {
                                n.broadcast(&Msg::MobHit { uid: m.uid, dmg: pr.damage }, None);
                            }
                            m.hurt_t = 0.3;
                        } else {
                            m.damage(pr.damage, pr.pos - pr.vel.normalize_or_zero());
                        }
                        pr.age = 99.0;
                        g.ach_pending_archer = true;
                        break;
                    }
                }
            }
            if std::mem::take(&mut g.ach_pending_archer) {
                g.ach_later.push(ach::ARCHER);
            }
            let gained = g.ents.update_orbs(world, g.player.body.pos, dt);
            if gained > 0 {
                let before = g.player.level().0;
                g.player.xp += gained;
                sfx.push((music::Sfx::Pickup, 0.35));
                if g.player.level().0 > before {
                    g.toast = Some((format!("Уровень {}!", g.player.level().0), 2.0));
                }
            }
        }
        for r in g.remotes.values_mut() {
            let k = (dt * 12.0).min(1.0);
            for a in 0..3 {
                r.shown[a] += (r.st.pos[a] - r.shown[a]) * k;
            }
            r.hurt_t = (r.hurt_t - dt).max(0.0);
        }
        for c in g.chat.iter_mut() {
            c.1 -= dt;
        }
        g.chat.retain(|c| c.1 > 0.0);
        g.label_t -= dt;
        g.use_cd -= dt;
        g.attack_cd = (g.attack_cd - dt).max(0.0);
        g.crit_t = (g.crit_t - dt).max(0.0);
        // взмах длится 0.3 с; смена предмета — рука опускается и поднимается
        g.swing_p = (g.swing_p + dt / 0.3).min(1.0);
        let held_now = g.player.held();
        if held_now != g.last_held {
            g.last_held = held_now;
            g.equip = 0.8;
            // смена предмета сбрасывает зарядку удара
            g.attack_max = held_now.cooldown();
            g.attack_cd = g.attack_max;
        }
        g.equip = (g.equip - dt * 4.0).max(0.0);
        let look = vec2(g.player.yaw, g.player.pitch);
        if (look - g.lag_look).length() > 1.5 {
            g.lag_look = look;
        }
        g.lag_look += (look - g.lag_look) * (dt * 14.0).min(1.0);
        g.portal_cd -= dt;
        if let Some((_, t)) = &mut g.toast {
            *t -= dt;
        }
        if g.toast.as_ref().is_some_and(|(_, t)| *t <= 0.0) {
            g.toast = None;
        }

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
        if g.portal_cd <= 0.0 && g.player.body.touches(&g.worlds[dim as usize], -0.05, |b| b == ABYSS_PORTAL) {
            if dim == Dim::Overworld {
                g.abyss_home = Some(g.player.body.pos);
                to_dim = Some(Dim::Abyss);
            } else if dim == Dim::Abyss {
                to_dim = Some(Dim::Overworld);
            }
        }
        if g.portal_cd <= 0.0 && to_dim.is_none() && g.player.body.touches(&g.worlds[dim as usize], -0.05, |b| b == PORTAL) {
            to_dim = Some(if dim == Dim::Overworld { Dim::Heaven } else { Dim::Overworld });
        }

        if playing {
            let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8, KeyCode::Key9];
            for (i, k) in keys.iter().enumerate() {
                if is_key_pressed(*k) {
                    g.player.selected = i;
                    g.label_t = 2.0;
                    g.player.eat_t = 0.0;
                }
            }
            let (_, wheel) = mouse_wheel();
            if let Some(k) = touch::out().hotbar {
                g.player.selected = k;
                g.label_t = 2.0;
                g.player.eat_t = 0.0;
            }
            if wheel != 0.0 {
                g.player.selected = (g.player.selected as i32 - wheel.signum() as i32).rem_euclid(9) as usize;
                g.label_t = 2.0;
                g.player.eat_t = 0.0;
            }
            if (is_key_pressed(KeyCode::F) || touch::out().pressed(touch::Act::Fly)) && creative {
                g.player.flying = !g.player.flying;
            }
            // выбросить предмет
            if is_key_pressed(KeyCode::Q) || touch::out().pressed(touch::Act::Drop) {
                let sel = g.player.selected;
                if let Some(it) = g.player.inv.take_one(sel) {
                    let p = g.player.eye() + g.player.forward() * 0.6;
                    let r = xr(&mut g.rng);
                    g.ents.spawn_drop(it, 1, p, r);
                    if let Some(d) = g.ents.drops.last_mut() {
                        d.body.vel = g.player.forward() * 6.0 + vec3(0.0, 2.0, 0.0);
                    }
                }
            }

            let world = &g.worlds[dim as usize];
            let eye = g.player.eye();
            let dir = g.player.forward();
            let held = g.player.held();
            let reach_block = if creative { 6.0 } else { 5.0 };
            let block_hit = world.raycast(eye, dir, reach_block);
            let block_d = block_hit.map_or(f32::MAX, |h| h.2);
            let reach = held.reach();
            let mut mob_hit: Option<(usize, f32)> = None;
            for (i, m) in g.mobs.list.iter().enumerate() {
                if let Some(t) = m.ray_hit(eye, dir) {
                    if t < reach && t < block_d && mob_hit.is_none_or(|(_, bt)| t < bt) {
                        mob_hit = Some((i, t));
                    }
                }
            }
            let mut player_hit: Option<(u8, f32)> = None;
            for (id, r) in g.remotes.iter() {
                if r.st.dim != dim_code(dim) {
                    continue;
                }
                let p = Vec3::from(r.shown);
                if let Some(t) = mobs::ray_aabb(eye, dir, p - vec3(0.3, 0.0, 0.3), p + vec3(0.3, 1.8, 0.3)) {
                    if t < reach && t < block_d && mob_hit.is_none_or(|(_, bt)| t < bt) && player_hit.is_none_or(|(_, bt)| t < bt) {
                        player_hit = Some((*id, t));
                    }
                }
            }
            let tc = touch::out();
            // короткое касание экрана: по существу или игроку — удар, иначе — «ставить»
            let tap_hit = tc.tap && (mob_hit.is_some() || player_hit.is_some());
            let tap_use = tc.tap && !tap_hit;
            let (lmb, lmb_p) = if tc.active {
                (tc.down(touch::Act::Attack) || tap_hit, tc.pressed(touch::Act::Attack) || tap_hit)
            } else {
                (is_mouse_button_down(MouseButton::Left), is_mouse_button_pressed(MouseButton::Left))
            };
            let held_ench = g.player.inv.slots[g.player.selected].ench;
            // удар по лодке/вагонетке — забрать её
            if lmb_p && g.riding.is_none() {
                let hit = g.ents.vehicles.iter().enumerate().filter_map(|(i, v)| {
                    let half = vec3(0.7, 0.5, 0.7);
                    mobs::ray_aabb(eye, dir, v.pos - half.with_y(0.0), v.pos + half).filter(|t| *t < 4.0).map(|t| (i, t))
                }).min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((i, _)) = hit {
                    let v = g.ents.vehicles.remove(i);
                    let it = Item::Thing(if v.boat { items::BOAT } else { items::MINECART });
                    g.ents.spawn_drop(it, 1, v.pos + vec3(0.0, 0.5, 0.0), 0.3);
                }
            }
            let (rmb, rmb_p) = if tc.active {
                (tc.down(touch::Act::Use) || tap_use, tc.pressed(touch::Act::Use) || tap_use)
            } else {
                (is_mouse_button_down(MouseButton::Right), is_mouse_button_pressed(MouseButton::Right))
            };
            if lmb_p && (g.swing_p >= 0.5) {
                g.swing_p = 0.0;
            }
            // зарядка удара: 0 сразу после взмаха, 1 — полный удар
            let charge = 1.0 - g.attack_cd / g.attack_max.max(0.01);
            let charge_mul = 0.2 + 0.8 * charge * charge;
            let crit = charge > 0.9 && !g.player.body.on_ground && g.player.body.vel.y < -0.5 && !g.player.flying;
            let atk_mul = charge_mul * if crit { 1.5 } else { 1.0 };
            let attacking = lmb_p && (player_hit.is_some() || mob_hit.is_some() || block_hit.is_none());
            if attacking {
                g.attack_max = held.cooldown();
                g.attack_cd = g.attack_max;
                let hit_something = player_hit.is_some() || mob_hit.is_some();
                sfx.push(if crit && hit_something {
                    (music::Sfx::Crit, 0.9)
                } else if hit_something {
                    (music::Sfx::Hit, 0.4 + 0.5 * charge)
                } else {
                    (music::Sfx::Swing, 0.35)
                });
            }

            if let Some((rid, _)) = player_hit {
                g.break_target = None;
                if lmb_p {
                    if crit {
                        g.crit_t = 0.4;
                    }
                    if pvp {
                        let dmg = held.damage() * g.player.damage_bonus() * atk_mul * ench::melee_mult(held_ench);
                        let knock = ((dir.with_y(0.0).normalize_or_zero() * 6.0 + vec3(0.0, 4.0, 0.0)) * (0.3 + 0.7 * charge)).to_array();
                        if let Some(n) = &g.net {
                            let m = Msg::Hit { target: rid, from: n.my_id, dmg, knock };
                            if n.host { n.send_to(rid, &m) } else { n.broadcast(&m, None) }
                        }
                        if let Some(r) = g.remotes.get_mut(&rid) {
                            r.hurt_t = 0.3;
                        }
                    } else {
                        g.toast = Some(("PvP выключено в настройках".into(), 2.0));
                    }
                }
            } else if let Some((mi, _)) = mob_hit {
                g.break_target = None;
                if lmb_p {
                    let from = g.player.body.pos;
                    let dmg = held.damage() * g.player.damage_bonus() * atk_mul * ench::melee_mult(held_ench);
                    // вампиризм лечит, огненный клинок — искры
                    if ench::kind(held_ench) == ench::VAMP {
                        g.player.health = (g.player.health + 0.5 * ench::level(held_ench) as f32).min(MAX_HEALTH);
                    }
                    let host_dim = g.remotes.get(&0).map(|r| r.st.dim);
                    let client_mode = g.net.as_ref().is_some_and(|n| !n.host) && host_dim == Some(dim_code(dim));
                    if client_mode {
                        // удар решает хост; у себя только вспышка
                        if let Some(n) = &g.net {
                            n.broadcast(&Msg::MobHit { uid: g.mobs.list[mi].uid, dmg }, None);
                        }
                        g.mobs.list[mi].hurt_t = 0.3;
                    } else {
                        g.mobs.list[mi].damage(dmg, from);
                        // секира, коса и алебарда при полном замахе задевают соседей цели
                        if let Item::Tool(k, _) = held {
                            if k.sweeps() && charge > 0.9 {
                                let center = g.mobs.list[mi].body.pos;
                                for (j, o) in g.mobs.list.iter_mut().enumerate() {
                                    if j != mi && o.alive() && o.body.pos.distance(center) < 2.5 {
                                        o.damage(dmg * 0.5, from);
                                    }
                                }
                            }
                        }
                    }
                    let m = &g.mobs.list[mi];
                    let d = m.d();
                    let at = m.body.pos + vec3(0.0, m.body.h * 0.6, 0.0);
                    let cols = [Color::from_rgba(d.c1.0, d.c1.1, d.c1.2, 255), Color::from_rgba(200, 30, 30, 255), Color::from_rgba(d.c2.0, d.c2.1, d.c2.2, 255)];
                    g.ents.burst(at, if !m.alive() { 22 } else if charge > 0.9 { 8 } else { 3 }, 0.5, &cols, 0.09, 3.5);
                    if crit {
                        // критический удар: россыпь искр
                        let sparks = [Color::from_rgba(255, 240, 160, 255), Color::from_rgba(255, 200, 60, 255), WHITE];
                        g.ents.burst(at, 14, 0.6, &sparks, 0.06, 5.0);
                        g.crit_t = 0.4;
                    }
                    g.player.exhaustion += 0.1;
                }
            } else if let Some((hit, _, _)) = block_hit {
                let b = world.get(hit.x, hit.y, hit.z);
                if lmb || lmb_p {
                    if creative {
                        if (lmb_p || g.use_cd <= 0.0) && b != BEDROCK {
                            edits.push((hit, AIR));
                            g.use_cd = 0.22;
                        }
                    } else if let Some(bt) = break_time(b, held, g.player.has(items::Eff::Haste)).map(|t| t / ench::mine_mult(held_ench)) {
                        if g.break_target != Some(hit) {
                            g.break_target = Some(hit);
                            g.break_progress = 0.0;
                        }
                        g.break_progress += if bt <= 0.0 { 1.0 } else { dt / bt };
                        if g.swing_p >= 1.0 {
                            g.swing_p = 0.0;
                        }
                        if g.break_progress >= 1.0 {
                            edits.push((hit, AIR));
                            let r = xr(&mut g.rng);
                            for (it, n) in entities::block_drops(b, held, r) {
                                let rr = xr(&mut g.rng);
                                g.ents.spawn_drop(it, n, hit.as_vec3() + Vec3::splat(0.5), rr);
                            }
                            g.player.exhaustion += 0.02;
                            g.break_target = None;
                            g.break_progress = 0.0;
                        }
                    }
                } else {
                    g.break_target = None;
                    g.break_progress = 0.0;
                }
            } else {
                g.break_target = None;
            }

            // правая кнопка: сундук, станция, еда, установка блока, броня
            let target_block = block_hit.map(|(h, prev, _)| (h, prev, world.get(h.x, h.y, h.z)));
            let mut used = false;
            if rmb_p && held == Item::Thing(items::RECIPE_BOOK) {
                open_book = true;
                used = true;
            }
            if let Item::Thing(st) = held {
                if let (true, Some(boss)) = (rmb_p, items::summon_boss(st)) {
                    used = true;
                    if g.net.as_ref().is_some_and(|n| !n.host) {
                        g.toast = Some(("Призывать боссов может только хозяин мира".into(), 2.5));
                    } else if g.mobs.boss_alive() {
                        g.toast = Some(("Другой босс ещё жив".into(), 2.0));
                    } else if let Some(text) = g.mobs.summon_boss(world, boss, g.player.body.pos, dir) {
                        sfx.push((music::Sfx::Boss, 1.0));
                        if !creative {
                            let sel = g.player.selected;
                            g.player.inv.slots[sel] = Stack::EMPTY;
                        }
                        g.chat.push((text.clone(), 10.0));
                        g.toast = Some((text.clone(), 3.0));
                        if let Some(n) = &g.net {
                            n.broadcast(&Msg::Chat { text }, None);
                        }
                    }
                }
            }
            if rmb_p && !used {
                if let Some((h, _, b)) = target_block {
                    if b == CHEST && !g.player.body.overlaps_block(h) {
                        let key = [h.x, h.y, h.z];
                        if g.opened.insert(key) {
                            if h.y < 28 {
                                g.ach_later.push(ach::DUNGEON);
                            }
                            let tier = if dim == Dim::Heaven { 3 } else if h.y < 45 { 1 } else { 2 };
                            let seed = (h.x as u32).wrapping_mul(73_856_093) ^ (h.y as u32).wrapping_mul(19_349_663) ^ (h.z as u32).wrapping_mul(83_492_791);
                            for (it, n) in entities::chest_loot(tier, seed) {
                                let r = xr(&mut g.rng);
                                g.ents.spawn_drop(it, n, h.as_vec3() + vec3(0.5, 1.2, 0.5), r);
                            }
                            g.toast = Some(("Сундук открыт!".into(), 2.0));
                        } else {
                            g.toast = Some(("Сундук пуст".into(), 1.5));
                        }
                        used = true;
                    } else if matches!(b, WORKBENCH | FURNACE | CAULDRON | ANVIL) && !g.player.body.overlaps_block(h) {
                        open_inv = true;
                        used = true;
                    }
                }
                if !used {
                    if let Item::Armor(slot, _) = held {
                        let old = g.player.equip(held);
                        let sel = g.player.selected;
                        let new_e = g.player.inv.slots[sel].ench;
                        let old_e = std::mem::replace(&mut g.player.armor_ench[slot as usize], new_e);
                        g.player.inv.slots[sel] = Stack::new(old, if old == Item::Empty { 0 } else { 1 });
                        g.player.inv.slots[sel].ench = if old == Item::Empty { 0 } else { old_e };
                        g.toast = Some((format!("Надето: {}", held.name()), 2.0));
                        used = true;
                    }
                }
            }
            if !used && rmb && g.player.can_eat(held) && (creative || held.food().is_some()) {
                g.player.eat_t += dt;
                let need = if matches!(held, Item::Thing(t) if matches!(items::thing(t).kind, items::TKind::Potion(..))) { 0.8 } else { 1.2 };
                if g.player.eat_t >= need {
                    g.player.eat_t = 0.0;
                    let sel = g.player.selected;
                    if !creative {
                        g.player.inv.take_one(sel);
                    }
                    g.player.eat(held);
                    if let Item::Thing(t) = held {
                        if matches!(items::thing(t).kind, items::TKind::Potion(..)) || t == items::MILK || t == items::NECTAR {
                            g.player.inv.add(Item::Thing(items::BOTTLE), 1);
                        }
                        if matches!(t, items::MUSHROOM_SOUP | items::STEW | items::PORRIDGE) {
                            g.player.inv.add(Item::Thing(items::BOWL), 1);
                        }
                    }
                    g.toast = Some((format!("Съедено: {}", held.name()), 1.5));
                }
                used = true;
            } else if !rmb {
                g.player.eat_t = 0.0;
            }
            // двери, люки, калитки открываются по ПКМ (с Shift — ставим блок рядом)
            // лук, арбалет, щит
            g.player.blocking = false;
            if let Item::Tool(kind @ (items::ToolKind::Bow | items::ToolKind::Crossbow | items::ToolKind::Shield), mat) = held {
                let arrows = creative || g.player.inv.count(Item::Thing(items::ARROW)) > 0;
                let eye_p = g.player.eye() + dir * 0.4;
                let shoot = |ents: &mut Entities, player: &mut Player, rng: &mut u32, power: f32, mult: f32| {
                    let dmg = (3.0 + mat.tier() * 1.2) * power * mult * ench::shot_mult(held_ench);
                    ents.projectiles.push(entities::Projectile { pos: eye_p, vel: dir * (12.0 + 26.0 * power), damage: dmg, color: Color::from_rgba(220, 220, 225, 255), age: 0.0, from_player: true });
                    let thrift = ench::kind(held_ench) == ench::THRIFT && xr(rng) < 0.25 * ench::level(held_ench) as f32;
                    if !creative && !thrift {
                        player.inv.remove(Item::Thing(items::ARROW), 1);
                    }
                };
                match kind {
                    items::ToolKind::Bow => {
                        if rmb && arrows {
                            g.draw_t = (g.draw_t + dt).min(1.0);
                            used = true;
                        } else if g.draw_t > 0.15 {
                            let p = g.draw_t;
                            shoot(&mut g.ents, &mut g.player, &mut g.rng, p, 1.0);
                            g.draw_t = 0.0;
                            sfx.push((music::Sfx::Swing, 0.6));
                            used = true;
                        } else {
                            g.draw_t = 0.0;
                        }
                    }
                    items::ToolKind::Crossbow => {
                        if g.cross_loaded {
                            if rmb_p {
                                shoot(&mut g.ents, &mut g.player, &mut g.rng, 1.0, 1.5);
                                g.cross_loaded = false;
                                sfx.push((music::Sfx::Hit, 0.6));
                                used = true;
                            }
                        } else if rmb && arrows {
                            g.draw_t += dt;
                            if g.draw_t >= 1.1 {
                                g.cross_loaded = true;
                                g.draw_t = 0.0;
                                g.toast = Some(("Арбалет заряжен".into(), 1.0));
                            }
                            used = true;
                        } else {
                            g.draw_t = 0.0;
                        }
                    }
                    _ => {
                        if rmb {
                            g.player.blocking = true;
                            used = true;
                        }
                    }
                }
            } else {
                g.draw_t = 0.0;
            }
            // стол улучшений
            if !used && rmb_p {
                if let Some((_, _, ENCHANT_TABLE)) = target_block {
                    open_enchant = true;
                    used = true;
                }
            }
            // лодки и вагонетки: поставить, сесть
            if !used && rmb_p && g.riding.is_none() {
                let veh_hit = g.ents.vehicles.iter().enumerate().filter_map(|(i, v)| {
                    let half = vec3(0.7, 0.5, 0.7);
                    mobs::ray_aabb(eye, dir, v.pos - half.with_y(0.0), v.pos + half).filter(|t| *t < 4.0).map(|t| (i, t))
                }).min_by(|a, b| a.1.total_cmp(&b.1));
                match held {
                    Item::Thing(items::BOAT) => {
                        for k in 1..50 {
                            let q = (eye + dir * (k as f32 * 0.1)).floor().as_ivec3();
                            let bq = world.get(q.x, q.y, q.z);
                            if fluid(bq).is_some_and(|(w, _)| w) {
                                g.ents.vehicles.push(entities::Vehicle { boat: true, pos: q.as_vec3() + vec3(0.5, 0.85, 0.5), yaw: g.player.yaw, speed: 0.0, dir: IVec3::X, vy: 0.0 });
                                if !creative {
                                    let sel = g.player.selected;
                                    g.player.inv.take_one(sel);
                                }
                                used = true;
                                break;
                            }
                            if is_solid(bq) {
                                break;
                            }
                        }
                    }
                    Item::Thing(items::MINECART) => {
                        if let Some((h, _, RAIL)) = target_block {
                            let d = shapes::facing(g.player.yaw);
                            let dv = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z][d as usize];
                            g.ents.vehicles.push(entities::Vehicle { boat: false, pos: h.as_vec3() + vec3(0.5, 0.05, 0.5), yaw: 0.0, speed: 0.0, dir: dv, vy: 0.0 });
                            if !creative {
                                let sel = g.player.selected;
                                g.player.inv.take_one(sel);
                            }
                            used = true;
                        }
                    }
                    _ => {
                        if let Some((i, _)) = veh_hit {
                            g.riding = Some(i);
                            let boat = g.ents.vehicles[i].boat;
                            g.ach_later.push(if boat { ach::BOAT } else { ach::CART });
                            g.toast = Some(("W/S — ехать, Shift — сойти".into(), 2.5));
                            used = true;
                        }
                    }
                }
            }
            // существа: приручение, команда «сидеть», разведение, торговля
            if !used && rmb_p {
                if let Some((mi, mt)) = mob_hit {
                    let m = &g.mobs.list[mi];
                    let food = if let Item::Thing(t) = held { Some(t) } else { None };
                    let trader = { let d = m.d(); d.friendly && d.plan == mobdefs::Plan::Biped && trade::offers(m.def, m.uid).is_some() };
                    if mt <= 4.0 && m.alive() && !g.is_client() {
                        let me_owner = Some(mobs::OWNER_LOCAL);
                        if m.owner == me_owner {
                            let sit = !m.sitting;
                            g.mobs.list[mi].sitting = sit;
                            g.toast = Some((if sit { "Питомец сидит".into() } else { "Питомец идёт за вами".into() }, 1.5));
                            used = true;
                        } else if m.owner.is_none() && food.is_some() && mobs::tame_food(m.def) == food {
                            let ok = xr(&mut g.rng) < 0.34;
                            let pos = m.body.pos + vec3(0.0, m.body.h, 0.0);
                            if ok {
                                let mm = &mut g.mobs.list[mi];
                                mm.owner = me_owner;
                                mm.angry = false;
                                g.ach_later.push(ach::PET);
                                let hearts = [Color::from_rgba(250, 80, 110, 255), Color::from_rgba(255, 150, 170, 255)];
                                g.ents.burst(pos, 14, 0.6, &hearts, 0.1, 2.0);
                                g.toast = Some((format!("{} теперь ваш питомец!", g.mobs.list[mi].d().name), 2.5));
                            } else {
                                g.ents.burst(pos, 6, 0.4, &[Color::from_rgba(90, 90, 90, 255)], 0.08, 1.5);
                            }
                            if !creative {
                                let sel = g.player.selected;
                                g.player.inv.take_one(sel);
                            }
                            used = true;
                        } else if mobs::breedable(m.def) && m.baby <= 0.0 && m.love == 0.0 && matches!(food, Some(items::WHEAT | items::WHEAT_SEEDS | items::CARROT | items::BERRIES | items::APPLE)) {
                            let pos = m.body.pos + vec3(0.0, m.body.h, 0.0);
                            g.mobs.list[mi].love = 15.0;
                            let hearts = [Color::from_rgba(250, 80, 110, 255)];
                            g.ents.burst(pos, 8, 0.5, &hearts, 0.1, 1.5);
                            if !creative {
                                let sel = g.player.selected;
                                g.player.inv.take_one(sel);
                            }
                            used = true;
                        }
                    }
                    if !used && mt <= 4.0 && trader {
                        open_trade = Some((g.mobs.list[mi].def, g.mobs.list[mi].uid));
                        used = true;
                    }
                }
            }
            // ферма: мотыга вспахивает, семена сажаются на пашню; кровать
            if !used && rmb_p {
                if let Some((h, prev, b)) = target_block {
                    let above_free = world.get(h.x, h.y + 1, h.z) == AIR;
                    if matches!(held, Item::Tool(items::ToolKind::Hoe, _)) && matches!(b, GRASS | DIRT | SNOW_GRASS | PODZOL | COARSE_DIRT) && above_free {
                        edits.push((h, FARMLAND));
                        g.swing_p = 0.0;
                        used = true;
                    } else if b == FARMLAND && prev.y > h.y && above_free {
                        let kind = match held {
                            Item::Thing(items::WHEAT_SEEDS) => Some(0),
                            Item::Thing(items::CARROT) => Some(1),
                            Item::Thing(items::POTATO) => Some(2),
                            _ => None,
                        };
                        if let Some(k) = kind {
                            edits.push((prev, crop_block(k, 0)));
                            g.crops.push((prev, 50.0));
                            g.ach_later.push(ach::FARMER);
                            if !creative {
                                let sel = g.player.selected;
                                g.player.inv.take_one(sel);
                            }
                            used = true;
                        }
                    } else if b == BED {
                        g.spawn = h.as_vec3() + vec3(0.5, 0.6, 0.5);
                        if g.daylight() < 0.35 && g.dim == Dim::Overworld {
                            if g.is_client() {
                                g.toast = Some(("Точка возрождения — здесь. Ночь пропускает хозяин мира".into(), 3.0));
                            } else {
                                g.sleep_t = 2.5;
                                g.toast = Some(("Вы засыпаете…".into(), 2.5));
                            }
                        } else {
                            g.toast = Some(("Точка возрождения установлена. Спать можно только ночью".into(), 3.0));
                        }
                        used = true;
                    }
                }
            }
            // вёдра: набрать или вылить жидкость
            if !used && rmb_p {
                if let Item::Thing(t) = held {
                    if t == items::BUCKET {
                        // ищем источник воды или лавы по лучу
                        let mut found = None;
                        for k in 1..50 {
                            let q = (eye + dir * (k as f32 * 0.1)).floor().as_ivec3();
                            let bq = world.get(q.x, q.y, q.z);
                            if matches!(bq, WATER | LAVA) {
                                found = Some((q, bq));
                                break;
                            }
                            if is_solid(bq) {
                                break;
                            }
                        }
                        if let Some((q, bq)) = found {
                            edits.push((q, AIR));
                            let sel = g.player.selected;
                            g.player.inv.take_one(sel);
                            let full = Item::Thing(if bq == WATER { items::WATER_BUCKET } else { items::LAVA_BUCKET });
                            let left = g.player.inv.add(full, 1);
                            if left > 0 {
                                g.ents.spawn_drop(full, 1, g.player.body.pos + vec3(0.0, 1.0, 0.0), 0.3);
                            }
                            used = true;
                        }
                    } else if t == items::WATER_BUCKET || t == items::LAVA_BUCKET {
                        if let Some((_, prev, _)) = target_block {
                            let cur = world.get(prev.x, prev.y, prev.z);
                            if cur == AIR || def(cur).shape == blocks::Shape::Cross || fluid(cur).is_some_and(|(_, l)| l > 0) {
                                edits.push((prev, if t == items::WATER_BUCKET { WATER } else { LAVA }));
                                if !creative {
                                    let sel = g.player.selected;
                                    g.player.inv.slots[sel] = Stack::new(Item::Thing(items::BUCKET), 1);
                                }
                                used = true;
                            }
                        }
                    }
                }
            }
            // рычаг и кнопка
            if !used && rmb_p {
                if let Some((h, _, b)) = target_block {
                    match b {
                        LEVER_OFF | LEVER_ON => {
                            edits.push((h, if b == LEVER_OFF { LEVER_ON } else { LEVER_OFF }));
                            used = true;
                        }
                        BUTTON => {
                            edits.push((h, BUTTON_ON));
                            g.mech.buttons.push((h, 1.0));
                            used = true;
                        }
                        _ => {}
                    }
                }
            }
            if !used && rmb_p && !is_key_down(KeyCode::LeftShift) && !touch::out().down(touch::Act::Sneak) {
                if let Some((h, _, b)) = target_block {
                    if let Some(nb2) = shapes::toggled(b) {
                        edits.push((h, nb2));
                        if let Some(shapes::Form::Door { upper, .. }) = shapes::form(b) {
                            let other = if upper { h - IVec3::Y } else { h + IVec3::Y };
                            if let Some(ob) = shapes::toggled(world.get(other.x, other.y, other.z)) {
                                edits.push((other, ob));
                            }
                        }
                        g.swing_p = 0.0;
                        used = true;
                    }
                }
            }
            if !used && (rmb || rmb_p) && (rmb_p || g.use_cd <= 0.0) {
                if let (Some((h, prev, _)), Item::Block(nb)) = (target_block, held) {
                    let cur = world.get(prev.x, prev.y, prev.z);
                    let replace = cur == AIR || def(cur).shape == blocks::Shape::Liquid || def(cur).shape == blocks::Shape::Cross;
                    // где именно попали по грани: для полублоков и ступенек сверху/снизу
                    let hit_t = block_hit.map_or(0.0, |(_, _, t)| t);
                    let hit_y = eye.y + dir.y * hit_t - prev.y as f32;
                    let mut nb = shapes::placed(nb, g.player.yaw, hit_y, prev.y < h.y);
                    if nb == PUSHER0 {
                        let d = if g.player.pitch < -0.8 { 4 } else { shapes::facing(g.player.yaw) as u16 };
                        nb = PUSHER0 + d * 2;
                    }
                    let mut at = prev;
                    let mut ok = replace;
                    // полублок на такой же полублок — целый блок
                    if let (Some(shapes::Form::Slab { m, top }), Some(shapes::Form::Slab { m: m2, .. })) = (shapes::form(world.get(h.x, h.y, h.z)), shapes::form(nb)) {
                        if m == m2 && ((!top && prev.y > h.y) || (top && prev.y < h.y)) {
                            nb = shapes::SLAB_MATS[m as usize];
                            at = h;
                            ok = true;
                        }
                    }
                    let door = matches!(shapes::form(nb), Some(shapes::Form::Door { .. }));
                    if door {
                        g.ach_later.push(ach::DOOR);
                    }
                    let up = at + IVec3::Y;
                    if door && world.get(up.x, up.y, up.z) != AIR {
                        ok = false;
                    }
                    let body_hit = |c: IVec3| g.player.body.overlaps_block(c) || g.mobs.list.iter().any(|m| m.body.overlaps_block(c));
                    let blocked = (is_solid(nb) || shapes::form(nb).is_some()) && (body_hit(at) || (door && body_hit(up)));
                    if ok && !blocked {
                        edits.push((at, nb));
                        if door {
                            if let Some(shapes::Form::Door { m, dir, open, .. }) = shapes::form(nb) {
                                edits.push((up, shapes::id(shapes::Form::Door { m, dir, open, upper: true })));
                            }
                        }
                        let prev = at;
                        g.swing_p = 0.0;
                        if nb == CHEST {
                            g.opened.insert([prev.x, prev.y, prev.z]);
                        }
                        if !creative {
                            let sel = g.player.selected;
                            g.player.inv.take_one(sel);
                        }
                    }
                }
                g.use_cd = 0.22;
            }
            if is_mouse_button_pressed(MouseButton::Middle) && creative {
                if let Some((_, _, b)) = target_block {
                    let sel = g.player.selected;
                    g.player.inv.slots[sel] = Stack::new(Item::Block(b), 64);
                    g.label_t = 2.0;
                }
            }
        }
        if g.player.health <= 0.0 {
            died = true;
        }
        g.autosave_t -= dt;
        g.swing_t = if g.swing_p < 1.0 { g.swing_p } else { 0.0 };
        if g.player.health < hp_before - 0.01 {
            sfx.push((music::Sfx::Hurt, 0.8));
        }
        if g.player.hunger > hunger_before + 0.5 {
            sfx.push((music::Sfx::Eat, 0.7));
            if let Item::Thing(t) = g.player.held() {
                if !matches!(t, items::RAW_BEEF | items::RAW_CHICKEN | items::RAW_MUTTON | items::RAW_FISH | items::APPLE | items::BERRIES | items::CARROT | items::POTATO) {
                    g.ach_later.push(ach::COOK);
                }
            }
        }
    }
    app.sfx.append(&mut sfx);
    // механизмы и жидкости считает хозяин мира или одиночная игра
    if app.game.as_ref().is_some_and(|g| !g.is_client()) {
        let dim = app.game.as_ref().unwrap().dim;
        mech::plates_and_buttons(app, dim, dt);
        mech::fluid_tick(app, dim, dt);
    }

    let dim = app.game.as_ref().unwrap().dim;
    for (_, b) in &edits {
        app.sfx.push((if *b == AIR { music::Sfx::Break } else { music::Sfx::Place }, 0.6));
    }
    for (p, b) in edits {
        if b == AIR {
            if let Some(g) = app.game.as_mut() {
                let old = g.world().get(p.x, p.y, p.z);
                if old != AIR {
                    let tile = def(old).tex[1];
                    g.ents.block_burst(p.as_vec3() + Vec3::splat(0.5), tile);
                }
            }
        }
        let old_b = app.game.as_ref().map_or(AIR, |g| g.world().get(p.x, p.y, p.z));
        let old_form = shapes::form(old_b);
        edit_block(app, dim, p, b, true);
        let simulate = app.game.as_ref().is_some_and(|g| !g.is_client());
        if simulate {
            after_edit(app, dim, p, old_b, b);
        }
        // опыт за руду
        if b == AIR {
            let xp = match old_b {
                COAL_ORE => 1,
                COPPER_ORE | IRON_ORE => 2,
                GOLD_ORE => 3,
                RUBY_ORE | SAPPHIRE_ORE | MALACHITE_ORE => 5,
                CRYSTAL_ORE => 7,
                _ => 0,
            };
            if xp > 0 {
                if let Some(g) = app.game.as_mut() {
                    if !g.creative {
                        g.ents.spawn_xp(p.as_vec3() + Vec3::splat(0.5), xp);
                    }
                }
            }
        }
        // сломали половину двери — вторая тоже исчезает
        if b == AIR {
            if let Some(shapes::Form::Door { upper, .. }) = old_form {
                let other = if upper { p - IVec3::Y } else { p + IVec3::Y };
                let ob = app.game.as_ref().map_or(AIR, |g| g.world().get(other.x, other.y, other.z));
                if matches!(shapes::form(ob), Some(shapes::Form::Door { .. })) {
                    edit_block(app, dim, other, AIR, true);
                }
            }
        }
        if b == AIR {
            let mut y = p.y + 1;
            loop {
                let w = app.game.as_ref().unwrap().world();
                let above = w.get(p.x, y, p.z);
                if above != AIR && def(above).shape == blocks::Shape::Cross && above != COBWEB {
                    edit_block(app, dim, ivec3(p.x, y, p.z), AIR, true);
                    y += 1;
                } else {
                    break;
                }
            }
        }
    }
    if open_inv {
        app.screen = Screen::Inventory;
        app.just_opened = true;
    }
    if open_book {
        open_recipe_book(app);
    }
    if open_enchant {
        app.screen = Screen::Enchant;
        app.just_opened = true;
    }
    if let Some(t) = open_trade {
        app.trade = Some(t);
        app.screen = Screen::Trade;
        app.just_opened = true;
    }
    // отложенные правки (рост посевов)
    let pend: Vec<(IVec3, u16)> = app.game.as_mut().map(|g| std::mem::take(&mut g.pending_edits)).unwrap_or_default();
    let pdim = app.game.as_ref().map_or(Dim::Overworld, |g| g.dim);
    for (p, b) in pend {
        edit_block(app, pdim, p, b, true);
    }
    if let Some(d) = to_dim {
        switch_dimension(app, d);
    }
    if died {
        let g = app.game.as_mut().unwrap();
        if !g.creative {
            // снаряжение остаётся на месте гибели; зачарованные вещи остаются у игрока
            let p = g.player.body.pos + vec3(0.0, 0.8, 0.0);
            let slots: Vec<Stack> = g.player.inv.slots.to_vec();
            let armor = g.player.armor;
            let mut kept = inventory::Inventory::new();
            let mut kept_n = 0;
            for (k, s) in slots.iter().enumerate() {
                if s.is_empty() {
                    continue;
                }
                if s.ench != 0 {
                    kept.slots[kept_n] = *s;
                    kept_n += 1;
                } else {
                    g.ents.spawn_drop(s.item, s.count, p, k as f32 * 0.137);
                }
            }
            for (k, a) in armor.iter().enumerate() {
                if g.player.armor_ench[k] == 0 {
                    g.ents.spawn_drop(*a, 1, p, 0.5 + k as f32 * 0.21);
                    g.player.armor[k] = Item::Empty;
                }
            }
            // половина опыта рассыпается
            let xp = g.player.xp / 2;
            if xp > 0 {
                g.ents.spawn_xp(p, xp);
            }
            g.player.xp = 0;
            g.player.inv = kept;
        }
        app.screen = Screen::Dead;
    }
    let need_save = app.game.as_ref().is_some_and(|g| g.autosave_t <= 0.0);
    if need_save {
        app.game.as_mut().unwrap().autosave_t = AUTOSAVE;
        if save_game(app) {
            app.game.as_mut().unwrap().toast = Some(("Мир сохранён".into(), 1.5));
        }
    }
}

// ---------------------------------------------------------------------------
// Экраны
// ---------------------------------------------------------------------------

/// Поле ввода текста. Возвращает true, если по нему щёлкнули.
fn text_field(app: &mut App, field: Field, x: f32, y: f32, w: f32, h: f32, label: &str) {
    let s = app.ui.s;
    app.ui.text(label, x, y - 8.0 * s, 15.0, Color::new(0.8, 0.8, 0.85, 1.0));
    let active = app.focus == field;
    draw_rectangle(x, y, w, h, Color::new(0.03, 0.03, 0.05, 1.0));
    draw_rectangle_lines(x, y, w, h, 2.0, if active { ACCENT } else { Color::new(1.0, 1.0, 1.0, 0.2) });
    if app.ui.pressed && app.ui.hover(x, y, w, h) {
        app.focus = field;
    }
    let value = match field {
        Field::Name => &mut app.name_input,
        Field::Seed => &mut app.seed_input,
        Field::Ip => &mut app.ip_input,
        Field::Nick => &mut app.settings.name,
        Field::None => return,
    };
    if active {
        while let Some(c) = get_char_pressed() {
            if !c.is_control() && value.chars().count() < 32 {
                value.push(c);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            value.pop();
        }
    }
    let caret = if active && (app.time * 2.0).fract() < 0.5 { "|" } else { "" };
    let shown = format!("{}{}", value, caret);
    app.ui.text(&shown, x + 14.0 * s, y + h * 0.66, 19.0, WHITE);
}

fn backdrop(app: &App) {
    let (w, h) = (screen_width(), screen_height());
    for i in 0..30 {
        let t = i as f32 / 30.0;
        let c = Color::new(0.2 + t * 0.4, 0.42 + t * 0.35, 0.8 + t * 0.15, 1.0);
        draw_rectangle(0.0, h * t * 0.8, w, h * 0.8 / 30.0 + 1.0, c);
    }
    let s = app.ui.s;
    for i in 0..9 {
        let x = ((app.time * (12.0 + i as f32 * 3.0) + i as f32 * 211.0) % (w + 400.0)) - 200.0;
        let y = h * (0.06 + hash(i, 0, 3) * 0.3);
        let cw = 120.0 + hash(i, 1, 3) * 160.0;
        draw_rectangle(x, y, cw * s, 18.0 * s, Color::new(1.0, 1.0, 1.0, 0.8));
        draw_rectangle(x + 24.0 * s, y - 12.0 * s, cw * 0.5 * s, 14.0 * s, Color::new(1.0, 1.0, 1.0, 0.8));
    }
    let cell = 56.0 * s;
    let row = [GRASS, GRASS, OAK_LOG, GRASS, SAND, CHISELED_SANDSTONE, SNOW_GRASS, SPRUCE_LOG, GRASS, STONE, RUBY_ORE, GRASS, CRYSTAL_ORE, HEAVEN_GRASS, GOLD_TILES, TERRACOTTA0 + 6, LAMP0 + 3, PUMPKIN];
    let ground = h * 0.8;
    draw_rectangle(0.0, ground, w, h - ground, Color::new(0.33, 0.24, 0.17, 1.0));
    let n = (w / cell) as i32 + 2;
    for i in 0..n {
        let b = row[(i as usize * 7 + 3) % row.len()];
        let d = def(b);
        ui::iso_cube(&app.ui.atlas, d.tex[0], d.tex[1], i as f32 * cell, ground + cell * 0.1, cell * 1.15);
    }
}

fn draw_title(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    backdrop(app);
    let title = "РАСТКРАФТ";
    let size = 92.0;
    let tw = app.ui.measure(title, size);
    let ty = h * 0.24;
    for k in (1..=6).rev() {
        let o = k as f32 * 1.5 * s;
        app.ui.text(title, (w - tw) / 2.0 + o, ty + o, size, Color::new(0.35, 0.16, 0.05, 1.0));
    }
    app.ui.text(title, (w - tw) / 2.0, ty, size, ACCENT);
    let pulse = 1.0 + (app.time * 3.0).sin() * 0.04;
    app.ui.text_center("воксельная песочница на Rust", w / 2.0, ty + 48.0 * s, 20.0 * pulse, WHITE);

    let bw = 380.0 * s;
    let bh = 46.0 * s;
    let bx = (w - bw) / 2.0;
    let mut y = h * 0.38;
    let mut next = None;
    if app.ui.button(bx, y, bw, bh, "Одиночная игра") {
        app.worlds_list = save::list_worlds();
        next = Some(Screen::Worlds);
    }
    y += bh + 10.0 * s;
    if app.ui.button(bx, y, bw, bh, "Сетевая игра") {
        next = Some(Screen::Multiplayer);
    }
    y += bh + 10.0 * s;
    if app.ui.button(bx, y, bw, bh, "Достижения") {
        next = Some(Screen::Achievements);
    }
    y += bh + 8.0 * s;
    if app.ui.button(bx, y, bw, bh, "Настройки") {
        app.settings_back = Screen::Title;
        next = Some(Screen::Settings);
    }
    y += bh + 10.0 * s;
    if app.ui.button(bx, y, bw, bh, "Выход") {
        std::process::exit(0);
    }
    app.ui.text("Версия 0.3", 10.0 * s, h - 12.0 * s, 13.0, Color::new(1.0, 1.0, 1.0, 0.7));
    next
}

fn worlds_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 1.0));
    app.ui.text_center("Выбор мира", w / 2.0, h * 0.1, 34.0, WHITE);
    let bw = 560.0 * s;
    let bx = (w - bw) / 2.0;
    let rh = 56.0 * s;
    let mut y = h * 0.16;
    let mut next = None;
    let mut to_load: Option<String> = None;
    let mut to_delete: Option<String> = None;
    if app.worlds_list.is_empty() {
        app.ui.text_center("Сохранённых миров пока нет", w / 2.0, y + 40.0 * s, 18.0, GRAY);
    }
    let names: Vec<(String, bool, u32)> = app.worlds_list.iter().map(|wi| (wi.name.clone(), wi.creative, wi.seed)).collect();
    for (name, creative, seed) in names.iter().take(7) {
        draw_rectangle(bx, y, bw, rh, Color::new(0.13, 0.14, 0.18, 0.95));
        app.ui.text(name, bx + 14.0 * s, y + 24.0 * s, 18.0, WHITE);
        let mode = if *creative { "Творчество" } else { "Выживание" };
        app.ui.text(&format!("{mode} · зерно {seed}"), bx + 14.0 * s, y + 46.0 * s, 13.0, GRAY);
        if app.ui.button(bx + bw - 250.0 * s, y + 8.0 * s, 130.0 * s, rh - 16.0 * s, "Играть") {
            to_load = Some(name.clone());
        }
        if app.ui.button(bx + bw - 112.0 * s, y + 8.0 * s, 104.0 * s, rh - 16.0 * s, "Удалить") {
            to_delete = Some(name.clone());
        }
        y += rh + 8.0 * s;
    }
    let by = h * 0.82;
    if app.ui.button(bx, by, bw / 2.0 - 6.0 * s, 46.0 * s, "Назад") || is_key_pressed(KeyCode::Escape) {
        next = Some(Screen::Title);
    }
    if app.ui.button(bx + bw / 2.0 + 6.0 * s, by, bw / 2.0 - 6.0 * s, 46.0 * s, "Создать новый мир") {
        app.name_input = format!("Мир {}", app.worlds_list.len() + 1);
        app.focus = Field::Name;
        next = Some(Screen::NewWorld);
    }
    if let Some(n) = to_delete {
        save::delete_world(&n);
        app.worlds_list = save::list_worlds();
    }
    if let Some(n) = to_load {
        if load_game(app, &n) {
            next = Some(Screen::Loading);
        } else {
            app.status = Some(("Не удалось загрузить мир".into(), 3.0));
        }
    }
    next
}

fn draw_new_world(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 1.0));
    app.ui.text_center("Создание мира", w / 2.0, h * 0.13, 34.0, WHITE);
    let bw = 460.0 * s;
    let bh = 46.0 * s;
    let bx = (w - bw) / 2.0;
    let mut y = h * 0.22;
    text_field(app, Field::Name, bx, y, bw, bh, "Название мира:");
    y += bh + 34.0 * s;
    text_field(app, Field::Seed, bx, y, bw, bh, "Зерно мира (пусто — случайное):");
    y += bh + 20.0 * s;
    let mode = if app.new_creative { "Режим: Творчество" } else { "Режим: Выживание" };
    if app.ui.button(bx, y, bw, bh, mode) {
        app.new_creative = !app.new_creative;
    }
    y += bh + 6.0 * s;
    let hint = if app.new_creative { "Полёт (F), все предметы, мгновенное разрушение" } else { "Здоровье и голод, крафт, добыча, существа и их трофеи" };
    app.ui.text_center(hint, w / 2.0, y + 14.0 * s, 14.0, Color::new(0.75, 0.78, 0.85, 1.0));
    y += 30.0 * s;
    let diff = format!("Сложность: {}", settings::DIFFICULTY[app.settings.difficulty as usize]);
    if app.ui.button(bx, y, bw, bh, &diff) {
        app.settings.difficulty = (app.settings.difficulty + 1) % 4;
        app.settings.save();
    }
    y += bh + 26.0 * s;
    let mut next = None;
    if app.ui.button(bx, y, bw / 2.0 - 6.0 * s, bh, "Назад") {
        next = Some(Screen::Worlds);
    }
    if app.ui.button(bx + bw / 2.0 + 6.0 * s, y, bw / 2.0 - 6.0 * s, bh, "Создать мир") || is_key_pressed(KeyCode::Enter) {
        let seed = if app.seed_input.trim().is_empty() { (miniquad::date::now() * 1000.0) as u64 as u32 } else { hash_seed(&app.seed_input) };
        let name = app.name_input.clone();
        start_world(app, seed, app.new_creative, &name);
        app.focus = Field::None;
        next = Some(Screen::Loading);
    }
    next
}

fn servers_path() -> std::path::PathBuf {
    save::saves_dir().parent().map(|p| p.join("servers.txt")).unwrap_or_else(|| "servers.txt".into())
}

fn load_servers() -> Vec<(String, String)> {
    std::fs::read_to_string(servers_path())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('|').map(|(a, b)| (a.to_string(), b.to_string())))
        .collect()
}

fn save_servers(list: &[(String, String)]) {
    let text: String = list.iter().map(|(n, a)| format!("{n}|{a}\n")).collect();
    let _ = std::fs::write(servers_path(), text);
}

fn try_join(app: &mut App, addr: &str) {
    app.settings.save();
    match Net::join(addr.trim(), &app.settings.name) {
        Ok(n) => {
            app.pending_net = Some(n);
            app.focus = Field::None;
        }
        Err(e) => app.status = Some((format!("Не удалось подключиться к {addr}: {e}"), 4.0)),
    }
}

fn multiplayer_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 1.0));
    app.ui.text_center("Сетевая игра", w / 2.0, h * 0.08, 32.0, WHITE);
    if app.finder.is_none() {
        app.finder = net::Finder::start();
    }
    let col_w = (w * 0.42).min(520.0 * s);
    let gap = 24.0 * s;
    let lx = w / 2.0 - col_w - gap / 2.0;
    let rx = w / 2.0 + gap / 2.0;
    let bh = 42.0 * s;
    let top = h * 0.14;
    let mut join: Option<String> = None;

    // левая колонка: имя, найденные игры
    let mut y = top + 18.0 * s;
    text_field(app, Field::Nick, lx, y, col_w, bh, "Ваше имя:");
    y += bh + 30.0 * s;
    app.ui.text("Игры в вашей сети (находятся сами):", lx, y, 16.0, ACCENT);
    y += 12.0 * s;
    let found = app.finder.as_ref().map(|f| f.list()).unwrap_or_default();
    if found.is_empty() {
        app.ui.text("Пока никого… Друг должен нажать Esc → «Открыть для сети».", lx, y + 26.0 * s, 13.0, GRAY);
    }
    for f in found.iter().take(5) {
        draw_rectangle(lx, y + 6.0 * s, col_w, bh + 8.0 * s, Color::new(0.13, 0.16, 0.14, 0.95));
        app.ui.text(&f.info, lx + 10.0 * s, y + 26.0 * s, 14.0, WHITE);
        app.ui.text(&f.addr, lx + 10.0 * s, y + 44.0 * s, 12.0, GRAY);
        if app.pending_net.is_none() && app.ui.button(lx + col_w - 110.0 * s, y + 12.0 * s, 100.0 * s, bh - 4.0 * s, "Войти") {
            join = Some(f.addr.clone());
        }
        y += bh + 14.0 * s;
    }

    // правая колонка: свои серверы и прямое подключение
    let mut y = top + 18.0 * s;
    text_field(app, Field::Ip, rx, y, col_w, bh, "Адрес (IP или имя:порт):");
    y += bh + 8.0 * s;
    let half = (col_w - 8.0 * s) / 2.0;
    if app.pending_net.is_none() && (app.ui.button(rx, y, half, bh, "Подключиться") || (is_key_pressed(KeyCode::Enter) && app.focus == Field::Ip)) {
        join = Some(app.ip_input.clone());
    }
    if app.ui.button(rx + half + 8.0 * s, y, half, bh, "Сохранить в список") && !app.ip_input.trim().is_empty() {
        let mut list = load_servers();
        let addr = app.ip_input.trim().to_string();
        if !list.iter().any(|(_, a)| *a == addr) {
            list.push((format!("Сервер {}", list.len() + 1), addr));
            save_servers(&list);
        }
    }
    y += bh + 26.0 * s;
    app.ui.text("Мои серверы:", rx, y, 16.0, ACCENT);
    y += 12.0 * s;
    let mut list = load_servers();
    let mut remove: Option<usize> = None;
    if list.is_empty() {
        app.ui.text("Список пуст — введите адрес и нажмите «Сохранить».", rx, y + 26.0 * s, 13.0, GRAY);
    }
    for (i, (name, addr)) in list.iter().enumerate().take(5) {
        draw_rectangle(rx, y + 6.0 * s, col_w, bh + 8.0 * s, Color::new(0.13, 0.14, 0.18, 0.95));
        app.ui.text(name, rx + 10.0 * s, y + 26.0 * s, 14.0, WHITE);
        app.ui.text(addr, rx + 10.0 * s, y + 44.0 * s, 12.0, GRAY);
        if app.pending_net.is_none() && app.ui.button(rx + col_w - 200.0 * s, y + 12.0 * s, 92.0 * s, bh - 4.0 * s, "Войти") {
            join = Some(addr.clone());
        }
        if app.ui.button(rx + col_w - 100.0 * s, y + 12.0 * s, 90.0 * s, bh - 4.0 * s, "Удалить") {
            remove = Some(i);
        }
        y += bh + 14.0 * s;
    }
    if let Some(i) = remove {
        list.remove(i);
        save_servers(&list);
    }

    let hy = h * 0.74;
    let hints = [
        "В одной сети (Wi-Fi дома): друг открывает мир (Esc → «Открыть для сети»), игра появится слева сама.",
        "Через интернет: хосту нужен проброс порта 7777 на роутере, либо бесплатный туннель playit.gg,",
        "либо оба ставят Radmin VPN / ZeroTier — тогда это как одна сеть. Мир, существа и время общие.",
    ];
    for (i, l) in hints.iter().enumerate() {
        app.ui.text_center(l, w / 2.0, hy + i as f32 * 20.0 * s, 13.0, Color::new(0.75, 0.78, 0.85, 1.0));
    }
    if app.pending_net.is_some() {
        app.ui.text_center("Подключение…", w / 2.0, h * 0.86, 18.0, ACCENT);
    }
    let mut next = None;
    if let Some(addr) = join {
        try_join(app, &addr);
    }
    if app.ui.button((w - 300.0 * s) / 2.0, h * 0.89, 300.0 * s, bh, "Назад") || (is_key_pressed(KeyCode::Escape) && app.focus != Field::Ip) {
        app.pending_net = None;
        app.focus = Field::None;
        next = Some(Screen::Title);
    }
    next
}

/// Строка чата (T): ввод сообщения.
fn chat_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    if app.just_opened {
        while get_char_pressed().is_some() {}
    } else {
        while let Some(c) = get_char_pressed() {
            if !c.is_control() && app.chat_input.chars().count() < 100 {
                app.chat_input.push(c);
            }
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        app.chat_input.pop();
    }
    let bh = 34.0 * s;
    let y = h - 110.0 * s;
    draw_rectangle(8.0 * s, y, w - 16.0 * s, bh, Color::new(0.0, 0.0, 0.0, 0.6));
    let caret = if (app.time * 2.0).fract() < 0.5 { "|" } else { "" };
    app.ui.text(&format!("> {}{}", app.chat_input, caret), 18.0 * s, y + bh * 0.68, 16.0, WHITE);
    if let Some(g) = app.game.as_ref() {
        // вся недавняя история
        for (i, (t, _)) in g.chat.iter().rev().enumerate() {
            let ly = y - 12.0 * s - i as f32 * 20.0 * s;
            draw_rectangle(8.0 * s, ly - 15.0 * s, app.ui.measure(t, 14.0) + 12.0 * s, 20.0 * s, Color::new(0.0, 0.0, 0.0, 0.45));
            app.ui.text(t, 14.0 * s, ly, 14.0, WHITE);
        }
    }
    if app.just_opened {
        return None;
    }
    if is_key_pressed(KeyCode::Enter) {
        let text = app.chat_input.trim().to_string();
        app.chat_input.clear();
        if !text.is_empty() {
            let name = app.settings.name.clone();
            if let Some(g) = app.game.as_mut() {
                let line = format!("<{name}> {text}");
                if let Some(n) = &g.net {
                    n.broadcast(&Msg::Chat { text: line.clone() }, None);
                }
                g.say(line);
            }
        }
        return Some(Screen::Playing);
    }
    if is_key_pressed(KeyCode::Escape) {
        app.chat_input.clear();
        return Some(Screen::Playing);
    }
    None
}

/// Список игроков (пока зажат Tab).
fn draw_player_list(app: &mut App) {
    let Some(g) = app.game.as_ref() else { return };
    let (w, _) = (screen_width(), screen_height());
    let s = app.ui.s;
    let mut rows = vec![(format!("{} (вы)", app.settings.name), g.player.health)];
    for r in g.remotes.values() {
        rows.push((r.st.name.clone(), r.st.health));
    }
    let pw = 360.0 * s;
    let ph = (40.0 + rows.len() as f32 * 26.0) * s;
    let x = (w - pw) / 2.0;
    let y = 60.0 * s;
    app.ui.panel(x, y, pw, ph);
    let title = match &g.net {
        Some(n) if n.host => format!("Игроки — вы хост · {}", n.info),
        Some(_) => "Игроки на сервере".to_string(),
        None => "Одиночная игра".to_string(),
    };
    app.ui.text_center(&title, w / 2.0, y + 24.0 * s, 14.0, ACCENT);
    for (i, (name, hp)) in rows.iter().enumerate() {
        let ry = y + (48.0 + i as f32 * 26.0) * s;
        app.ui.text(name, x + 16.0 * s, ry, 15.0, WHITE);
        let t = format!("{:.0} ♥", hp.max(0.0));
        app.ui.text(&t, x + pw - app.ui.measure(&t, 14.0) - 16.0 * s, ry, 14.0, Color::new(1.0, 0.5, 0.5, 1.0));
    }
}

fn settings_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    if app.game.is_none() {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 1.0));
    } else {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.55));
    }
    app.ui.text_center("Настройки", w / 2.0, h * 0.07, 34.0, WHITE);
    let cw = 330.0 * s;
    let bh = 34.0 * s;
    let gap = 6.0 * s;
    let x1 = w / 2.0 - cw - gap / 2.0;
    let x2 = w / 2.0 + gap / 2.0;
    let mut y = h * 0.095;
    let old_smooth = app.settings.smooth_light;
    let old_fast = opt::get().fast_leaves;
    let st = &mut app.settings;
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
    if ui.button(x1, y, cw, bh, &format!("Счётчик FPS (F7): {}", onoff(st.show_fps))) {
        st.show_fps = !st.show_fps;
    }
    if ui.button(x2, y, cw, bh, &format!("Инверсия мыши: {}", onoff(st.invert_mouse))) {
        st.invert_mouse = !st.invert_mouse;
    }
    y += bh + gap;
    if ui.button(x1, y, cw, bh, &format!("PvP (урон по игрокам): {}", onoff(st.pvp))) {
        st.pvp = !st.pvp;
    }
    let preset = if st.render_dist <= 5 && !st.smooth_light {
        "Низкое"
    } else if st.render_dist >= 12 && st.smooth_light && st.clouds {
        "Высокое"
    } else {
        "Среднее"
    };
    if ui.button(x2, y, cw, bh, &format!("Качество графики: {preset}")) {
        match preset {
            "Низкое" => {
                st.render_dist = 8;
                st.smooth_light = true;
                st.clouds = true;
                st.render_scale = 1.0;
            }
            "Среднее" => {
                st.render_dist = 12;
                st.smooth_light = true;
                st.clouds = true;
                st.fog = true;
            }
            _ => {
                st.render_dist = 5;
                st.smooth_light = false;
                st.clouds = false;
                st.fog = true;
                st.render_scale = 0.7;
            }
        }
    }
    y += bh + gap;
    let rs = ui.slider(6, x1, y, cw, bh, &format!("Масштаб рендера: {:.0}%", st.render_scale * 100.0), st.render_scale, 0.4, 1.0);
    st.render_scale = (rs * 20.0).round() / 20.0;
    if ui.button(x2, y, cw, bh, &format!("Авто-дальность (60 FPS): {}", onoff(st.auto_dist))) {
        st.auto_dist = !st.auto_dist;
    }
    y += bh + gap;
    if ui.button(x1, y, cw, bh, &format!("Оптимизация: {}", opt::NAMES[st.opt_level as usize])) {
        st.opt_level = (st.opt_level + 1) % 4;
    }
    let lim = if st.fps_limit == 0 { "без ограничения".to_string() } else { st.fps_limit.to_string() };
    if ui.button(x2, y, cw, bh, &format!("Лимит FPS: {lim}")) {
        let i = settings::FPS_LIMITS.iter().position(|&f| f == st.fps_limit).unwrap_or(0);
        st.fps_limit = settings::FPS_LIMITS[(i + 1) % settings::FPS_LIMITS.len()];
    }
    y += bh + 4.0 * s;
    ui.text(&format!("Оптимизация: {}", opt::HINTS[st.opt_level as usize]), x1 + 6.0 * s, y + 14.0 * s, 13.0, Color::new(0.9, 0.9, 0.85, 1.0));
    y += 22.0 * s + gap;
    st.music_vol = ui.slider(7, x1, y, cw, bh, &format!("Музыка: {:.0}%", st.music_vol * 100.0), st.music_vol, 0.0, 1.0);
    st.sound_vol = ui.slider(8, x2, y, cw, bh, &format!("Звуки: {:.0}%", st.sound_vol * 100.0), st.sound_vol, 0.0, 1.0);
    y += bh + gap;
    let open_tex = ui.button(x1, y, cw, bh, "Текстуры и небо…");
    let vs_note = if Some(&st.vsync) != VSYNC_AT_START.get() { " (после перезапуска)" } else { "" };
    if ui.button(x2, y, cw, bh, &format!("Верт. синхронизация: {}{vs_note}", onoff(st.vsync))) {
        st.vsync = !st.vsync;
    }
    y += bh + gap;
    if ui.button(x1, y, cw, bh, &format!("Мини-карта и компас: {}", onoff(st.minimap))) {
        st.minimap = !st.minimap;
    }
    let open_look = ui.button(x2, y, cw, bh, "Внешность игрока…");
    y += bh + gap;
    if ui.button(x1, y, cw, bh, &format!("Сенсорное управление: {}", onoff(st.touch))) {
        st.touch = !st.touch;
    }
    let open_controls = ui.button(x2, y, cw, bh, "Настроить кнопки…");
    y += bh + gap;
    let exporting = app.export_rx.is_some();
    let export = ui.button(x1, y, cw, bh, if exporting { "Сохраняю музыку…" } else { "Сохранить музыку в папку music" });
    let done = ui.button(x2, y, cw, bh, "Готово") || is_key_pressed(KeyCode::Escape);
    if export && !exporting {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let msg = match audio::export_music() {
                Ok(d) => format!("Музыка сохранена: {}", d.display()),
                Err(e) => format!("Не удалось сохранить музыку: {e}"),
            };
            let _ = tx.send(msg);
        });
        app.export_rx = Some(rx);
    }
    if let Some(rx) = &app.export_rx {
        if let Ok(msg) = rx.try_recv() {
            app.status = Some((msg, 6.0));
            app.export_rx = None;
        }
    }
    st.clamp();
    opt::set_level(st.opt_level);
    if st.smooth_light != old_smooth || opt::get().fast_leaves != old_fast {
        app.loader.epoch += 1;
    }
    if open_look {
        return Some(Screen::Look);
    }
    if open_tex {
        app.settings.save();
        app.pack_list = packs::list();
        return Some(Screen::Textures);
    }
    if open_controls {
        app.settings.save();
        return Some(Screen::Controls);
    }
    if done {
        app.settings.save();
        return Some(app.settings_back);
    }
    None
}

fn leave_world(app: &mut App) {
    save_game(app);
    let ctx = unsafe { get_internal_gl() }.quad_context;
    app.loader.clear(ctx);
    app.game = None;
}

fn pause_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.5));
    app.ui.text_center("Пауза", w / 2.0, h * 0.2, 36.0, WHITE);
    let bw = 400.0 * s;
    let bh = 44.0 * s;
    let bx = (w - bw) / 2.0;
    let mut y = h * 0.27;
    let mut next = None;
    if app.ui.button(bx, y, bw, bh, "Продолжить") {
        next = Some(Screen::Playing);
    }
    y += bh + 8.0 * s;
    let is_client = app.game.as_ref().is_some_and(|g| g.is_client());
    let creative = app.game.as_ref().is_some_and(|g| g.creative);
    let label = if creative { "Режим: Творчество" } else { "Режим: Выживание" };
    if app.ui.button_ex(bx, y, bw, bh, label, !is_client) {
        if let Some(g) = app.game.as_mut() {
            g.creative = !g.creative;
            g.player.flying = false;
        }
    }
    y += bh + 8.0 * s;
    if app.ui.button(bx, y, bw, bh, "Настройки") {
        app.settings_back = Screen::Paused;
        next = Some(Screen::Settings);
    }
    y += bh + 8.0 * s;
    let net_info = app.game.as_ref().and_then(|g| g.net.as_ref().map(|n| (n.info.clone(), n.peer_count())));
    match net_info {
        Some((info, peers)) => {
            app.ui.text_center(&format!("{info} · игроков: {}", peers + 1), w / 2.0, y + bh * 0.6, 15.0, ACCENT);
        }
        None => {
            let half = (bw - 8.0 * s) / 2.0;
            if app.ui.button(bx + half + 8.0 * s, y, half, bh, "Через интернет…") {
                next = Some(Screen::Relay);
            }
            if app.ui.button(bx, y, half, bh, "Открыть для сети") {
                match Net::host(&app.settings.name) {
                    Ok(n) => {
                        let g = app.game.as_mut().unwrap();
                        g.toast = Some((n.info.clone(), 6.0));
                        g.say(n.info.clone());
                        g.net = Some(n);
                    }
                    Err(e) => app.status = Some((format!("Не удалось открыть сервер: {e}"), 4.0)),
                }
            }
        }
    }
    y += bh + 8.0 * s;
    if !is_client && app.ui.button(bx, y, bw, bh, "Сохранить") {
        let ok = save_game(app);
        app.status = Some((if ok { "Мир сохранён".into() } else { "Не удалось сохранить".into() }, 2.0));
    }
    y += bh + 8.0 * s;
    if app.ui.button(bx, y, bw, bh, if is_client { "Отключиться" } else { "Сохранить и выйти в меню" }) {
        leave_world(app);
        next = Some(Screen::Title);
    }
    y += bh + 8.0 * s;
    if app.ui.button(bx, y, bw, bh, "Выйти из игры") {
        save_game(app);
        std::process::exit(0);
    }
    next
}

fn dead_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.45, 0.0, 0.0, 0.45));
    app.ui.text_center("Вы погибли!", w / 2.0, h * 0.3, 48.0, WHITE);
    let creative = app.game.as_ref().is_some_and(|g| g.creative);
    if !creative {
        app.ui.text_center("Вещи остались на месте гибели", w / 2.0, h * 0.36, 16.0, Color::new(1.0, 0.85, 0.85, 1.0));
    }
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
        g.player.hunger = MAX_HUNGER;
        g.player.saturation = 5.0;
        g.player.fall_start = None;
        g.player.effects.clear();
        g.spawn_pending = false;
        return Some(Screen::Loading);
    }
    if app.ui.button(bx, h * 0.42 + bh + 12.0 * s, bw, bh, "Главное меню") {
        leave_world(app);
        return Some(Screen::Title);
    }
    None
}

fn inventory_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.45));
    let creative = app.game.as_ref().unwrap().creative;
    let slot = 44.0 * s;
    let gap = 5.0 * s;
    let grid_w = 9.0 * (slot + gap) - gap;
    let left_w = 160.0 * s;
    let right_w = if creative { 10.0 * (slot + gap) } else { 420.0 * s };
    let pw = left_w + grid_w.max(0.0) + right_w + 70.0 * s;
    let pw = pw.min(w - 20.0);
    let ph = (h * 0.9).min(660.0 * s);
    let px = (w - pw) / 2.0;
    let py = (h - ph) / 2.0;
    app.ui.panel(px, py, pw, ph);
    app.ui.text(if creative { "Инвентарь — Творчество" } else { "Инвентарь и крафт" }, px + 18.0 * s, py + 32.0 * s, 20.0, WHITE);

    let mut cursor = app.cursor;
    let mut drop_out: Option<Stack> = None;
    let g = app.game.as_mut().unwrap();

    // броня и характеристики
    let ax = px + 18.0 * s;
    let mut ay = py + 66.0 * s;
    app.ui.text("Броня", ax, ay - 8.0 * s, 15.0, ACCENT);
    let names = ["Шлем", "Нагрудник", "Поножи", "Ботинки"];
    for i in 0..4 {
        let (_, click, _) = app.ui.slot(ax, ay, slot, g.player.armor[i], false);
        if g.player.armor[i] == Item::Empty {
            app.ui.text(names[i], ax + slot + 8.0 * s, ay + slot * 0.6, 12.0, GRAY);
        }
        if click {
            match cursor.item {
                Item::Armor(sl, _) if sl as usize == i => {
                    let old = g.player.armor[i];
                    let old_e = std::mem::replace(&mut g.player.armor_ench[i], cursor.ench);
                    g.player.armor[i] = cursor.item;
                    cursor = Stack::new(old, if old == Item::Empty { 0 } else { 1 });
                    cursor.ench = if old == Item::Empty { 0 } else { old_e };
                }
                Item::Empty => {
                    let old = g.player.armor[i];
                    g.player.armor[i] = Item::Empty;
                    cursor = Stack::new(old, 1);
                    cursor.ench = std::mem::take(&mut g.player.armor_ench[i]);
                }
                _ => {}
            }
        }
        ay += slot + gap;
    }
    let p = &g.player;
    let stats = [
        format!("Защита: {}", p.armor_points()),
        format!("Урон: {:.1}", p.held().damage() * p.damage_bonus()),
        format!("Здоровье: {:.0}/20", p.health),
        format!("Сытость: {:.0}/20", p.hunger),
    ];
    for (i, l) in stats.iter().enumerate() {
        app.ui.text(l, ax, ay + (18.0 + i as f32 * 20.0) * s, 14.0, WHITE);
    }

    // инвентарь игрока
    let gx = px + left_w + 30.0 * s;
    let inv_y = py + ph - 4.0 * (slot + gap) - 30.0 * s;
    app.ui.text("Рюкзак", gx, inv_y - 10.0 * s, 15.0, ACCENT);
    for i in 0..36 {
        let (col, row) = if i < 9 { (i, 3) } else { ((i - 9) % 9, (i - 9) / 9) };
        let x = gx + col as f32 * (slot + gap);
        let y = inv_y + row as f32 * (slot + gap) + if row == 3 { 8.0 * s } else { 0.0 };
        let st = g.player.inv.slots[i];
        let (hov, click, rclick) = app.ui.slot(x, y, slot, st.item, i == g.player.selected);
        if st.ench != 0 {
            let a = 0.18 + (app.time * 3.0 + i as f32).sin().abs() * 0.15;
            draw_rectangle(x, y, slot, slot, Color::new(0.65, 0.35, 1.0, a));
            if hov {
                let desc = st.item.description().map_or(String::new(), |d| format!(" · {d}"));
                app.ui.set_tooltip(st.item.name(), Some(format!("{}{desc}", ench::name(st.ench).unwrap_or_default())));
            }
        }
        if st.count > 1 {
            app.ui.text(&st.count.to_string(), x + slot - app.ui.measure(&st.count.to_string(), 12.0) - 3.0 * s, y + slot - 4.0 * s, 12.0, WHITE);
        }
        if click || rclick {
            if creative && is_key_down(KeyCode::LeftShift) && click {
                g.player.inv.slots[i] = Stack::EMPTY;
            } else {
                g.player.inv.click(i, &mut cursor, rclick);
            }
        }
    }

    // правая часть: каталог (творчество) или крафт (выживание)
    let rx = gx + grid_w + 24.0 * s;
    let top = py + 52.0 * s;
    if creative {
        let cat_x = gx;
        let cat_w = rx + right_w - gx;
        let tab_w = (cat_w - gap * 5.0) / 6.0;
        for (i, name) in items::TABS.iter().enumerate() {
            let x = cat_x + (i % 6) as f32 * (tab_w + gap);
            let y = top + (i / 6) as f32 * (30.0 * s + gap);
            if app.ui.button_small(x, y, tab_w, 30.0 * s, name) {
                app.inv_tab = i;
                app.inv_scroll = 0.0;
            }
            if app.inv_tab == i {
                draw_rectangle(x, y + 27.0 * s, tab_w, 3.0 * s, WHITE);
            }
        }
        let cat = items::catalog(app.inv_tab);
        let cols = ((cat_w + gap) / (slot + gap)) as usize;
        let grid_top = top + 2.0 * (30.0 * s + gap) + 8.0 * s;
        let grid_h = inv_y - 26.0 * s - grid_top;
        let rows = cat.len().div_ceil(cols);
        let vis = (grid_h / (slot + gap)).floor().max(1.0) as usize;
        let max_scroll = rows.saturating_sub(vis) as f32;
        let (_, wheel) = mouse_wheel();
        app.inv_scroll = (app.inv_scroll - wheel.signum()).clamp(0.0, max_scroll);
        let first = app.inv_scroll as usize * cols;
        for (k, item) in cat.iter().enumerate().skip(first).take(vis * cols) {
            let i = k - first;
            let x = cat_x + (i % cols) as f32 * (slot + gap);
            let y = grid_top + (i / cols) as f32 * (slot + gap);
            let (_, click, rclick) = app.ui.slot(x, y, slot, *item, false);
            if click {
                cursor = Stack::new(*item, item.max_stack());
            } else if rclick {
                cursor = Stack::new(*item, 1);
            }
        }
        if max_scroll > 0.0 {
            app.ui.text(&format!("Колесо — прокрутка ({}/{})", app.inv_scroll as usize + 1, max_scroll as usize + 1), cat_x, inv_y - 30.0 * s, 12.0, GRAY);
        }
    } else {
        let stations = stations_near(g);
        let st_names: Vec<&str> = stations.iter().skip(1).map(|s| s.name()).collect();
        app.ui.text("Крафт", rx, top + 10.0 * s, 18.0, ACCENT);
        let near = if st_names.is_empty() { "рядом нет станций".to_string() } else { format!("рядом: {}", st_names.join(", ")) };
        app.ui.text(&near, rx + 70.0 * s, top + 10.0 * s, 12.0, GRAY);
        if app.ui.button_small(rx + right_w - 150.0 * s, top - 6.0 * s, 150.0 * s, 26.0 * s, if app.craft_all { "Показать: все" } else { "Показать: доступные" }) {
            app.craft_all = !app.craft_all;
            app.craft_scroll = 0.0;
        }
        let list: Vec<&crafting::Recipe> = crafting::recipes()
            .iter()
            .filter(|r| app.craft_all || (r.can_craft(&g.player.inv) && stations.contains(&r.station)))
            .collect();
        let row_h = 40.0 * s;
        let list_top = top + 28.0 * s;
        let list_h = ph - (list_top - py) - 16.0 * s;
        let vis = (list_h / row_h).floor() as usize;
        let max_scroll = list.len().saturating_sub(vis) as f32;
        let (_, wheel) = mouse_wheel();
        if app.ui.hover(rx, list_top, right_w, list_h) {
            app.craft_scroll = (app.craft_scroll - wheel.signum() * 2.0).clamp(0.0, max_scroll);
        }
        if list.is_empty() {
            app.ui.text("Нечего сделать: соберите материалы", rx, list_top + 30.0 * s, 14.0, GRAY);
            app.ui.text("или подойдите к верстаку/печи/котлу.", rx, list_top + 50.0 * s, 14.0, GRAY);
        }
        let mut craft: Option<&crafting::Recipe> = None;
        for (k, r) in list.iter().enumerate().skip(app.craft_scroll as usize).take(vis) {
            let i = k - app.craft_scroll as usize;
            let y = list_top + i as f32 * row_h;
            let ok = r.can_craft(&g.player.inv) && stations.contains(&r.station);
            let hov = app.ui.hover(rx, y, right_w, row_h - 3.0 * s);
            draw_rectangle(rx, y, right_w, row_h - 3.0 * s, if hov && ok { Color::new(0.3, 0.3, 0.2, 0.9) } else if ok { Color::new(0.16, 0.2, 0.14, 0.9) } else { Color::new(0.12, 0.12, 0.14, 0.8) });
            app.ui.item_icon(r.out.0, rx + 2.0 * s, y + 1.0 * s, row_h - 6.0 * s);
            let title = if r.out.1 > 1 { format!("{} ×{}", r.out.0.name(), r.out.1) } else { r.out.0.name() };
            app.ui.text(&title, rx + row_h + 2.0 * s, y + 15.0 * s, 12.5, if ok { WHITE } else { GRAY });
            let mut ix = rx + row_h + 2.0 * s;
            for (it, n) in &r.ins {
                let have = g.player.inv.count(*it) >= *n as u32;
                app.ui.item_icon(*it, ix, y + 17.0 * s, 18.0 * s);
                app.ui.text(&format!("{n}"), ix + 19.0 * s, y + 31.0 * s, 11.0, if have { WHITE } else { Color::new(1.0, 0.45, 0.45, 1.0) });
                ix += 40.0 * s;
            }
            if r.station != crafting::Station::Hand {
                let has_st = stations.contains(&r.station);
                app.ui.text(r.station.name(), rx + right_w - app.ui.measure(r.station.name(), 11.0) - 6.0 * s, y + 31.0 * s, 11.0, if has_st { ACCENT } else { Color::new(1.0, 0.45, 0.45, 1.0) });
            }
            if hov {
                app.ui.set_tooltip(r.out.0.name(), r.out.0.description());
                if app.ui.pressed && ok {
                    craft = Some(r);
                }
            }
        }
        if let Some(r) = craft {
            let times = if is_key_down(KeyCode::LeftShift) { 8 } else { 1 };
            let mut made = 0;
            for _ in 0..times {
                if crafting::craft(r, &mut g.player.inv) {
                    made += 1;
                }
            }
            if made > 0 {
                g.toast = Some((format!("Сделано: {} ×{}", r.out.0.name(), r.out.1 as u32 * made), 1.5));
            }
        }
    }

    // предмет «на курсоре»
    if !cursor.is_empty() {
        let m = app.ui.mouse;
        app.ui.item_icon(cursor.item, m.x - slot * 0.4, m.y - slot * 0.4, slot * 0.8);
        if cursor.count > 1 {
            app.ui.text(&cursor.count.to_string(), m.x + slot * 0.2, m.y + slot * 0.4, 12.0, WHITE);
        }
        if app.ui.pressed && !app.ui.hover(px, py, pw, ph) {
            drop_out = Some(cursor);
            cursor = Stack::EMPTY;
        }
    }
    let close = !app.just_opened && (is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Escape));
    if close && !cursor.is_empty() {
        let left = g.player.inv.add(cursor.item, cursor.count);
        if left > 0 {
            drop_out = Some(Stack::new(cursor.item, left));
        }
        cursor = Stack::EMPTY;
    }
    if let Some(d) = drop_out {
        if !creative {
            let pos = g.player.eye() + g.player.forward() * 0.8;
            g.ents.spawn_drop(d.item, d.count, pos, 0.3);
        }
    }
    app.cursor = cursor;
    if close {
        return Some(Screen::Playing);
    }
    None
}

// ---------------------------------------------------------------------------
// Интерфейс поверх игры
// ---------------------------------------------------------------------------

fn draw_hud(app: &mut App) {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    let g = app.game.as_ref().unwrap();
    if g.flash_t > 0.0 {
        draw_rectangle(0.0, 0.0, w, h, Color::new(1.0, 1.0, 1.0, (g.flash_t / 0.35) * 0.7));
    }
    if g.sleep_t > 0.0 {
        let a = (1.0 - g.sleep_t / 2.5).clamp(0.0, 1.0);
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.03, a * 0.95));
    }

    if !g.third_person {
        let c = Color::new(1.0, 1.0, 1.0, 0.85);
        draw_rectangle(w / 2.0 - 10.0 * s, h / 2.0 - 1.0 * s, 20.0 * s, 2.0 * s, c);
        draw_rectangle(w / 2.0 - 1.0 * s, h / 2.0 - 10.0 * s, 2.0 * s, 20.0 * s, c);
        // индикатор зарядки удара под прицелом
        let charge = 1.0 - g.attack_cd / g.attack_max.max(0.01);
        if charge < 1.0 {
            let (bw, bh) = (32.0 * s, 4.0 * s);
            let (bx, by) = (w / 2.0 - bw / 2.0, h / 2.0 + 16.0 * s);
            draw_rectangle(bx - s, by - s, bw + 2.0 * s, bh + 2.0 * s, Color::new(0.0, 0.0, 0.0, 0.55));
            draw_rectangle(bx, by, bw * charge, bh, Color::new(1.0, 1.0, 1.0, 0.9));
        }
        if g.crit_t > 0.0 {
            let a = (g.crit_t / 0.4).min(1.0);
            app.ui.text("Крит!", w / 2.0 - app.ui.measure("Крит!", 16.0) / 2.0, h / 2.0 - 18.0 * s, 16.0, Color::new(1.0, 0.85, 0.3, a));
        }
    }

    if app.settings.touch && app.screen == Screen::Playing && !app.hide_hud {
        touch::draw(&app.touch_layout, &app.ui, g.creative);
    }
    // полоска здоровья ближайшего босса
    let ppos = g.player.body.pos;
    if let Some(b) = g.mobs.list.iter().filter(|m| m.is_boss() && m.alive() && (m.body.pos - ppos).length() < 64.0).min_by(|a, b| (a.body.pos - ppos).length().total_cmp(&(b.body.pos - ppos).length())) {
        let bw = (w * 0.5).min(520.0 * s);
        let bx = (w - bw) / 2.0;
        let by = 34.0 * s;
        let f = b.health_frac();
        app.ui.text_center(b.d().name, w / 2.0, by - 8.0 * s, 17.0, if b.enraged { Color::new(1.0, 0.45, 0.35, 1.0) } else { WHITE });
        draw_rectangle(bx - 2.0 * s, by - 2.0 * s, bw + 4.0 * s, 14.0 * s, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_rectangle(bx, by, bw, 10.0 * s, Color::new(0.25, 0.05, 0.08, 0.9));
        let c = if b.enraged { Color::new(0.95, 0.25, 0.15, 1.0) } else { Color::new(0.75, 0.2, 0.85, 1.0) };
        draw_rectangle(bx, by, bw * f, 10.0 * s, c);
        draw_rectangle(bx, by, bw * f, 3.0 * s, Color::new(1.0, 1.0, 1.0, 0.25));
        for k in 1..4 {
            draw_rectangle(bx + bw * k as f32 / 4.0 - s, by, 2.0 * s, 10.0 * s, Color::new(0.0, 0.0, 0.0, 0.5));
        }
    }

    let slot = 46.0 * s;
    let gap = 5.0 * s;
    let total = 9.0 * (slot + gap) - gap;
    let x0 = (w - total) / 2.0;
    let y0 = h - slot - 12.0 * s;
    draw_rectangle(x0 - 8.0 * s, y0 - 8.0 * s, total + 16.0 * s, slot + 16.0 * s, Color::new(0.04, 0.05, 0.07, 0.55));
    for i in 0..9 {
        let x = x0 + i as f32 * (slot + gap);
        draw_rectangle(x, y0, slot, slot, Color::new(0.1, 0.11, 0.13, 0.75));
        if g.player.selected == i {
            draw_rectangle_lines(x - 3.0 * s, y0 - 3.0 * s, slot + 6.0 * s, slot + 6.0 * s, 4.0 * s, ACCENT);
        }
        let st = g.player.inv.slots[i];
        app.ui.item_icon(st.item, x, y0, slot);
        if st.ench != 0 {
            draw_rectangle(x, y0, slot, slot, Color::new(0.65, 0.35, 1.0, 0.18 + (app.time * 3.0).sin().abs() * 0.12));
        }
        if st.count > 1 {
            let t = st.count.to_string();
            app.ui.text(&t, x + slot - app.ui.measure(&t, 12.0) - 3.0 * s, y0 + slot - 4.0 * s, 12.0, WHITE);
        }
        app.ui.text(&(i + 1).to_string(), x + 3.0 * s, y0 + 12.0 * s, 10.0, Color::new(1.0, 1.0, 1.0, 0.55));
    }
    // компас: стороны света и метки по направлению взгляда
    if !g.third_person || true {
        let cw = 360.0 * s;
        let cx0 = w / 2.0 - cw / 2.0;
        let cy = 8.0 * s;
        draw_rectangle(cx0, cy, cw, 20.0 * s, Color::new(0.0, 0.0, 0.0, 0.35));
        let yaw = g.player.yaw;
        let fov_h = 2.0;
        let mark = |ang: f32, label: &str, col: Color, size: f32| {
            let mut d = ang - yaw;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            if d.abs() < fov_h / 2.0 {
                let x = w / 2.0 + d / fov_h * cw;
                app.ui.text_center(label, x, cy + 15.0 * s, size, col);
            }
        };
        // север — к −z
        for (ang, l) in [(-std::f32::consts::FRAC_PI_2, "С"), (0.0, "В"), (std::f32::consts::FRAC_PI_2, "Ю"), (std::f32::consts::PI, "З")] {
            mark(ang, l, if l == "С" { Color::new(1.0, 0.4, 0.35, 1.0) } else { WHITE }, 14.0);
        }
        let dc = dim_code(g.dim);
        let me = g.player.body.pos;
        for (d, p, _) in &g.markers {
            if *d == dc {
                mark((p.y - me.z).atan2(p.x - me.x), "◆", Color::new(1.0, 0.85, 0.2, 1.0), 13.0);
            }
        }
        draw_rectangle(w / 2.0 - 1.0, cy, 2.0, 20.0 * s, Color::new(1.0, 1.0, 1.0, 0.6));
    }
    if app.settings.minimap && !app.hide_hud {
        let size = 150.0 * s;
        let me = vec2(g.player.body.pos.x, g.player.body.pos.z);
        let dc = dim_code(g.dim);
        let mk: Vec<map::Marker> = g.markers.iter().filter(|m| m.0 == dc).map(|(_, p, n)| map::Marker { pos: *p, label: n.clone(), color: Color::new(1.0, 0.85, 0.2, 1.0), yaw: None }).collect();
        app.minimap.draw(g.seed, g.dim, me, g.player.yaw, &mk, w - size - 12.0 * s, 12.0 * s, size);
        app.ui.text_center("С", w - size / 2.0 - 12.0 * s, 26.0 * s, 13.0, Color::new(1.0, 0.4, 0.35, 1.0));
    }
    // всплывающие достижения
    for (k, (id, t)) in g.ach_pop.iter().enumerate() {
        let a = ach::LIST[*id as usize].name;
        let pw = 300.0 * s;
        let x = w - pw - 14.0 * s + if *t > 4.6 { (*t - 4.6) * 600.0 * s } else { 0.0 };
        let y = 180.0 * s + k as f32 * 62.0 * s;
        draw_rectangle(x, y, pw, 54.0 * s, Color::new(0.08, 0.06, 0.12, 0.92));
        draw_rectangle_lines(x, y, pw, 54.0 * s, 2.5 * s, Color::new(1.0, 0.8, 0.3, 1.0));
        app.ui.text("Достижение!", x + 12.0 * s, y + 20.0 * s, 13.0, Color::new(1.0, 0.8, 0.3, 1.0));
        app.ui.text(a, x + 12.0 * s, y + 42.0 * s, 16.0, WHITE);
    }
    // опыт: полоска над панелью и уровень
    if !g.creative {
        let (lvl, frac) = g.player.level();
        let by = y0 - 15.0 * s;
        draw_rectangle(x0, by, total, 5.0 * s, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_rectangle(x0, by, total * frac, 5.0 * s, Color::new(0.5, 1.0, 0.3, 1.0));
        if lvl > 0 {
            app.ui.text_center(&lvl.to_string(), w / 2.0, by - 2.0 * s, 15.0, Color::new(0.6, 1.0, 0.4, 1.0));
        }
    }
    // натяжение лука / зарядка арбалета, блок щитом
    if g.draw_t > 0.0 {
        draw_rectangle(w / 2.0 - 30.0 * s, h / 2.0 + 24.0 * s, 60.0 * s, 4.0 * s, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_rectangle(w / 2.0 - 30.0 * s, h / 2.0 + 24.0 * s, 60.0 * s * g.draw_t.min(1.0), 4.0 * s, Color::new(1.0, 0.8, 0.3, 1.0));
    }
    if g.cross_loaded && matches!(g.player.held(), Item::Tool(items::ToolKind::Crossbow, _)) {
        app.ui.text_center("Заряжен", w / 2.0, h / 2.0 + 36.0 * s, 12.0, Color::new(1.0, 0.8, 0.3, 1.0));
    }
    if g.player.blocking {
        app.ui.text_center("Блок щитом", w / 2.0, h / 2.0 + 36.0 * s, 12.0, Color::new(0.7, 0.85, 1.0, 1.0));
    }
    // полоска поедания
    if g.player.eat_t > 0.0 {
        let t = (g.player.eat_t / 1.2).min(1.0);
        draw_rectangle(w / 2.0 - 60.0 * s, h / 2.0 + 24.0 * s, 120.0 * s, 6.0 * s, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_rectangle(w / 2.0 - 60.0 * s, h / 2.0 + 24.0 * s, 120.0 * s * t, 6.0 * s, Color::new(0.9, 0.6, 0.2, 1.0));
    }

    if !g.creative {
        let hs = 18.0 * s;
        let hy = y0 - hs - 12.0 * s;
        for i in 0..10 {
            let v = g.player.health - i as f32 * 2.0;
            let tile = if v >= 2.0 { items::HUD_HEART_FULL } else if v >= 1.0 { items::HUD_HEART_HALF } else { items::HUD_HEART_EMPTY };
            let shake = if g.player.health <= 4.0 { ((app.time * 30.0 + i as f32).sin() * 1.5 * s).round() } else { 0.0 };
            app.ui.sprite(blocks::item_tile(tile), x0 + i as f32 * (hs + 1.0), hy + shake, hs, 0.0, WHITE);
            let f = g.player.hunger - i as f32 * 2.0;
            let ft = if f >= 2.0 { items::HUD_FOOD_FULL } else if f >= 1.0 { items::HUD_FOOD_HALF } else { items::HUD_FOOD_EMPTY };
            let fshake = if g.player.hunger <= 3.0 { ((app.time * 25.0 + i as f32 * 2.0).sin() * 1.5 * s).round() } else { 0.0 };
            app.ui.sprite(blocks::item_tile(ft), x0 + total - (i + 1) as f32 * (hs + 1.0), hy + fshake, hs, 0.0, WHITE);
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

    if g.label_t > 0.0 {
        let item = g.player.held();
        if item != Item::Empty {
            let a = g.label_t.min(1.0);
            let off = if g.creative { 28.0 } else { 70.0 };
            app.ui.text_center(&item.name(), w / 2.0, y0 - off * s, 17.0, Color::new(1.0, 1.0, 1.0, a));
        }
    }
    if let Some((t, time)) = &g.toast {
        app.ui.text_center(t, w / 2.0, h * 0.2, 24.0, Color::new(1.0, 0.9, 0.6, time.min(1.0)));
    }
    // эффекты
    for (i, (e, t)) in g.player.effects.iter().enumerate() {
        let txt = format!("{} {:.0} с", e.name(), t);
        app.ui.text(&txt, w - app.ui.measure(&txt, 14.0) - 14.0 * s, (30.0 + i as f32 * 20.0) * s, 14.0, Color::new(0.7, 0.9, 1.0, 1.0));
    }
    // чат
    for (i, (t, life)) in g.chat.iter().rev().enumerate() {
        let y = y0 - (90.0 + i as f32 * 20.0) * s;
        let a = life.min(1.0);
        draw_rectangle(8.0 * s, y - 15.0 * s, app.ui.measure(t, 14.0) + 12.0 * s, 20.0 * s, Color::new(0.0, 0.0, 0.0, 0.35 * a));
        app.ui.text(t, 14.0 * s, y, 14.0, Color::new(1.0, 1.0, 1.0, a));
    }

    if app.show_debug {
        let p = g.player.body.pos;
        let (cx, cz) = chunk_of(p.x.floor() as i32, p.z.floor() as i32);
        let biome = if g.dim == Dim::Heaven { worldgen::Biome::Islands } else { worldgen::column(g.seed, p.x as i32, p.z as i32).biome };
        let hours = (g.time * 24.0 + 6.0) % 24.0;
        let (fr, ho) = g.mobs.counts();
        let lines = [
            format!("Расткрафт 0.3   {:.0} к/с", app.fps),
            format!("XYZ: {:.1} / {:.1} / {:.1}", p.x, p.y, p.z),
            format!("Чанк: {cx} {cz}   Измерение: {}", g.dim.name()),
            format!("Биом: {}", biome.name()),
            format!("Чанков в памяти: {}   отрисовано: {}   дальность: {}/{}", g.world().chunks.len(), app.loader.gpu.len(), app.eff_dist, app.settings.render_dist),
            format!("Время: {:02}:{:02}   Существ: {} мирных, {} злых", hours as i32, (hours.fract() * 60.0) as i32, fr, ho),
            format!("Предметов на земле: {}   Игроков онлайн: {}", g.ents.drops.len(), g.remotes.len() + 1),
            format!("Зерно: {}   Мир: {}", g.seed, if g.name.is_empty() { "сетевой" } else { &g.name }),
        ];
        draw_rectangle(6.0 * s, 6.0 * s, 470.0 * s, (lines.len() as f32 * 20.0 + 14.0) * s, Color::new(0.0, 0.0, 0.0, 0.45));
        for (i, l) in lines.iter().enumerate() {
            app.ui.text(l, 14.0 * s, (26.0 + i as f32 * 20.0) * s, 14.0, WHITE);
        }
    } else if app.settings.show_fps {
        // при сенсорном управлении слева вверху кнопки — счётчик ниже них
        let fy = if app.settings.touch { screen_height() * 0.16 + 20.0 * s } else { 24.0 * s };
        app.ui.text(&format!("{:.0} FPS", app.fps), 12.0 * s, fy, 15.0, Color::new(1.0, 1.0, 0.6, 0.95));
    }
}

fn player_light(g: &Game) -> f32 {
    let e = g.player.eye();
    let exposed = g.world().sky_exposed(e.x.floor() as i32, e.y.floor() as i32, e.z.floor() as i32);
    if exposed { g.daylight().max(0.3) } else { 0.5 }
}

/// Можно ли рисовать 3D в текстуру с буфером глубины (на старых телефонах с
/// OpenGL ES 2 без расширения OES_depth_texture — нельзя, рисуем сразу на экран).
fn depth_target_ok() -> bool {
    static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OK.get_or_init(|| {
        if !cfg!(target_os = "android") {
            return true;
        }
        let get = |name| unsafe {
            let p = macroquad::miniquad::gl::glGetString(name);
            if p.is_null() { String::new() } else { std::ffi::CStr::from_ptr(p as *const _).to_string_lossy().into_owned() }
        };
        let ver = get(macroquad::miniquad::gl::GL_VERSION);
        let ext = get(macroquad::miniquad::gl::GL_EXTENSIONS);
        ver.contains("ES 3") || ext.contains("depth_texture")
    })
}

/// 3D-сцена: небо, мир, существа, игроки, предметы, рука.
fn render_world(app: &mut App) {
    // масштаб рендера: 3D рисуется в текстуру меньшего размера и растягивается
    let (sw, sh) = (screen_width(), screen_height());
    let scale = app.settings.render_scale;
    let use_rt = scale < 0.99 && depth_target_ok();
    if use_rt {
        let (w, h) = (((sw * scale) as u32).max(64), ((sh * scale) as u32).max(64));
        if app.rt.as_ref().is_none_or(|(_, rw, rh)| *rw != w || *rh != h) {
            let rt = render_target_ex(w, h, RenderTargetParams { sample_count: 1, depth: true });
            rt.texture.set_filter(FilterMode::Linear);
            app.rt = Some((rt, w, h));
        }
    } else {
        app.rt = None;
    }
    let rt = app.rt.as_ref().map(|r| r.0.clone());
    let pass = rt.as_ref().map(|r| r.render_pass.raw_miniquad_id());
    let g = app.game.as_ref().unwrap();
    let (top, hor) = sky_colors(g);
    if let Some(p) = pass {
        let mut gl = unsafe { get_internal_gl() };
        gl.flush();
        gl.quad_context.begin_pass(Some(p), miniquad::PassAction::clear_color(hor.r, hor.g, hor.b, 1.0));
        gl.quad_context.end_render_pass();
    } else {
        clear_background(hor);
    }
    let dir = g.player.forward();
    let right0 = dir.cross(Vec3::Y).normalize_or_zero();
    let walk_amp = if app.settings.view_bob && !g.player.flying && g.player.body.on_ground {
        (vec2(g.player.body.vel.x, g.player.body.vel.z).length() / 4.4).min(1.3)
    } else {
        0.0
    };
    // покачивание «восьмёркой» при ходьбе
    let wp = g.player.walk_phase;
    let bob = right0 * wp.sin() * 0.05 * walk_amp + Vec3::Y * (-(wp.cos().abs()) * 0.07 * walk_amp + 0.035 * walk_amp);
    // лёгкий крен при ходьбе и при получении урона
    let roll = wp.sin() * 0.012 * walk_amp + (g.player.hurt_t * 9.0).sin() * g.player.hurt_t * 0.25;
    let cam_up = (Vec3::Y * roll.cos() + right0 * roll.sin()).normalize();
    let mut eye = g.player.eye() + bob;
    if g.third_person {
        // камера позади игрока, не заходит в стены
        let back = g.world().raycast(eye, -dir, 4.0).map_or(4.0, |h| (h.2 - 0.3).max(0.5));
        eye -= dir * back;
    }
    let zoom = is_key_down(KeyCode::C) && app.screen == Screen::Playing;
    let fovy = app.settings.fov.to_radians() * if zoom { 0.35 } else { 1.0 };
    let aspect = screen_width() / screen_height().max(1.0);
    let proj = Mat4::perspective_rh_gl(fovy, aspect, 0.05, 1500.0);
    let view = Mat4::look_to_rh(Vec3::ZERO, dir, cam_up);
    let vp = proj * view;
    match &rt {
        Some(t) => set_camera(&Camera2D { render_target: Some(t.clone()), ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, sw, sh)) }),
        None => set_default_camera(),
    }
    draw_sky(g, &vp, top, hor);
    {
        let mut gl = unsafe { get_internal_gl() };
        gl.flush();
        let r = app.eff_dist as f32 * CS as f32;
        let (fog_color, fog_range) = if g.player.eye_in_water {
            ([0.1, 0.25, 0.5, 1.0], [1.0, 26.0])
        } else if app.settings.fog {
            // в непогоду туман ближе (в песчаную бурю — гораздо ближе)
            let p = g.player.body.pos;
            let desert = g.rain_k > 0.0 && worldgen::column(g.seed, p.x as i32, p.z as i32).biome == worldgen::Biome::Desert;
            let k = 1.0 - g.rain_k * if desert { 0.75 } else { 0.35 };
            ([hor.r, hor.g, hor.b, 1.0], [r * 0.5 * k, r * 0.98 * k])
        } else {
            ([hor.r, hor.g, hor.b, 1.0], [1e5, 1e5 + 1.0])
        };
        let night_vision = g.player.has(items::Eff::Night);
        let frame = Frame {
            cam: eye,
            view_proj: vp,
            fog_color,
            fog_range,
            daylight: if night_vision { 1.0 } else { g.daylight() },
            bright: if night_vision { 1.0 } else { app.settings.brightness },
            time: app.time,
            underground: {
                let e = g.player.eye();
                !g.world().sky_exposed(e.x.floor() as i32, e.y.floor() as i32 + 1, e.z.floor() as i32) && e.y < 70.0 || g.player.body.pos.y < 40.0
            },
        };
        let clouds = if app.settings.clouds && g.dim != Dim::Abyss { Some((g.dim, app.time)) } else { None };
        if std::env::var("RK_NOWORLD").is_err() {
            app.renderer.draw(gl.quad_context, &app.loader, &frame, clouds, pass);
        }
    }
    let cam3 = Camera3D { position: eye, target: eye + dir, up: cam_up, fovy, z_near: 0.05, z_far: 1500.0, render_target: rt.clone(), aspect: Some(sw / sh.max(1.0)), ..Default::default() };
    set_camera(&cam3);
    model::begin(&app.ui.atlas);
    let daylight = g.daylight();
    if std::env::var("RK_NOMOBS").is_err() {
        g.mobs.draw(g.world(), daylight, app.time, eye, dir);
    }
    let right = dir.cross(Vec3::Y).normalize_or_zero();
    g.ents.draw(&app.ui.atlas, app.time, right);
    // другие игроки
    for (id, r) in g.remotes.iter() {
        if r.st.dim != dim_code(g.dim) {
            continue;
        }
        let armor = [Item::from_code(r.st.armor[0]), Item::from_code(r.st.armor[1]), Item::from_code(r.st.armor[2]), Item::from_code(r.st.armor[3])];
        let lk = r.st.look.to_le_bytes();
        let look = model::look_with(lk, *id as u32 * 7 + hash_seed(&r.st.name));
        model::draw_player(&app.ui.atlas, Vec3::from(r.shown), r.st.yaw, r.st.pitch, r.st.walk, r.st.speed, r.st.swing, Item::from_code(r.st.held), &armor, &look, daylight.max(0.35), r.hurt_t > 0.0);
    }
    let light = player_light(g);
    if g.third_person {
        let p = &g.player;
        let look = model::look_with(app.settings.look, hash_seed(&app.settings.name));
        let sp = vec2(p.body.vel.x, p.body.vel.z).length() / 4.0;
        model::draw_player(&app.ui.atlas, p.body.pos, p.yaw, p.pitch, p.walk_phase, sp, g.swing_t, p.held(), &p.armor, &look, light, p.hurt_t > 0.0);
    }
    model::flush();
    // осадки вокруг камеры: дождь, снег в тайге, песок в пустыне
    if g.rain_k > 0.05 && g.dim == Dim::Overworld {
        let biome = worldgen::column(g.seed, eye.x as i32, eye.z as i32).biome;
        let n = (360.0 * g.rain_k) as i32;
        let t = app.time;
        for i in 0..n {
            let h1 = blocks::hash(i, 1, 77);
            let h2 = blocks::hash(i, 2, 77);
            let h3 = blocks::hash(i, 3, 77);
            let (ox, oz) = ((h1 - 0.5) * 32.0, (h2 - 0.5) * 32.0);
            let (x, z) = (eye.x.floor() + ox, eye.z.floor() + oz);
            // под крышей не идёт
            let top = g.world().surface_y(x.floor() as i32, z.floor() as i32) as f32;
            match biome {
                worldgen::Biome::Desert => {
                    let y = eye.y - 4.0 + h3 * 12.0;
                    let px = x + ((t * 9.0 + h3 * 40.0) % 32.0) - 16.0;
                    if y > top {
                        draw_line_3d(vec3(px, y, z), vec3(px + 0.9, y + 0.05, z), Color::new(0.9, 0.76, 0.5, 0.85));
                    }
                }
                worldgen::Biome::Taiga => {
                    let y = eye.y + 14.0 - ((t * 2.0 + h3 * 20.0) % 20.0);
                    let sx = x + (t * 0.8 + h1 * 6.0).sin() * 0.4;
                    if y > top {
                        draw_cube(vec3(sx, y, z), Vec3::splat(0.07), None, Color::new(1.0, 1.0, 1.0, 0.9));
                    }
                }
                _ => {
                    let y = eye.y + 14.0 - ((t * 18.0 + h3 * 20.0) % 20.0);
                    if y > top {
                        draw_line_3d(vec3(x, y, z), vec3(x + 0.08, y - 1.1, z), Color::new(0.75, 0.82, 0.95, 0.85));
                    }
                }
            }
        }
    }
    if app.screen == Screen::Playing && !app.hide_hud {
        if let Some((hit, _, _)) = g.world().raycast(g.player.eye(), dir, if g.creative { 6.0 } else { 5.0 }) {
            let c = hit.as_vec3() + Vec3::splat(0.5);
            draw_cube_wires(c, Vec3::splat(1.006), Color::new(0.02, 0.02, 0.02, 0.85));
            if g.break_target == Some(hit) && g.break_progress > 0.0 {
                let stage = ((g.break_progress * 4.0) as usize).min(3);
                draw_cube(c, Vec3::splat(1.01), Some(&app.cracks[stage]), WHITE);
            }
        }
    }
    // рука от первого лица — поверх мира (очищаем глубину)
    if !g.third_person && !app.hide_hud && matches!(app.screen, Screen::Playing | Screen::Paused | Screen::Inventory) {
        let mut gl = unsafe { get_internal_gl() };
        gl.flush();
        gl.quad_context.begin_pass(pass, miniquad::PassAction::Clear { color: None, depth: Some(1.0), stencil: None });
        gl.quad_context.end_render_pass();
        let p = &g.player;
        let look = model::look_with(app.settings.look, hash_seed(&app.settings.name));
        let sleeve = model::armor_color(p.armor[1]).unwrap_or(look.shirt);
        let hand_eye = g.player.eye() + bob;
        let hand_cam = Camera3D { position: hand_eye, target: hand_eye + dir, up: cam_up, fovy: 70f32.to_radians(), z_near: 0.02, z_far: 10.0, render_target: rt.clone(), aspect: Some(sw / sh.max(1.0)), ..Default::default() };
        set_camera(&hand_cam);
        let lag = vec2(g.player.yaw, g.player.pitch) - g.lag_look;
        let swing = std::env::var("RK_SWING").ok().and_then(|v| v.parse().ok()).unwrap_or(g.swing_p);
        let recharge = 1.0 - (1.0 - g.attack_cd / g.attack_max.max(0.01));
        // пока удар заряжается, предмет чуть опущен
        let anim = model::HandAnim { swing, equip: g.equip.max(recharge * recharge * 0.3), eat: p.eat_t, lag: lag.clamp(Vec2::splat(-0.5), Vec2::splat(0.5)), walk: wp, bob: walk_amp };
        let hr = dir.cross(cam_up).normalize_or_zero();
        let hu = hr.cross(dir).normalize_or_zero();
        model::draw_first_person(&app.ui.atlas, hand_eye, dir, hr, hu, p.held(), &anim, sleeve, look.skin, light);
        model::flush();
    }
    set_default_camera();
    if let Some(t) = &rt {
        draw_texture_ex(&t.texture, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(sw, sh)), flip_y: true, ..Default::default() });
    }
    // имена игроков над головами
    for r in g.remotes.values() {
        if r.st.dim != dim_code(g.dim) {
            continue;
        }
        let head = Vec3::from(r.shown) + vec3(0.0, 2.25, 0.0) - eye;
        let c = vp * head.extend(1.0);
        if c.w > 0.0 && head.length() < 64.0 {
            let (sx, sy) = ((c.x / c.w + 1.0) * 0.5 * screen_width(), (1.0 - c.y / c.w) * 0.5 * screen_height());
            let tw = app.ui.measure(&r.st.name, 15.0);
            draw_rectangle(sx - tw / 2.0 - 6.0, sy - 18.0, tw + 12.0, 22.0, Color::new(0.0, 0.0, 0.0, 0.45));
            app.ui.text(&r.st.name, sx - tw / 2.0, sy, 15.0, WHITE);
            let hp = (r.st.health / MAX_HEALTH).clamp(0.0, 1.0);
            draw_rectangle(sx - 25.0, sy + 6.0, 50.0, 4.0, Color::new(0.2, 0.0, 0.0, 0.7));
            draw_rectangle(sx - 25.0, sy + 6.0, 50.0 * hp, 4.0, Color::new(0.9, 0.15, 0.15, 1.0));
        }
    }
    // мягкое затемнение краёв экрана
    {
        let (w, h) = (screen_width(), screen_height());
        let n = 14;
        let band = h.min(w) * 0.16 / n as f32;
        for i in 0..n {
            let a = 0.035 * (1.0 - i as f32 / n as f32).powf(1.5);
            let c = Color::new(0.0, 0.0, 0.0, a);
            let o = i as f32 * band;
            draw_rectangle(0.0, o, w, band, c);
            draw_rectangle(0.0, h - o - band, w, band, c);
            draw_rectangle(o, 0.0, band, h, c);
            draw_rectangle(w - o - band, 0.0, band, h, c);
        }
    }
    if g.player.eye_in_water {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.1, 0.3, 0.6, 0.35));
    }
    if g.player.hurt_t > 0.0 {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.8, 0.0, 0.0, g.player.hurt_t * 0.6));
    }
}

// ---------------------------------------------------------------------------

/// Заставка на чёрном экране при запуске (можно пропустить любой клавишей).
async fn splash(font: &Font, prep: &std::thread::JoinHandle<()>) {
    const LINE1: &str = "Эта игра полностью сделана и написана";
    const LINE2: &str = "maktraxer228";
    const TOTAL: f32 = 3.6;
    let mut t = 0.0f32;
    loop {
        t += get_frame_time().min(0.05);
        let skip = get_last_key_pressed().is_some() || is_mouse_button_pressed(MouseButton::Left);
        if (t >= TOTAL || (skip && t > 0.3)) && prep.is_finished() {
            break;
        }
        let a = if t < 0.8 { t / 0.8 } else if t > TOTAL - 0.8 { ((TOTAL - t) / 0.8).max(0.0) } else { 1.0 };
        let a = a * a * (3.0 - 2.0 * a);
        clear_background(BLACK);
        let (w, h) = (screen_width(), screen_height());
        let k = (h / 720.0).clamp(0.6, 3.0);
        let s1 = (30.0 * k) as u16;
        let s2 = (64.0 * k) as u16;
        let d1 = measure_text(LINE1, Some(font), s1, 1.0);
        let d2 = measure_text(LINE2, Some(font), s2, 1.0);
        draw_text_ex(LINE1, (w - d1.width) / 2.0, h / 2.0 - 30.0 * k, TextParams { font: Some(font), font_size: s1, color: Color::new(0.85, 0.85, 0.9, a), ..Default::default() });
        // имя автора — с тёплым свечением
        for (o, al) in [(3.0, 0.12), (1.5, 0.2)] {
            draw_text_ex(LINE2, (w - d2.width) / 2.0 + o * k, h / 2.0 + 50.0 * k + o * k, TextParams { font: Some(font), font_size: s2, color: Color::new(1.0, 0.55, 0.1, a * al), ..Default::default() });
        }
        draw_text_ex(LINE2, (w - d2.width) / 2.0, h / 2.0 + 50.0 * k, TextParams { font: Some(font), font_size: s2, color: Color::new(1.0, 0.75, 0.25, a), ..Default::default() });
        let lw = d2.width * a;
        draw_rectangle((w - lw) / 2.0, h / 2.0 + 66.0 * k, lw, 2.0 * k, Color::new(1.0, 0.7, 0.2, a * 0.8));
        if let Ok(p) = std::env::var("RK_SPLASH_SHOT") {
            if t > 1.5 {
                get_screen_data().export_png(&p);
                std::process::exit(0);
            }
        }
        next_frame().await;
    }
}

fn main() {
    // режим ретранслятора для игры через интернет — без окна
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--relay") {
        let port = args.get(i + 1).and_then(|p| p.parse().ok()).unwrap_or(net::RELAY_PORT);
        net::run_relay(port);
        return;
    }
    macroquad::Window::from_config(window_conf(), amain());
}

async fn amain() {
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/DejaVuSans-Bold.ttf")).expect("шрифт");
    // стиль текстур нужен до постройки атласа
    let boot = Settings::load();
    let style = std::env::var("RK_STYLE").ok().and_then(|v| v.parse().ok()).unwrap_or(boot.tex_style);
    blocks::set_style(style);
    // тяжёлая подготовка (атлас, рецепты) — в фоне, пока идёт заставка
    let prep = std::thread::spawn(|| {
        let _ = atlas_pixels();
        let _ = crafting::recipes();
    });
    let test_mode = std::env::var("RK_SHOT").is_ok() || std::env::var("RK_BENCH").is_ok();
    if !test_mode || std::env::var("RK_SPLASH").is_ok() {
        splash(&font, &prep).await;
    }
    let _ = prep.join();

    // текстуры в выбранном разрешении, но не больше, чем умеет видеокарта
    let mut startup_note: Option<String> = None;
    let want_res = std::env::var("RK_TEXRES").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(Settings::load().tex_res) as usize;
    let max_tex = unsafe {
        let mut v: i32 = 0;
        macroquad::miniquad::gl::glGetIntegerv(macroquad::miniquad::gl::GL_MAX_TEXTURE_SIZE, &mut v);
        if v <= 0 { 4096 } else { v as usize }
    };
    let mut res = want_res;
    while res > 8 && ATLAS_TILES * res > max_tex {
        res /= 2;
    }
    if res != want_res {
        startup_note = Some(format!("Видеокарта не тянет текстуры {want_res}×{want_res}: включено {res}×{res}"));
    }
    // текстур-пак поверх встроенных текстур (если выбран и читается)
    let pack = std::env::var("RK_PACK").unwrap_or(boot.tex_pack.clone());
    let mut pack_active = String::new();
    let mut built = None;
    if !pack.is_empty() {
        match packs::load(&pack) {
            Ok((px, w, h, tr)) if w <= max_tex => {
                built = Some((px, w, h));
                pack_active = pack.clone();
                res = tr;
            }
            Ok(_) => startup_note = Some(format!("Пак «{pack}» слишком большой для видеокарты")),
            Err(e) => startup_note = Some(format!("Пак «{pack}»: {e}")),
        }
    }
    let atlas = {
        let (px, w, h) = built.unwrap_or_else(|| display_atlas(res));
        Texture2D::from_rgba8(w as u16, h as u16, &px)
    };
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
        pending_net: None,
        seed_input: String::new(),
        name_input: "Мой мир".into(),
        ip_input: "127.0.0.1".into(),
        focus: Field::None,
        new_creative: false,
        inv_tab: 0,
        inv_scroll: 0.0,
        craft_scroll: 0.0,
        craft_all: false,
        cursor: Stack::EMPTY,
        grabbed: false,
        last_mouse: mouse_position().into(),
        show_debug: false,
        hide_hud: false,
        cracks: crack_textures(),
        time: 0.0,
        fps: 60.0,
        status: None,
        worlds_list: Vec::new(),
        just_opened: false,
        finder: None,
        chat_input: String::new(),
        audio: audio::Audio::new(),
        map: None,
        minimap: map::MiniMap::default(),
        touch_layout: touch::Layout::default(),
        touch_edit: None,
        keyboard_shown: false,
        trade: None,
        tex_res_active: 16,
        style_active: 1,
        pack_active: String::new(),
        pack_list: Vec::new(),
        sfx: Vec::new(),
        export_rx: None,
        book_tab: 0,
        book_scroll: 0.0,
        book_sel: None,
        book_back: Vec::new(),
        book_search: String::new(),
        rt: None,
        eff_dist: 4,
        auto_t: 0.0,
    };

    app.eff_dist = app.settings.render_dist;
    app.tex_res_active = res as u32;
    app.style_active = style;
    app.pack_active = pack_active;
    if let Some(n) = startup_note {
        app.status = Some((n, 6.0));
    }
    if let Some(l) = std::env::var("RK_OPT").ok().and_then(|v| v.parse::<u8>().ok()) {
        app.settings.opt_level = l.min(3);
    }
    opt::set_level(app.settings.opt_level);
    app.touch_layout = touch::Layout::from_text(&app.settings.touch_layout);
    let mut frame_start = std::time::Instant::now();
    if std::env::var("RK_BENCH").is_ok() || std::env::var("RK_SHOT").is_ok() {
        app.settings.auto_dist = false;
    }
    // режим для автоматических скриншотов и проверок
    let shot = std::env::var("RK_SHOT").ok();
    let mut shot_frames = 0;
    let mut bench: (u32, f64, u32) = (0, 0.0, 0);
    let mut prof: (f64, f64) = (0.0, 0.0);
    if let Ok(seed) = std::env::var("RK_SEED") {
        if let Ok(r) = std::env::var("RK_DIST") {
            app.settings.render_dist = r.parse().unwrap_or(8);
            app.eff_dist = app.settings.render_dist;
        }
        if let Ok(load) = std::env::var("RK_LOAD") {
            load_game(&mut app, &load);
        } else {
            start_world(&mut app, hash_seed(&seed), std::env::var("RK_CREATIVE").is_ok(), &std::env::var("RK_NAME").unwrap_or_default());
            if let Some(g) = app.game.as_mut() {
                if std::env::var("RK_NAME").is_err() {
                    g.name.clear();
                }
            }
        }
        app.screen = Screen::Loading;
        if std::env::var("RK_HOST").is_ok() {
            if let Ok(n) = Net::host(&app.settings.name) {
                app.game.as_mut().unwrap().net = Some(n);
            }
            if let Some((relay, room)) = std::env::var("RK_RELAY_HOST").ok().as_deref().and_then(|v| v.split_once('#')).map(|(a, b)| (a.to_string(), b.to_string())) {
                if let Err(e) = net::relay_host(&relay, &room) {
                    eprintln!("NETLOG relay error {e}");
                }
            }
        }
    }
    if let Ok(ip) = std::env::var("RK_JOIN") {
        app.settings.name = std::env::var("RK_NICK").unwrap_or("Гость".into());
        if let Ok(n) = Net::join(&ip, &app.settings.name) {
            app.pending_net = Some(n);
            app.screen = Screen::Loading;
        }
    }
    let shot_screen = std::env::var("RK_SCREEN").unwrap_or_default();
    // проверка сенсорного управления на ПК
    if std::env::var("RK_TOUCH").is_ok() {
        app.settings.touch = true;
    }
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
        SKY_STYLE.store(app.settings.sky.min(1), std::sync::atomic::Ordering::Relaxed);
        let mouse: Vec2 = mouse_position().into();
        // касания: джойстик, обзор и кнопки (в меню касания работают как мышь)
        {
            let s = app.ui.s;
            let slot = 46.0 * s;
            let gap = 5.0 * s;
            let total = 9.0 * (slot + gap) - gap;
            let hot = ((screen_width() - total) / 2.0, screen_height() - slot - 12.0 * s, slot, slot + gap);
            touch::update(&app.touch_layout, app.settings.touch, app.screen == Screen::Playing, hot);
        }
        let mut md = if app.grabbed { mouse - app.last_mouse } else { Vec2::ZERO };
        if app.settings.touch {
            md += touch::out().look * 1.6;
        }
        // экранная клавиатура там, где вводят текст
        let want_kb = app.settings.touch && matches!(app.screen, Screen::Chat | Screen::NewWorld | Screen::Relay | Screen::RecipeBook | Screen::Multiplayer);
        if want_kb != app.keyboard_shown {
            macroquad::miniquad::window::show_keyboard(want_kb);
            app.keyboard_shown = want_kb;
        }
        app.last_mouse = mouse;
        if let Some((_, t)) = &mut app.status {
            *t -= dt;
        }
        if app.status.as_ref().is_some_and(|s| s.1 <= 0.0) {
            app.status = None;
        }

        if is_key_pressed(KeyCode::F7) {
            app.settings.show_fps = !app.settings.show_fps;
            app.settings.save();
        }
        match app.screen {
            Screen::Playing => {
                let tc = touch::out();
                if is_key_pressed(KeyCode::Escape) || tc.pressed(touch::Act::Pause) {
                    app.screen = Screen::Paused;
                    app.just_opened = true;
                } else if is_key_pressed(KeyCode::E) || tc.pressed(touch::Act::Inventory) {
                    app.screen = Screen::Inventory;
                    app.just_opened = true;
                }
                if is_key_pressed(KeyCode::T) || is_key_pressed(KeyCode::Enter) || tc.pressed(touch::Act::Chat) {
                    app.screen = Screen::Chat;
                    app.just_opened = true;
                }
                // M — карта мира
                if (is_key_pressed(KeyCode::M) || tc.pressed(touch::Act::Map)) && app.screen == Screen::Playing {
                    if let Some(g) = app.game.as_ref() {
                        let me = vec2(g.player.body.pos.x, g.player.body.pos.z);
                        if app.map.is_none() {
                            app.map = Some(map::MapView::new(me));
                        } else if let Some(m) = app.map.as_mut() {
                            m.center = me;
                        }
                        app.screen = Screen::Map;
                        app.just_opened = true;
                    }
                }
                // B — книга рецептов, если она есть в инвентаре (или в творчестве)
                if (is_key_pressed(KeyCode::B) || tc.pressed(touch::Act::Book)) && app.screen == Screen::Playing {
                    let has = app.game.as_ref().is_some_and(|g| g.creative || g.player.inv.count(Item::Thing(items::RECIPE_BOOK)) > 0);
                    if has {
                        open_recipe_book(&mut app);
                    } else if let Some(g) = app.game.as_mut() {
                        g.toast = Some(("Нужна книга рецептов".into(), 1.5));
                    }
                }
                if is_key_pressed(KeyCode::F3) {
                    app.show_debug = !app.show_debug;
                }
                if is_key_pressed(KeyCode::F1) {
                    app.hide_hud = !app.hide_hud;
                }
                if is_key_pressed(KeyCode::F5) || tc.pressed(touch::Act::Camera) {
                    if let Some(g) = app.game.as_mut() {
                        g.third_person = !g.third_person;
                    }
                }
            }
            Screen::Paused if is_key_pressed(KeyCode::Escape) && !app.just_opened => app.screen = Screen::Playing,
            _ => {}
        }
        let want_grab = app.screen == Screen::Playing && shot.is_none() && !app.settings.touch;
        set_grab(&mut app, want_grab);

        process_net(&mut app);

        // авто-дальность: держим ~60 кадров в секунду
        if app.settings.auto_dist && app.screen == Screen::Playing {
            app.auto_t += dt;
            if app.fps < 50.0 && app.eff_dist > 3 && app.auto_t > 1.5 {
                app.eff_dist -= 1;
                app.auto_t = 0.0;
            } else if app.fps > 58.0 && app.eff_dist < app.settings.render_dist && app.auto_t > 5.0 {
                app.eff_dist += 1;
                app.auto_t = 0.0;
            } else if app.fps >= 50.0 && app.fps <= 58.0 {
                app.auto_t = app.auto_t.min(1.0);
            }
            app.eff_dist = app.eff_dist.min(app.settings.render_dist);
        } else if !app.settings.auto_dist {
            app.eff_dist = app.settings.render_dist;
        }

        if app.game.is_some() {
            let center = {
                let g = app.game.as_ref().unwrap();
                let p = g.player.body.pos;
                chunk_of(p.x.floor() as i32, p.z.floor() as i32)
            };
            {
                let ctx = unsafe { get_internal_gl() }.quad_context;
                let g = app.game.as_mut().unwrap();
                let w = &mut g.worlds[g.dim as usize];
                app.loader.update(ctx, &mut app.pool, w, center, app.eff_dist, app.settings.smooth_light);
            }
            // жители построек
            let spawns: Vec<(u16, [i32; 3])> = app.loader.new_spawns.drain(..).collect();
            if let Some(g) = app.game.as_mut() {
                for (def, p) in spawns {
                    if g.spawned.insert(p) {
                        g.mobs.spawn_inhabitant(def as usize, vec3(p[0] as f32 + 0.5, p[1] as f32, p[2] as f32 + 0.5));
                    }
                }
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
                        if let Some(gp) = ground_spot(g.world(), g.player.body.pos.x as i32, g.player.body.pos.z as i32) {
                            g.player.body.pos = gp;
                        }
                        let p = g.player.body.pos;
                        let f = g.player.forward().with_y(0.0).normalize_or_zero();
                        let side = vec3(-f.z, 0.0, f.x);
                        let ids: Vec<usize> = std::env::var("RK_MOBS").unwrap().split(',').filter_map(|s| s.parse().ok()).collect();
                        let n = ids.len() as f32;
                        for (i, id) in ids.into_iter().enumerate() {
                            let md: f32 = std::env::var("RK_MOBDIST").ok().and_then(|v| v.parse().ok()).unwrap_or(9.0);
                            let at = p + f * md + side * (i as f32 - (n - 1.0) / 2.0) * (md * 0.28).max(1.6);
                            let y = ground_spot(g.world(), at.x.floor() as i32, at.z.floor() as i32).map_or(p.y, |v| v.y);
                            let mut m = mobs::Mob::new(id.min(mobdefs::MOBS.len() - 1), vec3(at.x, y, at.z));
                            m.yaw = (-f.z).atan2(-f.x);
                            m.home = Some(m.body.pos);
                            g.mobs.list.push(m);
                        }
                        g.creative = true;
                    }
                    if let Ok(t) = std::env::var("RK_TIME") {
                        g.time = t.parse().unwrap_or(g.time);
                    }
                    if let Some(slot) = std::env::var("RK_SLOT").ok().and_then(|v| v.parse::<usize>().ok()) {
                        g.player.selected = slot.min(8);
                        g.last_held = g.player.held();
                    }
                    if std::env::var("RK_THIRD").is_ok() {
                        g.third_person = true;
                    }
                    app.screen = Screen::Playing;
                    if std::env::var("RK_DIM").is_ok_and(|d| d == "heaven") && g.dim == Dim::Overworld {
                        switch_dimension(&mut app, Dim::Heaven);
                    } else if std::env::var("RK_DIM").is_ok_and(|d| d == "abyss") && g.dim == Dim::Overworld {
                        switch_dimension(&mut app, Dim::Abyss);
                    }
                }
            }
            if matches!(app.screen, Screen::Playing | Screen::Inventory | Screen::Chat | Screen::RecipeBook | Screen::Map | Screen::Trade | Screen::Enchant) {
                let t0 = std::time::Instant::now();
                update_game(&mut app, dt, md);
                prof.0 += t0.elapsed().as_secs_f64();
            } else if app.screen == Screen::Paused && app.game.as_ref().is_some_and(|g| g.net.is_some()) {
                // в сетевой игре мир не останавливается
                update_game(&mut app, dt, Vec2::ZERO);
            }
        }

        let t_render = std::time::Instant::now();
        if app.game.is_some() && app.screen != Screen::Loading {
            render_world(&mut app);
            if !app.hide_hud && matches!(app.screen, Screen::Playing | Screen::Paused | Screen::Inventory | Screen::Chat | Screen::RecipeBook) {
                draw_hud(&mut app);
            }
            if app.screen == Screen::Playing && (is_key_down(KeyCode::Tab) || touch::out().down(touch::Act::Players)) {
                draw_player_list(&mut app);
            }
        } else {
            clear_background(Color::new(0.08, 0.09, 0.11, 1.0));
        }

        prof.1 += t_render.elapsed().as_secs_f64();
        let next = match app.screen {
            Screen::Title => draw_title(&mut app),
            Screen::Worlds => worlds_screen(&mut app),
            Screen::NewWorld => draw_new_world(&mut app),
            Screen::Multiplayer => multiplayer_screen(&mut app),
            Screen::Settings => settings_screen(&mut app),
            Screen::Paused => pause_screen(&mut app),
            Screen::Dead => dead_screen(&mut app),
            Screen::Inventory => inventory_screen(&mut app),
            Screen::Chat => chat_screen(&mut app),
            Screen::RecipeBook => recipe_book_screen(&mut app),
            Screen::Map => map_screen(&mut app),
            Screen::Trade => trade::screen(&mut app),
            Screen::Enchant => enchant_screen(&mut app),
            Screen::Achievements => achievements_screen(&mut app),
            Screen::Look => look_screen(&mut app),
            Screen::Textures => textures_screen(&mut app),
            Screen::Relay => relay_screen(&mut app),
            Screen::Controls => controls_screen(&mut app),
            Screen::Loading => {
                let (w, h) = (screen_width(), screen_height());
                draw_rectangle(0.0, 0.0, w, h, Color::new(0.07, 0.08, 0.1, 1.0));
                let label = if app.pending_net.is_some() {
                    "Подключение к серверу…"
                } else if app.game.as_ref().is_some_and(|g| g.dim == Dim::Heaven) {
                    "Путь в Рай…"
                } else {
                    "Строим мир…"
                };
                app.ui.text_center(label, w / 2.0, h / 2.0 - 10.0, 32.0, WHITE);
                let n = app.loader.gpu.len() as f32;
                let t = (n / 9.0).min(1.0);
                let bw = 360.0 * app.ui.s;
                draw_rectangle((w - bw) / 2.0, h / 2.0 + 20.0, bw, 8.0, Color::new(1.0, 1.0, 1.0, 0.15));
                draw_rectangle((w - bw) / 2.0, h / 2.0 + 20.0, bw * t, 8.0, ACCENT);
                if app.pending_net.is_some() && app.ui.button((w - bw) / 2.0, h / 2.0 + 60.0, bw, 40.0, "Отмена") {
                    app.pending_net = None;
                    Some(Screen::Multiplayer)
                } else {
                    None
                }
            }
            Screen::Playing => None,
        };
        // на телефоне нет Esc и E: кнопка «закрыть» на наложенных экранах
        let mut next = next;
        if app.settings.touch && next.is_none() && matches!(app.screen, Screen::Inventory | Screen::RecipeBook | Screen::Map | Screen::Trade | Screen::Enchant | Screen::Chat) {
            let s = app.ui.s;
            let b = 52.0 * s;
            if app.ui.button(screen_width() - b - 10.0 * s, 10.0 * s, b, b, "✕") && !app.just_opened {
                next = Some(Screen::Playing);
            }
        }
        if let Some(n) = next {
            if n == Screen::Playing && app.screen == Screen::Inventory && !app.cursor.is_empty() {
                let c = app.cursor;
                if let Some(g) = app.game.as_mut() {
                    g.player.inv.add(c.item, c.count);
                }
                app.cursor = Stack::EMPTY;
            }
            app.screen = n;
        }
        if let Some((t, life)) = &app.status {
            let s = app.ui.s;
            app.ui.text_center(t, screen_width() / 2.0, screen_height() - 30.0 * s, 17.0, Color::new(1.0, 0.8, 0.5, life.min(1.0)));
        }
        app.ui.end();
        app.just_opened = false;

        if let Ok(b) = std::env::var("RK_BENCH") {
            let n: u32 = b.parse().unwrap_or(300);
            if app.screen == Screen::Playing {
                bench.0 += 1;
                if bench.0 > 60 {
                    bench.1 += raw_dt as f64;
                    bench.2 += 1;
                }
                if bench.2 >= n {
                    let avg = bench.1 / bench.2 as f64;
                    let (fr, ho) = app.game.as_ref().map_or((0, 0), |g| g.mobs.counts());
                    let fr_n = bench.0 as f64;
                    println!("BENCH frames={} avg_ms={:.2} fps={:.1} chunks={} mobs={} update_ms={:.2} render_cpu_ms={:.2} meshes_tris={:?}", bench.2, avg * 1000.0, 1.0 / avg, app.loader.gpu.len(), fr + ho, prof.0 * 1000.0 / fr_n, prof.1 * 1000.0 / fr_n, app.loader.stats());
                    std::process::exit(0);
                }
            }
        }
        if let Some(path) = &shot {
            let ready = match shot_screen.as_str() {
                "title" | "worlds" | "multiplayer" => true,
                _ => app.game.is_some() && app.screen != Screen::Loading,
            };
            if ready {
                shot_frames += 1;
                if shot_frames == 2 {
                    match shot_screen.as_str() {
                        "book" => {
                            open_recipe_book(&mut app);
                            if let Some(g) = app.game.as_mut() {
                                if std::env::var("RK_RICH").is_ok() {
                                    for it in [items::IRON_INGOT, items::STICK, items::COAL, items::FLOUR, items::EGG, items::MILK] {
                                        g.player.inv.add(Item::Thing(it), 20);
                                    }
                                    g.player.inv.add(Item::Block(OAK_PLANKS), 30);
                                }
                            }
                            app.book_tab = std::env::var("RK_TAB").ok().and_then(|t| t.parse().ok()).unwrap_or(0);
                            app.book_sel = std::env::var("RK_BOOKSEL").ok().and_then(|t| t.parse().ok());
                            if let Ok(q) = std::env::var("RK_BOOKQ") {
                                app.book_search = q;
                            }
                        }
                        "inventory" => {
                            app.screen = Screen::Inventory;
                            app.inv_tab = std::env::var("RK_TAB").ok().and_then(|t| t.parse().ok()).unwrap_or(0);
                            if let Some(g) = app.game.as_mut() {
                                if std::env::var("RK_RICH").is_ok() {
                                    for (k, it) in [items::IRON_INGOT, items::STICK, items::COAL, items::WHEAT, items::RUBY, items::LEATHER].iter().enumerate() {
                                        g.player.inv.add(Item::Thing(*it), 20 + k as u16);
                                    }
                                    g.player.inv.add(Item::Block(OAK_LOG), 16);
                                    g.player.inv.add(Item::Block(COBBLE), 32);
                                    g.player.armor = [Item::Armor(items::Slot::Helmet, items::Mat::Ruby), Item::Armor(items::Slot::Chest, items::Mat::Ruby), Item::Empty, Item::Armor(items::Slot::Boots, items::Mat::Iron)];
                                }
                            }
                        }
                        "settings" => {
                            app.settings_back = Screen::Playing;
                            app.screen = Screen::Settings;
                        }
                        "worlds" => {
                            app.worlds_list = save::list_worlds();
                            app.screen = Screen::Worlds;
                        }
                        "multiplayer" => app.screen = Screen::Multiplayer,
                        "textures" => {
                            app.settings_back = Screen::Playing;
                            app.pack_list = packs::list();
                            app.screen = Screen::Textures;
                        }
                        "controls" => {
                            app.settings_back = Screen::Playing;
                            app.touch_edit = Some(touch::Act::Attack);
                            app.screen = Screen::Controls;
                        }
                        "pause" => app.screen = Screen::Paused,
                        "enchant" => {
                            if let Some(g) = app.game.as_mut() {
                                g.player.xp = 600;
                                g.player.inv.slots[0].ench = ench::pack(ench::SHARP, 2);
                            }
                            app.screen = Screen::Enchant;
                        }
                        "trade" => {
                            app.trade = Some((mobdefs::by_name("Кузнец"), 1));
                            app.screen = Screen::Trade;
                        }
                        "map" => {
                            if let Some(g) = app.game.as_ref() {
                                let me = vec2(g.player.body.pos.x, g.player.body.pos.z);
                                let mut v = map::MapView::new(me);
                                v.scale_i = std::env::var("RK_MAPSCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(2usize).min(5);
                                app.map = Some(v);
                                app.screen = Screen::Map;
                            }
                        }
                        _ => {}
                    }
                }
                // витрина фигурных блоков перед игроком (для проверки)
                if std::env::var("RK_FORMS").is_ok() && shot_frames == 25 {
                    if let Some(g) = app.game.as_ref() {
                        use shapes::Form;
                        let p = g.player.body.pos.floor().as_ivec3();
                        let dim = g.dim;
                        let gy = g.world().surface_y(p.x + 5, p.z);
                        let mut put: Vec<(IVec3, u16)> = Vec::new();
                        let base = ivec3(p.x + 5, gy, p.z - 6);
                        for k in 0..4u8 {
                            put.push((base + ivec3(0, 0, k as i32), shapes::id(Form::Slab { m: k, top: k % 2 == 1 })));
                            put.push((base + ivec3(1, 0, k as i32), shapes::id(Form::Stairs { m: k + 8, dir: k, top: false })));
                        }
                        put.push((base + ivec3(0, 0, 5), shapes::id(Form::Door { m: 0, dir: 1, open: false, upper: false })));
                        put.push((base + ivec3(0, 1, 5), shapes::id(Form::Door { m: 0, dir: 1, open: false, upper: true })));
                        put.push((base + ivec3(0, 0, 6), shapes::id(Form::Door { m: 6, dir: 1, open: true, upper: false })));
                        put.push((base + ivec3(0, 1, 6), shapes::id(Form::Door { m: 6, dir: 1, open: true, upper: true })));
                        put.push((base + ivec3(1, 0, 7), shapes::id(Form::Trapdoor { m: 3, dir: 0, open: false, top: false })));
                        for k in 0..4 {
                            put.push((base + ivec3(0, 0, 8 + k), shapes::id(Form::Fence { m: 0 })));
                        }
                        put.push((base + ivec3(0, 0, 12), shapes::id(Form::Gate { m: 0, axis: 1, open: false })));
                        put.push((base + ivec3(0, 0, 13), shapes::id(Form::Fence { m: 0 })));
                        for k in 0..3 {
                            put.push((base + ivec3(0, 0, 15 + k), shapes::PANE));
                            put.push((base + ivec3(0, 1, 15 + k), shapes::PANE));
                        }
                        for (q, b) in put {
                            edit_block(&mut app, dim, q, b, false);
                        }
                    }
                }
                // испытание механизмов и воды
                if std::env::var("RK_MECH").is_ok() && (shot_frames == 25 || shot_frames == 32) {
                    if let Some(g) = app.game.as_ref() {
                        let p = g.player.body.pos.floor().as_ivec3();
                        let dim = g.dim;
                        let gy = g.world().surface_y(p.x + 6, p.z);
                        let base = ivec3(p.x + 6, gy, p.z - 4);
                        if shot_frames == 25 {
                            let mut put: Vec<(IVec3, u16)> = vec![(base, LEVER_OFF)];
                            for k in 1..6 {
                                put.push((base + ivec3(0, 0, k), shapes::WIRE0));
                            }
                            put.push((base + ivec3(0, 0, 6), LAMP_OFF));
                            put.push((base + ivec3(1, 0, 3), PUSHER0));
                            put.push((base + ivec3(2, 0, 3), BRICKS));
                            put.push((base + ivec3(-1, 0, 5), shapes::id(shapes::Form::Door { m: 11, dir: 0, open: false, upper: false })));
                            put.push((base + ivec3(-1, 1, 5), shapes::id(shapes::Form::Door { m: 11, dir: 0, open: false, upper: true })));
                            put.push((base + ivec3(0, 3, 10), WATER));
                            for k in 0..8 {
                                put.push((base + ivec3(4, 0, k), RAIL));
                            }
                            for k in 1..5 {
                                put.push((base + ivec3(4 - k, 0, 7), RAIL));
                            }
                            for (q, b) in put {
                                edit_block(&mut app, dim, q, b, false);
                                after_edit(&mut app, dim, q, AIR, b);
                            }
                        } else {
                            if let Some(g) = app.game.as_mut() {
                                g.ents.vehicles.push(entities::Vehicle { boat: false, pos: (base + ivec3(4, 0, 2)).as_vec3() + vec3(0.5, 0.05, 0.5), yaw: 0.0, speed: 3.0, dir: IVec3::Z, vy: 0.0 });
                                g.ents.vehicles.push(entities::Vehicle { boat: true, pos: (base + ivec3(2, 0, 12)).as_vec3() + vec3(0.5, 0.3, 0.5), yaw: 0.5, speed: 0.0, dir: IVec3::X, vy: 0.0 });
                            }
                            edit_block(&mut app, dim, base, LEVER_ON, false);
                            after_edit(&mut app, dim, base, LEVER_OFF, LEVER_ON);
                        }
                    }
                }
                if std::env::var("RK_SAVE").is_ok() && shot_frames == 3 {
                    save_game(&mut app);
                }
                let wait: i32 = std::env::var("RK_WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
                if shot_frames >= wait {
                    get_screen_data().export_png(path);
                    std::process::exit(0);
                }
            }
        }
        // музыка по обстановке и звуки кадра
        let track: &'static [music::Track] = match (&app.game, app.screen) {
            (None, _) | (_, Screen::Title | Screen::Worlds | Screen::NewWorld | Screen::Multiplayer) => &[music::Track::Menu],
            (Some(_), Screen::Loading) => &[],
            (Some(g), _) => music_for(g),
        };
        app.audio.want(track);
        app.audio.update(dt, app.settings.music_vol);
        let rain = app.game.as_ref().map_or(0.0, |g| {
            let e = g.player.eye();
            let inside = !g.world().sky_exposed(e.x.floor() as i32, e.y.floor() as i32, e.z.floor() as i32);
            g.rain_k * if inside { 0.25 } else { 0.7 }
        });
        app.audio.set_rain(rain * app.settings.sound_vol);
        if app.ui.clicked {
            app.sfx.push((music::Sfx::Click, 0.5));
        }
        for (s, v) in app.sfx.drain(..).take(6) {
            app.audio.play(s, v * app.settings.sound_vol);
        }
        if let Some((t, _)) = &app.audio.now_playing {
            if app.game.is_some() && app.screen == Screen::Playing && !app.hide_hud {
                let w = screen_width();
                app.ui.text(t, w - app.ui.measure(t, 14.0) - 14.0 * app.ui.s, 52.0 * app.ui.s, 14.0, Color::new(1.0, 0.9, 0.6, 0.85));
            }
        }
        // ограничение FPS: ждём остаток кадра (меньше нагрев и шум ноутбука)
        if app.settings.fps_limit > 0 {
            let target = std::time::Duration::from_secs_f64(1.0 / app.settings.fps_limit as f64);
            let spent = frame_start.elapsed();
            // на Windows сон длится шагами ~15,6 мс: спим с запасом,
            // а последние миллисекунды дожидаемся точно
            let margin = std::time::Duration::from_millis(3);
            if spent + margin < target {
                std::thread::sleep(target - spent - margin);
            }
            while frame_start.elapsed() < target {
                std::thread::yield_now();
            }
        }
        next_frame().await;
        frame_start = std::time::Instant::now();
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

fn open_recipe_book(app: &mut App) {
    app.screen = Screen::RecipeBook;
    app.just_opened = true;
    app.book_search.clear();
    app.book_back.clear();
}

/// Разделы книги рецептов.
const BOOK_TABS: [&str; 8] = ["Все", "Можно сделать", "Блоки", "Оружие", "Броня", "Еда", "Зелья", "Прочее"];

fn book_tab_of(r: &crafting::Recipe) -> usize {
    match r.out.0 {
        Item::Block(_) => 2,
        Item::Tool(..) => 3,
        Item::Armor(..) => 4,
        Item::Thing(t) => match items::thing(t).kind {
            items::TKind::Food(..) => 5,
            items::TKind::Potion(..) => 6,
            _ => 7,
        },
        Item::Empty => 7,
    }
}

/// Книга рецептов: поиск, разделы, страница рецепта с ингредиентами и «где используется».
fn recipe_book_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.5));

    // ввод поиска; латинская E при пустом поиске закрывает книгу
    let mut close = false;
    if app.just_opened {
        while get_char_pressed().is_some() {}
    } else {
        while let Some(c) = get_char_pressed() {
            if (c == 'e' || c == 'E') && app.book_search.is_empty() {
                close = true;
            } else if !c.is_control() && app.book_search.chars().count() < 24 {
                app.book_search.push(c);
                app.book_scroll = 0.0;
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            app.book_search.pop();
            app.book_scroll = 0.0;
        }
        if is_key_pressed(KeyCode::Delete) {
            app.book_search.clear();
            app.book_scroll = 0.0;
        }
    }

    // обложка и два листа
    let bw = (w - 24.0 * s).min(1120.0 * s);
    let bh = (h - 24.0 * s).min(680.0 * s);
    let (bx, by) = ((w - bw) / 2.0, (h - bh) / 2.0);
    draw_rectangle(bx - 10.0 * s, by - 10.0 * s, bw + 20.0 * s, bh + 20.0 * s, Color::from_rgba(70, 40, 24, 255));
    draw_rectangle_lines(bx - 6.0 * s, by - 6.0 * s, bw + 12.0 * s, bh + 12.0 * s, 2.0 * s, Color::from_rgba(214, 170, 70, 255));
    let paper = Color::from_rgba(238, 226, 196, 255);
    let paper2 = Color::from_rgba(230, 216, 182, 255);
    let half = bw / 2.0;
    draw_rectangle(bx, by, half - 4.0 * s, bh, paper);
    draw_rectangle(bx + half + 4.0 * s, by, half - 4.0 * s, bh, paper2);
    draw_rectangle(bx + half - 4.0 * s, by, 8.0 * s, bh, Color::from_rgba(150, 120, 80, 255));
    let ink = Color::from_rgba(52, 36, 24, 255);
    let faded = Color::from_rgba(120, 100, 76, 255);
    let good = Color::from_rgba(40, 120, 50, 255);
    let bad = Color::from_rgba(170, 40, 40, 255);

    let g = app.game.as_mut().unwrap();
    let stations = stations_near(g);
    let all = crafting::recipes();
    let ready = |r: &crafting::Recipe| r.can_craft(&g.player.inv) && stations.contains(&r.station);

    // ---------- левый лист: поиск, разделы, сетка ----------
    let lx = bx + 18.0 * s;
    let lw = half - 40.0 * s;
    app.ui.text("Книга рецептов", lx, by + 34.0 * s, 24.0, ink);
    let sy = by + 50.0 * s;
    draw_rectangle(lx, sy, lw, 28.0 * s, Color::from_rgba(250, 244, 226, 255));
    draw_rectangle_lines(lx, sy, lw, 28.0 * s, 1.5 * s, faded);
    let caret = if (app.time * 2.0).fract() < 0.5 { "|" } else { "" };
    if app.book_search.is_empty() {
        app.ui.text(&format!("Поиск: начните печатать название{caret}"), lx + 8.0 * s, sy + 19.0 * s, 13.0, faded);
    } else {
        app.ui.text(&format!("{}{caret}", app.book_search), lx + 8.0 * s, sy + 19.0 * s, 14.0, ink);
    }
    let tab_w = (lw - 3.0 * 6.0 * s) / 4.0;
    for (i, name) in BOOK_TABS.iter().enumerate() {
        let x = lx + (i % 4) as f32 * (tab_w + 6.0 * s);
        let y = sy + 36.0 * s + (i / 4) as f32 * 30.0 * s;
        let hov = app.ui.hover(x, y, tab_w, 25.0 * s);
        let on = app.book_tab == i;
        draw_rectangle(x, y, tab_w, 25.0 * s, if on { Color::from_rgba(120, 80, 46, 255) } else if hov { Color::from_rgba(214, 196, 156, 255) } else { Color::from_rgba(224, 208, 170, 255) });
        app.ui.text_center(name, x + tab_w / 2.0, y + 17.0 * s, 12.0, if on { paper } else { ink });
        if hov && app.ui.pressed {
            app.book_tab = i;
            app.book_scroll = 0.0;
        }
    }
    let q = app.book_search.to_lowercase();
    let list: Vec<usize> = (0..all.len())
        .filter(|&i| {
            let r = &all[i];
            let tab_ok = match app.book_tab {
                0 => true,
                1 => ready(r),
                t => book_tab_of(r) == t,
            };
            tab_ok && (q.is_empty() || r.out.0.name().to_lowercase().contains(&q))
        })
        .collect();
    let cell = 42.0 * s;
    let gap = 4.0 * s;
    let cols = ((lw + gap) / (cell + gap)).floor().max(1.0) as usize;
    let grid_top = sy + 104.0 * s;
    let grid_h = by + bh - 34.0 * s - grid_top;
    let vis_rows = (grid_h / (cell + gap)).floor().max(1.0) as usize;
    let rows = list.len().div_ceil(cols);
    let max_scroll = rows.saturating_sub(vis_rows) as f32;
    let (_, wheel) = mouse_wheel();
    if app.ui.hover(lx, grid_top, lw, grid_h) {
        app.book_scroll = (app.book_scroll - wheel.signum()).clamp(0.0, max_scroll);
    }
    app.book_scroll = app.book_scroll.min(max_scroll);
    let first = app.book_scroll as usize * cols;
    let mut pick: Option<(usize, bool)> = None;
    for (k, &ri) in list.iter().enumerate().skip(first).take(vis_rows * cols) {
        let i = k - first;
        let r = &all[ri];
        let x = lx + (i % cols) as f32 * (cell + gap);
        let y = grid_top + (i / cols) as f32 * (cell + gap);
        let hov = app.ui.hover(x, y, cell, cell);
        let can = ready(r);
        let has = r.can_craft(&g.player.inv);
        let bg = if app.book_sel == Some(ri) {
            Color::from_rgba(250, 210, 110, 255)
        } else if hov {
            Color::from_rgba(214, 196, 156, 255)
        } else {
            Color::from_rgba(226, 212, 176, 255)
        };
        draw_rectangle(x, y, cell, cell, bg);
        let frame = if can { good } else if has { Color::from_rgba(200, 150, 30, 255) } else { Color::from_rgba(180, 160, 120, 255) };
        draw_rectangle_lines(x, y, cell, cell, if can || has { 2.5 * s } else { 1.0 * s }, frame);
        app.ui.item_icon(r.out.0, x + 2.0 * s, y + 2.0 * s, cell - 4.0 * s);
        if r.out.1 > 1 {
            let t = r.out.1.to_string();
            app.ui.text(&t, x + cell - app.ui.measure(&t, 11.0) - 3.0 * s, y + cell - 3.0 * s, 11.0, WHITE);
        }
        if hov {
            app.ui.set_tooltip(r.out.0.name(), Some(format!("{} · {}", r.station.name(), if can { "можно сделать" } else if has { "нужна станция" } else { "не хватает материалов" })));
            if app.ui.pressed {
                pick = Some((ri, false));
            }
        }
    }
    let footer = if list.is_empty() {
        "Ничего не найдено".to_string()
    } else if max_scroll > 0.0 {
        format!("Найдено: {} · колесо — листать ({}/{})", list.len(), app.book_scroll as usize + 1, max_scroll as usize + 1)
    } else {
        format!("Найдено: {}", list.len())
    };
    app.ui.text(&footer, lx, by + bh - 14.0 * s, 12.0, faded);

    // ---------- правый лист: страница рецепта ----------
    let rx = bx + half + 24.0 * s;
    let rw = half - 48.0 * s;
    let mut ry = by + 34.0 * s;
    let mut craft_times = 0;
    if let Some(ri) = app.book_sel {
        let r = &all[ri];
        if !app.book_back.is_empty() && app.ui.button_small(rx + rw - 90.0 * s, by + 14.0 * s, 90.0 * s, 26.0 * s, "← Назад") {
            app.book_sel = app.book_back.pop();
            return None;
        }
        let big = 72.0 * s;
        draw_rectangle(rx, ry, big, big, Color::from_rgba(224, 208, 170, 255));
        app.ui.item_icon(r.out.0, rx + 4.0 * s, ry + 4.0 * s, big - 8.0 * s);
        let title = if r.out.1 > 1 { format!("{} ×{}", r.out.0.name(), r.out.1) } else { r.out.0.name() };
        app.ui.text(&title, rx + big + 14.0 * s, ry + 26.0 * s, 18.0, ink);
        if let Some(d) = r.out.0.description() {
            app.ui.text(&d, rx + big + 14.0 * s, ry + 48.0 * s, 12.0, faded);
        }
        let st_ok = stations.contains(&r.station);
        let where_ = if r.station == crafting::Station::Hand { "Делается в руках".to_string() } else { format!("Нужна станция: {}", r.station.name()) };
        app.ui.text(&where_, rx + big + 14.0 * s, ry + 68.0 * s, 13.0, if st_ok { good } else { bad });
        ry += big + 26.0 * s;
        app.ui.text("Ингредиенты", rx, ry, 16.0, ink);
        ry += 10.0 * s;
        for (it, n) in &r.ins {
            let have = g.player.inv.count(*it);
            let row = 34.0 * s;
            let hov = app.ui.hover(rx, ry, rw, row);
            if hov {
                draw_rectangle(rx, ry, rw, row, Color::from_rgba(214, 196, 156, 255));
            }
            app.ui.item_icon(*it, rx + 2.0 * s, ry + 2.0 * s, row - 4.0 * s);
            app.ui.text(&it.name(), rx + row + 8.0 * s, ry + 22.0 * s, 14.0, ink);
            let cnt = format!("{have} / {n}");
            app.ui.text(&cnt, rx + rw - app.ui.measure(&cnt, 14.0) - 4.0 * s, ry + 22.0 * s, 14.0, if have >= *n as u32 { good } else { bad });
            // переход к рецепту ингредиента
            let src = all.iter().position(|x| x.out.0 == *it);
            if hov {
                app.ui.set_tooltip(it.name(), Some(if src.is_some() { "Щёлкните — открыть рецепт".into() } else { "Добывается в мире".into() }));
                if app.ui.pressed {
                    if let Some(si) = src {
                        pick = Some((si, true));
                    }
                }
            }
            ry += row + 2.0 * s;
        }
        // кнопки крафта
        ry += 10.0 * s;
        let ok = ready(r);
        let bwid = (rw - 10.0 * s) / 2.0;
        if ok {
            if app.ui.button_small(rx, ry, bwid, 32.0 * s, "Сделать") {
                craft_times = 1;
            }
            if app.ui.button_small(rx + bwid + 10.0 * s, ry, bwid, 32.0 * s, "Сделать ×8") {
                craft_times = 8;
            }
        } else {
            let why = if !r.can_craft(&g.player.inv) { "Не хватает материалов" } else { "Подойдите к нужной станции" };
            draw_rectangle(rx, ry, rw, 32.0 * s, Color::from_rgba(214, 196, 156, 255));
            app.ui.text_center(why, rx + rw / 2.0, ry + 21.0 * s, 13.0, bad);
        }
        ry += 50.0 * s;
        // где используется
        let uses: Vec<usize> = (0..all.len()).filter(|&i| all[i].ins.iter().any(|(it, _)| *it == r.out.0)).collect();
        app.ui.text(&format!("Используется в рецептах: {}", uses.len()), rx, ry, 15.0, ink);
        ry += 8.0 * s;
        let uc = 36.0 * s;
        let ucols = ((rw + gap) / (uc + gap)).floor().max(1.0) as usize;
        let urows = ((by + bh - 16.0 * s - ry) / (uc + gap)).floor().max(0.0) as usize;
        for (k, &ui) in uses.iter().take(ucols * urows).enumerate() {
            let x = rx + (k % ucols) as f32 * (uc + gap);
            let y = ry + (k / ucols) as f32 * (uc + gap);
            let hov = app.ui.hover(x, y, uc, uc);
            draw_rectangle(x, y, uc, uc, if hov { Color::from_rgba(214, 196, 156, 255) } else { Color::from_rgba(224, 208, 170, 255) });
            app.ui.item_icon(all[ui].out.0, x + 2.0 * s, y + 2.0 * s, uc - 4.0 * s);
            if hov {
                app.ui.set_tooltip(all[ui].out.0.name(), Some("Щёлкните — открыть рецепт".into()));
                if app.ui.pressed {
                    pick = Some((ui, true));
                }
            }
        }
    } else {
        // титульная страница
        let total = all.len();
        let can = all.iter().filter(|r| ready(r)).count();
        let lines = [
            ("Как пользоваться", 18.0, ink),
            ("", 8.0, ink),
            ("Выберите предмет слева, чтобы увидеть рецепт.", 14.0, ink),
            ("Зелёная рамка — можно сделать прямо сейчас.", 14.0, good),
            ("Жёлтая рамка — материалы есть, нужна станция.", 14.0, Color::from_rgba(170, 120, 20, 255)),
            ("Щелчок по ингредиенту открывает его рецепт.", 14.0, ink),
            ("Печатайте на клавиатуре, чтобы искать по названию.", 14.0, ink),
            ("Backspace стирает поиск.", 14.0, ink),
            ("", 8.0, ink),
            ("Станции", 18.0, ink),
            ("Верстак, печь, котёл и наковальня работают,", 14.0, ink),
            ("если стоят в 4 блоках от вас.", 14.0, ink),
            ("", 8.0, ink),
        ];
        for (t, size, c) in lines {
            if !t.is_empty() {
                app.ui.text(t, rx, ry, size, c);
            }
            ry += (size + 10.0) * s;
        }
        let near: Vec<&str> = stations.iter().skip(1).map(|s| s.name()).collect();
        app.ui.text(&format!("Рядом: {}", if near.is_empty() { "нет станций".to_string() } else { near.join(", ") }), rx, ry, 14.0, faded);
        ry += 30.0 * s;
        app.ui.text(&format!("Всего рецептов: {total}"), rx, ry, 16.0, ink);
        ry += 24.0 * s;
        app.ui.text(&format!("Можно сделать сейчас: {can}"), rx, ry, 16.0, good);
        app.ui.text("Esc — закрыть книгу (или E, если поиск пуст)", rx, by + bh - 14.0 * s, 12.0, faded);
    }
    if let Some(ri) = app.book_sel {
        if craft_times > 0 {
            let r = &all[ri];
            let mut made = 0;
            for _ in 0..craft_times {
                if crafting::craft(r, &mut g.player.inv) {
                    made += 1;
                }
            }
            if made > 0 {
                g.toast = Some((format!("Сделано: {} ×{}", r.out.0.name(), r.out.1 as u32 * made), 1.5));
            }
        }
    }
    if let Some((p, link)) = pick {
        if link {
            // переход по ссылке: запомнить страницу для кнопки «Назад»
            if let Some(cur) = app.book_sel {
                app.book_back.push(cur);
            }
        } else {
            app.book_back.clear();
        }
        app.book_sel = Some(p);
    }
    if !app.just_opened && (close || is_key_pressed(KeyCode::Escape)) {
        return Some(Screen::Playing);
    }
    None
}

/// Какие мелодии подходят к месту и времени (играют по очереди).
fn music_for(g: &Game) -> &'static [music::Track] {
    use music::Track as T;
    let p = g.player.body.pos;
    if let Some(b) = g.mobs.list.iter().find(|m| m.is_boss() && m.alive() && (m.body.pos - p).length() < 48.0) {
        return if b.enraged { &[T::Fury] } else { &[T::Boss, T::Fury] };
    }
    if g.dim == Dim::Heaven {
        return &[T::Heaven, T::Clouds];
    }
    if g.dim == Dim::Abyss {
        return &[T::Caves, T::Night];
    }
    let (x, y, z) = (p.x.floor() as i32, (p.y + 1.6).floor() as i32, p.z.floor() as i32);
    if y < 56 && !g.world().sky_exposed(x, y, z) {
        return &[T::Caves];
    }
    let biome = worldgen::column(g.seed, x, z).biome;
    if g.daylight() < 0.3 {
        return if biome == worldgen::Biome::Taiga { &[T::Aurora, T::Night] } else { &[T::Night, T::Dusk] };
    }
    if g.creative {
        return &[T::Workshop, T::Meadow, T::Village];
    }
    match biome {
        worldgen::Biome::Desert => &[T::Desert, T::Dusk],
        worldgen::Biome::Taiga => &[T::Taiga, T::Aurora],
        worldgen::Biome::Ocean => &[T::Ocean, T::Dawn],
        _ => &[T::Meadow, T::Village, T::Dawn],
    }
}

fn map_screen(app: &mut App) -> Option<Screen> {
    let Some(g) = app.game.as_ref() else { return Some(Screen::Title) };
    let me = vec2(g.player.body.pos.x, g.player.body.pos.z);
    let dc = dim_code(g.dim);
    let mut markers: Vec<map::Marker> = Vec::new();
    if g.dim == Dim::Overworld {
        markers.push(map::Marker { pos: vec2(g.spawn.x, g.spawn.z), label: "Дом (возрождение)".into(), color: Color::new(0.3, 0.9, 0.4, 1.0), yaw: None });
        if let Some(p) = g.portal_home {
            markers.push(map::Marker { pos: vec2(p.x as f32, p.z as f32), label: "Портал".into(), color: Color::new(0.75, 0.45, 1.0, 1.0), yaw: None });
        }
    }
    for r in g.remotes.values() {
        if r.st.dim == dc {
            markers.push(map::Marker { pos: vec2(r.st.pos[0], r.st.pos[2]), label: r.st.name.clone(), color: Color::new(0.35, 0.7, 1.0, 1.0), yaw: Some(r.st.yaw) });
        }
    }
    for (d, p, name) in &g.markers {
        if *d == dc {
            markers.push(map::Marker { pos: *p, label: name.clone(), color: Color::new(1.0, 0.85, 0.2, 1.0), yaw: None });
        }
    }
    for m in g.mobs.list.iter().filter(|m| m.is_boss() && m.alive()) {
        markers.push(map::Marker { pos: vec2(m.body.pos.x, m.body.pos.z), label: format!("Босс: {}", m.d().name), color: Color::new(1.0, 0.25, 0.25, 1.0), yaw: None });
    }
    let (seed, dim, yaw, just) = (g.seed, g.dim, g.player.yaw, app.just_opened);
    let view = app.map.get_or_insert_with(|| map::MapView::new(me));
    let close = view.draw(&mut app.ui, seed, dim, me, yaw, &markers, just);
    // ПКМ по карте: поставить метку или убрать ближайшую
    if app.ui.rpressed {
        if let Some(wp) = view.screen_to_world(app.ui.mouse) {
            let near_r = view.scale() * 8.0;
            let g = app.game.as_mut().unwrap();
            if let Some(i) = g.markers.iter().position(|(d, p, _)| *d == dc && (*p - wp).length() < near_r) {
                g.markers.remove(i);
            } else {
                let n = g.markers.len() + 1;
                g.markers.push((dc, wp, format!("Метка {n}")));
                g.unlock(ach::MARKER);
            }
        }
    }
    if close {
        return Some(Screen::Playing);
    }
    None
}

/// Стол улучшений: зачарования для предмета в руке за уровни опыта.
fn enchant_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.5));
    let (pw, ph) = ((600.0 * s).min(w - 20.0), 420.0 * s);
    let (px, py) = ((w - pw) / 2.0, (h - ph) / 2.0);
    app.ui.panel(px, py, pw, ph);
    app.ui.text("Стол улучшений", px + 18.0 * s, py + 34.0 * s, 22.0, Color::new(0.75, 0.6, 1.0, 1.0));
    let g = app.game.as_mut().unwrap();
    let (lvl, _) = g.player.level();
    let lt = format!("Ваш уровень: {lvl}");
    app.ui.text(&lt, px + pw - app.ui.measure(&lt, 15.0) - 18.0 * s, py + 34.0 * s, 15.0, Color::new(0.6, 1.0, 0.4, 1.0));
    let sel = g.player.selected;
    let st = g.player.inv.slots[sel];
    app.ui.item_icon(st.item, px + 20.0 * s, py + 56.0 * s, 56.0 * s);
    let cur = ench::name(st.ench).map_or("без зачарования".to_string(), |n| format!("сейчас: {n}"));
    app.ui.text(&st.item.name(), px + 90.0 * s, py + 80.0 * s, 16.0, WHITE);
    app.ui.text(&cur, px + 90.0 * s, py + 102.0 * s, 13.0, GRAY);
    let opts = ench::options(st.item);
    let mut chosen: Option<(u8, u8)> = None;
    if opts.is_empty() {
        app.ui.text("Возьмите в руку оружие, инструмент, броню, лук или щит.", px + 20.0 * s, py + 150.0 * s, 14.0, WHITE);
    }
    let mut y = py + 132.0 * s;
    for k in &opts {
        app.ui.text(ench::kind_name(*k), px + 20.0 * s, y + 26.0 * s, 15.0, WHITE);
        for l in 1..=3u8 {
            let cost = ench::cost(l);
            let bw = 120.0 * s;
            let bx = px + pw - (4 - l) as f32 * (bw + 8.0 * s) - 12.0 * s;
            let label = format!("{} — {} ур.", ["", "I", "II", "III"][l as usize], cost);
            if app.ui.button_small(bx, y + 6.0 * s, bw, 30.0 * s, &label) && (lvl >= cost || g.creative) {
                chosen = Some((*k, l));
            }
        }
        y += 44.0 * s;
    }
    app.ui.text("Опыт — зелёные шарики с существ и руды. Esc или E — закрыть", px + 18.0 * s, py + ph - 16.0 * s, 12.0, GRAY);
    if let Some((k, l)) = chosen {
        if g.creative || g.player.spend_levels(ench::cost(l)) {
            g.player.inv.slots[sel].ench = ench::pack(k, l);
            g.unlock(ach::ENCHANT);
            g.toast = Some((format!("Зачаровано: {}", ench::name(ench::pack(k, l)).unwrap_or_default()), 2.0));
            app.sfx.push((music::Sfx::Crit, 0.7));
        }
    }
    if !app.just_opened && (is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::E)) {
        return Some(Screen::Playing);
    }
    None
}

fn achievements_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.6));
    let got = app.game.as_ref().map_or(0, |g| g.ach);
    let n = got.count_ones();
    app.ui.text_center(&format!("Достижения: {n} из {}", ach::COUNT), w / 2.0, 50.0 * s, 26.0, WHITE);
    let cols = 2;
    let cw = (w - 60.0 * s) / cols as f32;
    let rh = 44.0 * s;
    for (i, a) in ach::LIST.iter().enumerate() {
        let x = 30.0 * s + (i % cols) as f32 * cw;
        let y = 76.0 * s + (i / cols) as f32 * (rh + 4.0 * s);
        let ok = got & (1 << i) != 0;
        draw_rectangle(x, y, cw - 10.0 * s, rh, if ok { Color::new(0.2, 0.17, 0.06, 0.9) } else { Color::new(0.1, 0.1, 0.12, 0.85) });
        app.ui.text(if ok { "★" } else { "☆" }, x + 10.0 * s, y + rh * 0.68, 18.0, if ok { Color::new(1.0, 0.8, 0.3, 1.0) } else { GRAY });
        app.ui.text(a.name, x + 36.0 * s, y + 19.0 * s, 15.0, if ok { WHITE } else { GRAY });
        app.ui.text(a.hint, x + 36.0 * s, y + 36.0 * s, 12.0, GRAY);
    }
    if app.ui.button((w - 240.0 * s) / 2.0, h - 54.0 * s, 240.0 * s, 40.0 * s, "Назад") || is_key_pressed(KeyCode::Escape) {
        return Some(Screen::Paused);
    }
    None
}

/// Выбор внешности: рубашка, штаны, волосы, кожа.
fn look_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.05, 0.06, 0.08, 0.92));
    app.ui.text_center("Внешность игрока", w / 2.0, 70.0 * s, 28.0, WHITE);
    app.ui.text_center("Её видят другие игроки по сети и вы — в виде от третьего лица (F5)", w / 2.0, 96.0 * s, 13.0, GRAY);
    let rows: [(&str, Vec<(u8, u8, u8)>); 4] = [
        ("Рубашка", model::SHIRTS.to_vec()),
        ("Штаны", model::PANTS.to_vec()),
        ("Волосы", model::HAIRS.to_vec()),
        ("Кожа", model::SKINS.to_vec()),
    ];
    let bw = 520.0 * s;
    let x0 = (w - bw) / 2.0;
    let mut y = 130.0 * s;
    for (i, (name, pal)) in rows.iter().enumerate() {
        app.ui.text(name, x0, y + 26.0 * s, 17.0, WHITE);
        let cur = app.settings.look[i];
        // «по имени» и образцы цветов
        let sw = 40.0 * s;
        let mut x = x0 + 120.0 * s;
        let auto = app.ui.hover(x, y, sw * 1.6, sw);
        draw_rectangle(x, y, sw * 1.6, sw, Color::new(0.2, 0.2, 0.24, 1.0));
        app.ui.text_center("авто", x + sw * 0.8, y + sw * 0.62, 12.0, WHITE);
        if cur == 255 {
            draw_rectangle_lines(x - 2.0, y - 2.0, sw * 1.6 + 4.0, sw + 4.0, 3.0, ACCENT);
        }
        if auto && app.ui.pressed {
            app.settings.look[i] = 255;
        }
        x += sw * 1.6 + 8.0 * s;
        for (k, c) in pal.iter().enumerate() {
            draw_rectangle(x, y, sw, sw, Color::from_rgba(c.0, c.1, c.2, 255));
            if cur as usize == k {
                draw_rectangle_lines(x - 2.0, y - 2.0, sw + 4.0, sw + 4.0, 3.0, ACCENT);
            }
            if app.ui.hover(x, y, sw, sw) && app.ui.pressed {
                app.settings.look[i] = k as u8;
            }
            x += sw + 6.0 * s;
        }
        y += sw + 22.0 * s;
    }
    // простой предпросмотр: фигурка из прямоугольников
    let l = model::look_with(app.settings.look, hash_seed(&app.settings.name));
    let (px, py) = (w / 2.0 - 30.0 * s, y + 10.0 * s);
    let u = 10.0 * s;
    draw_rectangle(px + u, py, 4.0 * u, 4.0 * u, l.skin);
    draw_rectangle(px + u, py, 4.0 * u, 1.2 * u, l.hair);
    draw_rectangle(px + u, py + 4.0 * u, 4.0 * u, 5.0 * u, l.shirt);
    draw_rectangle(px - 0.6 * u, py + 4.0 * u, 1.6 * u, 4.5 * u, l.shirt);
    draw_rectangle(px + 5.0 * u, py + 4.0 * u, 1.6 * u, 4.5 * u, l.shirt);
    draw_rectangle(px + u, py + 9.0 * u, 4.0 * u, 4.5 * u, l.pants);
    if app.ui.button((w - 240.0 * s) / 2.0, h - 60.0 * s, 240.0 * s, 42.0 * s, "Готово") || is_key_pressed(KeyCode::Escape) {
        app.settings.save();
        return Some(Screen::Settings);
    }
    None
}

/// «Текстуры и небо»: стиль, разрешение, текстур-пак, небо, шаблон для своего пака.
fn textures_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.05, 0.06, 0.08, 0.92));
    app.ui.text_center("Текстуры и небо", w / 2.0, 60.0 * s, 28.0, WHITE);
    let bw = (520.0 * s).min(w - 32.0);
    let x = (w - bw) / 2.0;
    let bh = 38.0 * s;
    let gap = 8.0 * s;
    let mut y = 86.0 * s;
    let restart = |changed: bool| if changed { " (после перезапуска)" } else { "" };
    let st = &mut app.settings;
    let lab = format!("Стиль: {}{}", blocks::STYLE_NAMES[st.tex_style.min(1) as usize], restart(st.tex_style != app.style_active));
    if app.ui.button(x, y, bw, bh, &lab) {
        st.tex_style = 1 - st.tex_style.min(1);
    }
    y += bh + gap;
    let pack_on = !st.tex_pack.is_empty();
    let res_note = if pack_on { " (задаёт пак)".to_string() } else { restart(st.tex_res != app.tex_res_active).to_string() };
    if app.ui.button(x, y, bw, bh, &format!("Разрешение: x{0} ({0}×{0} пикс.){res_note}", st.tex_res)) {
        let i = blocks::TEX_RES.iter().position(|&r| r == st.tex_res as usize).unwrap_or(1);
        st.tex_res = blocks::TEX_RES[(i + 1) % blocks::TEX_RES.len()] as u32;
    }
    y += bh + gap;
    let pname = if pack_on { st.tex_pack.clone() } else { "встроенные".to_string() };
    let lab = format!("Текстур-пак: {pname}{}", restart(st.tex_pack != app.pack_active));
    if app.ui.button(x, y, bw, bh, &lab) {
        // по кругу: встроенные → паки из папки
        let mut opts = vec![String::new()];
        opts.extend(app.pack_list.iter().cloned());
        let i = opts.iter().position(|p| *p == st.tex_pack).unwrap_or(0);
        st.tex_pack = opts[(i + 1) % opts.len()].clone();
    }
    y += bh + gap;
    let sky = ["голубое", "розовое"][st.sky.min(1) as usize];
    if app.ui.button(x, y, bw, bh, &format!("Небо: {sky}")) {
        st.sky = 1 - st.sky.min(1);
    }
    y += bh + gap;
    let half = (bw - gap) / 2.0;
    let export = app.ui.button(x, y, half, bh, "Сохранить шаблон");
    let refresh = app.ui.button(x + half + gap, y, half, bh, "Обновить список паков");
    y += bh + gap * 2.0;
    let dir = packs::dir();
    let lines = [
        "Свой пак: нажмите «Сохранить шаблон», перерисуйте template.png".to_string(),
        "в любом редакторе и сохраните под своим именем (например, mypack.png).".to_string(),
        "Плитки можно увеличить: 32, 64… пикселей — картинку растянуть целиком.".to_string(),
        "Прозрачные плитки берутся из встроенных текстур.".to_string(),
        format!("Папка: {}", dir.display()),
        format!("Найдено паков: {}", app.pack_list.len()),
    ];
    for l in &lines {
        app.ui.text_center(l, w / 2.0, y + 14.0 * s, 13.0, Color::new(0.85, 0.85, 0.8, 1.0));
        y += 20.0 * s;
    }
    if export {
        app.status = Some((
            match packs::export_template() {
                Ok(p) => format!("Шаблон сохранён: {}", p.display()),
                Err(e) => format!("Не удалось сохранить шаблон: {e}"),
            },
            8.0,
        ));
        app.pack_list = packs::list();
    }
    if refresh {
        app.pack_list = packs::list();
    }
    if app.ui.button((w - 240.0 * s) / 2.0, h - 60.0 * s, 240.0 * s, 42.0 * s, "Готово") || is_key_pressed(KeyCode::Escape) {
        app.settings.save();
        return Some(Screen::Settings);
    }
    None
}

/// Открыть мир через ретранслятор: ввести его адрес, получить код комнаты.
fn relay_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.03, 0.04, 0.06, 0.92));
    app.ui.text_center("Игра через интернет", w / 2.0, 80.0 * s, 28.0, WHITE);
    let lines = [
        "Нужен ретранслятор — компьютер с внешним адресом (например, арендованный сервер).",
        "На нём запустите: rastkraft --relay   (порт 7780 должен быть открыт).",
        "Здесь впишите его адрес. Друг вводит в «Сетевой игре»: адрес#КОД.",
    ];
    for (i, l) in lines.iter().enumerate() {
        app.ui.text_center(l, w / 2.0, 120.0 * s + i as f32 * 22.0 * s, 14.0, GRAY);
    }
    if !app.just_opened {
        while let Some(c) = get_char_pressed() {
            if !c.is_control() && app.settings.relay.chars().count() < 60 {
                app.settings.relay.push(c);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            app.settings.relay.pop();
        }
    } else {
        while get_char_pressed().is_some() {}
    }
    let bw = 520.0 * s;
    let bx = (w - bw) / 2.0;
    let y = 200.0 * s;
    draw_rectangle(bx, y, bw, 40.0 * s, Color::new(0.12, 0.13, 0.16, 1.0));
    draw_rectangle_lines(bx, y, bw, 40.0 * s, 2.0, ACCENT);
    let caret = if (app.time * 2.0).fract() < 0.5 { "|" } else { "" };
    let shown = if app.settings.relay.is_empty() { format!("адрес ретранслятора, например 203.0.113.5{caret}") } else { format!("{}{caret}", app.settings.relay) };
    app.ui.text(&shown, bx + 12.0 * s, y + 26.0 * s, 15.0, if app.settings.relay.is_empty() { GRAY } else { WHITE });
    if app.ui.button(bx, y + 60.0 * s, bw, 44.0 * s, "Открыть мир через ретранслятор") && !app.settings.relay.is_empty() {
        app.settings.save();
        let room = format!("{:04}", (app.time * 7919.0) as u32 % 10000);
        let relay = app.settings.relay.clone();
        // сначала обычный сервер на этом компьютере, затем комната на ретрансляторе
        let have_net = app.game.as_ref().is_some_and(|g| g.net.is_some());
        let net_ok = have_net || match Net::host(&app.settings.name) {
            Ok(n) => {
                app.game.as_mut().unwrap().net = Some(n);
                true
            }
            Err(e) => {
                app.status = Some((format!("Не удалось открыть сервер: {e}"), 5.0));
                false
            }
        };
        if net_ok {
            match net::relay_host(&relay, &room) {
                Ok(()) => {
                    let msg = format!("Код комнаты: {room}. Друг вводит: {relay}#{room}");
                    let g = app.game.as_mut().unwrap();
                    g.say(msg.clone());
                    g.toast = Some((msg.clone(), 10.0));
                    if let Some(n) = g.net.as_mut() {
                        n.info = msg;
                    }
                    return Some(Screen::Playing);
                }
                Err(e) => app.status = Some((format!("Ретранслятор не отвечает: {e}"), 5.0)),
            }
        }
    }
    if app.ui.button(bx, y + 116.0 * s, bw, 44.0 * s, "Назад") || is_key_pressed(KeyCode::Escape) {
        return Some(Screen::Paused);
    }
    None
}

/// Редактор сенсорных кнопок: перетащите кнопку пальцем, ниже — её размер,
/// общая прозрачность и масштаб.
fn controls_screen(app: &mut App) -> Option<Screen> {
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.12, 0.16, 0.12, 1.0));
    // «мир» на фоне для наглядности
    draw_rectangle(0.0, h * 0.62, w, h * 0.38, Color::new(0.35, 0.55, 0.3, 1.0));
    app.ui.text_center("Перетащите кнопки, выберите кнопку и меняйте её размер", w / 2.0, h * 0.19, 16.0, WHITE);
    let hint = if app.touch_layout.tap { "Касание экрана: коротко — поставить блок или ударить, удерживать — ломать" } else { "Касание экрана выключено: ставьте и ломайте кнопками" };
    app.ui.text_center(hint, w / 2.0, h * 0.25, 12.0, Color::new(0.85, 0.9, 0.8, 1.0));
    let mouse = app.ui.mouse;
    // выбор и перетаскивание
    if app.ui.pressed {
        let hit = touch::ACTS.iter().copied().find(|a| {
            let (x, y, r) = app.touch_layout.get(*a);
            (mouse - vec2(x * w, y * h)).length() < r * h * app.touch_layout.scale * 1.1
        });
        if hit.is_some() {
            app.touch_edit = hit;
        }
    }
    if let Some(a) = app.touch_edit {
        if app.ui.down && mouse.y < h * 0.98 {
            let (x, y, r) = app.touch_layout.get(a);
            let inside_panel = mouse.y > h * 0.3 && mouse.y < h * 0.3 + 200.0 * s && mouse.x > w * 0.25 && mouse.x < w * 0.75;
            let (cx, cy) = (x * w, y * h);
            if !inside_panel && (mouse - vec2(cx, cy)).length() < r * h * app.touch_layout.scale * 3.0 {
                app.touch_layout.pos.insert(a, ((mouse.x / w).clamp(0.02, 0.98), (mouse.y / h).clamp(0.02, 0.98), r));
            }
        }
    }
    touch::draw(&app.touch_layout, &app.ui, true);
    if let Some(a) = app.touch_edit {
        let (x, y, r) = app.touch_layout.get(a);
        draw_circle_lines(x * w, y * h, r * h * app.touch_layout.scale + 4.0, 4.0, ACCENT);
    }
    // панель настроек
    let pw = w * 0.5;
    let (px, py) = (w * 0.25, h * 0.3);
    let bh = 32.0 * s;
    draw_rectangle(px, py, pw, 26.0 * s + bh * 5.0 + 8.0 * s, Color::new(0.0, 0.0, 0.0, 0.7));
    let label = app.touch_edit.map_or("Кнопка не выбрана".to_string(), |a| format!("Размер «{}»", a.label()));
    if let Some(a) = app.touch_edit {
        let (x, y, r) = app.touch_layout.get(a);
        let nr = app.ui.slider(21, px + 10.0 * s, py + 8.0 * s, pw - 20.0 * s, bh, &label, r, 0.025, 0.15);
        app.touch_layout.pos.insert(a, (x, y, nr));
    } else {
        app.ui.text_center(&label, w / 2.0, py + 28.0 * s, 14.0, GRAY);
    }
    let op = app.touch_layout.opacity;
    app.touch_layout.opacity = app.ui.slider(22, px + 10.0 * s, py + 12.0 * s + bh, pw - 20.0 * s, bh, &format!("Прозрачность: {:.0}%", op * 100.0), op, 0.1, 1.0);
    let sc = app.touch_layout.scale;
    app.touch_layout.scale = app.ui.slider(23, px + 10.0 * s, py + 16.0 * s + bh * 2.0, pw - 20.0 * s, bh, &format!("Все кнопки: {:.0}%", sc * 100.0), sc, 0.6, 1.8);
    let half = (pw - 30.0 * s) / 2.0;
    let ty = py + 20.0 * s + bh * 3.0;
    let lab = format!("Подписи: {}", if app.touch_layout.labels { "Вкл" } else { "Выкл" });
    if app.ui.button_small(px + 10.0 * s, ty, half, bh, &lab) {
        app.touch_layout.labels = !app.touch_layout.labels;
    }
    let tap = format!("Касание экрана: {}", if app.touch_layout.tap { "Вкл" } else { "Выкл" });
    if app.ui.button_small(px + 20.0 * s + half, ty, half, bh, &tap) {
        app.touch_layout.tap = !app.touch_layout.tap;
    }
    let by = ty + bh + 6.0 * s;
    if app.ui.button_small(px + 10.0 * s, by, half, bh, "Сбросить") {
        app.touch_layout = touch::Layout::default();
        app.touch_edit = None;
    }
    if app.ui.button_small(px + 20.0 * s + half, by, half, bh, "Готово") || is_key_pressed(KeyCode::Escape) {
        app.settings.touch_layout = app.touch_layout.to_text();
        app.settings.save();
        app.touch_edit = None;
        return Some(Screen::Settings);
    }
    None
}
