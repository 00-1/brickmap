//! Goblin Gold text — the legible 2-D prose path is now an **engine service**
//! ([`brickmap::text2d`], banked into `bm-render` in full-port phase 1); this module re-exports it
//! so the game's call sites are unchanged. The **font face is game content**, so the bake/layout
//! tests live here (where the embedded Instrument Sans asset is) and exercise the engine path.

pub use brickmap::text2d::*;

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::FontRef;
    const FONT: &[u8] = include_bytes!("../assets/InstrumentSans-Regular.ttf");

    #[test]
    fn bakes_a_nonempty_aa_atlas_with_grays() {
        let font = FontRef::try_from_slice(FONT).expect("font");
        let atlas = Atlas::bake(&font, 40.0);
        assert!(atlas.width > 0 && atlas.height > 0);
        // letters present with real bitmaps
        let a = atlas.glyphs.get(&'a').expect("'a'");
        assert!(a.w > 0 && a.h > 0 && a.advance > 0.0);
        // anti-aliased: coverage must contain mid-gray values, not just 0/255
        let mids = atlas
            .coverage
            .iter()
            .filter(|&&v| v > 20 && v < 235)
            .count();
        assert!(mids > 50, "expected AA mid-tones, got {mids}");
        assert!(atlas.line_height > atlas.px * 0.8);
    }

    #[test]
    fn layout_wraps_long_prose_to_multiple_lines() {
        let font = FontRef::try_from_slice(FONT).expect("font");
        let atlas = Atlas::bake(&font, 32.0);
        let prose = "Halving means splitting a number into two equal parts that add back up to it.";
        let (wide, h_wide) = atlas.layout(prose, 0.0, 0.0, 4000.0); // one line
        let (narrow, h_narrow) = atlas.layout(prose, 0.0, 0.0, 300.0); // must wrap
        assert!(!wide.is_empty() && !narrow.is_empty());
        assert!(
            h_narrow > h_wide,
            "narrow column should be taller (more wrapped lines)"
        );
    }
}
