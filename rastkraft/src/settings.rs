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
        }
    }
}

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
    }

    pub fn save(&self) {
        let text = format!(
            "render_dist={}\nfov={}\nsensitivity={}\nbrightness={}\nfog={}\nclouds={}\nsmooth_light={}\nview_bob={}\nshow_fps={}\ngui_scale={}\ndifficulty={}\ninvert_mouse={}\n",
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
            self.invert_mouse as u8
        );
        let _ = std::fs::write(path(), text);
    }
}
