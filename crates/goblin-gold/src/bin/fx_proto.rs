//! `fx_proto` — BRICKMAP-GG1 mini-gate #3 evidence + golden blesser.
//!
//! Renders the **correct-answer FX flourish** (engine-native: gold spark burst via the engine's
//! particle system + the engine's palette-dither post) through the REAL wgpu path → a PNG. With
//! `GG_BLESS=1` it also (re)writes the committed golden the diff test asserts against.
//!
//! Run:  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!         cargo run -p goblin-gold --bin fx_proto -- <out_dir>
//!       GG_BLESS=1 VK_ICD_FILENAMES=... cargo run -p goblin-gold --bin fx_proto -- <out_dir>

use ab_glyph::FontRef;
use goblin_gold::fx::{self, Variant};
use goblin_gold::headless::{write_png, Painter};

fn main() {
    let out_dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    std::fs::create_dir_all(&out_dir).ok();
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");
    let painter = Painter::new();

    // The real FX moment → the deliverable screenshot.
    let rgba = fx::render_fx_moment(&painter, &font, Variant::Correct);
    let shot = format!("{out_dir}/gg-fx-correct.png");
    write_png(&shot, fx::W, fx::H, &rgba);
    println!("wrote {shot}");

    // Also dump the two regression frames (handy when eyeballing what the golden guards against).
    for (v, name) in [
        (Variant::NoSparks, "gg-fx-nosparks.png"),
        (Variant::BrokenPalette, "gg-fx-brokenpalette.png"),
    ] {
        let r = fx::render_fx_moment(&painter, &font, v);
        let p = format!("{out_dir}/{name}");
        write_png(&p, fx::W, fx::H, &r);
        println!("wrote {p}");
    }

    if std::env::var("GG_BLESS").is_ok() {
        let golden = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/fx-correct.png");
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens")).ok();
        write_png(golden, fx::W, fx::H, &rgba);
        println!("BLESSED golden {golden}");
    }
}
