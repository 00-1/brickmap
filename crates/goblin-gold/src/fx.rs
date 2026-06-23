//! The **correct-answer FX flourish** — engine-native, built on brickmap's OWN recipes (NOT a
//! port of the web `fxgl.js`): a gold **spark burst** from the engine's CPU
//! [`brickmap::particles::ParticleSystem`], composited over the answer and then recoloured by the
//! engine's **palette dither** post ([`crate::headless::Painter::paint_palettized`] →
//! `bm-render`'s Bayer-4×4 luminance ramp). On a correct answer the screen blooms into dithered
//! gold with a shower of bright flecks.
//!
//! The burst is **deterministic** (a fixed seed + a fixed simulated slice), so the FX moment
//! renders pixel-stably — which is what lets a headless **golden-PNG diff** assert it (see
//! `tests/`/`fx_proto`). Particle motion + fade come from the engine's system; only the burst
//! *shape* (a 2-D screen pop in gold) is chosen here.

use crate::headless::{Painter, RectRun, TextRun};
use crate::text::Atlas;
use ab_glyph::FontRef;
use brickmap::particles::ParticleSystem;
use glam::Vec3;

/// A curated **gold ramp** (dark→light, luminance-ascending) fed to the engine's palette pass.
/// The engine holds no curated set — the game owns the look (palette.rs §"ramps are content").
pub const GOLD_RAMP: [[f32; 3]; 6] = [
    [0.06, 0.04, 0.02],
    [0.28, 0.16, 0.05],
    [0.55, 0.34, 0.10],
    [0.82, 0.58, 0.20],
    [1.00, 0.82, 0.40],
    [1.00, 0.95, 0.80],
];
/// Dither spread for the palette pass (≈1 step → a lively Bayer shimmer between ramp stops).
pub const FX_DITHER: f32 = 1.0;

/// FX canvas (phone portrait, half the drill screen → a quick, lean, stable golden artifact;
/// the full-frame dither compresses poorly, so the proof render is kept small).
pub const W: u32 = 540;
pub const H: u32 = 800;
/// Fixed RNG seed so the burst — and therefore the golden — is reproducible.
pub const FX_SEED: u32 = 0x6c1d_9e37;

/// One screen-space spark (a small bright quad the palette maps to a pale-gold fleck).
#[derive(Clone, Copy, Debug)]
pub struct Spark {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub rgba: [f32; 4],
}

/// Spawn a gold burst at (`anchor_x`, `anchor_y`) and run the engine particle system for a fixed
/// slice (30 steps — caught mid-arc), returning the live sparks in screen space. `pos_scale` maps
/// world units → px (the shower radius) and `size_scale` sizes each fleck. Deterministic in
/// `seed` — this is the variant the golden captures.
pub fn celebrate(
    anchor_x: f32,
    anchor_y: f32,
    pos_scale: f32,
    size_scale: f32,
    seed: u32,
) -> Vec<Spark> {
    celebrate_steps(anchor_x, anchor_y, pos_scale, size_scale, seed, 30)
}

/// As [`celebrate`], but simulate `steps` of the engine particle system (1/120 s each) so the
/// live app can *animate* the burst by feeding elapsed time → step count.
pub fn celebrate_steps(
    anchor_x: f32,
    anchor_y: f32,
    pos_scale: f32,
    size_scale: f32,
    seed: u32,
    steps: u32,
) -> Vec<Spark> {
    let mut sys = ParticleSystem::new(vec![]); // no auto-emitters: we seed the burst ourselves
    let mut rng = seed | 1;
    let mut rand = || {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        (rng >> 8) as f32 / (1u32 << 24) as f32
    };
    // A radial pop (world units): up-biased, varied speed/life/size — gravity + fade come from
    // the engine system. Bright near-white so the luminance ramp lands them at the gold highlight.
    for _ in 0..64 {
        let a = rand() * std::f32::consts::TAU;
        let spread = 3.0 + rand() * 9.0;
        let up = 10.0 + rand() * 16.0;
        let vel = Vec3::new(a.cos() * spread, up, a.sin() * spread);
        let life = 0.5 + rand() * 0.45;
        let size = 0.55 + rand() * 0.6;
        let warm = 0.85 + rand() * 0.15;
        sys.spawn(Vec3::ZERO, vel, Vec3::new(1.0, warm, 0.7), life, size);
    }
    let dt = 1.0 / 120.0;
    for _ in 0..steps {
        sys.update(dt);
    }
    // Position spread reads as a wide shower; spark *size* is its own (smaller) scale so flecks
    // stay crisp rather than chunky.
    sys.instances()
        .iter()
        .map(|p| Spark {
            x: anchor_x + p.offset[0] * pos_scale,
            y: anchor_y - p.offset[1] * pos_scale, // world y is up; screen y is down
            size: (p.size * size_scale).max(5.0),
            rgba: [p.color[0], p.color[1], p.color[2], 1.0],
        })
        .collect()
}

/// Which frame to render — the real FX, or a deliberately-broken variant for "test the test".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// The real correct-answer flourish (this is what the golden captures).
    Correct,
    /// Regression: the burst failed to fire (no sparks).
    NoSparks,
    /// Regression: the palette recipe collapsed to a single flat tone.
    BrokenPalette,
}

fn centered(atlas: &Atlas, text: &str, cx: f32, top: f32, h: f32) -> Vec<crate::text::Quad> {
    let w = atlas.text_width(text);
    atlas
        .layout(
            text,
            cx - w / 2.0,
            top + h / 2.0 - 0.59 * atlas.px,
            f32::INFINITY,
        )
        .0
}

/// Render the correct-answer FX moment for `variant` and return the readback RGBA. Both the
/// blesser bin and the golden test call THIS, so a blessed golden and a re-render are produced by
/// exactly the same path (only `variant` differs).
pub fn render_fx_moment(painter: &Painter, font: &FontRef<'_>, variant: Variant) -> Vec<u8> {
    let bg = [20.0 / 255.0, 12.0 / 255.0, 34.0 / 255.0];
    let cx = W as f32 / 2.0;
    let col_w = W as f32 - 64.0; // 32-px margins

    let a_head = Atlas::bake(font, 44.0);
    let a_q = Atlas::bake(font, 42.0);
    let a_body = Atlas::bake(font, 32.0);

    let gold = [1.0, 0.84, 0.43, 1.0];
    let panel = [34.0 / 255.0, 22.0 / 255.0, 54.0 / 255.0, 1.0];
    let bright = [0.95, 0.98, 1.0, 1.0]; // high-luma → maps to the ramp highlight after palettise

    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();

    // Heading.
    let (q, _h) = a_head.layout("Halves", 32.0, 35.0, col_w);
    texts.push(TextRun {
        atlas: &a_head,
        quads: q,
        rgba: gold,
    });

    // Question card.
    let (cy, ch) = (150.0, 130.0);
    rects.push(RectRun {
        x: 32.0,
        y: cy,
        w: col_w,
        h: ch,
        rgba: [gold[0], gold[1], gold[2], 0.5],
    });
    rects.push(RectRun {
        x: 34.0,
        y: cy + 2.0,
        w: col_w - 4.0,
        h: ch - 4.0,
        rgba: panel,
    });
    texts.push(TextRun {
        atlas: &a_q,
        quads: centered(&a_q, "Half of 100", cx, cy, ch),
        rgba: gold,
    });

    // The answer, bright (so the burst + answer both bloom to the ramp highlight).
    let (ay, ah) = (320.0, 75.0);
    texts.push(TextRun {
        atlas: &a_q,
        quads: centered(&a_q, "50", cx, ay, ah),
        rgba: bright,
    });

    // The spark burst around the answer (unless the regression suppresses it).
    if variant != Variant::NoSparks {
        for s in celebrate(cx, ay + ah * 0.4, 26.0, 6.5, FX_SEED) {
            rects.push(RectRun {
                x: s.x - s.size / 2.0,
                y: s.y - s.size / 2.0,
                w: s.size,
                h: s.size,
                rgba: s.rgba,
            });
        }
    }

    // Banner.
    let (q, _h) = a_body.layout(
        "Correct!",
        cx - a_body.text_width("Correct!") / 2.0,
        490.0,
        col_w,
    );
    texts.push(TextRun {
        atlas: &a_body,
        quads: q,
        rgba: bright,
    });

    // Palette dither (the engine recipe). The broken variant collapses the ramp to one flat tone.
    let (ramp, count): (&[[f32; 3]], u32) = match variant {
        Variant::BrokenPalette => (&GOLD_RAMP[..1], 1),
        _ => (&GOLD_RAMP, GOLD_RAMP.len() as u32),
    };
    painter.paint_palettized(W, H, bg, &rects, &texts, ramp, count, FX_DITHER, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celebrate_is_deterministic_and_nonempty() {
        let a = celebrate(540.0, 760.0, 52.0, 13.0, FX_SEED);
        let b = celebrate(540.0, 760.0, 52.0, 13.0, FX_SEED);
        assert!(!a.is_empty(), "the burst must produce sparks");
        assert_eq!(a.len(), b.len(), "same seed → same spark count");
        for (p, q) in a.iter().zip(&b) {
            assert_eq!(
                p.x.to_bits(),
                q.x.to_bits(),
                "same seed → identical positions"
            );
            assert_eq!(p.y.to_bits(), q.y.to_bits());
        }
    }

    #[test]
    fn a_different_seed_gives_a_different_burst() {
        let a = celebrate(540.0, 760.0, 52.0, 13.0, FX_SEED);
        let b = celebrate(540.0, 760.0, 52.0, 13.0, FX_SEED ^ 0x1234_5678);
        // Not identical position-for-position (overwhelmingly likely with 52 particles).
        let same = a
            .iter()
            .zip(&b)
            .all(|(p, q)| p.x.to_bits() == q.x.to_bits());
        assert!(!same, "a different seed should reshape the burst");
    }

    #[test]
    fn sparks_land_near_the_anchor() {
        // The fixed sim slice keeps the burst a sane radius from the anchor (a sanity bound,
        // not a golden) — so the FX moment composites on-screen.
        let (ax, ay) = (540.0, 760.0);
        for s in celebrate(ax, ay, 52.0, 13.0, FX_SEED) {
            assert!(
                (s.x - ax).abs() < 420.0,
                "spark x {} too far from {ax}",
                s.x
            );
            assert!(
                (s.y - ay).abs() < 420.0,
                "spark y {} too far from {ay}",
                s.y
            );
            assert!(s.size >= 4.0);
        }
    }
}
