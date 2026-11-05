//! Настройки игры; сохраняются в файл `rastkraft_settings.txt` рядом с игрой.

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub render_dist: i32,
    pub fov: f32,
    pub sensitivity: f32,
    pub brightness: f32,
    pub fog: bool,
    pub clouds: bool,
    pub smooth_light: bool,
    pub view_bob: bool,
    pub show_fps: bool,
    pub gui_scale: f32,
    pub difficulty: u8,
    pub invert_mouse: bool,
    pub pvp: bool,
    pub name: String,
    /// Доля разрешения, в которой рисуется 3D (0.5–1.0).
    pub render_scale: f32,
    /// Сама снижать дальность, чтобы держать 60 FPS.
    pub auto_dist: bool,
    /// Уровень оптимизации 0–3 (см. opt.rs).
    pub opt_level: u8,
    /// Ограничение FPS (0 — без ограничения).
    pub fps_limit: u32,
    /// Громкость музыки и звуков (0–1).
    pub music_vol: f32,
    pub sound_vol: f32,
    /// Размер плитки текстур в пикселях: 8, 16, 32, 64, 128, 256 (применяется после перезапуска).
    pub tex_res: u32,
    pub minimap: bool,
    /// Внешность: рубашка, штаны, волосы, кожа (номера в палитрах).
    pub look: [u8; 4],
    /// Адрес ретранслятора для игры через интернет.
    pub relay: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            render_dist: 8,
            fov: 75.0,
            sensitivity: 1.0,
            brightness: 0.35,
            fog: true,
            clouds: true,
            smooth_light: true,
            view_bob: true,
            show_fps: true,
            gui_scale: 1.0,
            difficulty: 2,
            invert_mouse: false,
            pvp: true,
            name: "Игрок".to_string(),
            render_scale: 1.0,
            auto_dist: true,
            opt_level: 1,
            fps_limit: 0,
            music_vol: 0.6,
            sound_vol: 0.8,
            tex_res: 16,
            minimap: true,
            look: [255; 4],
            relay: String::new(),
        }
    }
}

/// Варианты ограничения FPS (0 — без ограничения).
pub const FPS_LIMITS: [u32; 5] = [0, 30, 60, 120, 144];

pub const DIFFICULTY: [&str; 4] = ["Мирная", "Лёгкая", "Обычная", "Сложная"];

fn path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("rastkraft_settings.txt")))
        .unwrap_or_else(|| "rastkraft_settings.txt".into())
}

impl Settings {
    pub fn load() -> Self {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(path()) else { return s };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let b = v == "1" || v == "true";
            match k {
                "render_dist" => s.render_dist = v.parse().unwrap_or(s.render_dist),
                "fov" => s.fov = v.parse().unwrap_or(s.fov),
                "sensitivity" => s.sensitivity = v.parse().unwrap_or(s.sensitivity),
                "brightness" => s.brightness = v.parse().unwrap_or(s.brightness),
                "fog" => s.fog = b,
                "clouds" => s.clouds = b,
                "smooth_light" => s.smooth_light = b,
                "view_bob" => s.view_bob = b,
                "show_fps" => s.show_fps = b,
                "gui_scale" => s.gui_scale = v.parse().unwrap_or(s.gui_scale),
                "difficulty" => s.difficulty = v.parse().unwrap_or(s.difficulty),
                "invert_mouse" => s.invert_mouse = b,
                "pvp" => s.pvp = b,
                "name" => s.name = v.to_string(),
                "render_scale" => s.render_scale = v.parse().unwrap_or(s.render_scale),
                "auto_dist" => s.auto_dist = b,
                "opt_level" => s.opt_level = v.parse().unwrap_or(s.opt_level),
                "fps_limit" => s.fps_limit = v.parse().unwrap_or(s.fps_limit),
                "music_vol" => s.music_vol = v.parse().unwrap_or(s.music_vol),
                "sound_vol" => s.sound_vol = v.parse().unwrap_or(s.sound_vol),
                "tex_res" => s.tex_res = v.parse().unwrap_or(s.tex_res),
                "minimap" => s.minimap = b,
                "relay" => s.relay = v.to_string(),
                "look" => {
                    let v: Vec<u8> = v.split(',').filter_map(|x| x.parse().ok()).collect();
                    if v.len() == 4 {
                        s.look = [v[0], v[1], v[2], v[3]];
                    }
                }
                _ => {}
            }
        }
        s.clamp();
        s
    }

    pub fn clamp(&mut self) {
        self.render_dist = self.render_dist.clamp(2, 32);
        self.fov = self.fov.clamp(50.0, 110.0);
        self.sensitivity = self.sensitivity.clamp(0.1, 4.0);
        self.brightness = self.brightness.clamp(0.0, 1.0);
        self.gui_scale = self.gui_scale.clamp(0.75, 2.0);
        self.difficulty = self.difficulty.min(3);
        self.render_scale = self.render_scale.clamp(0.4, 1.0);
        self.opt_level = self.opt_level.min(3);
        self.music_vol = self.music_vol.clamp(0.0, 1.0);
        self.sound_vol = self.sound_vol.clamp(0.0, 1.0);
        if !crate::blocks::TEX_RES.contains(&(self.tex_res as usize)) {
            self.tex_res = 16;
        }
        if !FPS_LIMITS.contains(&self.fps_limit) {
            self.fps_limit = 0;
        }
    }

    pub fn save(&self) {
        let text = format!(
            "render_dist={}\nfov={}\nsensitivity={}\nbrightness={}\nfog={}\nclouds={}\nsmooth_light={}\nview_bob={}\nshow_fps={}\ngui_scale={}\ndifficulty={}\ninvert_mouse={}\npvp={}\nname={}\nrender_scale={}\nauto_dist={}\nopt_level={}\nfps_limit={}\nmusic_vol={}\nsound_vol={}\ntex_res={}\nminimap={}\nlook={},{},{},{}\nrelay={}\n",
            self.render_dist,
            self.fov,
            self.sensitivity,
            self.brightness,
            self.fog as u8,
            self.clouds as u8,
            self.smooth_light as u8,
            self.view_bob as u8,
            self.show_fps as u8,
            self.gui_scale,
            self.difficulty,
            self.invert_mouse as u8,
            self.pvp as u8,
            self.name,
            self.render_scale,
            self.auto_dist as u8,
            self.opt_level,
            self.fps_limit,
            self.music_vol,
            self.sound_vol,
            self.tex_res,
            self.minimap as u8,
            self.look[0],
            self.look[1],
            self.look[2],
            self.look[3],
            self.relay
        );
        let _ = std::fs::write(path(), text);
    }
}
