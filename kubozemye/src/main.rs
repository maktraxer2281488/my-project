//! Кубоземье — воксельная песочница на Rust.

mod mesh;
mod player;
mod textures;
mod world;

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Comparison, Equation, PipelineParams, UniformType};
use macroquad::prelude::*;
use mesh::{CLOUD_PERIOD, ChunkMesh, build_chunk, build_clouds};
use player::Player;
use world::{Block, CHUNK, CHUNKS, World};

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;
varying mediump float dist;

uniform mat4 Model;
uniform mat4 Projection;
uniform vec3 CamPos;

void main() {
    vec4 wp = Model * vec4(position, 1.0);
    gl_Position = Projection * wp;
    color = color0 / 255.0;
    uv = texcoord;
    dist = length(wp.xyz - CamPos);
}"#;

const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
varying mediump float dist;

uniform sampler2D Texture;
uniform vec4 FogColor;
uniform vec2 Fog;

void main() {
    vec4 t = texture2D(Texture, uv);
    if (t.a < 0.5) discard;
    vec4 c = color * t;
    float f = clamp((dist - Fog.x) / (Fog.y - Fog.x), 0.0, 1.0);
    f = f * f;
    gl_FragColor = vec4(mix(c.rgb, FogColor.rgb, f), c.a);
}"#;

const FOG_START: f32 = 50.0;
const FOG_END: f32 = 140.0;
const REACH: f32 = 7.0;

const SKY_TOP: Color = Color::new(0.28, 0.52, 0.86, 1.0);
const SKY_HORIZON: Color = Color::new(0.74, 0.86, 0.97, 1.0);

const HOTBAR: [Block; 9] = [
    Block::Grass,
    Block::Dirt,
    Block::Stone,
    Block::Cobble,
    Block::Planks,
    Block::Log,
    Block::Bricks,
    Block::Glass,
    Block::Sand,
];

fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Кубоземье".to_owned(),
            window_width: 1280,
            window_height: 720,
            window_resizable: true,
            ..Default::default()
        },
        draw_call_vertex_capacity: mesh::MAX_VERTS,
        draw_call_index_capacity: mesh::MAX_VERTS / 4 * 6,
        ..Default::default()
    }
}

fn make_material(blend: bool) -> Material {
    // Глубину пишем и для прозрачного прохода: иначе в miniquad «протекают»
    // грани воды сквозь ближнюю геометрию. Вода рисуется после непрозрачного.
    let pipeline_params = PipelineParams {
        depth_write: true,
        depth_test: Comparison::LessOrEqual,
        color_blend: blend.then(|| {
            BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )
        }),
        ..Default::default()
    };
    load_material(
        ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
        MaterialParams {
            pipeline_params,
            uniforms: vec![
                UniformDesc::new("CamPos", UniformType::Float3),
                UniformDesc::new("FogColor", UniformType::Float4),
                UniformDesc::new("Fog", UniformType::Float2),
            ],
            ..Default::default()
        },
    )
    .expect("не удалось собрать шейдер")
}

fn set_fog(m: &Material, cam: Vec3) {
    m.set_uniform("CamPos", [cam.x, cam.y, cam.z]);
    m.set_uniform("FogColor", [SKY_HORIZON.r, SKY_HORIZON.g, SKY_HORIZON.b, 1.0]);
    m.set_uniform("Fog", [FOG_START, FOG_END]);
}

/// Чанки, которые надо перестроить после изменения блока (включая соседей — из-за затенения).
fn dirty_chunks(p: IVec3) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for dz in -1..=1 {
        for dx in -1..=1 {
            let (x, z) = (p.x + dx, p.z + dz);
            if x < 0 || z < 0 {
                continue;
            }
            let (cx, cz) = (x as usize / CHUNK, z as usize / CHUNK);
            if cx < CHUNKS && cz < CHUNKS && !out.contains(&(cx, cz)) {
                out.push((cx, cz));
            }
        }
    }
    out
}

fn text(font: &Font, s: &str, x: f32, y: f32, size: u16, color: Color) {
    let params = TextParams { font: Some(font), font_size: size, color: Color::new(0.0, 0.0, 0.0, 0.45 * color.a), ..Default::default() };
    draw_text_ex(s, x + 2.0, y + 2.0, params.clone());
    draw_text_ex(s, x, y, TextParams { color, ..params });
}

fn draw_sky(cam: &Camera3D, sun_dir: Vec3) {
    let (w, h) = (screen_width(), screen_height());
    // линия горизонта на экране
    let m = cam.matrix();
    let ahead = vec3(cam.target.x - cam.position.x, 0.0, cam.target.z - cam.position.z).normalize_or_zero();
    let clip = m * (cam.position + ahead * 1000.0).extend(1.0);
    let horizon = if clip.w > 0.0 { (1.0 - (clip.y / clip.w)) * 0.5 * h } else { h * 0.5 };
    const BANDS: i32 = 48;
    let top = horizon - h * 1.2;
    for i in 0..BANDS {
        let t0 = i as f32 / BANDS as f32;
        let y0 = top + (horizon - top) * t0;
        let y1 = top + (horizon - top) * (i + 1) as f32 / BANDS as f32;
        let k = t0.powf(2.2);
        let c = Color::new(
            SKY_TOP.r + (SKY_HORIZON.r - SKY_TOP.r) * k,
            SKY_TOP.g + (SKY_HORIZON.g - SKY_TOP.g) * k,
            SKY_TOP.b + (SKY_HORIZON.b - SKY_TOP.b) * k,
            1.0,
        );
        draw_rectangle(0.0, y0, w, y1 - y0 + 1.0, c);
    }
    if horizon < h {
        draw_rectangle(0.0, horizon, w, h - horizon, SKY_HORIZON);
    }
    if top > 0.0 {
        draw_rectangle(0.0, 0.0, w, top, SKY_TOP);
    }

    // солнце
    let sc = m * (cam.position + sun_dir * 1000.0).extend(1.0);
    if sc.w > 0.0 {
        let sx = (sc.x / sc.w + 1.0) * 0.5 * w;
        let sy = (1.0 - sc.y / sc.w) * 0.5 * h;
        let r = h * 0.045;
        for (k, a) in [(3.2, 0.08), (2.2, 0.14), (1.5, 0.25)] {
            draw_circle(sx, sy, r * k, Color::new(1.0, 0.96, 0.8, a));
        }
        draw_circle(sx, sy, r, Color::new(1.0, 0.98, 0.88, 1.0));
    }
}

fn draw_hud(font: &Font, atlas: &Texture2D, player: &Player, selected: usize, fps: i32, label_timer: f32, show_help: bool, grabbed: bool) {
    let (w, h) = (screen_width(), screen_height());

    // прицел
    let cc = Color::new(1.0, 1.0, 1.0, 0.9);
    draw_rectangle(w / 2.0 - 10.0, h / 2.0 - 1.0, 20.0, 2.0, cc);
    draw_rectangle(w / 2.0 - 1.0, h / 2.0 - 10.0, 2.0, 20.0, cc);

    // панель блоков
    let slot = 52.0;
    let gap = 6.0;
    let total = HOTBAR.len() as f32 * (slot + gap) - gap;
    let x0 = (w - total) / 2.0;
    let y0 = h - slot - 18.0;
    draw_rectangle(x0 - 10.0, y0 - 10.0, total + 20.0, slot + 20.0, Color::new(0.06, 0.07, 0.09, 0.55));
    for (i, b) in HOTBAR.iter().enumerate() {
        let x = x0 + i as f32 * (slot + gap);
        let sel = i == selected;
        let bg = if sel { Color::new(1.0, 0.82, 0.3, 0.95) } else { Color::new(1.0, 1.0, 1.0, 0.12) };
        draw_rectangle(x - 2.0, y0 - 2.0, slot + 4.0, slot + 4.0, bg);
        draw_rectangle(x, y0, slot, slot, Color::new(0.1, 0.11, 0.13, 0.85));
        let (_, side, _) = b.tiles();
        let (u0, v0, u1, v1) = textures::tile_uv(side);
        let a = textures::ATLAS as f32;
        draw_texture_ex(
            atlas,
            x + 8.0,
            y0 + 8.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(slot - 16.0, slot - 16.0)),
                source: Some(Rect::new(u0 * a, v0 * a, (u1 - u0) * a, (v1 - v0) * a)),
                ..Default::default()
            },
        );
        text(font, &(i + 1).to_string(), x + 4.0, y0 + 14.0, 13, Color::new(1.0, 1.0, 1.0, 0.75));
    }

    // название выбранного блока
    if label_timer > 0.0 {
        let a = label_timer.min(1.0);
        let name = HOTBAR[selected].name();
        let dim = measure_text(name, Some(font), 22, 1.0);
        text(font, name, (w - dim.width) / 2.0, y0 - 22.0, 22, Color::new(1.0, 1.0, 1.0, a));
    }

    // информация
    let p = player.pos;
    text(font, "КУБОЗЕМЬЕ", 18.0, 32.0, 22, Color::new(1.0, 0.85, 0.35, 1.0));
    text(font, &format!("{fps} к/с   x {:.0}  y {:.0}  z {:.0}", p.x, p.y, p.z), 18.0, 56.0, 15, WHITE);
    let mode = if player.flying { "полёт" } else if player.in_water { "плавание" } else { "ходьба" };
    text(font, &format!("Режим: {mode}   ·   F1 — подсказка"), 18.0, 78.0, 15, Color::new(1.0, 1.0, 1.0, 0.8));

    if show_help {
        let lines = [
            "WASD — идти,  Пробел — прыжок / всплыть",
            "Ctrl или R — бег,  F — полёт (Shift — вниз)",
            "ЛКМ — сломать,  ПКМ — поставить",
            "СКМ — взять блок под прицелом в руку",
            "1–9 или колесо — выбрать блок",
            "Esc — освободить мышь,  щелчок — вернуться",
        ];
        let bw = 470.0;
        let bh = 24.0 * lines.len() as f32 + 28.0;
        draw_rectangle(18.0, 96.0, bw, bh, Color::new(0.06, 0.07, 0.09, 0.6));
        for (i, l) in lines.iter().enumerate() {
            text(font, l, 34.0, 124.0 + i as f32 * 24.0, 16, WHITE);
        }
    }

    if !grabbed {
        let msg = "Пауза — щёлкните, чтобы продолжить";
        let dim = measure_text(msg, Some(font), 30, 1.0);
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.35));
        text(font, msg, (w - dim.width) / 2.0, h / 2.0 - 40.0, 30, WHITE);
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/DejaVuSans-Bold.ttf")).expect("шрифт");

    // экран загрузки
    clear_background(Color::new(0.08, 0.09, 0.11, 1.0));
    let msg = "Создаём мир…";
    let dim = measure_text(msg, Some(&font), 36, 1.0);
    text(&font, msg, (screen_width() - dim.width) / 2.0, screen_height() / 2.0, 36, WHITE);
    next_frame().await;

    let shot_path = std::env::var("KUBO_SHOT").ok();
    let seed: u32 = std::env::var("KUBO_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| (macroquad::miniquad::date::now() * 1000.0) as u32);

    let atlas = Texture2D::from_rgba8(textures::ATLAS as u16, textures::ATLAS as u16, &textures::build_atlas());
    atlas.set_filter(FilterMode::Nearest);

    let mut world = World::generate(seed);
    let mut chunks: Vec<ChunkMesh> = Vec::with_capacity(CHUNKS * CHUNKS);
    for cz in 0..CHUNKS {
        for cx in 0..CHUNKS {
            chunks.push(build_chunk(&world, cx, cz, &atlas));
        }
    }
    let clouds = build_clouds(seed);

    let opaque_mat = make_material(false);
    let blend_mat = make_material(true);

    // точка появления: ближайшая к центру суша
    let mut spawn = vec3(128.5, 0.0, 128.5);
    'search: for r in 0..100 {
        for dz in -r..=r {
            for dx in -r..=r {
                let (x, z) = (128 + dx, 128 + dz);
                let y = world.surface_y(x, z);
                if y > world::WATER_LEVEL + 1 && world.get(x, y - 1, z) != Block::Leaves {
                    spawn = vec3(x as f32 + 0.5, y as f32, z as f32 + 0.5);
                    break 'search;
                }
            }
        }
    }
    let mut player = Player::new(spawn);

    // поза для скриншота: KUBO_POSE="x,y,z,yaw,pitch"
    if let Ok(pose) = std::env::var("KUBO_POSE") {
        let v: Vec<f32> = pose.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if v.len() == 5 {
            player.pos = vec3(v[0], v[1], v[2]);
            player.yaw = v[3];
            player.pitch = v[4];
            player.flying = true;
        }
    }

    let sun_dir = vec3(0.45, 0.62, 0.64).normalize();
    let mut selected = 0usize;
    let mut label_timer = 2.5f32;
    let mut show_help = true;
    let mut grabbed = shot_path.is_none();
    set_cursor_grab(grabbed);
    show_mouse(!grabbed);
    let mut last_mouse: Vec2 = mouse_position().into();
    let mut frame = 0u32;
    let mut time = 0.0f32;
    let mut fps_smooth = 60.0f32;

    loop {
        let dt = get_frame_time().min(0.05);
        time += dt;
        frame += 1;
        fps_smooth += (1.0 / get_frame_time().max(1e-4) - fps_smooth) * 0.05;

        // --- ввод ---
        let mouse: Vec2 = mouse_position().into();
        let md = mouse - last_mouse;
        last_mouse = mouse;
        let mut click_consumed = false;

        if is_key_pressed(KeyCode::Escape) && grabbed {
            grabbed = false;
            set_cursor_grab(false);
            show_mouse(true);
        }
        if !grabbed && shot_path.is_none() && is_mouse_button_pressed(MouseButton::Left) {
            grabbed = true;
            set_cursor_grab(true);
            show_mouse(false);
            click_consumed = true;
        }

        if grabbed {
            player.look(md.x, md.y);
            if is_key_pressed(KeyCode::F) {
                player.flying = !player.flying;
                player.vel = Vec3::ZERO;
            }
            if is_key_pressed(KeyCode::F1) {
                show_help = !show_help;
            }
            let keys = [
                KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5,
                KeyCode::Key6, KeyCode::Key7, KeyCode::Key8, KeyCode::Key9,
            ];
            for (i, k) in keys.iter().enumerate() {
                if is_key_pressed(*k) {
                    selected = i;
                    label_timer = 2.0;
                }
            }
            let (_, wheel) = mouse_wheel();
            if wheel != 0.0 {
                let n = HOTBAR.len() as i32;
                selected = (selected as i32 - wheel.signum() as i32).rem_euclid(n) as usize;
                label_timer = 2.0;
            }
            player.update(&world, dt);
        }
        label_timer -= dt;

        let eye = player.eye();
        let fwd = player.forward();
        let target = world.raycast(eye, fwd, REACH);

        // --- ломаем / ставим ---
        if grabbed && !click_consumed {
            let mut changed: Option<IVec3> = None;
            if let Some((hit, prev)) = target {
                if is_mouse_button_pressed(MouseButton::Left) && hit.y > 0 {
                    world.set(hit.x, hit.y, hit.z, Block::Air);
                    // вода затекает в образовавшуюся пустоту
                    let near_water = [ivec3(1, 0, 0), ivec3(-1, 0, 0), ivec3(0, 0, 1), ivec3(0, 0, -1), ivec3(0, 1, 0)]
                        .iter()
                        .any(|o| world.get(hit.x + o.x, hit.y + o.y, hit.z + o.z) == Block::Water);
                    if near_water && hit.y <= world::WATER_LEVEL {
                        world.set(hit.x, hit.y, hit.z, Block::Water);
                    }
                    changed = Some(hit);
                }
                if is_mouse_button_pressed(MouseButton::Right) {
                    let cur = world.get(prev.x, prev.y, prev.z);
                    if (cur == Block::Air || cur == Block::Water) && !player.overlaps_block(prev) {
                        if world.set(prev.x, prev.y, prev.z, HOTBAR[selected]) {
                            changed = Some(prev);
                        }
                    }
                }
                if is_mouse_button_pressed(MouseButton::Middle) {
                    let b = world.get(hit.x, hit.y, hit.z);
                    if let Some(i) = HOTBAR.iter().position(|h| *h == b) {
                        selected = i;
                    }
                    label_timer = 2.0;
                }
            }
            if let Some(p) = changed {
                for (cx, cz) in dirty_chunks(p) {
                    chunks[cz * CHUNKS + cx] = build_chunk(&world, cx, cz, &atlas);
                }
            }
        }

        // --- рендер ---
        clear_background(SKY_HORIZON);
        let cam = Camera3D {
            position: eye,
            target: eye + fwd,
            up: Vec3::Y,
            fovy: 70f32.to_radians(),
            z_near: 0.05,
            z_far: 600.0,
            ..Default::default()
        };
        set_default_camera();
        draw_sky(&cam, sun_dir);
        set_camera(&cam);

        let flat_fwd = vec3(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let visible = |cx: usize, cz: usize| {
            let c = vec3((cx * CHUNK) as f32 + 8.0, eye.y, (cz * CHUNK) as f32 + 8.0);
            let to = c - eye;
            let d = to.length();
            if d > FOG_END + 12.0 {
                return false;
            }
            d < 28.0 || fwd.y.abs() > 0.8 || to.normalize().dot(flat_fwd) > -0.25
        };

        gl_use_material(&opaque_mat);
        set_fog(&opaque_mat, eye);
        for cz in 0..CHUNKS {
            for cx in 0..CHUNKS {
                if visible(cx, cz) {
                    for m in &chunks[cz * CHUNKS + cx].opaque {
                        draw_mesh(m);
                    }
                }
            }
        }

        // обводка блока под прицелом
        gl_use_default_material();
        if let Some((hit, _)) = target {
            let c = hit.as_vec3() + Vec3::splat(0.5);
            draw_cube_wires(c, Vec3::splat(1.004), Color::new(0.05, 0.05, 0.05, 0.9));
        }

        gl_use_material(&blend_mat);
        set_fog(&blend_mat, eye);
        for cz in 0..CHUNKS {
            for cx in 0..CHUNKS {
                if visible(cx, cz) {
                    for m in &chunks[cz * CHUNKS + cx].water {
                        draw_mesh(m);
                    }
                }
            }
        }

        // облака плывут и повторяются
        let shift = (time * 1.4) % CLOUD_PERIOD;
        for k in [-1.0f32, 0.0, 1.0] {
            let gl = unsafe { get_internal_gl() }.quad_gl;
            gl.push_model_matrix(Mat4::from_translation(vec3(shift + k * CLOUD_PERIOD, 0.0, 0.0)));
            for m in &clouds {
                draw_mesh(m);
            }
            let gl = unsafe { get_internal_gl() }.quad_gl;
            gl.pop_model_matrix();
        }
        gl_use_default_material();

        // затемнение под водой
        set_default_camera();
        let eye_block = world.get(eye.x.floor() as i32, eye.y.floor() as i32, eye.z.floor() as i32);
        if eye_block == Block::Water {
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.1, 0.3, 0.6, 0.45));
        }

        draw_hud(&font, &atlas, &player, selected, fps_smooth.round() as i32, label_timer, show_help, grabbed || shot_path.is_some());

        if let Some(path) = &shot_path {
            if frame == 4 {
                get_screen_data().export_png(path);
                std::process::exit(0);
            }
        }

        next_frame().await;
    }
}
