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

/// Распаковать вшитую OGG-запись в WAV (в фоновом потоке).
fn ogg_to_wav(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut r = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes)).ok()?;
    let mut pcm: Vec<f32> = Vec::new();
    while let Ok(Some(p)) = r.read_dec_packet_itl() {
        pcm.extend(p.iter().map(|&x| x as f32 / 32768.0));
    }
    Some(music::wav(&pcm))
}

/// Где на ПК хранятся уже сочинённые мелодии (чтобы со второго запуска играли сразу).
/// На телефоне не храним: это ~250 МБ, мелодия сочиняется заново за несколько секунд.
fn cache_path(t: Track) -> Option<std::path::PathBuf> {
    if crate::touch::is_mobile() {
        return None;
    }
    // номер версии музыки в имени: после переделки мелодий старые записи не берутся
    Some(crate::save::data_dir().join("music_cache").join(format!("{}_v2.wav", t.file())))
}

/// Песни rtxx131, вшитые как есть (OGG): (мелодия, запись).
/// На ПК песни вшиты в exe; на Android лежат одной копией в assets APK
/// (а не в каждой из двух библиотек под разные процессоры).
macro_rules! song {
    ($t:expr, $name:literal) => {
        ($t, $name, if cfg!(target_os = "android") { &[] as &[u8] } else { include_bytes!(concat!("../assets/music/", $name, ".ogg")) as &[u8] })
    };
}
const SONGS: &[(Track, &str, &[u8])] = &[
    song!(Track::Menu, "menu_rtxx131"),
    song!(Track::Heaven, "heaven_rtxx131"),
    song!(Track::Meadow, "world_rtxx131"),
    song!(Track::Village, "world2_rtxx131"),
    song!(Track::Caves, "caves_rtxx131"),
    song!(Track::Boss, "boss_rtxx131"),
    song!(Track::Fury, "fury_rtxx131"),
];

/// Сжатая запись песни: из exe или (на Android) из assets APK. Читать в главном потоке.
fn song_packed(t: Track) -> Option<Vec<u8>> {
    let (_, name, bytes) = SONGS.iter().find(|(st, _, _)| *st == t)?;
    if !bytes.is_empty() {
        return Some(bytes.to_vec());
    }
    let got = std::rc::Rc::new(std::cell::RefCell::new(None));
    let g = got.clone();
    macroquad::miniquad::fs::load_file(&format!("music/{name}.ogg"), move |r| *g.borrow_mut() = r.ok());
    got.take()
}

/// Готовая запись мелодии в WAV: вшитая, сохранённая ранее или сочинённая заново.
/// Сочинённая в игре мелодия совпадает с записью байт в байт — качество то же.
fn track_wav(t: Track) -> Vec<u8> {
    // настоящая песня вместо сочинённой (вшитая в exe)
    if let Some(w) = SONGS.iter().find(|(st, _, b)| *st == t && !b.is_empty()).and_then(|(_, _, b)| ogg_to_wav(b)) {
        return w;
    }
    match embedded(t) {
        Some(b) if EMBED_OGG => return ogg_to_wav(b).unwrap_or_else(|| music::wav(&music::render(t))),
        Some(b) => return b.to_vec(),
        None => {}
    }
    let cache = cache_path(t);
    if let Some(b) = cache.as_ref().and_then(|p| std::fs::read(p).ok()) {
        if b.len() > 44 && b.starts_with(b"RIFF") && u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize + 8 == b.len() {
            return b;
        }
    }
    let wav = music::wav(&music::render(t));
    if let Some(p) = cache {
        let tmp = p.with_extension("part");
        if std::fs::create_dir_all(p.parent().unwrap()).is_ok() && std::fs::write(&tmp, &wav).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }
    wav
}

/// Подготовить мелодию в фоновом потоке.
fn spawn_track(t: Track) -> Receiver<Vec<u8>> {
    let (tx, rx) = channel();
    // песню читаем здесь (на Android — из assets, только в главном потоке), распаковываем в фоне
    let packed = song_packed(t);
    std::thread::spawn(move || {
        let wav = packed.as_deref().and_then(ogg_to_wav).unwrap_or_else(|| track_wav(t));
        let _ = tx.send(wav);
    });
    rx
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
    /// Следующая мелодия, сочиняемая заранее, пока играет текущая.
    ahead: Option<(Track, Receiver<Vec<u8>>)>,
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
        let mut a = Audio { enabled, current: None, want: &[], left: 0.0, turn: 0, pending: None, ahead: None, fade: 0.0, sent_vol: -1.0, sfx: HashMap::new(), rain: None, rain_vol: -1.0, now_playing: None };
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
                if let Some(bytes) = embedded(w).filter(|_| !EMBED_OGG && !SONGS.iter().any(|(st, _, _)| *st == w)) {
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
                        // заранее сочинённая — берём её, иначе сочиняем в фоне
                        let rx = match self.ahead.take() {
                            Some((at, rx)) if at == w => rx,
                            _ => spawn_track(w),
                        };
                        self.pending = Some((w, rx));
                    }
                }
            }
        } else {
            self.fade = (self.fade + dt / 2.5).min(1.0);
            // пока играет мелодия — заранее готовим следующую
            if self.ahead.is_none() && self.want.len() > 1 && embedded(self.want[0]).is_none() {
                let mut next = self.want[(self.turn + 1) % self.want.len()];
                if Some(next) == cur {
                    next = self.want[(self.turn + 2) % self.want.len()];
                }
                if Some(next) != cur {
                    self.ahead = Some((next, spawn_track(next)));
                }
            }
        }
        // подходящие мелодии сменились — заготовка больше не нужна
        if self.ahead.as_ref().is_some_and(|(t, _)| !self.want.contains(t)) {
            self.ahead = None;
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
        let (bytes, ext) = match embedded(t).filter(|_| EMBED_OGG).map(|b| b.to_vec()) {
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

    /// Мелодия, сочинённая в игре, совпадает с вшитой записью байт в байт.
    #[test]
    fn render_matches_embedded() {
        if EMBED_OGG || EMBEDDED.is_empty() {
            return;
        }
        let mut total = 0.0;
        for t in music::TRACKS {
            let t0 = std::time::Instant::now();
            let w = music::wav(&music::render(t));
            total += t0.elapsed().as_secs_f32();
            if let Some(e) = embedded(t) {
                assert!(e == &w[..], "{:?} отличается", t);
            }
        }
        eprintln!("синтез 16 мелодий: {total:.1} с на одном ядре");
    }

    /// Сочинённая мелодия сохраняется и со второго раза читается с диска той же.
    #[test]
    fn music_cache_roundtrip() {
        if !EMBEDDED.is_empty() {
            return;
        }
        let t = Track::Desert;
        if let Some(p) = cache_path(t) {
            let _ = std::fs::remove_file(&p);
        }
        let a = track_wav(t);
        assert!(cache_path(t).unwrap().exists());
        let t0 = std::time::Instant::now();
        let b = track_wav(t);
        assert_eq!(a, b);
        assert!(t0.elapsed().as_secs_f32() < 1.0, "с диска должно быть быстро");
    }

    /// Песня rtxx131 читается и даёт ту же длину, что и файл (~32 с).
    #[test]
    fn rtx_song_decodes() {
        let w = track_wav(Track::Menu);
        let secs = wav_secs(&w);
        assert!((secs - 31.8).abs() < 0.5, "{secs}");
        let secs = wav_secs(&track_wav(Track::Heaven));
        assert!((secs - 53.2).abs() < 0.5, "{secs}");
        let secs = wav_secs(&track_wav(Track::Meadow));
        assert!((secs - 58.1).abs() < 0.5, "{secs}");
        let secs = wav_secs(&track_wav(Track::Village));
        assert!((secs - 191.8).abs() < 0.5, "{secs}");
        let secs = wav_secs(&track_wav(Track::Caves));
        assert!((secs - 213.1).abs() < 0.5, "{secs}");
        let secs = wav_secs(&track_wav(Track::Boss));
        assert!((secs - 207.7).abs() < 0.5, "{secs}");
        let secs = wav_secs(&track_wav(Track::Fury));
        assert!((secs - 128.8).abs() < 0.5, "{secs}");
    }
}
