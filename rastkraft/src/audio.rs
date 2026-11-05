//! Проигрывание музыки и звуков: мелодии синтезируются в фоне, переключаются
//! с плавным затуханием. Без звуковой карты всё молча отключается.

use crate::music::{self, Sfx, Track};
use macroquad::audio::{self, PlaySoundParams, Sound};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, channel};

// готовые записи мелодий, вшитые в exe при сборке (см. build.rs)
include!(concat!(env!("OUT_DIR"), "/music_embed.rs"));

/// Вшитая запись мелодии, если есть.
fn embedded(t: Track) -> Option<&'static [u8]> {
    EMBEDDED.iter().find(|(n, _)| *n == t.file()).map(|(_, b)| *b)
}

/// Распаковать вшитую OGG-запись в WAV (на телефоне — в фоновом потоке).
fn ogg_to_wav(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut r = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes)).ok()?;
    let mut pcm: Vec<f32> = Vec::new();
    while let Ok(Some(p)) = r.read_dec_packet_itl() {
        pcm.extend(p.iter().map(|&x| x as f32 / 32768.0));
    }
    Some(music::wav(&pcm))
}

/// Сжатая запись мелодии из assets APK (на телефоне читается сразу, без ожидания).
fn asset_ogg(t: Track) -> Option<Vec<u8>> {
    if !ASSET_OGG {
        return None;
    }
    let got = std::rc::Rc::new(std::cell::RefCell::new(None));
    let g = got.clone();
    macroquad::miniquad::fs::load_file(&format!("music/{}.ogg", t.file()), move |r| *g.borrow_mut() = r.ok());
    got.take()
}

/// Готовая запись мелодии в WAV: вшитая, распакованная или сочинённая заново.
fn track_wav(t: Track) -> Vec<u8> {
    match embedded(t) {
        Some(b) if EMBED_OGG => ogg_to_wav(b).unwrap_or_else(|| music::wav(&music::render(t))),
        Some(b) => b.to_vec(),
        None => music::wav(&music::render(t)),
    }
}

/// Длительность WAV (16 бит, стерео, 44.1 кГц) в секундах.
fn wav_secs(bytes: &[u8]) -> f32 {
    (bytes.len().saturating_sub(44)) as f32 / 4.0 / music::RATE as f32
}

/// Выполнить «асинхронную» загрузку звука сразу: на ПК она не ждёт ничего.
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

fn load(bytes: &[u8]) -> Option<Sound> {
    block_on(audio::load_sound_from_bytes(bytes)).ok()
}

pub struct Audio {
    pub enabled: bool,
    current: Option<(Track, Sound)>,
    /// Подходящие сейчас мелодии (по месту и времени).
    want: &'static [Track],
    /// Сколько ещё играть текущей мелодии до смены на следующую.
    left: f32,
    turn: usize,
    pending: Option<(Track, Receiver<Vec<u8>>)>,
    fade: f32,
    sent_vol: f32,
    sfx: HashMap<Sfx, Sound>,
    rain: Option<Sound>,
    rain_vol: f32,
    /// Название мелодии для всплывающей надписи.
    pub now_playing: Option<(String, f32)>,
}

impl Audio {
    pub fn new() -> Self {
        // на Linux без устройства звука (сервер, контейнер) звук не включаем
        let device = !cfg!(target_os = "linux") || std::path::Path::new("/dev/snd").exists();
        let enabled = device && std::env::var("RK_NOAUDIO").is_err();
        let mut a = Audio { enabled, current: None, want: &[], left: 0.0, turn: 0, pending: None, fade: 0.0, sent_vol: -1.0, sfx: HashMap::new(), rain: None, rain_vol: -1.0, now_playing: None };
        if enabled {
            for s in music::SFX {
                if let Some(snd) = load(&music::wav(&music::render_sfx(s))) {
                    a.sfx.insert(s, snd);
                }
            }
            a.rain = load(&music::wav(&music::render_rain()));
            if let Some(r) = &a.rain {
                audio::play_sound(r, PlaySoundParams { looped: true, volume: 0.0 });
            }
        }
        a
    }

    /// Громкость шума дождя.
    pub fn set_rain(&mut self, v: f32) {
        if let Some(r) = &self.rain {
            if (v - self.rain_vol).abs() > 0.02 {
                audio::set_sound_volume(r, v.clamp(0.0, 1.0));
                self.rain_vol = v;
            }
        }
    }

    pub fn want(&mut self, set: &'static [Track]) {
        self.want = set;
    }

    fn start(&mut self, w: Track, bytes: &[u8]) {
        if let Some(s) = load(bytes) {
            audio::play_sound(&s, PlaySoundParams { looped: true, volume: 0.0 });
            self.sent_vol = 0.0;
            self.fade = 0.0;
            self.left = wav_secs(bytes);
            self.current = Some((w, s));
            self.now_playing = Some((format!("♪ {}", w.title()), 5.0));
        }
    }

    pub fn play(&self, s: Sfx, volume: f32) {
        if !self.enabled || volume <= 0.01 {
            return;
        }
        if let Some(snd) = self.sfx.get(&s) {
            audio::play_sound(snd, PlaySoundParams { looped: false, volume: volume.clamp(0.0, 1.0) });
        }
    }

    pub fn update(&mut self, dt: f32, music_vol: f32) {
        if let Some((_, t)) = &mut self.now_playing {
            *t -= dt;
        }
        if self.now_playing.as_ref().is_some_and(|(_, t)| *t <= 0.0) {
            self.now_playing = None;
        }
        if !self.enabled {
            return;
        }
        let cur = self.current.as_ref().map(|c| c.0);
        self.left -= dt;
        // мелодия доиграла — дальше следующая из подходящих (если их несколько)
        let finished = cur.is_some() && self.left <= 0.0 && self.want.len() > 1;
        let want = if music_vol <= 0.01 || self.want.is_empty() {
            None
        } else if let Some(c) = cur.filter(|c| self.want.contains(c) && !finished) {
            Some(c)
        } else {
            let mut pick = self.want[self.turn % self.want.len()];
            if Some(pick) == cur && self.want.len() > 1 {
                self.turn += 1;
                pick = self.want[self.turn % self.want.len()];
            }
            Some(pick)
        };
        if finished {
            self.turn += 1;
            self.left = f32::MAX;
        }
        if cur.is_some() && cur != want {
            // затухание старой мелодии
            self.fade -= dt / 1.5;
            if self.fade <= 0.0 {
                if let Some((_, s)) = self.current.take() {
                    audio::stop_sound(&s);
                }
                self.fade = 0.0;
            }
        } else if cur.is_none() {
            if let Some(w) = want {
                if let Some(bytes) = embedded(w).filter(|_| !EMBED_OGG) {
                    self.start(w, bytes);
                    return;
                }
                match &self.pending {
                    Some((pt, rx)) if *pt == w => {
                        if let Ok(bytes) = rx.try_recv() {
                            self.pending = None;
                            self.start(w, &bytes);
                        }
                    }
                    _ => {
                        // распаковать (или сочинить) мелодию в фоне
                        let (tx, rx) = channel();
                        let packed = asset_ogg(w);
                        std::thread::spawn(move || {
                            let wav = packed.as_deref().and_then(ogg_to_wav).unwrap_or_else(|| track_wav(w));
                            let _ = tx.send(wav);
                        });
                        self.pending = Some((w, rx));
                    }
                }
            }
        } else {
            self.fade = (self.fade + dt / 2.5).min(1.0);
        }
        if let Some((_, s)) = &self.current {
            let v = self.fade * music_vol * 0.8;
            if (v - self.sent_vol).abs() > 0.01 {
                audio::set_sound_volume(s, v);
                self.sent_vol = v;
            }
        }
    }
}

/// Сохранить все мелодии в папку `music/` рядом с игрой (WAV 44.1 кГц стерео; на телефоне — OGG).
pub fn export_music() -> std::io::Result<std::path::PathBuf> {
    let dir = crate::save::data_dir().join("music");
    std::fs::create_dir_all(&dir)?;
    for t in music::TRACKS {
        // сжатые записи сохраняются как есть (.ogg), остальные — WAV
        let (bytes, ext) = match asset_ogg(t).or_else(|| embedded(t).filter(|_| EMBED_OGG).map(|b| b.to_vec())) {
            Some(b) => (b, "ogg"),
            None => (track_wav(t), "wav"),
        };
        std::fs::write(dir.join(format!("{}.{ext}", t.file())), bytes)?;
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Сжатые мелодии для телефона (если tools/musicpack уже запускался)
    /// распаковываются в WAV той же длины, что и исходная мелодия.
    #[test]
    fn packed_ogg_decodes() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/android-assets/music/01_menu.ogg");
        let Ok(ogg) = std::fs::read(&p) else { return };
        let wav = ogg_to_wav(&ogg).expect("OGG не распаковался");
        let secs = wav_secs(&wav);
        let want = music::render(Track::Menu).len() as f32 / 2.0 / music::RATE as f32;
        assert!((secs - want).abs() < 0.5, "{secs} vs {want}");
    }

    /// В лёгком exe вшитые OGG дают ту же мелодию, что и полный WAV.
    #[test]
    fn small_exe_music() {
        if !EMBED_OGG {
            return;
        }
        let wav = track_wav(Track::Boss);
        let want = music::render(Track::Boss).len() as f32 / 2.0 / music::RATE as f32;
        assert!((wav_secs(&wav) - want).abs() < 0.5);
    }
}
