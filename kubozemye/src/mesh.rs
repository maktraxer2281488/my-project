//! Построение мешей чанков: отсечение скрытых граней, затенение граней
//! и мягкое «фоновое затенение» (ambient occlusion) в углах.

use crate::textures::tile_uv;
use crate::world::{Block, CHUNK, CHUNKS, World, WY, value_noise};
use macroquad::prelude::*;

/// Вершин в одном меше (≤ буфера отрисовки macroquad, см. window_conf).
pub const MAX_VERTS: usize = 20_000;

pub struct ChunkMesh {
    pub opaque: Vec<Mesh>,
    pub water: Vec<Mesh>,
}

struct Builder {
    meshes: Vec<Mesh>,
    verts: Vec<Vertex>,
    idx: Vec<u16>,
    tex: Option<Texture2D>,
}

impl Builder {
    fn new(tex: Option<Texture2D>) -> Self {
        Builder { meshes: Vec::new(), verts: Vec::new(), idx: Vec::new(), tex }
    }

    fn flush(&mut self) {
        if !self.verts.is_empty() {
            self.meshes.push(Mesh {
                vertices: std::mem::take(&mut self.verts),
                indices: std::mem::take(&mut self.idx),
                texture: self.tex.clone(),
            });
        }
    }

    fn quad(&mut self, pos: [Vec3; 4], uv: [Vec2; 4], col: [Color; 4], flip: bool) {
        if self.verts.len() + 4 > MAX_VERTS {
            self.flush();
        }
        let b = self.verts.len() as u16;
        for i in 0..4 {
            self.verts.push(Vertex::new2(pos[i], uv[i], col[i]));
        }
        if flip {
            self.idx.extend_from_slice(&[b + 1, b + 2, b + 3, b + 1, b + 3, b]);
        } else {
            self.idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        }
    }

    fn finish(mut self) -> Vec<Mesh> {
        self.flush();
        self.meshes
    }
}

/// (нормаль, углы BL-BR-TR-TL снаружи, яркость грани)
const FACES: [([i32; 3], [[f32; 3]; 4], f32); 6] = [
    ([1, 0, 0], [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]], 0.80),
    ([-1, 0, 0], [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]], 0.80),
    ([0, 1, 0], [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]], 1.00),
    ([0, -1, 0], [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]], 0.55),
    ([0, 0, 1], [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]], 0.68),
    ([0, 0, -1], [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]], 0.68),
];

const AO_LEVELS: [f32; 4] = [0.42, 0.62, 0.81, 1.0];

fn face_visible(b: Block, nb: Block) -> bool {
    match b {
        Block::Water => nb == Block::Air,
        Block::Glass => !nb.is_opaque() && nb != Block::Glass,
        _ => !nb.is_opaque(),
    }
}

fn occludes(b: Block) -> bool {
    b.is_opaque() || b == Block::Leaves
}

pub fn build_chunk(world: &World, cx: usize, cz: usize, atlas: &Texture2D) -> ChunkMesh {
    let mut opaque = Builder::new(Some(atlas.clone()));
    let mut water = Builder::new(Some(atlas.clone()));
    let (x0, z0) = ((cx * CHUNK) as i32, (cz * CHUNK) as i32);

    for y in 0..WY as i32 {
        for z in z0..z0 + CHUNK as i32 {
            for x in x0..x0 + CHUNK as i32 {
                let b = world.get(x, y, z);
                if b == Block::Air {
                    continue;
                }
                let (t_top, t_side, t_bot) = b.tiles();
                let water_top = b == Block::Water && world.get(x, y + 1, z) != Block::Water;

                for (fi, (n, corners, light)) in FACES.iter().enumerate() {
                    let nb = world.get(x + n[0], y + n[1], z + n[2]);
                    if !face_visible(b, nb) {
                        continue;
                    }
                    let tile = match fi {
                        2 => t_top,
                        3 => t_bot,
                        _ => t_side,
                    };
                    let (u0, v0, u1, v1) = tile_uv(tile);
                    let uv = [vec2(u0, v1), vec2(u1, v1), vec2(u1, v0), vec2(u0, v0)];

                    // оси, касательные к грани
                    let na = n.iter().position(|&v| v != 0).unwrap();
                    let (ta, tb) = match na {
                        0 => (1, 2),
                        1 => (0, 2),
                        _ => (0, 1),
                    };
                    let p = [x + n[0], y + n[1], z + n[2]];

                    let mut pos = [Vec3::ZERO; 4];
                    let mut col = [WHITE; 4];
                    let mut ao = [3usize; 4];
                    for (i, c) in corners.iter().enumerate() {
                        let mut cy = c[1];
                        if water_top && cy > 0.5 {
                            cy = 0.88;
                        }
                        pos[i] = vec3(x as f32 + c[0], y as f32 + cy, z as f32 + c[2]);

                        let sa = if c[ta] > 0.5 { 1 } else { -1 };
                        let sb = if c[tb] > 0.5 { 1 } else { -1 };
                        let mut q1 = p;
                        q1[ta] += sa;
                        let mut q2 = p;
                        q2[tb] += sb;
                        let mut q3 = q1;
                        q3[tb] += sb;
                        let s1 = occludes(world.get(q1[0], q1[1], q1[2]));
                        let s2 = occludes(world.get(q2[0], q2[1], q2[2]));
                        let s3 = occludes(world.get(q3[0], q3[1], q3[2]));
                        ao[i] = if s1 && s2 { 0 } else { 3 - (s1 as usize + s2 as usize + s3 as usize) };

                        let k = light * AO_LEVELS[ao[i]];
                        let alpha = if b == Block::Water { 0.72 } else { 1.0 };
                        col[i] = Color::new(k, k, k, alpha);
                    }
                    let flip = ao[0] + ao[2] < ao[1] + ao[3];
                    if b == Block::Water {
                        water.quad(pos, uv, col, flip);
                    } else {
                        opaque.quad(pos, uv, col, flip);
                    }
                }
            }
        }
    }
    ChunkMesh { opaque: opaque.finish(), water: water.finish() }
}

pub const CLOUD_PERIOD: f32 = 512.0;
pub const CLOUD_Y: f32 = 84.0;

/// Слой облаков: плоские «подушки» по шуму, повторяется с периодом CLOUD_PERIOD.
pub fn build_clouds(seed: u32) -> Vec<Mesh> {
    const CELL: f32 = 8.0;
    const N: i32 = (CLOUD_PERIOD / CELL) as i32;
    let filled = |cx: i32, cz: i32| {
        let (wx, wz) = (cx.rem_euclid(N), cz.rem_euclid(N));
        // периодичный шум: сэмплируем на торе
        let a = wx as f32 / N as f32 * std::f32::consts::TAU;
        let b = wz as f32 / N as f32 * std::f32::consts::TAU;
        let v = value_noise(a.cos() * 3.0 + 10.0, b.sin() * 3.0 + a.sin() * 3.0 + 10.0, seed ^ 0xC10D)
            * 0.6
            + value_noise(b.cos() * 5.0 + 20.0, a.cos() * 2.0 + b.sin() * 5.0, seed ^ 0xBEEF) * 0.4;
        v > 0.56
    };
    let mut builder = Builder::new(None);
    let off = -(CHUNK as f32 * CHUNKS as f32) / 2.0;
    let (h0, h1) = (CLOUD_Y, CLOUD_Y + 4.0);
    let uv = [vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)];
    let c = |k: f32| Color::new(k, k, k, 0.82);
    for cz in 0..N {
        for cx in 0..N {
            if !filled(cx, cz) {
                continue;
            }
            let (x0, z0) = (off + cx as f32 * CELL, off + cz as f32 * CELL);
            let (x1, z1) = (x0 + CELL, z0 + CELL);
            builder.quad(
                [vec3(x0, h1, z1), vec3(x1, h1, z1), vec3(x1, h1, z0), vec3(x0, h1, z0)],
                uv,
                [c(1.0); 4],
                false,
            );
            builder.quad(
                [vec3(x0, h0, z0), vec3(x1, h0, z0), vec3(x1, h0, z1), vec3(x0, h0, z1)],
                uv,
                [c(0.8); 4],
                false,
            );
            if !filled(cx + 1, cz) {
                builder.quad([vec3(x1, h0, z1), vec3(x1, h0, z0), vec3(x1, h1, z0), vec3(x1, h1, z1)], uv, [c(0.9); 4], false);
            }
            if !filled(cx - 1, cz) {
                builder.quad([vec3(x0, h0, z0), vec3(x0, h0, z1), vec3(x0, h1, z1), vec3(x0, h1, z0)], uv, [c(0.9); 4], false);
            }
            if !filled(cx, cz + 1) {
                builder.quad([vec3(x0, h0, z1), vec3(x1, h0, z1), vec3(x1, h1, z1), vec3(x0, h1, z1)], uv, [c(0.86); 4], false);
            }
            if !filled(cx, cz - 1) {
                builder.quad([vec3(x1, h0, z0), vec3(x0, h0, z0), vec3(x0, h1, z0), vec3(x1, h1, z0)], uv, [c(0.86); 4], false);
            }
        }
    }
    builder.finish()
}
