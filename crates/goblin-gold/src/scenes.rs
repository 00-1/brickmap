//! GG1 procedural **backdrops + banners** (F3+F4) — the two deterministic full-colour 2-D
//! generators behind the Arena region scenery and the daily-event emblem crests, re-implemented from
//! `scenery.js` (`buildGrid`) + `eventart.js` (`buildGrid`) and **proven byte-identical vs
//! `scenes-vectors.json`**. Pure logic; the screens paint these colour grids.
//!
//! - **F3 scenery** ([`scenery_grid`]) — a 28×11 PRE-SCRIM backdrop per Arena region (0..9): a
//!   per-row sky gradient (`lerp` of the theme's `[top,bot]`), a themed silhouette (`topRow` shape
//!   from the horizon down), and a few seeded accents (stars/embers/snow). Seed
//!   `mulberry32((region+1)·2654435761)`. (The live `draw` adds a 0.64 scrim on top; this is the
//!   pre-scrim grid the vectors hold.)
//! - **F4 eventart** ([`eventart_grid`]) — a 24×16 emblem per event seeded by its `artSeed`: a
//!   hue-seeded HSL sky gradient + a centred edge-lit diamond crest (Manhattan `|c−cx|+|r−cy|·1.35 ≤
//!   R`) + a seeded mirror-symmetric rune + a few sparks. Seed `mulberry32((artSeed)||1)`.

use crate::synth::Rng;

const SCN_COLS: usize = 28;
const SCN_ROWS: usize = 11;
const EVT_COLS: usize = 24;
const EVT_ROWS: usize = 16;

/// A full-colour grid: `rows × cols` of lowercase `#rrggbb` hex strings.
pub type ColorGrid = Vec<Vec<String>>;

// ── shared colour helpers (scenery.js / eventart.js hx/toHex/lerp) ────────────────────────────────

fn hx(c: &str) -> [f64; 3] {
    let h = c.trim_start_matches('#');
    let v = |i: usize| i64::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f64;
    [v(0), v(2), v(4)]
}

/// JS `Math.round` (half up).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn to_hex(rgb: [f64; 3]) -> String {
    let c = |v: f64| js_round(v).clamp(0.0, 255.0) as u32;
    format!("#{:02x}{:02x}{:02x}", c(rgb[0]), c(rgb[1]), c(rgb[2]))
}

fn lerp(a: &str, b: &str, t: f64) -> String {
    let (a, b) = (hx(a), hx(b));
    to_hex([
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ])
}

/// `eventart.js` HSL → hex (`h` 0..360, `s`/`l` 0..100), the "k-formula".
fn hsl(h: f64, s: f64, l: f64) -> String {
    let s = s / 100.0;
    let l = l / 100.0;
    let k = |n: f64| (n + h / 30.0) % 12.0;
    let a = s * l.min(1.0 - l);
    let f = |n: f64| l - a * (-1.0f64).max((k(n) - 3.0).min((9.0 - k(n)).min(1.0)));
    to_hex([255.0 * f(0.0), 255.0 * f(8.0), 255.0 * f(4.0)])
}

// ── F3: Arena region scenery (scenery.js buildGrid) ───────────────────────────────────────────────

/// One themed region: sky `[top, bot]`, silhouette colour, shape, and optional accent kind.
struct Theme {
    sky: [&'static str; 2],
    sil: &'static str,
    shape: &'static str,
    accent: Option<&'static str>,
}

/// The 10 region themes (`scenery.js THEMES`).
fn themes() -> [Theme; 10] {
    let t = |sky0, sky1, sil, shape, accent| Theme {
        sky: [sky0, sky1],
        sil,
        shape,
        accent,
    };
    [
        t("#1c2614", "#32401e", "#101808", "bumps", None),
        t("#1e1e28", "#32323c", "#0c0c12", "posts", None),
        t("#14202a", "#243038", "#0a1216", "trees", None),
        t("#142219", "#243528", "#0a120c", "reeds", None),
        t("#1e2e3c", "#3a4e60", "#14202c", "peaks", Some("snow")),
        t("#0e1a26", "#1a2e42", "#08101a", "spires", None),
        t("#261006", "#46220e", "#180a04", "bumps", Some("embers")),
        t("#1a1824", "#302c40", "#0c0a14", "posts", None),
        t("#220e10", "#441e18", "#120606", "crags", Some("embers")),
        t("#08060f", "#150b22", "#050308", "spires", Some("stars")),
    ]
}

/// Top silhouette row for column `c` (smaller = taller), by shape — deterministic (no RNG).
fn top_row(shape: &str, c: i64, horizon: i64) -> i64 {
    let span = (SCN_ROWS as i64 - horizon) as f64;
    match shape {
        "bumps" => horizon - js_round((c as f64 * 0.5 + 1.0).sin().abs() * (span * 0.45)) as i64,
        "crags" | "peaks" => {
            let p = c % 6;
            horizon - js_round((3.0 - (p - 3).abs() as f64) / 3.0 * (span * 0.9)) as i64
        }
        "posts" => {
            if c % 7 == 2 {
                horizon - js_round(span * 1.1) as i64
            } else {
                horizon
            }
        }
        "trees" => {
            let m = c % 5;
            if m < 3 {
                horizon - js_round(span * 0.8) as i64 - if m == 1 { 1 } else { 0 }
            } else {
                horizon
            }
        }
        "reeds" => {
            if c % 3 == 0 {
                horizon - js_round(span * 0.7) as i64
            } else {
                horizon
            }
        }
        "spires" => {
            let m = c % 5;
            if m == 2 {
                horizon - js_round(span * 1.2) as i64
            } else if m == 1 || m == 3 {
                horizon - js_round(span * 0.4) as i64
            } else {
                horizon
            }
        }
        _ => horizon,
    }
}

/// Build the pre-scrim 28×11 scenery backdrop for `region` (`scenery.js buildGrid`).
pub fn scenery_grid(region: i64) -> ColorGrid {
    let th = &themes()[region.rem_euclid(10) as usize];
    let mut rnd = Rng::new(((region as u32).wrapping_add(1)).wrapping_mul(2_654_435_761));
    let horizon = js_round(SCN_ROWS as f64 * 0.58) as i64;
    let mut g: ColorGrid = vec![vec![String::new(); SCN_COLS]; SCN_ROWS];
    // sky gradient (per row).
    for (r, row) in g.iter_mut().enumerate() {
        let col = lerp(th.sky[0], th.sky[1], r as f64 / (SCN_ROWS as f64 - 1.0));
        for cell in row.iter_mut() {
            *cell = col.clone();
        }
    }
    // silhouette.
    for c in 0..SCN_COLS {
        let t = top_row(th.shape, c as i64, horizon).max(0) as usize;
        for row in g.iter_mut().take(SCN_ROWS).skip(t) {
            row[c] = th.sil.to_string();
        }
    }
    // accents above the horizon.
    if let Some(kind) = th.accent {
        let col = match kind {
            "embers" => "#5a2e16",
            "snow" => "#424a56",
            _ => "#42445a",
        };
        let count = 6 + (rnd.next() * 5.0).floor() as usize;
        for _ in 0..count {
            let c = (rnd.next() * SCN_COLS as f64).floor() as usize;
            let r = (rnd.next() * horizon as f64).floor() as usize;
            g[r][c] = col.to_string();
        }
    }
    g
}

// ── F4: per-event emblem crests (eventart.js buildGrid) ───────────────────────────────────────────

/// Build the 24×16 emblem crest for an event `art_seed` (`eventart.js buildGrid`).
pub fn eventart_grid(art_seed: u32) -> ColorGrid {
    let mut rnd = Rng::new(if art_seed == 0 { 1 } else { art_seed });
    let hue = (rnd.next() * 360.0).floor();
    let sky_top = hsl(hue, 32.0, 9.0);
    let sky_bot = hsl(
        (hue + 18.0 + (rnd.next() * 44.0).floor()) % 360.0,
        40.0,
        16.0,
    );
    let crest = hsl(hue, 46.0, 24.0);
    let edge = hsl(hue, 58.0, 42.0);
    let accent = hsl(
        (hue + 38.0 + (rnd.next() * 90.0).floor()) % 360.0,
        76.0,
        58.0,
    );
    let spark = hsl(
        (hue + 170.0 + (rnd.next() * 40.0).floor()) % 360.0,
        70.0,
        62.0,
    );

    let mut g: ColorGrid = vec![vec![String::new(); EVT_COLS]; EVT_ROWS];
    for (r, row) in g.iter_mut().enumerate() {
        let col = lerp(&sky_top, &sky_bot, r as f64 / (EVT_ROWS as f64 - 1.0));
        for cell in row.iter_mut() {
            *cell = col.clone();
        }
    }

    let cx = (EVT_COLS as f64 - 1.0) / 2.0;
    let cy = (EVT_ROWS as f64 - 1.0) / 2.0;
    let big_r = 6.0 + (rnd.next() * 2.0).floor();
    let man = |c: usize, r: usize| (c as f64 - cx).abs() + (r as f64 - cy).abs() * 1.35;
    // crest body + edge (a tall diamond).
    for (r, row) in g.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            let m = man(c, r);
            if m <= big_r {
                *cell = if m > big_r - 1.25 {
                    edge.clone()
                } else {
                    crest.clone()
                };
            }
        }
    }
    // seeded rune on the crest, mirrored left→right (rnd only where on the crest).
    let fill = 0.30 + rnd.next() * 0.22;
    let half = (cx.floor()) as usize;
    for (r, row) in g.iter_mut().enumerate() {
        for c in 0..=half {
            if man(c, r) <= big_r - 1.6 && rnd.next() < fill {
                row[c] = accent.clone();
                row[EVT_COLS - 1 - c] = accent.clone();
            }
        }
    }
    // accent sparks above the horizon, off the crest.
    let n = 5 + (rnd.next() * 5.0).floor() as usize;
    for _ in 0..n {
        let c = (rnd.next() * EVT_COLS as f64).floor() as usize;
        let r = (rnd.next() * (EVT_ROWS as f64 * 0.5)).floor() as usize;
        if man(c, r) > big_r {
            g[r][c] = spark.clone();
        }
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const VECTORS_JSON: &str = include_str!("../data/gg1/scenes-vectors.json");

    fn vectors() -> Value {
        serde_json::from_str(VECTORS_JSON).expect("scenes-vectors.json")
    }

    /// Reconstruct the expected colour grid from a palette-packed vector (`pal` + base-36 index rows).
    fn unpack(v: &Value) -> ColorGrid {
        let pal: Vec<String> = v["pal"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                row.as_str()
                    .unwrap()
                    .chars()
                    .map(|ch| pal[ch.to_digit(36).unwrap() as usize].clone())
                    .collect()
            })
            .collect()
    }

    /// F3: all 10 region backdrops reproduce the exact pre-scrim colour grid.
    #[test]
    fn scenery_matches_vectors() {
        for s in vectors()["scenery"].as_array().unwrap() {
            let region = s["region"].as_i64().unwrap();
            assert_eq!(
                scenery_grid(region),
                unpack(s),
                "scenery region {region} ({})",
                s["label"].as_str().unwrap_or("")
            );
        }
    }

    /// F4: all 14 event emblems reproduce the exact colour grid (HSL gradient + crest + rune + sparks).
    #[test]
    fn eventart_matches_vectors() {
        for e in vectors()["eventArt"].as_array().unwrap() {
            let seed = e["artSeed"].as_u64().unwrap() as u32;
            assert_eq!(
                eventart_grid(seed),
                unpack(e),
                "eventart {} (seed {seed})",
                e["event"].as_str().unwrap_or("")
            );
        }
    }

    /// Sanity: grids are the documented sizes.
    #[test]
    fn grids_are_the_right_size() {
        let s = scenery_grid(0);
        assert_eq!(s.len(), SCN_ROWS);
        assert_eq!(s[0].len(), SCN_COLS);
        let e = eventart_grid(1071);
        assert_eq!(e.len(), EVT_ROWS);
        assert_eq!(e[0].len(), EVT_COLS);
    }
}
