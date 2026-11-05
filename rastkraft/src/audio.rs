//! Проигрывание музыки и звуков: мелодии синтезируются в фоне, переключаются
//! с плавным затуханием. Без звуковой карты всё молча отключается.

use crate::music::{self, Sfx, Track};
use macroquad::audio::{self, PlaySoundParams, Sound};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, channel};

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
    want: Option<Track>,
    pending: Option<(Track, Receiver<Vec<u8>>)>,
    fade: f32,
    sent_vol: f32,
    sfx: HashMap<Sfx, Sound>,
    /// Название мелодии для всплывающей надписи.
    pub now_playing: Option<(String, f32)>,
}

impl Audio {
    pub fn new() -> Self {
        // на Linux без устройства звука (сервер, контейнер) звук не включаем
        let device = !cfg!(target_os = "linux") || std::path::Path::new("/dev/snd").exists();
        let enabled = device && std::env::var("RK_NOAUDIO").is_err();
        let mut a = Audio { enabled, current: None, want: None, pending: None, fade: 0.0, sent_vol: -1.0, sfx: HashMap::new(), now_playing: None };
        if enabled {
            for s in music::SFX {
                if let Some(snd) = load(&music::wav(&music::render_sfx(s))) {
                    a.sfx.insert(s, snd);
                }
            }
        }
        a
    }

    pub fn want(&mut self, t: Option<Track>) {
        self.want = t;
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
        let want = if music_vol > 0.01 { self.want } else { None };
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
                match &self.pending {
                    Some((pt, rx)) if *pt == w => {
                        if let Ok(bytes) = rx.try_recv() {
                            self.pending = None;
                            if let Some(s) = load(&bytes) {
                                audio::play_sound(&s, PlaySoundParams { looped: true, volume: 0.0 });
                                self.sent_vol = 0.0;
                                self.fade = 0.0;
                                self.current = Some((w, s));
                                self.now_playing = Some((format!("♪ {}", w.title()), 5.0));
                            }
                        }
                    }
                    _ => {
                        // сочинить и отрендерить мелодию в фоне
                        let (tx, rx) = channel();
                        std::thread::spawn(move || {
                            let _ = tx.send(music::wav(&music::render(w)));
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

/// Сохранить все мелодии в папку `music/` рядом с игрой (WAV, 44.1 кГц, стерео).
pub fn export_music() -> std::io::Result<std::path::PathBuf> {
    let dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("music"))).unwrap_or_else(|| "music".into());
    std::fs::create_dir_all(&dir)?;
    for t in music::TRACKS {
        std::fs::write(dir.join(format!("{}.wav", t.file())), music::wav(&music::render(t)))?;
    }
    Ok(dir)
}
