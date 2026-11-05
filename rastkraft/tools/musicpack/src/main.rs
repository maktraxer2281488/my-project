//! Записывает всю музыку игры в OGG Vorbis для версии на телефон:
//! `cargo run --release --manifest-path tools/musicpack/Cargo.toml`
//! Файлы ложатся в `target/android-assets/music/` и попадают в APK одной копией.

#[allow(dead_code)]
#[path = "../../../src/music.rs"]
mod music;

use std::num::{NonZeroU32, NonZeroU8};
use std::path::PathBuf;

/// Сжать стерео-запись (чередующиеся каналы, −1..1) в OGG Vorbis.
fn ogg(samples: &[f32]) -> Vec<u8> {
    use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};
    let mut out = Vec::new();
    let mut enc = VorbisEncoderBuilder::new(NonZeroU32::new(music::RATE).unwrap(), NonZeroU8::new(2).unwrap(), &mut out)
        .unwrap()
        // качество 0.4 ≈ 128 кбит/с: в телефоне на слух не отличить от WAV
        .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.4 })
        .build()
        .unwrap();
    for block in samples.chunks(4096 * 2) {
        let l: Vec<f32> = block.iter().step_by(2).copied().collect();
        let r: Vec<f32> = block.iter().skip(1).step_by(2).copied().collect();
        enc.encode_audio_block([&l[..], &r[..]]).unwrap();
    }
    enc.finish().unwrap();
    out
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| root.join("target/android-assets/music"));
    std::fs::create_dir_all(&dir).unwrap();
    let total = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for t in music::TRACKS {
            let (dir, total) = (&dir, &total);
            sc.spawn(move || {
                let bytes = ogg(&music::render(t));
                total.fetch_add(bytes.len(), std::sync::atomic::Ordering::Relaxed);
                std::fs::write(dir.join(format!("{}.ogg", t.file())), bytes).unwrap();
            });
        }
    });
    println!("{} мелодий, {:.1} МБ → {}", music::TRACKS.len(), total.into_inner() as f64 / 1e6, dir.display());
}
