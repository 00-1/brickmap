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
use goblin_gold::app;
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

    // The INITIAL drill frame (empty answer box, no FX) — the state that crashed on device.
    let drill_rgba = app::render_initial_drill(&painter, &font);
    let drill_shot = format!("{out_dir}/gg-drill-initial.png");
    write_png(&drill_shot, app::DRILL_W, app::DRILL_H, &drill_rgba);
    println!("wrote {drill_shot}");

    // The INITIAL topic-select (fresh progress → only the root topic unlocked).
    let sel_rgba = app::render_topic_select(&painter, &font);
    let sel_shot = format!("{out_dir}/gg-topic-select.png");
    write_png(&sel_shot, app::DRILL_W, app::DRILL_H, &sel_rgba);
    println!("wrote {sel_shot}");

    if std::env::var("GG_BLESS").is_ok() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens")).ok();
        for (name, w, h, data) in [
            ("fx-correct.png", fx::W, fx::H, &rgba),
            ("drill-initial.png", app::DRILL_W, app::DRILL_H, &drill_rgba),
            ("topic-select.png", app::DRILL_W, app::DRILL_H, &sel_rgba),
        ] {
            let path = format!(
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/{}"),
                name
            );
            write_png(&path, w, h, data);
            println!("BLESSED golden {path}");
        }
    }
}
