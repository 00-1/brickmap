//! Topic **pixel-glyphs** — a faithful port of `glyphs.js` (the T56 chunky bitmap-font) plus the
//! `TOPIC_GLYPHS` token map from `modes.js`. Each topic's glyph is a tiny operator mark (`×/2`,
//! `a×b`, `+9`, …) drawn from a 5×7 BIG cell-font (operands + operators) with 3×4 SMALL cells for
//! fraction numerators/denominators and superscripts. Pure + deterministic: [`build_grid`] returns
//! a `{w,h,cells}` grid of ink codes (0 empty · 1 operand body · 2 operator accent); the Home tree
//! paints the cells as little squares (body `#E6E9EF`, accent `#F5B544`).
//!
//! This is presentation data + a render recipe (like `fxgl.js seedHoard`), ported straight from the
//! web source rather than routed through a content export.

/// Grid height (big chars sit on rows 1–7; fractions use 0–8).
pub const H: usize = 9;
/// Blank column between adjacent tokens.
const GAP: usize = 1;

/// A built glyph grid: `cells[y*w + x]` ∈ {0 empty, 1 body, 2 accent}.
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<u8>,
}

/// The 5×7 BIG cell-font — exactly the symbols the glyphs use (0–9, x a b n k, × ÷ + − ± / %).
fn big(ch: &str) -> Option<[&'static str; 7]> {
    Some(match ch {
        "0" => [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
        "1" => [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        "2" => [
            ".###.", "#...#", "....#", "..##.", ".#...", "#....", "#####",
        ],
        "3" => [
            "#####", "...#.", "..##.", "....#", "....#", "#...#", ".###.",
        ],
        "4" => [
            "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
        ],
        "5" => [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
        "6" => [
            "..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###.",
        ],
        "7" => [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
        "8" => [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
        "9" => [
            ".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##..",
        ],
        "x" => [
            ".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#",
        ],
        "a" => [
            ".....", ".....", ".###.", "....#", ".####", "#...#", ".####",
        ],
        "b" => [
            "#....", "#....", "####.", "#...#", "#...#", "#...#", "####.",
        ],
        "n" => [
            ".....", ".....", "####.", "#...#", "#...#", "#...#", "#...#",
        ],
        "k" => [
            "#....", "#....", "#..#.", "#.#..", "##...", "#.#..", "#..#.",
        ],
        "×" => [
            ".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#",
        ],
        "÷" => [
            ".....", "..#..", ".....", "#####", ".....", "..#..", ".....",
        ],
        "+" => [
            ".....", "..#..", "..#..", "#####", "..#..", "..#..", ".....",
        ],
        "−" => [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
        "±" => [
            "..#..", "..#..", "#####", "..#..", "..#..", ".....", "#####",
        ],
        "/" => [
            "....#", "...#.", "..#..", "..#..", ".#...", ".#...", "#....",
        ],
        "%" => [
            "##..#", "##.#.", "...#.", "..#..", ".#...", ".#.##", "#..##",
        ],
        _ => return None,
    })
}

/// The 3×4 SMALL cell-font — fraction numerator/denominator + superscripts (1–4).
fn small(ch: char) -> Option<[&'static str; 4]> {
    Some(match ch {
        '1' => [".#.", ".#.", ".#.", ".#."],
        '2' => ["##.", "..#", ".#.", "###"],
        '3' => ["##.", "..#", "..#", "##."],
        '4' => ["#.#", "#.#", "###", "..#"],
        _ => return None,
    })
}

enum Tok {
    Char { ch: String, ink: u8 },
    Sup { ch: char, ink: u8 },
    Frac { num: char, den: char, ink: u8 },
}

impl Tok {
    /// Token cell-width (matches `glyphs.js parse`: char 5 · sup 3 · frac 13).
    fn width(&self) -> usize {
        match self {
            Tok::Char { .. } => 5,
            Tok::Sup { .. } => 3,
            Tok::Frac { .. } => 13,
        }
    }
}

/// Parse one token (leading `*` = accent ink; `f<n><d>` = fraction; `s<c>` = superscript; else char).
fn parse(s: &str) -> Tok {
    let (ink, body) = if let Some(rest) = s.strip_prefix('*') {
        (2, rest)
    } else {
        (1, s)
    };
    let first = body.chars().next().unwrap_or(' ');
    if first == 'f' {
        let mut it = body.chars();
        it.next();
        let num = it.next().unwrap_or('0');
        let den = it.next().unwrap_or('0');
        Tok::Frac { num, den, ink }
    } else if first == 's' {
        let ch = body.chars().nth(1).unwrap_or('0');
        Tok::Sup { ch, ink }
    } else {
        Tok::Char {
            ch: body.to_string(),
            ink,
        }
    }
}

fn set(cells: &mut [u8], w: usize, x: usize, y: usize, ink: u8) {
    cells[y * w + x] = ink;
}

/// Stamp `BIG[key]` (7×5) at column `x0`, rows 1–7.
fn stamp_big(cells: &mut [u8], w: usize, key: &str, x0: usize, ink: u8) {
    if let Some(g) = big(key) {
        for (r, row) in g.iter().enumerate() {
            for (c, &byte) in row.as_bytes().iter().enumerate().take(5) {
                if byte == b'#' {
                    set(cells, w, x0 + c, 1 + r, ink);
                }
            }
        }
    }
}

/// Build the glyph grid for a token list (faithful to `glyphs.js buildGrid`).
pub fn build_grid(tokens: &[&str]) -> Grid {
    let toks: Vec<Tok> = tokens.iter().map(|t| parse(t)).collect();
    let mut w = 0usize;
    for (i, t) in toks.iter().enumerate() {
        w += if i > 0 { GAP } else { 0 } + t.width();
    }
    let mut cells = vec![0u8; w * H];
    let mut x = 0usize;
    for (i, t) in toks.iter().enumerate() {
        if i > 0 {
            x += GAP;
        }
        match t {
            Tok::Char { ch, ink } => stamp_big(&mut cells, w, ch, x, *ink),
            Tok::Sup { ch, ink } => {
                if let Some(g) = small(*ch) {
                    for (r, row) in g.iter().enumerate() {
                        for (c, &byte) in row.as_bytes().iter().enumerate().take(3) {
                            if byte == b'#' {
                                set(&mut cells, w, x + c, r, *ink);
                            }
                        }
                    }
                }
            }
            Tok::Frac { num, den, ink } => {
                // BIG numerator (cols 0–4), a 3-wide diagonal slash (cols 5–7, rows 0–8), BIG
                // denominator (cols 8–12) — the T124 full-size slashed fraction.
                stamp_big(&mut cells, w, &num.to_string(), x, *ink);
                const SLASH: [(usize, usize); 9] = [
                    (8, 0),
                    (7, 0),
                    (6, 1),
                    (5, 1),
                    (4, 1),
                    (3, 1),
                    (2, 1),
                    (1, 2),
                    (0, 2),
                ];
                for (yy, px) in SLASH {
                    set(&mut cells, w, x + 5 + px, yy, *ink);
                }
                stamp_big(&mut cells, w, &den.to_string(), x + 8, *ink);
            }
        }
        x += t.width();
    }
    Grid { w, h: H, cells }
}

/// The `TOPIC_GLYPHS` map from `modes.js` — each topic's glyph token list (all 46 topics).
pub fn topic_glyph(id: &str) -> &'static [&'static str] {
    match id {
        "halves" => &["x", "*/", "2"],
        "times" => &["a", "*×", "b"],
        "doubles" => &["2", "*×", "x"],
        "addsub" => &["a", "*+", "b"],
        "addsub2" => &["a", "*±", "b"],
        "bonds" => &["+", "*1", "*0", "*0"],
        "bonds2" => &["+", "*1", "*k"],
        "placevalue" => &["×", "*÷"],
        "placevalue2" => &["*×", "÷"],
        "fractionsof" => &["*f12", "n"],
        "fractionsof2" => &["a", "*/", "b"],
        "percentages" => &["*%"],
        "percentages2" => &["n", "*%"],
        "fractions" => &["*f34"],
        "fractions2" => &["*f18"],
        "squares" => &["x", "*s2"],
        "scaling" => &["a", "*×", "n"],
        "percentoff" => &["%", "*−"],
        "partwhole" => &["%", "*/", "n"],
        "balance" => &["k", "*±"],
        "lcmhcf" => &["n", "*÷", "k"],
        "mean" => &["*+", "÷", "n"],
        "timegap" => &["n", "*−", "k"],
        "ratioshare" => &["a", "*÷", "b"],
        "cubes" => &["x", "*s3"],
        "money" => &["a", "*×", "k"],
        "digitsum" => &["*+", "9"],
        "rounding" => &["n", "*0"],
        "largermd" => &["*×", "*÷"],
        "metric" => &["a", "*/", "k"],
        "sequences" => &["n", "*+", "k"],
        "sequences2" => &["n", "*×", "k"],
        "roman" => &["*x"],
        "primes" => &["*1", "n"],
        "pctup" => &["*+", "%"],
        "fdp" => &["f12", "*%"],
        "bodmas" => &["*×", "+"],
        "algebra" => &["n", "*±", "k"],
        "xtricks" => &["*×", "k"],
        "negatives" => &["*−", "n"],
        "area" => &["k", "*×", "b"],
        "volume" => &["b", "*×", "x"],
        "angles" => &["n", "*−", "b"],
        "mmr" => &["a", "*−", "b"],
        "sdt" => &["a", "*÷", "n"],
        "factors" => &["n", "*÷", "b"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_has_a_renderable_glyph() {
        for m in crate::progression::modes() {
            let toks = topic_glyph(&m.id);
            assert!(!toks.is_empty(), "{} has no glyph tokens", m.id);
            let g = build_grid(toks);
            assert_eq!(g.h, H);
            assert!(
                g.w > 0 && g.cells.iter().any(|&v| v != 0),
                "{} empty grid",
                m.id
            );
            assert_eq!(g.cells.len(), g.w * g.h);
        }
    }

    #[test]
    fn ink_codes_and_accent_present() {
        // "x*/2" → halves: an operand body (1) and the accented slash (2).
        let g = build_grid(topic_glyph("halves"));
        assert!(g.cells.contains(&1), "body ink");
        assert!(g.cells.contains(&2), "accent ink");
        // Width = char(5) + gap + char(5) + gap + char(5) = 17.
        assert_eq!(g.w, 17);
    }

    #[test]
    fn fraction_token_is_thirteen_wide() {
        // A lone fraction (fractions: "*f34") spans the wide 13-cell frac box.
        let g = build_grid(&["*f34"]);
        assert_eq!(g.w, 13);
        assert!(g.cells.contains(&2));
    }
}
