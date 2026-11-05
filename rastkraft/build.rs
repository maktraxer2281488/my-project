// Сборка: заранее «записывает» всю музыку игры (её сочиняет src/music.rs)
// и вшивает готовые WAV прямо в exe — так всё лежит в одном файле.
// Для телефона музыка не вшивается в программу: она лежит одной сжатой
// копией (OGG) в assets APK, её готовит tools/musicpack.

#[allow(dead_code)]
#[path = "src/music.rs"]
mod music;

use std::io::Write;

fn main() {
    println!("cargo:rerun-if-changed=src/music.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RK_NO_EMBED");
    let out = std::env::var("OUT_DIR").unwrap();
    let android = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|t| t == "android");
    // RK_SMALL=1 — лёгкий exe: вшить сжатые OGG из tools/musicpack вместо WAV
    println!("cargo:rerun-if-env-changed=RK_SMALL");
    let ogg_dir = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("target/android-assets/music");
    let small = std::env::var("RK_SMALL").is_ok() && !android;
    if small {
        for t in music::TRACKS {
            let p = ogg_dir.join(format!("{}.ogg", t.file()));
            assert!(p.exists(), "нет {}: сначала запустите tools/musicpack", p.display());
            println!("cargo:rerun-if-changed={}", p.display());
        }
    }
    let embed = std::env::var("RK_NO_EMBED").is_err() && !android && !small;
    if embed {
        // все мелодии пишутся параллельно
        std::thread::scope(|sc| {
            for t in music::TRACKS {
                let out = out.clone();
                sc.spawn(move || {
                    let bytes = music::wav(&music::render(t));
                    std::fs::write(format!("{out}/{}.wav", t.file()), bytes).unwrap();
                });
            }
        });
    }
    let mut f = std::fs::File::create(format!("{out}/music_embed.rs")).unwrap();
    writeln!(f, "/// Мелодии лежат в assets APK (music/<имя>.ogg).").unwrap();
    writeln!(f, "pub const ASSET_OGG: bool = {};", android && std::env::var("RK_NO_EMBED").is_err()).unwrap();
    writeln!(f, "/// Вшитые записи сжаты в OGG (лёгкий exe), иначе — WAV.").unwrap();
    writeln!(f, "pub const EMBED_OGG: bool = {small};").unwrap();
    writeln!(f, "/// Готовые записи мелодий (имя файла, данные).").unwrap();
    writeln!(f, "pub static EMBEDDED: &[(&str, &[u8])] = &[").unwrap();
    if embed {
        for t in music::TRACKS {
            writeln!(f, "    (\"{0}\", include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{0}.wav\"))),", t.file()).unwrap();
        }
    }
    if small {
        for t in music::TRACKS {
            writeln!(f, "    (\"{}\", include_bytes!({:?})),", t.file(), ogg_dir.join(format!("{}.ogg", t.file()))).unwrap();
        }
    }
    writeln!(f, "];").unwrap();
}
