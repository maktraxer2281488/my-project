// Сборка: заранее «записывает» всю музыку игры (её сочиняет src/music.rs)
// и вшивает готовые WAV прямо в exe — так всё лежит в одном файле.

#[allow(dead_code)]
#[path = "src/music.rs"]
mod music;

use std::io::Write;

fn main() {
    println!("cargo:rerun-if-changed=src/music.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RK_NO_EMBED");
    let out = std::env::var("OUT_DIR").unwrap();
    let embed = std::env::var("RK_NO_EMBED").is_err();
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
    writeln!(f, "/// Готовые записи мелодий (имя файла, WAV).").unwrap();
    writeln!(f, "pub static EMBEDDED: &[(&str, &[u8])] = &[").unwrap();
    if embed {
        for t in music::TRACKS {
            writeln!(f, "    (\"{0}\", include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{0}.wav\"))),", t.file()).unwrap();
        }
    }
    writeln!(f, "];").unwrap();
}
