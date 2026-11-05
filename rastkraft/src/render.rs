//! Отрисовка мира напрямую через miniquad: у каждого чанка свои буферы
//! в видеопамяти, поэтому дальность прорисовки может быть большой.

use crate::blocks;
use crate::mesher::{MeshData, Nbhd, V};
use crate::workers::{Done, Job, Pool};
use crate::world::{CH, CS, Dim, World};
use macroquad::miniquad::*;
use macroquad::prelude::{Mat4, Vec3, Vec4, vec3};
use std::collections::{HashMap, HashSet};

const VERTEX: &str = r#"#version 100
attribute vec4 in_pos;
attribute vec4 in_uvt;
attribute vec4 in_light;

uniform mat4 mvp;
uniform vec3 offset;
uniform vec4 fog_color;
uniform vec2 fog_range;
uniform float daylight;
uniform float bright;
uniform vec3 cam;
uniform float time;
uniform float rows;

varying highp vec2 uv;
varying highp vec2 tile0;
varying lowp vec4 col;
varying mediump float fogf;

void main() {
    vec3 lp = in_pos.xyz / 64.0;
    vec3 p = lp + offset;
    float tid = in_uvt.z + in_uvt.w * 256.0;
    tile0 = vec2(mod(tid, 64.0) / 64.0, floor(tid / 64.0) / rows);
    vec3 w = p + cam;
    float a = in_light.a;
    // колыхание травы (250) и листвы (251)
    if (a > 249.5 && a < 251.5) {
        float k = a > 250.5 ? 0.025 : 0.07;
        p.x += sin(time * 1.7 + w.x * 0.7 + w.z * 0.4) * k;
        p.z += cos(time * 1.3 + w.x * 0.4 + w.z * 0.6) * k;
    }
    // волны на поверхности воды (183)
    if (a > 182.5 && a < 183.5) {
        p.y += sin(time * 1.6 + w.x * 0.55 + w.z * 0.35) * 0.045 + sin(time * 2.3 + w.z * 0.8) * 0.025 - 0.05;
    }
    gl_Position = mvp * vec4(p, 1.0);
    uv = in_uvt.xy / 16.0;
    vec4 l = in_light / 255.0;
    float sky = pow(0.8, (1.0 - l.r) * 15.0) * daylight;
    float blk = pow(0.8, (1.0 - l.g) * 15.0) * step(0.01, l.g);
    float lt = max(max(sky, blk), 0.035);
    lt = mix(lt, sqrt(lt), bright);
    vec3 tint = mix(vec3(1.0), vec3(1.0, 0.86, 0.64), clamp((blk - sky) * 1.6, 0.0, 1.0));
    float alpha = a > 240.0 ? 1.0 : l.a;
    col = vec4(tint * lt * l.b, alpha);
    float d = length(p);
    fogf = clamp((d - fog_range.x) / (fog_range.y - fog_range.x), 0.0, 1.0);
}"#;

/// Сплошные блоки: без discard — видеокарта может отбрасывать скрытое заранее.
const FRAGMENT_OPAQUE: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;
varying highp vec2 tile0;
varying lowp vec4 col;
varying mediump float fogf;

uniform sampler2D tex;
uniform vec4 fog_color;
uniform float rows;

void main() {
    vec2 t = tile0 + clamp(fract(uv), 0.002, 0.998) * vec2(1.0 / 64.0, 1.0 / rows);
    vec4 c = texture2D(tex, t) * col;
    gl_FragColor = vec4(mix(c.rgb, fog_color.rgb, fogf * fogf), 1.0);
}"#;

/// Листва, стекло, растения: прозрачные пиксели отбрасываются.
const FRAGMENT_CUTOUT: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;
varying highp vec2 tile0;
varying lowp vec4 col;
varying mediump float fogf;

uniform sampler2D tex;
uniform vec4 fog_color;
uniform float rows;

void main() {
    vec4 t = texture2D(tex, tile0 + clamp(fract(uv), 0.002, 0.998) * vec2(1.0 / 64.0, 1.0 / rows));
    if (t.a < 0.5) discard;
    vec4 c = t * col;
    gl_FragColor = vec4(mix(c.rgb, fog_color.rgb, fogf * fogf), c.a);
}"#;

#[repr(C)]
pub struct Uniforms {
    pub mvp: [f32; 16],
    pub offset: [f32; 3],
    pub fog_color: [f32; 4],
    pub fog_range: [f32; 2],
    pub daylight: f32,
    pub bright: f32,
    pub cam: [f32; 3],
    pub time: f32,
    pub rows: f32,
}

fn meta() -> ShaderMeta {
    ShaderMeta {
        images: vec!["tex".to_string()],
        uniforms: UniformBlockLayout {
            uniforms: vec![
                UniformDesc::new("mvp", UniformType::Mat4),
                UniformDesc::new("offset", UniformType::Float3),
                UniformDesc::new("fog_color", UniformType::Float4),
                UniformDesc::new("fog_range", UniformType::Float2),
                UniformDesc::new("daylight", UniformType::Float1),
                UniformDesc::new("bright", UniformType::Float1),
                UniformDesc::new("cam", UniformType::Float3),
                UniformDesc::new("time", UniformType::Float1),
                UniformDesc::new("rows", UniformType::Float1),
            ],
        },
    }
}

pub struct GpuMesh {
    vb: BufferId,
    ib: BufferId,
    n: i32,
}

impl GpuMesh {
    fn new(ctx: &mut dyn RenderingBackend, v: &[V], i: &[u32]) -> Option<GpuMesh> {
        if i.is_empty() {
            return None;
        }
        let vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(v));
        let ib = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(i));
        Some(GpuMesh { vb, ib, n: i.len() as i32 })
    }
    fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_buffer(self.vb);
        ctx.delete_buffer(self.ib);
    }
}

pub struct ChunkGpu {
    opaque: Option<GpuMesh>,
    cutout: Option<GpuMesh>,
    detail: Option<GpuMesh>,
    dark: Option<GpuMesh>,
    water: Option<GpuMesh>,
    lod: Option<GpuMesh>,
    lod_cut: Option<GpuMesh>,
    lod_water: Option<GpuMesh>,
    version: u32,
    epoch: u32,
}

pub struct Renderer {
    pip_opaque: Pipeline,
    pip_cutout: Pipeline,
    pip_blend: Pipeline,
    atlas: TextureId,
    clouds: Option<GpuMesh>,
    clouds_dim: Option<Dim>,
}

pub struct Frame {
    pub cam: Vec3,
    pub view_proj: Mat4,
    pub fog_color: [f32; 4],
    pub fog_range: [f32; 2],
    pub daylight: f32,
    pub bright: f32,
    pub time: f32,
    pub underground: bool,
}

impl Renderer {
    pub fn new(ctx: &mut dyn RenderingBackend, atlas: TextureId) -> Self {
        let shader = ctx.new_shader(ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT_CUTOUT }, meta()).expect("шейдер мира");
        let shader_opaque = ctx.new_shader(ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT_OPAQUE }, meta()).expect("шейдер мира");
        let attrs = [
            VertexAttribute::new("in_pos", VertexFormat::Short4),
            VertexAttribute::new("in_uvt", VertexFormat::Byte4),
            VertexAttribute::new("in_light", VertexFormat::Byte4),
        ];
        let base = PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() };
        let pip_opaque = ctx.new_pipeline(&[BufferLayout::default()], &attrs, shader_opaque, base);
        let pip_cutout = ctx.new_pipeline(&[BufferLayout::default()], &attrs, shader, base);
        let pip_blend = ctx.new_pipeline(
            &[BufferLayout::default()],
            &attrs,
            shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..base
            },
        );
        Renderer { pip_opaque, pip_cutout, pip_blend, atlas, clouds: None, clouds_dim: None }
    }

    fn uniforms(&self, f: &Frame, offset: Vec3) -> Uniforms {
        Uniforms {
            mvp: f.view_proj.to_cols_array(),
            offset: offset.to_array(),
            fog_color: f.fog_color,
            fog_range: f.fog_range,
            daylight: f.daylight,
            bright: f.bright,
            cam: f.cam.to_array(),
            time: f.time,
            rows: blocks::atlas_rows() as f32,
        }
    }

    fn draw_mesh(&self, ctx: &mut dyn RenderingBackend, m: &GpuMesh, u: &Uniforms) {
        ctx.apply_bindings(&Bindings { vertex_buffers: vec![m.vb], index_buffer: m.ib, images: vec![self.atlas] });
        ctx.apply_uniforms(UniformsSource::table(u));
        ctx.draw(0, m.n, 1);
    }

    /// Рисует видимые чанки (сначала непрозрачное, затем вода от дальних к ближним) и облака.
    pub fn draw(&mut self, ctx: &mut dyn RenderingBackend, loader: &Loader, f: &Frame, clouds: Option<(Dim, f32)>, pass: Option<RenderPass>) {
        let planes = frustum_planes(&f.view_proj);
        let max_d = f.fog_range[1] + 24.0;
        let mut vis: Vec<(f32, Vec3, &ChunkGpu)> = Vec::new();
        for (&(cx, cz), g) in &loader.gpu {
            let min = vec3((cx * CS) as f32, 0.0, (cz * CS) as f32) - f.cam;
            let max = min + vec3(CS as f32, CH as f32, CS as f32);
            let center = min + vec3(8.0, 0.0, 8.0);
            let d = (center.x * center.x + center.z * center.z).sqrt();
            if d > max_d + 12.0 || !aabb_visible(&planes, min, max) {
                continue;
            }
            vis.push((d, min, g));
        }
        // ближние чанки первыми: видеокарта не тратит время на закрытые пиксели
        vis.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let o = crate::opt::get();
        // дальние чанки — упрощённой копией (если она есть)
        let far = |d: f32, _g: &ChunkGpu| d > o.lod_dist;
        ctx.begin_pass(pass, PassAction::Nothing);
        ctx.apply_pipeline(&self.pip_opaque);
        for (d, off, g) in &vis {
            if far(*d, g) {
                if let Some(m) = &g.lod {
                    self.draw_mesh(ctx, m, &self.uniforms(f, *off));
                }
                continue;
            }
            if let Some(m) = &g.opaque {
                self.draw_mesh(ctx, m, &self.uniforms(f, *off));
            }
            // тёмные подземные грани: если игрок под землёй — все, иначе только рядом
            if f.underground || *d < o.dark_dist {
                if let Some(m) = &g.dark {
                    self.draw_mesh(ctx, m, &self.uniforms(f, *off));
                }
            }
        }
        ctx.apply_pipeline(&self.pip_cutout);
        for (d, off, g) in &vis {
            if far(*d, g) {
                if let Some(m) = &g.lod_cut {
                    self.draw_mesh(ctx, m, &self.uniforms(f, *off));
                }
                continue;
            }
            if let Some(m) = &g.cutout {
                self.draw_mesh(ctx, m, &self.uniforms(f, *off));
            }
            // трава и папоротники — только поблизости
            if *d < o.detail_dist {
                if let Some(m) = &g.detail {
                    self.draw_mesh(ctx, m, &self.uniforms(f, *off));
                }
            }
        }
        ctx.apply_pipeline(&self.pip_blend);
        vis.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        for (d, off, g) in &vis {
            if far(*d, g) {
                if let Some(m) = &g.lod_water {
                    self.draw_mesh(ctx, m, &self.uniforms(f, *off));
                }
                continue;
            }
            if let Some(m) = &g.water {
                self.draw_mesh(ctx, m, &self.uniforms(f, *off));
            }
        }
        if let Some((dim, t)) = clouds {
            if self.clouds_dim != Some(dim) {
                if let Some(c) = self.clouds.take() {
                    c.delete(ctx);
                }
                let (v, i) = build_clouds(dim);
                self.clouds = GpuMesh::new(ctx, &v, &i);
                self.clouds_dim = Some(dim);
            }
            if let Some(c) = &self.clouds {
                let y = if dim == Dim::Overworld { 112.0 } else { 26.0 };
                let period = CLOUD_PERIOD;
                let drift = (t * 1.3) % period;
                let base_x = (f.cam.x / period).floor() * period;
                let base_z = (f.cam.z / period).floor() * period;
                let reach = f.fog_range[1].min(320.0);
                for kx in -1..=1 {
                    for kz in -1..=1 {
                        let o = vec3(base_x + kx as f32 * period + drift, y, base_z + kz as f32 * period) - f.cam;
                        // копия облаков рисуется, только если попадает в поле видимости
                        let nx = (0.0f32).clamp(o.x, o.x + period);
                        let nz = (0.0f32).clamp(o.z, o.z + period);
                        if nx * nx + nz * nz > reach * reach {
                            continue;
                        }
                        let min = vec3(o.x, o.y, o.z);
                        if !aabb_visible(&planes, min, min + vec3(period, 4.0, period)) {
                            continue;
                        }
                        self.draw_mesh(ctx, c, &self.uniforms(f, o));
                    }
                }
            }
        }
        ctx.end_render_pass();
    }
}

const CLOUD_PERIOD: f32 = 384.0;

fn build_clouds(dim: Dim) -> (Vec<V>, Vec<u32>) {
    const CELL: f32 = 12.0;
    let n = (CLOUD_PERIOD / CELL) as i32;
    let seed = if dim == Dim::Overworld { 0xC10D } else { 0xBEEF };
    let filled = |cx: i32, cz: i32| {
        let (wx, wz) = (cx.rem_euclid(n), cz.rem_euclid(n));
        let a = wx as f32 / n as f32 * std::f32::consts::TAU;
        let b = wz as f32 / n as f32 * std::f32::consts::TAU;
        let v = crate::worldgen::noise2(a.cos() * 3.0 + 10.0, b.sin() * 3.0 + a.sin() * 3.0 + 10.0, seed) * 0.6
            + crate::worldgen::noise2(b.cos() * 5.0 + 20.0, a.cos() * 2.0 + b.sin() * 5.0, seed ^ 0x55) * 0.4;
        v > if dim == Dim::Overworld { 0.57 } else { 0.5 }
    };
    let tile = blocks::def(blocks::CLOUD).tex[0];
    let mut vs: Vec<V> = Vec::new();
    let mut is: Vec<u32> = Vec::new();
    let mut quad = |p: [[f32; 3]; 4], k: f32| {
        let b = vs.len() as u32;
        for c in p {
            vs.push(V::new(c, [0.0, 0.0], tile, [255, 0, (k * 255.0) as u8, 210]));
        }
        is.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    };
    let (h0, h1) = (0.0, 4.0);
    for cz in 0..n {
        for cx in 0..n {
            if !filled(cx, cz) {
                continue;
            }
            let (x0, z0) = (cx as f32 * CELL, cz as f32 * CELL);
            let (x1, z1) = (x0 + CELL, z0 + CELL);
            quad([[x0, h1, z1], [x1, h1, z1], [x1, h1, z0], [x0, h1, z0]], 1.0);
            quad([[x0, h0, z0], [x1, h0, z0], [x1, h0, z1], [x0, h0, z1]], 0.78);
            if !filled(cx + 1, cz) {
                quad([[x1, h0, z1], [x1, h0, z0], [x1, h1, z0], [x1, h1, z1]], 0.9);
            }
            if !filled(cx - 1, cz) {
                quad([[x0, h0, z0], [x0, h0, z1], [x0, h1, z1], [x0, h1, z0]], 0.9);
            }
            if !filled(cx, cz + 1) {
                quad([[x0, h0, z1], [x1, h0, z1], [x1, h1, z1], [x0, h1, z1]], 0.85);
            }
            if !filled(cx, cz - 1) {
                quad([[x1, h0, z0], [x0, h0, z0], [x0, h1, z0], [x1, h1, z0]], 0.85);
            }
        }
    }
    (vs, is)
}

fn frustum_planes(m: &Mat4) -> [Vec4; 6] {
    let r0 = m.row(0);
    let r1 = m.row(1);
    let r2 = m.row(2);
    let r3 = m.row(3);
    [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r3 + r2, r3 - r2]
}

fn aabb_visible(planes: &[Vec4; 6], min: Vec3, max: Vec3) -> bool {
    for p in planes {
        let v = vec3(if p.x >= 0.0 { max.x } else { min.x }, if p.y >= 0.0 { max.y } else { min.y }, if p.z >= 0.0 { max.z } else { min.z });
        if p.x * v.x + p.y * v.y + p.z * v.z + p.w < 0.0 {
            return false;
        }
    }
    true
}

// ---------- подгрузка чанков ----------
pub struct Loader {
    pub gpu: HashMap<(i32, i32), ChunkGpu>,
    pending_gen: HashSet<(i32, i32)>,
    pending_mesh: HashMap<(i32, i32), (u32, u32)>,
    order: Vec<(i32, i32)>,
    order_r: i32,
    pub epoch: u32,
    /// Жители построек из свежесозданных чанков.
    pub new_spawns: Vec<(u16, [i32; 3])>,
    tick: u32,
}

impl Loader {
    pub fn new() -> Self {
        Loader { gpu: HashMap::new(), pending_gen: HashSet::new(), pending_mesh: HashMap::new(), order: Vec::new(), order_r: -1, epoch: 0, new_spawns: Vec::new(), tick: 0 }
    }

    /// Выгрузить всё из видеопамяти (смена мира или измерения).
    pub fn clear(&mut self, ctx: &mut dyn RenderingBackend) {
        for (_, g) in self.gpu.drain() {
            if let Some(m) = g.opaque {
                m.delete(ctx);
            }
            if let Some(m) = g.cutout {
                m.delete(ctx);
            }
            if let Some(m) = g.detail {
                m.delete(ctx);
            }
            if let Some(m) = g.dark {
                m.delete(ctx);
            }
            if let Some(m) = g.water {
                m.delete(ctx);
            }
            for m in [g.lod, g.lod_cut, g.lod_water].into_iter().flatten() {
                m.delete(ctx);
            }
        }
        self.pending_gen.clear();
        self.pending_mesh.clear();
    }

    pub fn stats(&self) -> [usize; 5] {
        let mut t = [0usize; 5];
        for g in self.gpu.values() {
            for (k, m) in [&g.opaque, &g.cutout, &g.water, &g.detail, &g.dark].into_iter().enumerate() {
                if let Some(m) = m {
                    t[k] += m.n as usize / 3;
                }
            }
        }
        t
    }

    pub fn has_mesh(&self, cx: i32, cz: i32) -> bool {
        self.gpu.contains_key(&(cx, cz))
    }

    fn ensure_order(&mut self, r: i32) {
        if self.order_r == r {
            return;
        }
        self.order.clear();
        for dz in -r..=r {
            for dx in -r..=r {
                if dx * dx + dz * dz <= r * r + r {
                    self.order.push((dx, dz));
                }
            }
        }
        self.order.sort_by_key(|(x, z)| x * x + z * z);
        self.order_r = r;
    }

    fn neighborhood(world: &World, cx: i32, cz: i32) -> Option<Box<Nbhd>> {
        let mut arr = Vec::with_capacity(9);
        for dz in -1..=1 {
            for dx in -1..=1 {
                arr.push(world.chunks.get(&(cx + dx, cz + dz))?.clone());
            }
        }
        let c: [_; 9] = arr.try_into().ok()?;
        Some(Box::new(Nbhd { c }))
    }

    /// Перестроить чанк в фоне, но вне очереди (после правки блока).
    pub fn remesh_soon(&mut self, pool: &mut Pool, world: &World, cx: i32, cz: i32, smooth: bool) {
        let Some(n) = Self::neighborhood(world, cx, cz) else { return };
        let version = *world.versions.get(&(cx, cz)).unwrap_or(&0);
        self.pending_mesh.insert((cx, cz), (version, self.epoch));
        pool.submit_front(Job::Mesh { dim: world.dim, cx, cz, version, epoch: self.epoch, smooth, n });
    }

    /// Перестроить чанк немедленно (после правки игроком) — без очереди.
    pub fn remesh_now(&mut self, ctx: &mut dyn RenderingBackend, world: &World, cx: i32, cz: i32, smooth: bool) {
        let Some(n) = Self::neighborhood(world, cx, cz) else { return };
        let mesh = crate::mesher::build(&n, smooth);
        let version = *world.versions.get(&(cx, cz)).unwrap_or(&0);
        self.upload(ctx, cx, cz, version, mesh);
    }

    fn upload(&mut self, ctx: &mut dyn RenderingBackend, cx: i32, cz: i32, version: u32, mesh: MeshData) {
        let g = ChunkGpu {
            opaque: GpuMesh::new(ctx, &mesh.opaque_v, &mesh.opaque_i),
            cutout: GpuMesh::new(ctx, &mesh.cutout_v, &mesh.cutout_i),
            detail: GpuMesh::new(ctx, &mesh.detail_v, &mesh.detail_i),
            dark: GpuMesh::new(ctx, &mesh.dark_v, &mesh.dark_i),
            water: GpuMesh::new(ctx, &mesh.water_v, &mesh.water_i),
            lod: GpuMesh::new(ctx, &mesh.lod_v, &mesh.lod_i),
            lod_cut: GpuMesh::new(ctx, &mesh.lod_cut_v, &mesh.lod_cut_i),
            lod_water: GpuMesh::new(ctx, &mesh.lod_water_v, &mesh.lod_water_i),
            version,
            epoch: self.epoch,
        };
        if let Some(old) = self.gpu.insert((cx, cz), g) {
            if let Some(m) = old.opaque {
                m.delete(ctx);
            }
            if let Some(m) = old.cutout {
                m.delete(ctx);
            }
            if let Some(m) = old.detail {
                m.delete(ctx);
            }
            if let Some(m) = old.dark {
                m.delete(ctx);
            }
            if let Some(m) = old.water {
                m.delete(ctx);
            }
            for m in [old.lod, old.lod_cut, old.lod_water].into_iter().flatten() {
                m.delete(ctx);
            }
        }
    }

    pub fn update(&mut self, ctx: &mut dyn RenderingBackend, pool: &mut Pool, world: &mut World, center: (i32, i32), r: i32, smooth: bool) {
        // результаты фоновых потоков
        for d in pool.poll() {
            match d {
                Done::Generated { dim, seed, cx, cz, chunk } => {
                    if dim == world.dim && seed == world.seed {
                        self.pending_gen.remove(&(cx, cz));
                        if !world.chunks.contains_key(&(cx, cz)) {
                            if let Some(st) = world.stored.remove(&(cx, cz)) {
                                world.chunks.insert((cx, cz), std::sync::Arc::new(st));
                                world.versions.entry((cx, cz)).or_insert(1);
                            } else {
                                self.new_spawns.extend(chunk.spawns.iter().copied());
                                world.chunks.insert((cx, cz), std::sync::Arc::new(chunk));
                            }
                        }
                    }
                }
                Done::Meshed { dim, cx, cz, version, epoch, mesh } => {
                    let Some(&(pv, pe)) = self.pending_mesh.get(&(cx, cz)) else { continue };
                    if dim != world.dim || pv != version || pe != epoch {
                        continue;
                    }
                    self.pending_mesh.remove(&(cx, cz));
                    let (dx, dz) = (cx - center.0, cz - center.1);
                    if dx * dx + dz * dz > (r + 2) * (r + 2) {
                        continue;
                    }
                    let cur = *world.versions.get(&(cx, cz)).unwrap_or(&0);
                    if cur == version && pe == self.epoch {
                        self.upload(ctx, cx, cz, version, mesh);
                    }
                }
            }
        }

        self.ensure_order(r + 1);
        let order = std::mem::take(&mut self.order);
        for &(dx, dz) in &order {
            if pool.capacity() == 0 {
                break;
            }
            let (cx, cz) = (center.0 + dx, center.1 + dz);
            let inner = dx * dx + dz * dz <= r * r + r;
            if !world.chunks.contains_key(&(cx, cz)) {
                if let Some(st) = world.stored.remove(&(cx, cz)) {
                    world.chunks.insert((cx, cz), std::sync::Arc::new(st));
                    world.versions.entry((cx, cz)).or_insert(1);
                    continue;
                }
                if self.pending_gen.insert((cx, cz)) {
                    pool.submit(Job::Generate { dim: world.dim, seed: world.seed, cx, cz });
                }
                continue;
            }
            if !inner {
                continue;
            }
            let version = *world.versions.get(&(cx, cz)).unwrap_or(&0);
            let fresh = self.gpu.get(&(cx, cz)).is_some_and(|g| g.version == version && g.epoch == self.epoch);
            let queued = self.pending_mesh.get(&(cx, cz)).is_some_and(|&(v, e)| v == version && e == self.epoch);
            if fresh || queued {
                continue;
            }
            if let Some(n) = Self::neighborhood(world, cx, cz) {
                self.pending_mesh.insert((cx, cz), (version, self.epoch));
                pool.submit(Job::Mesh { dim: world.dim, cx, cz, version, epoch: self.epoch, smooth, n });
            }
        }
        self.order = order;

        // выгрузка — не каждый кадр
        self.tick = self.tick.wrapping_add(1);
        if self.tick % 30 != 0 {
            return;
        }
        // выгрузка дальних мешей
        let far: Vec<(i32, i32)> = self
            .gpu
            .keys()
            .copied()
            .filter(|(cx, cz)| {
                let (dx, dz) = (cx - center.0, cz - center.1);
                dx * dx + dz * dz > (r + 3) * (r + 3)
            })
            .collect();
        for k in far {
            if let Some(g) = self.gpu.remove(&k) {
                if let Some(m) = g.opaque {
                    m.delete(ctx);
                }
                if let Some(m) = g.cutout {
                    m.delete(ctx);
                }
                if let Some(m) = g.detail {
                    m.delete(ctx);
                }
                if let Some(m) = g.dark {
                    m.delete(ctx);
                }
                if let Some(m) = g.water {
                    m.delete(ctx);
                }
            }
        }
        // выгрузка дальних нетронутых чанков из памяти
        let limit = (r + 5) * (r + 5);
        world.chunks.retain(|&(cx, cz), _| {
            let (dx, dz) = (cx - center.0, cz - center.1);
            dx * dx + dz * dz <= limit || world.versions.contains_key(&(cx, cz))
        });
        self.pending_gen.retain(|&(cx, cz)| {
            let (dx, dz) = (cx - center.0, cz - center.1);
            dx * dx + dz * dz <= limit
        });
    }
}
