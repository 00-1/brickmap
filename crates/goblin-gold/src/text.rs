//! Goblin Gold — the legible **text core** (BRICKMAP-GG1 spike, mini-gate #1).
//!
//! The research flagged text as the **#1 blocker**: brickmap's only text today is the
//! `font8x8` 8×8 bitmap (great for HUD numbers, marginal for the guide/explain *prose* a
//! math-drills game leans on). This module is the prototype of the replacement: bake a
//! TrueType face to a **grayscale coverage atlas** (anti-aliased at the target pixel size —
//! the crispest path for small reading text) and lay out **word-wrapped paragraphs**.
//!
//! It is deliberately split CPU-side: bake → atlas bytes + per-glyph metrics; layout →
//! positioned quads. The GPU side just uploads the atlas as an `R8` texture and draws the
//! quads (the same RGBA→texture pattern `bm-render::hud` already uses) — so at 1:1 the
//! on-screen pixels equal this coverage exactly. Reusable engine work; later it sinks into
//! `bm-render` as the engine's text service.

use ab_glyph::{Font, FontRef, Glyph, PxScale, ScaleFont};
use std::collections::HashMap;

/// Per-glyph slot in the atlas (UVs in 0..1) + placement metrics (px).
#[derive(Clone, Copy)]
pub struct GlyphInfo {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub w: u32,
    pub h: u32,
    pub bearing_x: f32, // pen → glyph-bitmap left edge
    pub top: f32,       // baseline → glyph-bitmap top edge (negative = above baseline)
    pub advance: f32,   // horizontal pen advance
}

/// A baked grayscale coverage atlas for one face at one pixel size.
pub struct Atlas {
    pub px: f32,
    pub width: u32,
    pub height: u32,
    pub coverage: Vec<u8>, // width*height, 0..=255 alpha coverage
    pub glyphs: HashMap<char, GlyphInfo>,
    pub ascent: f32,
    pub descent: f32,
    pub line_height: f32,
    pub space_advance: f32,
}

/// One positioned, textured quad (pixel coords, top-left origin) ready for the GPU.
#[derive(Clone, Copy)]
pub struct Quad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl Atlas {
    /// Bake the printable ASCII range of `font` at `px` into a single coverage atlas.
    pub fn bake(font: &FontRef<'_>, px: f32) -> Atlas {
        let scaled = font.as_scaled(PxScale::from(px));
        let ascent = scaled.ascent();
        let descent = scaled.descent();
        let line_height = ascent - descent + scaled.line_gap();
        let space_advance = scaled.h_advance(font.glyph_id(' '));

        // Pass 1 — rasterise every glyph's coverage bitmap + record metrics.
        struct Raw {
            ch: char,
            w: u32,
            h: u32,
            bearing_x: f32,
            top: f32,
            advance: f32,
            cov: Vec<u8>,
        }
        let mut raws: Vec<Raw> = Vec::new();
        for code in 0x20u8..0x7f {
            let ch = code as char;
            let advance = scaled.h_advance(font.glyph_id(ch));
            let glyph: Glyph = font.glyph_id(ch).with_scale(px);
            if let Some(outlined) = font.outline_glyph(glyph) {
                let b = outlined.px_bounds();
                let w = b.width().ceil().max(0.0) as u32;
                let h = b.height().ceil().max(0.0) as u32;
                let mut cov = vec![0u8; (w * h) as usize];
                outlined.draw(|gx, gy, c| {
                    if gx < w && gy < h {
                        cov[(gy * w + gx) as usize] = (c * 255.0 + 0.5) as u8;
                    }
                });
                raws.push(Raw {
                    ch,
                    w,
                    h,
                    bearing_x: b.min.x,
                    top: b.min.y,
                    advance,
                    cov,
                });
            } else {
                // whitespace / no outline — advance only
                raws.push(Raw {
                    ch,
                    w: 0,
                    h: 0,
                    bearing_x: 0.0,
                    top: 0.0,
                    advance,
                    cov: Vec::new(),
                });
            }
        }

        // Pass 2 — shelf-pack into a fixed-width atlas (1px gutter), grow height as needed.
        const AW: u32 = 512;
        const PAD: u32 = 1;
        let (mut cx, mut cy, mut row_h) = (PAD, PAD, 0u32);
        let mut placed: Vec<(usize, u32, u32)> = Vec::with_capacity(raws.len()); // (idx, x, y)
        for (i, r) in raws.iter().enumerate() {
            if r.w == 0 || r.h == 0 {
                continue;
            }
            if cx + r.w + PAD > AW {
                cx = PAD;
                cy += row_h + PAD;
                row_h = 0;
            }
            placed.push((i, cx, cy));
            cx += r.w + PAD;
            row_h = row_h.max(r.h);
        }
        let ah = (cy + row_h + PAD).max(1);
        let mut coverage = vec![0u8; (AW * ah) as usize];
        let mut glyphs: HashMap<char, GlyphInfo> = HashMap::new();
        // whitespace entries (advance-only) still need a record
        for r in &raws {
            if r.w == 0 || r.h == 0 {
                glyphs.insert(
                    r.ch,
                    GlyphInfo {
                        u0: 0.0,
                        v0: 0.0,
                        u1: 0.0,
                        v1: 0.0,
                        w: 0,
                        h: 0,
                        bearing_x: r.bearing_x,
                        top: r.top,
                        advance: r.advance,
                    },
                );
            }
        }
        for (i, x, y) in placed {
            let r = &raws[i];
            for gy in 0..r.h {
                for gx in 0..r.w {
                    coverage[((y + gy) * AW + (x + gx)) as usize] = r.cov[(gy * r.w + gx) as usize];
                }
            }
            glyphs.insert(
                r.ch,
                GlyphInfo {
                    u0: x as f32 / AW as f32,
                    v0: y as f32 / ah as f32,
                    u1: (x + r.w) as f32 / AW as f32,
                    v1: (y + r.h) as f32 / ah as f32,
                    w: r.w,
                    h: r.h,
                    bearing_x: r.bearing_x,
                    top: r.top,
                    advance: r.advance,
                },
            );
        }

        Atlas {
            px,
            width: AW,
            height: ah,
            coverage,
            glyphs,
            ascent,
            descent,
            line_height,
            space_advance,
        }
    }

    /// Width (px) of a single word (no wrapping) — used by the wrapper.
    fn word_width(&self, word: &str) -> f32 {
        word.chars()
            .map(|c| {
                self.glyphs
                    .get(&c)
                    .map(|g| g.advance)
                    .unwrap_or(self.space_advance)
            })
            .sum()
    }

    /// Lay out `text` into positioned quads, word-wrapped to `max_w` px, starting with the
    /// baseline of the first line at (`x0`, `y0 + ascent`). Returns the quads + the total
    /// height consumed (px) so callers can stack blocks.
    pub fn layout(&self, text: &str, x0: f32, y0: f32, max_w: f32) -> (Vec<Quad>, f32) {
        let mut quads = Vec::new();
        let mut baseline = y0 + self.ascent;
        let mut pen_x = x0;
        let space = self.space_advance;

        let emit_word = |quads: &mut Vec<Quad>, pen_x: &mut f32, baseline: f32, word: &str| {
            for c in word.chars() {
                if let Some(g) = self.glyphs.get(&c) {
                    if g.w > 0 && g.h > 0 {
                        quads.push(Quad {
                            x: *pen_x + g.bearing_x,
                            y: baseline + g.top,
                            w: g.w as f32,
                            h: g.h as f32,
                            u0: g.u0,
                            v0: g.v0,
                            u1: g.u1,
                            v1: g.v1,
                        });
                    }
                    *pen_x += g.advance;
                } else {
                    *pen_x += space;
                }
            }
        };

        for (li, line) in text.split('\n').enumerate() {
            if li > 0 {
                baseline += self.line_height;
                pen_x = x0;
            }
            let mut first = true;
            for word in line.split(' ') {
                if word.is_empty() {
                    pen_x += space;
                    continue;
                }
                let ww = self.word_width(word);
                if !first && pen_x + ww > x0 + max_w {
                    baseline += self.line_height;
                    pen_x = x0;
                    first = true;
                }
                if !first {
                    pen_x += space;
                }
                emit_word(&mut quads, &mut pen_x, baseline, word);
                first = false;
            }
        }
        let total_h = (baseline - self.ascent) - y0 + self.line_height;
        (quads, total_h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
