//! Текстур-паки: картинка-атлас в папке `textures/` рядом с игрой
//! (на телефоне — Android/data/com.maktraxer.rastkraft/files/textures).
//!
//! Пак — это PNG той же раскладки, что и «шаблон» (кнопка «Сохранить шаблон
//! текстур»): 64 плитки в ширину, плитки идут построчно. Размер плитки любой
//! (16, 32, 64… — ширина картинки ÷ 64). Пустые (прозрачные) плитки пака
//! берутся из встроенных текстур, так что можно перерисовать только часть.

use crate::blocks::{ATLAS_TILES, atlas_pixels, atlas_rows, display_atlas, TILE, ATLAS};
use std::path::PathBuf;

/// Папка текстур-паков.
pub fn dir() -> PathBuf {
    if cfg!(target_os = "android") {
        // эту папку видно в файловом менеджере и при подключении к ПК
        PathBuf::from("/storage/emulated/0/Android/data/com.maktraxer.rastkraft/files/textures")
    } else {
        crate::save::data_dir().join("textures")
    }
}

/// Имена паков (PNG-файлы в папке, кроме шаблона).
pub fn list() -> Vec<String> {
    let _ = std::fs::create_dir_all(dir());
    let mut v: Vec<String> = std::fs::read_dir(dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            let low = n.to_lowercase();
            (low.ends_with(".png") && low != "template.png").then(|| n[..n.len() - 4].to_string())
        })
        .collect();
    v.sort();
    v
}

/// Атлас с паком поверх встроенных текстур: (пиксели, ширина, высота, размер плитки).
pub fn load(name: &str) -> Result<(Vec<u8>, usize, usize, usize), String> {
    let path = dir().join(format!("{name}.png"));
    let bytes = std::fs::read(&path).map_err(|e| format!("не открыть {}: {e}", path.display()))?;
    let img = macroquad::texture::Image::from_file_with_format(&bytes, None).map_err(|e| format!("не картинка: {e}"))?;
    let (pw, ph) = (img.width as usize, img.height as usize);
    if pw % ATLAS_TILES != 0 || pw < ATLAS_TILES * 4 {
        return Err(format!("ширина {pw} должна делиться на {ATLAS_TILES} (как у шаблона)"));
    }
    let tr = pw / ATLAS_TILES;
    // встроенные текстуры в размере плиток пака — подложка для пустых плиток
    let (mut px, w, h) = display_atlas(tr);
    let rows = atlas_rows();
    for ty in 0..rows.min(ph / tr) {
        for tx in 0..ATLAS_TILES {
            // плитка пака не пустая — берём её целиком
            let mut any = false;
            'scan: for y in 0..tr {
                for x in 0..tr {
                    if img.bytes[((ty * tr + y) * pw + tx * tr + x) * 4 + 3] != 0 {
                        any = true;
                        break 'scan;
                    }
                }
            }
            if !any {
                continue;
            }
            for y in 0..tr {
                let s = ((ty * tr + y) * pw + tx * tr) * 4;
                let d = ((ty * tr + y) * w + tx * tr) * 4;
                px[d..d + tr * 4].copy_from_slice(&img.bytes[s..s + tr * 4]);
            }
        }
    }
    Ok((px, w, h, tr))
}

/// Сохранить встроенные текстуры как шаблон для своего пака (16×16 на плитку).
pub fn export_template() -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let path = dir().join("template.png");
    let rows = atlas_rows();
    let data = &atlas_pixels()[..rows * TILE * ATLAS * 4];
    let f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), ATLAS as u32, (rows * TILE) as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(data).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    /// Шаблон сохраняется и читается обратно как пак той же раскладки.
    #[test]
    fn template_roundtrip() {
        let p = super::export_template().expect("шаблон");
        let bytes = std::fs::read(&p).unwrap();
        let img = macroquad::texture::Image::from_file_with_format(&bytes, None).unwrap();
        assert_eq!(img.width as usize, crate::blocks::ATLAS);
        assert_eq!(img.height as usize, crate::blocks::atlas_rows() * crate::blocks::TILE);
    }
}
