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

    // The FULLY-UNLOCKED topic-select (all 46 topics) — the worst case for layout; the multi-column
    // grid must keep every row on-screen (phase-5 polish: a fixed row height hid late topics).
    let selfull_rgba = app::render_topic_select_full(&painter, &font);
    let selfull_shot = format!("{out_dir}/gg-topic-select-full.png");
    write_png(&selfull_shot, app::DRILL_W, app::DRILL_H, &selfull_rgba);
    println!("wrote {selfull_shot}");

    // The Collection (metagame summary) screen for a representative save.
    let coll_rgba = app::render_collection(&painter, &font);
    let coll_shot = format!("{out_dir}/gg-collection.png");
    write_png(&coll_shot, app::DRILL_W, app::DRILL_H, &coll_rgba);
    println!("wrote {coll_shot}");

    // The Collector Ladder detail (some tiers earned, some locked).
    let ladder_rgba = app::render_ladder(&painter, &font);
    let ladder_shot = format!("{out_dir}/gg-ladder.png");
    write_png(&ladder_shot, app::DRILL_W, app::DRILL_H, &ladder_rgba);
    println!("wrote {ladder_shot}");

    // The end-of-round Results summary.
    let results_rgba = app::render_results(&painter, &font);
    let results_shot = format!("{out_dir}/gg-results.png");
    write_png(&results_shot, app::DRILL_W, app::DRILL_H, &results_rgba);
    println!("wrote {results_shot}");

    // The metagame drill-downs.
    let heroes_rgba = app::render_heroes(&painter, &font);
    write_png(
        &format!("{out_dir}/gg-heroes.png"),
        app::DRILL_W,
        app::DRILL_H,
        &heroes_rgba,
    );
    let events_rgba = app::render_events(&painter, &font);
    write_png(
        &format!("{out_dir}/gg-events.png"),
        app::DRILL_W,
        app::DRILL_H,
        &events_rgba,
    );
    let items_rgba = app::render_items(&painter, &font);
    write_png(
        &format!("{out_dir}/gg-items.png"),
        app::DRILL_W,
        app::DRILL_H,
        &items_rgba,
    );
    println!("wrote heroes/events/items");

    // The Arena screen (foe showcase + party-pick, with portraits/backdrops).
    let arena_rgba = app::render_arena(&painter, &font);
    write_png(
        &format!("{out_dir}/gg-arena.png"),
        app::DRILL_W,
        app::DRILL_H,
        &arena_rgba,
    );
    println!("wrote arena");

    // The Arena at the web reference aspect (430×880) → committed to halves visual-ref for review.
    let arena_ref = app::render_arena_ref(&painter, &font);
    write_png(
        &format!("{out_dir}/arena-prefight-brickmap.png"),
        app::REF_W,
        app::REF_H,
        &arena_ref,
    );
    println!("wrote arena-prefight-brickmap (430x880 visual-ref)");

    // The Daily-Event (event-play) screen at the web reference aspect → committed to halves visual-ref.
    let event_ref = app::render_event_play_ref(&painter, &font);
    write_png(
        &format!("{out_dir}/event-play-brickmap.png"),
        app::REF_W,
        app::REF_H,
        &event_ref,
    );
    println!("wrote event-play-brickmap (430x880 visual-ref)");

    // The Heroes roster at the web reference aspect (430×880) → committed to halves visual-ref.
    let heroes_ref = app::render_heroes_ref(&painter, &font);
    write_png(
        &format!("{out_dir}/heroes-brickmap.png"),
        app::REF_W,
        app::REF_H,
        &heroes_ref,
    );
    println!("wrote heroes-brickmap (430x880 visual-ref)");

    // The Heroes screen in a PARTIAL save (locked-hero rows) → halves visual-ref.
    let heroes_partial = app::render_heroes_partial_ref(&painter, &font);
    write_png(
        &format!("{out_dir}/heroes-partial-brickmap.png"),
        app::REF_W,
        app::REF_H,
        &heroes_partial,
    );
    println!("wrote heroes-partial-brickmap (430x880 visual-ref)");

    // The Inventory (Items) screen — Awards tab — at the web reference aspect → halves visual-ref.
    let items_ref = app::render_items_ref(&painter, &font);
    write_png(
        &format!("{out_dir}/inventory-awards-brickmap.png"),
        app::REF_W,
        app::REF_H,
        &items_ref,
    );
    println!("wrote inventory-awards-brickmap (430x880 visual-ref)");

    // The procedural-art contact sheet (F1 heroes · F2 foes · F3 scenery · F4 crests).
    let art_rgba = app::render_art_sheet(&painter, &font);
    write_png(
        &format!("{out_dir}/gg-art-sheet.png"),
        app::DRILL_W,
        app::DRILL_H,
        &art_rgba,
    );
    println!("wrote art-sheet");

    if std::env::var("GG_BLESS").is_ok() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens")).ok();
        for (name, w, h, data) in [
            ("fx-correct.png", fx::W, fx::H, &rgba),
            ("drill-initial.png", app::DRILL_W, app::DRILL_H, &drill_rgba),
            ("topic-select.png", app::DRILL_W, app::DRILL_H, &sel_rgba),
            (
                "topic-select-full.png",
                app::DRILL_W,
                app::DRILL_H,
                &selfull_rgba,
            ),
            ("collection.png", app::DRILL_W, app::DRILL_H, &coll_rgba),
            ("ladder.png", app::DRILL_W, app::DRILL_H, &ladder_rgba),
            ("results.png", app::DRILL_W, app::DRILL_H, &results_rgba),
            ("heroes.png", app::DRILL_W, app::DRILL_H, &heroes_rgba),
            ("events.png", app::DRILL_W, app::DRILL_H, &events_rgba),
            ("items.png", app::DRILL_W, app::DRILL_H, &items_rgba),
            ("arena.png", app::DRILL_W, app::DRILL_H, &arena_rgba),
            ("art-sheet.png", app::DRILL_W, app::DRILL_H, &art_rgba),
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
