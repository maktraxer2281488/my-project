//! Своя музыка и звуки: всё синтезируется кодом (аддитивный и FM-синтез),
//! внешних аудиофайлов нет. Каждая мелодия сочиняется по своим правилам
//! (лад, темп, гармония, инструменты), но с фиксированным зерном — звучит всегда одинаково.

use std::f32::consts::TAU;

pub const RATE: u32 = 44100;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Track {
    Menu,
    Meadow,
    Desert,
    Taiga,
    Night,
    Caves,
    Heaven,
    Boss,
    Ocean,
    Dawn,
    Dusk,
    Village,
    Aurora,
    Clouds,
    Fury,
    Workshop,
}

pub const TRACKS: [Track; 16] = [
    Track::Menu, Track::Meadow, Track::Desert, Track::Taiga, Track::Night, Track::Caves, Track::Heaven, Track::Boss,
    Track::Ocean, Track::Dawn, Track::Dusk, Track::Village, Track::Aurora, Track::Clouds, Track::Fury, Track::Workshop,
];

impl Track {
    pub fn title(self) -> &'static str {
        match self {
            Track::Menu => "Расткрафт — rtxx131",
            Track::Meadow => "Мир Расткрафта — rtxx131",
            Track::Desert => "Песчаный караван",
            Track::Taiga => "Снег над тайгой",
            Track::Night => "Ночная тишина",
            Track::Caves => "Глубины — rtxx131",
            Track::Heaven => "Небесные сады — rtxx131",
            Track::Boss => "Битва — rtxx131",
            Track::Ocean => "Морской бриз",
            Track::Dawn => "Рассвет",
            Track::Dusk => "Закат над холмами",
            Track::Village => "Просторы — rtxx131",
            Track::Aurora => "Северное сияние",
            Track::Clouds => "Над облаками",
            Track::Fury => "Отвага — rtxx131",
            Track::Workshop => "Мастерская",
        }
    }
    pub fn file(self) -> &'static str {
        match self {
            Track::Menu => "01_menu",
            Track::Meadow => "02_meadow",
            Track::Desert => "03_desert",
            Track::Taiga => "04_taiga",
            Track::Night => "05_night",
            Track::Caves => "06_caves",
            Track::Heaven => "07_heaven",
            Track::Boss => "08_boss",
            Track::Ocean => "09_ocean",
            Track::Dawn => "10_dawn",
            Track::Dusk => "11_dusk",
            Track::Village => "12_village",
            Track::Aurora => "13_aurora",
            Track::Clouds => "14_clouds",
            Track::Fury => "15_fury",
            Track::Workshop => "16_workshop",
        }
    }
}

// ---------- генератор случайных чисел ----------
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 & 0xffffff) as f32 / 16_777_215.0
    }
    fn pick(&mut self, n: usize) -> usize {
        (self.next() * n as f32) as usize % n.max(1)
    }
}

fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

#[derive(Clone, Copy)]
enum Inst {
    Pad,
    Pluck,
    Bell,
    Flute,
    Bass,
    Drone,
    Kick,
    Snare,
    Hat,
    Hand,
    Drip,
    Lead,
}

struct Note {
    t: f32,
    dur: f32,
    f: f32,
    inst: Inst,
    vel: f32,
    pan: f32,
}

/// Описание пьесы.
struct Song {
    bpm: f32,
    root: i32,
    scale: &'static [i32],
    prog: &'static [i32],
    seed: u32,
    pad: bool,
    arp: Option<Inst>,
    melody: Option<Inst>,
    bass: bool,
    drums: u8,
    drone: bool,
    drips: bool,
    density: f32,
}

const MAJOR: &[i32] = &[0, 2, 4, 5, 7, 9, 11];
const MINOR: &[i32] = &[0, 2, 3, 5, 7, 8, 10];
const HIJAZ: &[i32] = &[0, 1, 4, 5, 7, 8, 10];
const LYDIAN: &[i32] = &[0, 2, 4, 6, 7, 9, 11];
const DORIAN: &[i32] = &[0, 2, 3, 5, 7, 9, 10];
const MIXO: &[i32] = &[0, 2, 4, 5, 7, 9, 10];

fn song(t: Track) -> Song {
    match t {
        Track::Menu => Song { bpm: 80.0, root: 55, scale: MAJOR, prog: &[0, 4, 5, 3, 0, 4, 3, 4], seed: 11, pad: true, arp: Some(Inst::Pluck), melody: Some(Inst::Bell), bass: true, drums: 0, drone: false, drips: false, density: 0.6 },
        Track::Meadow => Song { bpm: 92.0, root: 60, scale: MAJOR, prog: &[0, 5, 3, 4, 0, 5, 1, 4], seed: 23, pad: true, arp: Some(Inst::Pluck), melody: Some(Inst::Flute), bass: true, drums: 1, drone: false, drips: false, density: 0.7 },
        Track::Desert => Song { bpm: 92.0, root: 62, scale: MIXO, prog: &[0, 3, 4, 0, 0, 5, 3, 4], seed: 37, pad: true, arp: Some(Inst::Pluck), melody: Some(Inst::Pluck), bass: true, drums: 2, drone: false, drips: false, density: 0.75 },
        Track::Taiga => Song { bpm: 80.0, root: 62, scale: MAJOR, prog: &[0, 5, 3, 4, 0, 5, 1, 4], seed: 41, pad: true, arp: Some(Inst::Bell), melody: Some(Inst::Flute), bass: false, drums: 0, drone: false, drips: false, density: 0.5 },
        Track::Night => Song { bpm: 70.0, root: 60, scale: MAJOR, prog: &[0, 3, 4, 0, 0, 5, 3, 4], seed: 53, pad: true, arp: Some(Inst::Bell), melody: Some(Inst::Bell), bass: false, drums: 0, drone: false, drips: false, density: 0.4 },
        Track::Caves => Song { bpm: 68.0, root: 60, scale: LYDIAN, prog: &[0, 1, 4, 0, 0, 1, 2, 4], seed: 67, pad: true, arp: Some(Inst::Bell), melody: Some(Inst::Bell), bass: false, drums: 0, drone: false, drips: true, density: 0.35 },
        Track::Heaven => Song { bpm: 76.0, root: 65, scale: LYDIAN, prog: &[0, 1, 0, 6, 0, 1, 2, 4], seed: 79, pad: true, arp: Some(Inst::Bell), melody: Some(Inst::Flute), bass: false, drums: 0, drone: false, drips: false, density: 0.55 },
        Track::Boss => Song { bpm: 126.0, root: 57, scale: MIXO, prog: &[0, 6, 3, 4, 0, 6, 4, 4], seed: 97, pad: true, arp: Some(Inst::Pluck), melody: Some(Inst::Flute), bass: true, drums: 2, drone: false, drips: false, density: 0.8 },
        Track::Ocean => Song { bpm: 72.0, root: 62, scale: MIXO, prog: &[0, 6, 3, 0, 0, 6, 4, 3], seed: 103, pad: true, arp: Some(Inst::Bell), melody: Some(Inst::Flute), bass: true, drums: 0, drone: false, drips: false, density: 0.5 },
        Track::Dawn => Song { bpm: 66.0, root: 64, scale: MAJOR, prog: &[0, 3, 5, 4, 0, 3, 1, 4], seed: 109, pad: true, arp: Some(Inst::Bell), melody: Some(Inst::Bell), bass: false, drums: 0, drone: false, drips: false, density: 0.4 },
        Track::Dusk => Song { bpm: 78.0, root: 60, scale: MAJOR, prog: &[0, 3, 0, 4, 0, 5, 3, 4], seed: 113, pad: true, arp: Some(Inst::Pluck), melody: Some(Inst::Flute), bass: true, drums: 0, drone: false, drips: false, density: 0.5 },
        Track::Village => Song { bpm: 112.0, root: 65, scale: MAJOR, prog: &[0, 3, 4, 0, 5, 3, 4, 0], seed: 127, pad: false, arp: Some(Inst::Pluck), melody: Some(Inst::Flute), bass: true, drums: 1, drone: false, drips: false, density: 0.8 },
        Track::Aurora => Song { bpm: 72.0, root: 62, scale: LYDIAN, prog: &[0, 1, 4, 0, 0, 5, 3, 4], seed: 131, pad: true, arp: Some(Inst::Bell), melody: Some(Inst::Bell), bass: false, drums: 0, drone: false, drips: false, density: 0.4 },
        Track::Clouds => Song { bpm: 70.0, root: 62, scale: LYDIAN, prog: &[0, 1, 4, 0, 0, 1, 2, 4], seed: 137, pad: true, arp: Some(Inst::Pluck), melody: Some(Inst::Bell), bass: false, drums: 0, drone: false, drips: false, density: 0.45 },
        Track::Fury => Song { bpm: 124.0, root: 59, scale: MAJOR, prog: &[0, 4, 5, 3, 0, 4, 3, 4], seed: 149, pad: false, arp: Some(Inst::Pluck), melody: Some(Inst::Flute), bass: true, drums: 2, drone: false, drips: false, density: 0.8 },
        Track::Workshop => Song { bpm: 100.0, root: 60, scale: MAJOR, prog: &[0, 4, 5, 3, 1, 4, 0, 4], seed: 151, pad: true, arp: Some(Inst::Pluck), melody: Some(Inst::Bell), bass: true, drums: 2, drone: false, drips: false, density: 0.65 },
    }
}

/// Сочинить ноты пьесы.
fn compose(s: &Song) -> (Vec<Note>, f32) {
    let mut r = Rng(s.seed.wrapping_mul(2_654_435_761) | 1);
    let beat = 60.0 / s.bpm;
    let bar = beat * 4.0;
    let deg = |d: i32, oct: i32| -> f32 {
        let n = s.scale.len() as i32;
        let o = d.div_euclid(n);
        let k = d.rem_euclid(n);
        midi((s.root + s.scale[k as usize] + 12 * (o + oct)) as f32)
    };
    // форма: вступление 2 такта, A (8), B (8), A (8), кода 2 такта
    let sections: [(usize, u8); 5] = [(2, 0), (8, 1), (8, 2), (8, 1), (2, 3)];
    let mut notes = Vec::new();
    // мотив из 2 тактов: ритм (доли) и ступени относительно аккорда
    let motif = |r: &mut Rng| -> Vec<(f32, f32, i32)> {
        let rhythms: [&[f32]; 5] = [&[1.0, 1.0, 2.0], &[0.5, 0.5, 1.0, 2.0], &[1.5, 0.5, 2.0], &[2.0, 1.0, 1.0], &[1.0, 0.5, 0.5, 1.0, 1.0]];
        let mut m = Vec::new();
        let mut t = 0.0;
        let mut step = r.pick(3) as i32 * 2;
        for _ in 0..2 {
            let rh = rhythms[r.pick(rhythms.len())];
            for &d in rh {
                if r.next() < s.density || t == 0.0 {
                    m.push((t, d, step));
                }
                t += d;
                step += [-2, -1, -1, 1, 1, 2, 0][r.pick(7)];
                step = step.clamp(-2, 7);
            }
        }
        m
    };
    let motif_a = motif(&mut r);
    let motif_b = motif(&mut r);
    let mut bar_i = 0usize;
    for (len, kind) in sections {
        for k in 0..len {
            let t0 = bar_i as f32 * bar;
            let chord = s.prog[bar_i % s.prog.len()];
            let fade = match kind {
                0 => 0.5 + 0.5 * k as f32 / len as f32,
                3 => 1.0 - 0.5 * k as f32 / len as f32,
                _ => 1.0,
            };
            // подушка — аккорд на весь такт
            if s.pad {
                for (j, d) in [0, 2, 4].iter().enumerate() {
                    notes.push(Note { t: t0, dur: bar * 1.05, f: deg(chord + d, -1), inst: Inst::Pad, vel: 0.11 * fade, pan: [-0.5, 0.0, 0.5][j] });
                }
            }
            if s.drone && k % 2 == 0 {
                notes.push(Note { t: t0, dur: bar * 2.1, f: deg(0, -2), inst: Inst::Drone, vel: 0.14 * fade, pan: 0.0 });
            }
            if kind == 0 || kind == 3 {
                bar_i += 1;
                continue;
            }
            // бас
            if s.bass {
                let pattern: &[f32] = if s.drums >= 3 { &[0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5] } else { &[0.0, 2.0] };
                for &p in pattern {
                    let oct = if s.drums >= 3 && (p * 2.0) as i32 % 2 == 1 { -1 } else { -2 };
                    notes.push(Note { t: t0 + p * beat, dur: beat * if s.drums >= 3 { 0.45 } else { 1.8 }, f: deg(chord, oct), inst: Inst::Bass, vel: 0.15, pan: 0.0 });
                }
            }
            // арпеджио
            if let Some(inst) = s.arp {
                let steps = if s.drums >= 3 { 8 } else { 8 };
                let pat = [0, 2, 4, 7, 4, 2, 4, 2];
                for i in 0..steps {
                    let v = if i % 2 == 0 { 0.1 } else { 0.07 };
                    let pan = if i % 2 == 0 { -0.35 } else { 0.35 };
                    notes.push(Note { t: t0 + i as f32 * beat * 0.5, dur: beat * 0.9, f: deg(chord + pat[i], if matches!(inst, Inst::Bell) { 1 } else { 0 }), inst, vel: v, pan });
                }
            }
            // мелодия: мотив A или B, каждые 2 такта, с вариацией
            if let Some(inst) = s.melody {
                if k % 2 == 0 {
                    let m = if kind == 1 { &motif_a } else { &motif_b };
                    let shift = if k % 4 == 2 { 1 } else { 0 };
                    for &(bt, d, st) in m {
                        let t = t0 + bt * beat;
                        let c = s.prog[(bar_i + (bt / 4.0) as usize) % s.prog.len()];
                        let f = deg(c + st + shift, 1);
                        notes.push(Note { t, dur: d * beat * 0.95, f, inst, vel: 0.16, pan: 0.1 });
                    }
                }
            }
            // ударные
            match s.drums {
                1 => {
                    for i in 0..8 {
                        notes.push(Note { t: t0 + i as f32 * beat * 0.5, dur: 0.1, f: 0.0, inst: Inst::Hat, vel: if i % 2 == 0 { 0.05 } else { 0.03 }, pan: 0.3 });
                    }
                }
                2 => {
                    for (p, v) in [(0.0, 0.25), (1.5, 0.15), (2.0, 0.2), (3.0, 0.12), (3.5, 0.12)] {
                        notes.push(Note { t: t0 + p * beat, dur: 0.3, f: if p == 0.0 { 110.0 } else { 220.0 }, inst: Inst::Hand, vel: v, pan: -0.2 });
                    }
                }
                3 => {
                    for i in 0..4 {
                        notes.push(Note { t: t0 + i as f32 * beat, dur: 0.4, f: 0.0, inst: Inst::Kick, vel: 0.32, pan: 0.0 });
                        if i % 2 == 1 {
                            notes.push(Note { t: t0 + i as f32 * beat, dur: 0.3, f: 0.0, inst: Inst::Snare, vel: 0.22, pan: 0.0 });
                        }
                    }
                    for i in 0..8 {
                        notes.push(Note { t: t0 + i as f32 * beat * 0.5, dur: 0.1, f: 0.0, inst: Inst::Hat, vel: 0.05, pan: 0.25 });
                    }
                }
                _ => {}
            }
            if s.drips {
                for _ in 0..3 {
                    if r.next() < 0.7 {
                        let t = t0 + r.next() * bar;
                        notes.push(Note { t, dur: 0.4, f: deg(r.pick(7) as i32, 2), inst: Inst::Drip, vel: 0.09, pan: r.next() * 1.6 - 0.8 });
                    }
                }
            }
            bar_i += 1;
        }
    }
    let total = bar_i as f32 * bar + 6.0; // тишина в конце перед повтором
    (notes, total)
}

#[inline]
fn noise(i: u32) -> f32 {
    let mut h = i.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    h = ((h >> ((h >> 28) + 4)) ^ h).wrapping_mul(277_803_737);
    ((h >> 22) ^ h) as f32 / u32::MAX as f32 * 2.0 - 1.0
}

/// Один отсчёт инструмента в момент t от начала ноты.
#[inline]
fn voice(inst: Inst, f: f32, t: f32, dur: f32, n: u32) -> f32 {
    let env_ar = |a: f32, rel: f32| -> f32 {
        let up = (t / a).min(1.0);
        let down = if t > dur { (1.0 - (t - dur) / rel).max(0.0) } else { 1.0 };
        up * down
    };
    match inst {
        Inst::Pad => {
            let e = env_ar(0.9, 1.2);
            let vib = (t * 5.0).sin() * 0.003;
            let p = TAU * f * t * (1.0 + vib);
            e * ((p).sin() + 0.5 * (p * 2.003).sin() + 0.25 * (p * 3.01).sin() + 0.15 * (p * 0.997).sin())
        }
        Inst::Pluck => {
            let p = TAU * f * t;
            let e = (-t * 4.5).exp() * (t / 0.004).min(1.0);
            e * ((p).sin() + 0.5 * (p * 2.0).sin() * (-t * 7.0).exp() + 0.3 * (p * 3.0).sin() * (-t * 11.0).exp())
        }
        Inst::Bell => {
            let e = (-t * 1.6).exp() * (t / 0.003).min(1.0);
            let m = (TAU * f * 3.5 * t).sin() * 2.2 * (-t * 3.0).exp();
            e * ((TAU * f * t + m).sin() + 0.25 * (TAU * f * 2.0 * t).sin() * (-t * 4.0).exp())
        }
        Inst::Flute => {
            let e = env_ar(0.08, 0.25);
            let vib = (t * 5.5).sin() * 0.006 * (t / 0.4).min(1.0);
            let p = TAU * f * t * (1.0 + vib);
            e * ((p).sin() + 0.18 * (p * 2.0).sin() + 0.06 * (p * 3.0).sin() + 0.04 * noise(n))
        }
        Inst::Lead => {
            let e = env_ar(0.01, 0.12);
            let p = TAU * f * t;
            e * (3.0 * (p).sin()).tanh() * 0.7
        }
        Inst::Bass => {
            let e = env_ar(0.01, 0.15);
            let p = TAU * f * t;
            e * ((p).sin() + 0.35 * (2.0 * (p).sin()).tanh())
        }
        Inst::Drone => {
            let e = env_ar(1.5, 1.5);
            let p = TAU * f * t;
            e * ((p).sin() + 0.6 * (p * 1.5).sin() + 0.3 * (p * 2.0).sin()) * (0.8 + 0.2 * (t * 0.7).sin())
        }
        Inst::Kick => {
            let fr = 45.0 + 90.0 * (-t * 25.0).exp();
            (TAU * fr * t).sin() * (-t * 9.0).exp()
        }
        Inst::Snare => (noise(n) * 0.8 + (TAU * 185.0 * t).sin() * 0.4) * (-t * 18.0).exp(),
        Inst::Hat => (noise(n) - noise(n.wrapping_sub(1))) * 0.5 * (-t * 45.0).exp(),
        Inst::Hand => ((TAU * f * t * (1.0 + 0.6 * (-t * 30.0).exp())).sin() + 0.3 * noise(n)) * (-t * 14.0).exp(),
        Inst::Drip => {
            let fr = f * (1.0 + 0.8 * (-t * 40.0).exp());
            (TAU * fr * t).sin() * (-t * 9.0).exp()
        }
    }
}

/// Отрендерить пьесу в стерео f32 (чередование L, R).
pub fn render(track: Track) -> Vec<f32> {
    let s = song(track);
    let (notes, total) = compose(&s);
    let len = (total * RATE as f32) as usize;
    let mut buf = vec![0.0f32; len * 2];
    for nt in &notes {
        let tail = match nt.inst {
            Inst::Pad | Inst::Drone => 1.5,
            Inst::Bell => 2.5,
            Inst::Pluck => 1.2,
            _ => 0.4,
        };
        let start = (nt.t * RATE as f32) as usize;
        let n = ((nt.dur + tail) * RATE as f32) as usize;
        let (gl, gr) = ((1.0 - nt.pan).min(1.0) * nt.vel, (1.0 + nt.pan).min(1.0) * nt.vel);
        for i in 0..n {
            let j = start + i;
            if j >= len {
                break;
            }
            let t = i as f32 / RATE as f32;
            let v = voice(nt.inst, nt.f, t, nt.dur, j as u32 ^ (nt.t.to_bits()));
            buf[j * 2] += v * gl;
            buf[j * 2 + 1] += v * gr;
        }
    }
    // простая реверберация (несколько задержек) и мягкое ограничение
    let delays = [(0.031f32, 0.32f32), (0.047, 0.27), (0.071, 0.22), (0.113, 0.17)];
    let mut wet = vec![0.0f32; buf.len()];
    for (d, g) in delays {
        let off = ((d * RATE as f32) as usize) * 2;
        for i in off..buf.len() {
            wet[i] += (buf[i - off] + wet[i - off] * 0.35) * g * 0.5;
        }
    }
    for (b, w) in buf.iter_mut().zip(wet.iter()) {
        *b = (*b + *w * 0.6).tanh() * 0.8;
    }
    buf
}

/// Короткие звуки игры.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Sfx {
    Hit,
    Break,
    Place,
    Hurt,
    Pickup,
    Eat,
    Swing,
    Boss,
    Click,
    Crit,
    Thunder,
}

pub const SFX: [Sfx; 11] = [Sfx::Hit, Sfx::Break, Sfx::Place, Sfx::Hurt, Sfx::Pickup, Sfx::Eat, Sfx::Swing, Sfx::Boss, Sfx::Click, Sfx::Crit, Sfx::Thunder];

pub fn render_sfx(s: Sfx) -> Vec<f32> {
    let dur = match s {
        Sfx::Boss => 1.6,
        Sfx::Thunder => 3.0,
        Sfx::Eat => 0.35,
        Sfx::Pickup | Sfx::Click => 0.12,
        _ => 0.25,
    };
    let len = (dur * RATE as f32) as usize;
    let mut out = Vec::with_capacity(len * 2);
    let mut lp = 0.0f32;
    for i in 0..len {
        let t = i as f32 / RATE as f32;
        let n = noise(i as u32 ^ (s as u32 * 7919));
        let v = match s {
            Sfx::Hit => ((TAU * (90.0 + 120.0 * (-t * 30.0).exp()) * t).sin() * 0.8 + n * 0.3) * (-t * 22.0).exp(),
            Sfx::Break => {
                lp += (n - lp) * 0.25;
                lp * 1.6 * (-t * 14.0).exp()
            }
            Sfx::Place => ((TAU * 160.0 * t).sin() * 0.6 + n * 0.25) * (-t * 35.0).exp(),
            Sfx::Hurt => ((TAU * (220.0 - 120.0 * t) * t).sin() * 0.7 + n * 0.15) * (-t * 9.0).exp(),
            Sfx::Pickup => (TAU * (880.0 + 1800.0 * t) * t).sin() * 0.5 * (-t * 18.0).exp(),
            Sfx::Eat => {
                let chew = ((t * 18.0).sin() * 0.5 + 0.5).powi(3);
                lp += (n - lp) * 0.4;
                lp * chew * 1.4 * (-t * 3.0).exp()
            }
            Sfx::Swing => {
                lp += (n - lp) * (0.05 + 0.3 * (t / dur));
                lp * 1.8 * (t * 12.0).min(1.0) * (-t * 10.0).exp()
            }
            Sfx::Boss => {
                let f = 55.0 + 10.0 * (t * 6.0).sin();
                (((TAU * f * t).sin() * 3.0).tanh() * 0.6 + n * 0.25) * (t / 0.2).min(1.0) * (1.0 - t / dur).max(0.0)
            }
            Sfx::Click => (TAU * 1200.0 * t).sin() * 0.4 * (-t * 60.0).exp(),
            Sfx::Crit => ((TAU * 1500.0 * t).sin() * 0.4 + (TAU * 2250.0 * t).sin() * 0.3) * (-t * 14.0).exp(),
            Sfx::Thunder => {
                // раскат: низкий отфильтрованный шум с несколькими ударами
                lp += (n - lp) * 0.02;
                let rumble = (1.0 + 0.6 * (t * 7.0).sin() * (t * 2.3).cos()) * (-t * 1.1).exp();
                (lp * 6.0 * rumble).tanh() * (t / 0.03).min(1.0)
            }
        };
        out.push(v);
        out.push(v);
    }
    out
}

/// Упаковать стерео f32 в WAV (16 бит, 44.1 кГц).
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&RATE.to_le_bytes());
    v.extend_from_slice(&(RATE * 4).to_le_bytes());
    v.extend_from_slice(&4u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_render_and_are_not_silent() {
        for t in [Track::Night, Track::Boss] {
            let b = render(t);
            assert!(b.len() > (RATE as usize) * 2 * 40, "{t:?} слишком короткая");
            let peak = b.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.05 && peak <= 1.0, "{t:?} пик {peak}");
            assert!(b.iter().all(|v| v.is_finite()));
        }
        for s in SFX {
            assert!(render_sfx(s).iter().any(|v| v.abs() > 0.01));
        }
    }
}

#[cfg(test)]
mod dump {
    #[test]
    #[ignore]
    fn dump_tracks() {
        let dir = std::env::var("RK_MUSIC_OUT").unwrap();
        for t in super::TRACKS {
            let t0 = std::time::Instant::now();
            let b = super::render(t);
            let secs = b.len() as f32 / 2.0 / super::RATE as f32;
            let rms = (b.iter().map(|v| v * v).sum::<f32>() / b.len() as f32).sqrt();
            println!("TRACK {:?} {:.0}s rms={:.3} render={:?}", t, secs, rms, t0.elapsed());
            std::fs::write(format!("{dir}/{}.wav", t.file()), super::wav(&b)).unwrap();
        }
    }
}

/// Шум дождя для бесшовного повтора (4 секунды).
pub fn render_rain() -> Vec<f32> {
    let len = (4.0 * RATE as f32) as usize;
    let mut out = Vec::with_capacity(len * 2);
    let (mut a, mut b) = (0.0f32, 0.0f32);
    for i in 0..len {
        let n1 = noise(i as u32 ^ 0xA5A5);
        let n2 = noise(i as u32 ^ 0x5A5A);
        a += (n1 - a) * 0.35;
        b += (n2 - b) * 0.35;
        // редкие капли
        let drop = if noise(i as u32 / 600 ^ 77) > 0.96 { noise(i as u32) * 0.4 } else { 0.0 };
        // плавное начало/конец, чтобы повтор был незаметен
        let edge = ((i as f32 / 2000.0).min(1.0)).min(((len - i) as f32 / 2000.0).min(1.0));
        out.push((a * 0.5 + drop) * edge);
        out.push((b * 0.5 + drop) * edge);
    }
    out
}
