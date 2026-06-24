//! Golden-PNG diff for the mini-gate #3 correct-answer FX moment.
//!
//! Two layers, by design:
//! - **Pure (CI, no GPU):** load the committed golden and prove the comparator has *teeth* — it
//!   flags an injected regression in the golden's own pixels. This runs everywhere.
//! - **GPU (`#[ignore]`, run under lavapipe):** re-render the FX moment and assert it matches the
//!   golden, AND that the two deliberately-broken variants (no sparks / collapsed palette) do
//!   NOT match. This is the real "test the test": the same render path, with a regression
//!   injected, must fail the golden. Run with:
//!   `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json cargo test -p goblin-gold -- --ignored`

use goblin_gold::headless::{diff, matches, rgba_from_png};

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/fx-correct.png");
const DRILL_GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/goldens/drill-initial.png"
);
const SELECT_GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/goldens/topic-select.png"
);
const COLLECTION_GOLDEN: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/collection.png");
const LADDER_GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/ladder.png");

/// Pure: the committed golden, compared against a copy with one tinted block, must be flagged as
/// changed — so the golden guard genuinely catches a regression (and isn't a no-op). No GPU.
#[test]
fn golden_comparator_catches_a_regression() {
    let (w, h, golden) = rgba_from_png(GOLDEN);
    assert!(w > 0 && h > 0 && !golden.is_empty(), "golden must decode");
    assert!(
        matches(&golden, &golden, 0, 0.0),
        "the golden must match itself exactly"
    );

    // Inject a 40×40 magenta block — a regression the diff must see.
    let mut broken = golden.clone();
    for y in 0..40 {
        for x in 0..40 {
            let i = ((y * w as usize) + x) * 4;
            broken[i] = 255; // R
            broken[i + 1] = 0; // G
            broken[i + 2] = 255; // B
        }
    }
    let d = diff(&golden, &broken, 8);
    assert!(
        d.changed >= 1,
        "the injected block must register as changed"
    );
    assert!(
        d.max_delta > 8,
        "the tint must exceed the per-channel tolerance"
    );
    assert!(
        !matches(&golden, &broken, 8, 0.0001),
        "an injected regression must fail the golden"
    );
}

/// GPU (run under lavapipe with `--ignored`): re-render the FX moment and check it against the
/// golden, then prove each broken variant fails the same check.
#[test]
#[ignore = "needs a Vulkan adapter (lavapipe); run with --ignored"]
fn fx_moment_matches_golden_and_regressions_do_not() {
    use ab_glyph::FontRef;
    use goblin_gold::fx::{render_fx_moment, Variant};
    use goblin_gold::headless::Painter;

    // Tolerance: the re-render is produced by the same code on the same adapter as the blessed
    // golden, so it should be all-but-identical. `TOL`/`FRAC` allow a sliver of software-
    // rasteriser jitter while staying far tighter than the spark burst's footprint, so a missing
    // burst is still caught.
    const TOL: u8 = 6;
    const FRAC: f32 = 0.001; // ≤0.1% of pixels may differ

    let (_w, _h, golden) = rgba_from_png(GOLDEN);
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");
    let painter = Painter::new();

    // The real frame matches the golden.
    let correct = render_fx_moment(&painter, &font, Variant::Correct);
    let dc = diff(&correct, &golden, TOL);
    assert!(
        matches(&correct, &golden, TOL, FRAC),
        "FX moment drifted from the golden: {} / {} px changed (max Δ {})",
        dc.changed,
        dc.total,
        dc.max_delta
    );

    // The injected regressions must NOT match the golden — the test has teeth at the render level.
    let nosparks = render_fx_moment(&painter, &font, Variant::NoSparks);
    let dn = diff(&nosparks, &golden, TOL);
    assert!(
        !matches(&nosparks, &golden, TOL, FRAC),
        "a missing burst must fail the golden, but only {} / {} px differed",
        dn.changed,
        dn.total
    );

    let broken = render_fx_moment(&painter, &font, Variant::BrokenPalette);
    let db = diff(&broken, &golden, TOL);
    assert!(
        !matches(&broken, &golden, TOL, FRAC),
        "a collapsed palette must fail the golden, but only {} / {} px differed",
        db.changed,
        db.total
    );
}

/// Pure: the committed INITIAL-drill golden decodes and self-matches. This golden exists because
/// the live `app.rs` first frame (empty answer box → an empty text run) was the untested state
/// that crashed the APK; capturing it locks the fix against regression.
#[test]
fn initial_drill_golden_is_present_and_self_consistent() {
    let (w, h, g) = rgba_from_png(DRILL_GOLDEN);
    assert!(
        w == goblin_gold::app::DRILL_W && h == goblin_gold::app::DRILL_H,
        "golden dims {w}x{h}"
    );
    assert!(
        matches(&g, &g, 0, 0.0),
        "the initial-drill golden must match itself"
    );
}

/// GPU (run under lavapipe with `--ignored`): re-render the INITIAL drill frame — the empty-answer
/// state that crashed the device — and assert it (a) does NOT panic on the empty text run and (b)
/// matches the committed golden. This is the on-device crash, reproduced and guarded headlessly.
#[test]
#[ignore = "needs a Vulkan adapter (lavapipe); run with --ignored"]
fn initial_drill_frame_renders_and_matches_golden() {
    use ab_glyph::FontRef;
    use goblin_gold::app::render_initial_drill;
    use goblin_gold::headless::Painter;

    let (_w, _h, golden) = rgba_from_png(DRILL_GOLDEN);
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");
    let painter = Painter::new();
    // Before the fix this call panicked ("buffer slices can not be empty") on the empty answer run.
    let frame = render_initial_drill(&painter, &font);
    let d = diff(&frame, &golden, 6);
    assert!(
        matches(&frame, &golden, 6, 0.001),
        "initial drill frame drifted from the golden: {} / {} px changed (max Δ {})",
        d.changed,
        d.total,
        d.max_delta
    );
}

/// Pure: the committed topic-select golden decodes + self-matches. (Phase 3 new screen.)
#[test]
fn topic_select_golden_is_present_and_self_consistent() {
    let (w, h, g) = rgba_from_png(SELECT_GOLDEN);
    assert!(
        w == goblin_gold::app::DRILL_W && h == goblin_gold::app::DRILL_H,
        "golden dims {w}x{h}"
    );
    assert!(
        matches(&g, &g, 0, 0.0),
        "the topic-select golden must match itself"
    );
}

/// GPU (`--ignored`, lavapipe): re-render the initial topic-select (fresh progress → only the root
/// topic unlocked) and assert it matches the committed golden — the new phase-3 screen, guarded.
#[test]
#[ignore = "needs a Vulkan adapter (lavapipe); run with --ignored"]
fn topic_select_renders_and_matches_golden() {
    use ab_glyph::FontRef;
    use goblin_gold::app::render_topic_select;
    use goblin_gold::headless::Painter;

    let (_w, _h, golden) = rgba_from_png(SELECT_GOLDEN);
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");
    let painter = Painter::new();
    let frame = render_topic_select(&painter, &font);
    let d = diff(&frame, &golden, 6);
    assert!(
        matches(&frame, &golden, 6, 0.001),
        "topic-select drifted from the golden: {} / {} px changed (max Δ {})",
        d.changed,
        d.total,
        d.max_delta
    );
}

/// Pure: the committed Collection-screen golden decodes + self-matches. (Phase 3 metagame surface.)
#[test]
fn collection_golden_is_present_and_self_consistent() {
    let (w, h, g) = rgba_from_png(COLLECTION_GOLDEN);
    assert!(
        w == goblin_gold::app::DRILL_W && h == goblin_gold::app::DRILL_H,
        "golden dims {w}x{h}"
    );
    assert!(
        matches(&g, &g, 0, 0.0),
        "the collection golden must match itself"
    );
}

/// GPU (`--ignored`, lavapipe): re-render the Collection screen for the representative save and
/// assert it matches the committed golden — the metagame surface, guarded.
#[test]
#[ignore = "needs a Vulkan adapter (lavapipe); run with --ignored"]
fn collection_renders_and_matches_golden() {
    use ab_glyph::FontRef;
    use goblin_gold::app::render_collection;
    use goblin_gold::headless::Painter;

    let (_w, _h, golden) = rgba_from_png(COLLECTION_GOLDEN);
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");
    let painter = Painter::new();
    let frame = render_collection(&painter, &font);
    let d = diff(&frame, &golden, 6);
    assert!(
        matches(&frame, &golden, 6, 0.001),
        "collection drifted from the golden: {} / {} px changed (max Δ {})",
        d.changed,
        d.total,
        d.max_delta
    );
}

/// Pure: the committed Collector-Ladder golden decodes + self-matches.
#[test]
fn ladder_golden_is_present_and_self_consistent() {
    let (w, h, g) = rgba_from_png(LADDER_GOLDEN);
    assert!(
        w == goblin_gold::app::DRILL_W && h == goblin_gold::app::DRILL_H,
        "golden dims {w}x{h}"
    );
    assert!(
        matches(&g, &g, 0, 0.0),
        "the ladder golden must match itself"
    );
}

/// GPU (`--ignored`, lavapipe): re-render the Collector Ladder (some tiers earned, some locked) and
/// assert it matches the committed golden.
#[test]
#[ignore = "needs a Vulkan adapter (lavapipe); run with --ignored"]
fn ladder_renders_and_matches_golden() {
    use ab_glyph::FontRef;
    use goblin_gold::app::render_ladder;
    use goblin_gold::headless::Painter;

    let (_w, _h, golden) = rgba_from_png(LADDER_GOLDEN);
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");
    let painter = Painter::new();
    let frame = render_ladder(&painter, &font);
    let d = diff(&frame, &golden, 6);
    assert!(
        matches(&frame, &golden, 6, 0.001),
        "ladder drifted from the golden: {} / {} px changed (max Δ {})",
        d.changed,
        d.total,
        d.max_delta
    );
}
