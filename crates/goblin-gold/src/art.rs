//! GG1 procedural **portraits** (T-art / F1+F2) — the two deterministic 16×16 generators that draw
//! every hero portrait, item icon and Arena foe, re-implemented from `collectibles.js`
//! (`drawIcon`/`heroSprite`/`iconRoleGrid`/`iconPalette`) + `monsters.js` (`buildGrid`) and **proven
//! byte-identical against `art-vectors.json`**. Pure logic; the screens paint these grids.
//!
//! Two generators, both seeded by `mulberry32(hashStr(...))` (the same PRNG the synth uses):
//! - **F1 hero icons** ([`hero_icon`]) — `"hero:"<id>` routes to `heroSprite`: a mirrored,
//!   centre-weighted creature-blob (~30% accent). The **role grid** is `0 empty · 1 outline · 2 body
//!   · 3 accent` (shape, palette-independent); the **palette** is the type base nudged in HSL per id
//!   (`iconPalette`). (Item icons — the `ARCH` archetype path — are a later pass; the screens here
//!   need only hero portraits + foes.)
//! - **F2 foes** ([`foe_grid`]) — `buildGrid({n,name,type})`: a lumpy vertically-symmetric blob with
//!   region-biased horns / eyes / mouth / feet, bosses (`n % 12 == 0`) larger + crowned. Role grid
//!   adds `4 eye`; the palette is per-**type** (no per-id shift).

use crate::arena::Kind;

/// The grid is a fixed 16×16 (`constants.gridSize`).
pub const G: usize = 16;
/// A region is 12 tiers; the 12th is a boss (`constants.regionSize`).
const REGION_SIZE: u32 = 12;

/// A 16×16 role grid: `0 empty · 1 outline · 2 body · 3 accent · 4 eye` (eye only for foes).
pub type RoleGrid = [[u8; G]; G];

/// A resolved palette (hex strings, lowercase `#rrggbb`). `eye` is `None` for hero icons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    pub body: String,
    pub accent: String,
    pub outline: String,
    pub eye: Option<String>,
}

impl Palette {
    /// The hex colour for a role cell (`1 outline · 2 body · 3 accent · 4 eye`); `None` for empty
    /// (`0`) or a missing eye. The screens paint a role grid by mapping each cell through this.
    pub fn role_hex(&self, role: u8) -> Option<&str> {
        match role {
            1 => Some(&self.outline),
            2 => Some(&self.body),
            3 => Some(&self.accent),
            4 => self.eye.as_deref(),
            _ => None,
        }
    }
}

/// Parse a `#rrggbb` (or `#rgb`-less) hex string to linear-ish RGBA floats (0..1, alpha 1).
pub fn hex_rgba(hex: &str) -> [f32; 4] {
    let h = hex.trim_start_matches('#');
    let v = |i: usize| {
        u8::from_str_radix(h.get(i..i + 2).unwrap_or("0"), 16).unwrap_or(0) as f32 / 255.0
    };
    [v(0), v(2), v(4), 1.0]
}

/// The per-type hero base palette (`art.json` `constants.heroPal` / `main.js HERO_PAL`).
fn hero_base(kind: Kind) -> (&'static str, &'static str, &'static str) {
    match kind {
        Kind::Brawn => ("#d05a4a", "#ff8a6e", "#3a1410"),
        Kind::Arcane => ("#8a5cf6", "#cda9ff", "#1f1340"),
        Kind::Cunning => ("#3fce8c", "#8ef0bf", "#0f3324"),
    }
}

/// The per-type foe palette (`monsters.js PAL` — body/accent/outline/eye).
fn foe_base(kind: Kind) -> (&'static str, &'static str, &'static str, &'static str) {
    match kind {
        Kind::Brawn => ("#c0563f", "#e8895f", "#3a1410", "#ffe6a8"),
        Kind::Cunning => ("#369d68", "#7fe0a8", "#0f3324", "#eafff0"),
        Kind::Arcane => ("#7d54d6", "#b89bf0", "#1f1340", "#fbe6ff"),
    }
}

// ── shared HSL palette nudge (collectibles.js nudge/shiftPalette) ──────────────────────────────

fn hex_to_rgb(h: &str) -> (f64, f64, f64) {
    let h = h.trim_start_matches('#');
    let v = |i: usize| i64::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f64;
    (v(0), v(2), v(4))
}

/// JS `Math.round` (round half up, toward +∞).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    let c = |v: f64| js_round(v).clamp(0.0, 255.0) as u32;
    format!("#{:02x}{:02x}{:02x}", c(r), c(g), c(b))
}

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (r, g, b) = (r / 255.0, g / 255.0, b / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let l = (mx + mn) / 2.0;
    if mx == mn {
        return (0.0, 0.0, l);
    }
    let d = mx - mn;
    let s = if l > 0.5 {
        d / (2.0 - mx - mn)
    } else {
        d / (mx + mn)
    };
    let mut h = if mx == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h /= 6.0;
    (h * 360.0, s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let h = (((h % 360.0) + 360.0) % 360.0) / 360.0;
    if s == 0.0 {
        return (l * 255.0, l * 255.0, l * 255.0);
    }
    let h2 = |p: f64, q: f64, mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    (
        h2(p, q, h + 1.0 / 3.0) * 255.0,
        h2(p, q, h) * 255.0,
        h2(p, q, h - 1.0 / 3.0) * 255.0,
    )
}

/// Nudge a hex colour's hue by `dh` degrees and lightness by `dl` (clamped to `[0.04, 0.96]`).
fn nudge(hex: &str, dh: f64, dl: f64) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    let (h, s, l) = rgb_to_hsl(r, g, b);
    let l = (l + dl).clamp(0.04, 0.96);
    let (rr, gg, bb) = hsl_to_rgb(h + dh, s, l);
    rgb_to_hex(rr, gg, bb)
}

// ── F1: hero portraits ─────────────────────────────────────────────────────────────────────────

/// Build hero `hero_id`'s portrait (role grid + the per-id HSL-shifted palette). `kind` selects the
/// type base palette. Reproduces `iconRoleGrid("hero:"<id>,"familiar")` + `iconPalette`.
pub fn hero_icon(hero_id: &str, kind: Kind) -> (RoleGrid, Palette) {
    let seed = crate::synth::hash_str(&format!("hero:{hero_id}"));
    let mut r_pick = crate::synth::Rng::new(seed);
    let mut r_tex = crate::synth::Rng::new(seed ^ 0x9e37_79b9);

    // heroSprite: left half (x 1..=7) filled with a centre-weighted probability, mirrored.
    let mut g = [[false; G]; G];
    let mut a = [[false; G]; G];
    let mid = (G as f64 - 1.0) / 2.0; // 7.5
    for y in 1..G - 1 {
        for x in 1..=(G / 2 - 1) {
            let on = r_pick.next() < (0.62 - (y as f64 - mid).abs() / G as f64 * 0.4);
            g[y][x] = on;
            g[y][G - 1 - x] = on;
            if on && r_pick.next() < 0.3 {
                a[y][x] = true;
                a[y][G - 1 - x] = true;
            }
        }
    }
    let role = role_from(&g, &a, None);

    // iconPalette: shift the type base by (hue, lum) drawn from the texture stream.
    let (body, accent, outline) = hero_base(kind);
    let hue = (r_tex.next() * 2.0 - 1.0) * 20.0;
    let lum = (r_tex.next() * 2.0 - 1.0) * 0.08;
    let pal = Palette {
        body: nudge(body, hue, lum),
        accent: nudge(accent, hue, lum),
        outline: outline.to_string(),
        eye: None,
    };
    (role, pal)
}

/// Role grid from the body/accent (and optional eye) masks: `eye→4, accent→3, body→2, outline→1`.
fn role_from(g: &[[bool; G]; G], a: &[[bool; G]; G], e: Option<&[[bool; G]; G]>) -> RoleGrid {
    let fl = |x: isize, y: isize| -> bool {
        x >= 0
            && (x as usize) < G
            && y >= 0
            && (y as usize) < G
            && (g[y as usize][x as usize] || e.map(|e| e[y as usize][x as usize]).unwrap_or(false))
    };
    let mut role = [[0u8; G]; G];
    for y in 0..G {
        for x in 0..G {
            if e.map(|e| e[y][x]).unwrap_or(false) {
                role[y][x] = 4;
            } else if g[y][x] {
                role[y][x] = if a[y][x] { 3 } else { 2 };
            } else {
                let (xi, yi) = (x as isize, y as isize);
                if fl(xi - 1, yi) || fl(xi + 1, yi) || fl(xi, yi - 1) || fl(xi, yi + 1) {
                    role[y][x] = 1;
                }
            }
        }
    }
    role
}

// ── F2: Arena foes (monsters.js buildGrid) ───────────────────────────────────────────────────────

/// Build the foe portrait for tier `n` named `name` of `kind`. Reproduces `monsters.js buildGrid`:
/// a lumpy symmetric blob with region-biased features; bosses (`n % 12 == 0`) larger + crowned.
pub fn foe_grid(n: u32, name: &str, kind: Kind) -> (RoleGrid, Palette) {
    let region = ((n.max(1) - 1) / REGION_SIZE) as i64;
    let boss = n.is_multiple_of(REGION_SIZE);
    let seed = crate::synth::hash_str(name) ^ n.wrapping_mul(2_654_435_761);
    let mut rnd = crate::synth::Rng::new(seed);

    let mut g = [[false; G]; G];
    let mut a = [[false; G]; G];
    let mut e = [[false; G]; G];
    let set_sym = |grid: &mut [[bool; G]; G], x: isize, y: isize| {
        if (0..G as isize).contains(&x) && (0..G as isize).contains(&y) {
            grid[y as usize][x as usize] = true;
        }
        let xm = G as isize - 1 - x;
        if (0..G as isize).contains(&xm) && (0..G as isize).contains(&y) {
            grid[y as usize][xm as usize] = true;
        }
    };

    // body: a lumpy, vertically-symmetric blob; bosses are larger.
    let rx = if boss { 6.4 } else { 4.4 + rnd.next() * 1.3 };
    let ry = if boss { 6.8 } else { 5.0 + rnd.next() * 1.3 };
    let cy = if boss { 8.2 } else { 8.6 };
    let lump = 0.8 + rnd.next() * 0.35;
    let (mut top_y, mut bot_y) = (G as i64, 0i64);
    for y in 0..G {
        let dy = (y as f64 - cy) / ry;
        if dy.abs() > 1.0 {
            continue;
        }
        let frac = (((y as i64 * 73 + n as i64) % 7) as f64) / 7.0;
        let hw = rx * (1.0 - dy * dy).sqrt() * (0.82 + 0.36 * frac * lump);
        for x in 0..8i64 {
            if 7.5 - x as f64 <= hw {
                set_sym(&mut g, x as isize, y as isize);
                if (y as i64) < top_y {
                    top_y = y as i64;
                }
                if (y as i64) > bot_y {
                    bot_y = y as i64;
                }
            }
        }
    }

    // horns / antennae — region-biased (void region 9+ smooth, else 1 or 2).
    let horns = if region >= 9 {
        0
    } else if region >= 4 {
        2
    } else if rnd.next() < 0.45 {
        1
    } else {
        2
    };
    if horns == 1 {
        for k in 1..=2 + region % 2 {
            set_sym(&mut g, 7, (top_y - k) as isize);
        }
    } else if horns == 2 {
        let hx = 5 - (region % 2);
        for k in 1..=2 {
            set_sym(&mut g, hx as isize, (top_y - k) as isize);
        }
        set_sym(&mut g, hx as isize, top_y as isize);
    }

    // eyes — 1..3, more in deeper/void regions; bosses never single-eyed.
    let mut ec = 1 + (rnd.next() * if region >= 8 { 3.0 } else { 2.0 }).floor() as i64;
    if boss {
        ec = ec.max(2);
    }
    let eye_y = (top_y + 1).max(js_round(cy - ry * 0.32) as i64);
    let set_eye = |e: &mut [[bool; G]; G], x: i64, y: i64| {
        if (0..G as i64).contains(&x) && (0..G as i64).contains(&y) {
            e[y as usize][x as usize] = true;
            e[y as usize][(G as i64 - 1 - x) as usize] = true;
        }
    };
    if ec == 1 {
        if (0..G as i64).contains(&eye_y) {
            e[eye_y as usize][7] = true;
            e[eye_y as usize][8] = true;
        }
    } else if ec == 2 {
        for x in [5, 6] {
            set_eye(&mut e, x, eye_y);
        }
    } else {
        for x in [4, 5] {
            set_eye(&mut e, x, eye_y);
        }
        if eye_y >= 1 {
            e[(eye_y - 1) as usize][7] = true;
            e[(eye_y - 1) as usize][8] = true;
        }
    }

    // mouth — a row of teeth (accent), region-jittered.
    let m_y = (bot_y - 1).min(eye_y + 2 + (region % 2));
    if (0..G as i64).contains(&m_y) {
        for x in 4..=7i64 {
            if g[m_y as usize][x as usize] && (x + region) % 2 == 0 {
                a[m_y as usize][x as usize] = true;
                a[m_y as usize][(G as i64 - 1 - x) as usize] = true;
            }
        }
    }

    // body texture: symmetric spots (short-circuit: rnd only for body non-eye cells).
    for y in top_y..=bot_y {
        for x in 0..8i64 {
            let (yu, xu) = (y as usize, x as usize);
            if g[yu][xu] && !e[yu][xu] && rnd.next() < 0.11 + region as f64 * 0.006 {
                a[yu][xu] = true;
                a[yu][(G as i64 - 1 - x) as usize] = true;
            }
        }
    }

    // feet / tentacle stubs at the base.
    if bot_y + 1 < G as i64 {
        let fx: &[i64] = if region >= 9 { &[4, 6] } else { &[5] };
        for &x in fx {
            set_sym(&mut g, x as isize, (bot_y + 1) as isize);
        }
    }

    // boss crown: a notched band above the head (accent) + a taller frame.
    if boss {
        let cy_top = (top_y - 1).max(0);
        for x in [4i64, 6, 7] {
            set_sym(&mut g, x as isize, cy_top as isize);
            a[cy_top as usize][x as usize] = true;
            a[cy_top as usize][(G as i64 - 1 - x) as usize] = true;
        }
        for x in [4i64, 7] {
            set_sym(&mut g, x as isize, (cy_top - 1).max(0) as isize);
        }
    }

    let role = role_from(&g, &a, Some(&e));
    let (body, accent, outline, eye) = foe_base(kind);
    let pal = Palette {
        body: body.to_string(),
        accent: accent.to_string(),
        outline: outline.to_string(),
        eye: Some(eye.to_string()),
    };
    (role, pal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const VECTORS_JSON: &str = include_str!("../data/gg1/art-vectors.json");

    fn vectors() -> Value {
        serde_json::from_str(VECTORS_JSON).expect("art-vectors.json")
    }

    fn kind_of(s: &str) -> Kind {
        match s {
            "Brawn" => Kind::Brawn,
            "Arcane" => Kind::Arcane,
            "Cunning" => Kind::Cunning,
            other => panic!("unknown type {other}"),
        }
    }

    /// Serialise a role grid to the vector's `["0123…", …]` row strings.
    fn ser(role: &RoleGrid) -> Vec<String> {
        role.iter()
            .map(|row| row.iter().map(|c| (b'0' + c) as char).collect())
            .collect()
    }

    fn want_rows(v: &Value) -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect()
    }

    /// F1: all 12 hero portraits reproduce the role grid AND the per-id HSL-shifted palette.
    #[test]
    fn hero_icons_match_vectors() {
        for h in vectors()["heroIcons"].as_array().unwrap() {
            let hero = h["hero"].as_str().unwrap();
            let kind = kind_of(h["type"].as_str().unwrap());
            let (role, pal) = hero_icon(hero, kind);
            assert_eq!(ser(&role), want_rows(&h["roleGrid"]), "{hero} role grid");
            let want = &h["pal"];
            assert_eq!(pal.body, want["body"].as_str().unwrap(), "{hero} body");
            assert_eq!(
                pal.accent,
                want["accent"].as_str().unwrap(),
                "{hero} accent"
            );
            assert_eq!(
                pal.outline,
                want["outline"].as_str().unwrap(),
                "{hero} outline"
            );
        }
    }

    /// F2: all 15 foe portraits reproduce the role grid (0..4) + the per-type palette.
    #[test]
    fn foes_match_vectors() {
        for f in vectors()["foes"].as_array().unwrap() {
            let tier = &f["tier"];
            let n = tier["n"].as_u64().unwrap() as u32;
            let name = tier["name"].as_str().unwrap();
            let kind = kind_of(tier["type"].as_str().unwrap());
            let (role, pal) = foe_grid(n, name, kind);
            assert_eq!(
                ser(&role),
                want_rows(&f["roleGrid"]),
                "foe {name} (tier {n})"
            );
            assert_eq!(
                f["boss"].as_bool().unwrap(),
                n.is_multiple_of(REGION_SIZE),
                "boss flag {n}"
            );
            let want = &f["pal"];
            assert_eq!(pal.body, want["body"].as_str().unwrap(), "foe {name} body");
            assert_eq!(
                pal.accent,
                want["accent"].as_str().unwrap(),
                "foe {name} accent"
            );
            assert_eq!(
                pal.outline,
                want["outline"].as_str().unwrap(),
                "foe {name} outline"
            );
            assert_eq!(pal.eye.as_deref(), want["eye"].as_str(), "foe {name} eye");
        }
    }

    /// Determinism + symmetry sanity (the generators are pure; portraits are vertically symmetric).
    #[test]
    fn portraits_are_deterministic_and_symmetric() {
        let (r1, _) = hero_icon("bram", Kind::Brawn);
        let (r2, _) = hero_icon("bram", Kind::Brawn);
        assert_eq!(r1, r2, "hero portrait deterministic");
        for (y, row) in r1.iter().enumerate() {
            for x in 0..G {
                assert_eq!(row[x], row[G - 1 - x], "hero portrait symmetric @ {x},{y}");
            }
        }
    }
}
