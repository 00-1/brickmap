//! Biome-driven auto mode (E8×E10×E16). The world is partitioned into large regions by a
//! low-frequency field; **every palette is a biome**, and each biome carries a full settings
//! preset — palette, spawn densities (foliage / structures / wisps / inscriptions), lighting +
//! wobble, ground amplitude, and the drone mix. The live app samples the field at the camera (and
//! at each chunk, for generation) and **blends** between the two nearest biomes, so the whole
//! look + feel transitions smoothly as you fly rather than snapping at borders.
//!
//! Pure logic (no wgpu): the renderer/app read [`Blended`] and apply it. Deterministic in the seed.

use crate::palette::PALETTES;

/// One biome preset. Scalar fields are levels/multipliers that get blended between neighbours;
/// `palette` indexes [`PALETTES`] (every palette is a biome).
#[derive(Clone, Copy)]
pub struct Biome {
    pub name: &'static str,
    pub palette: usize,
    pub count: u32,        // palette colours used
    pub dither: f32,       // ordered-dither spread
    pub foliage: f32,      // grass density multiplier
    pub forest: f32,       // tree/bush density multiplier
    pub colossi: f32,      // colossal-structure presence multiplier
    pub wisps: f32,        // drifting-wisp count multiplier
    pub inscriptions: f32, // inscription density multiplier
    pub wobble: f32,       // vertex-quantization snap (high = crisp, low = heavy wobble)
    pub sun: f32,          // directional-sun amount 0..1 (0 = point-lit mood)
    pub steps: f32,        // colour-posterise steps
    pub amplitude: f32,    // ground-height amplitude multiplier
    pub vol: f32,          // drone master volume
    pub murk: f32,         // drone tone (0 dark .. 1 open)
    pub heavy: f32,        // drone distortion drive
}

#[allow(clippy::too_many_arguments)]
const fn b(
    name: &'static str,
    palette: usize,
    count: u32,
    dither: f32,
    foliage: f32,
    forest: f32,
    colossi: f32,
    wisps: f32,
    inscriptions: f32,
    wobble: f32,
    sun: f32,
    steps: f32,
    amplitude: f32,
    vol: f32,
    murk: f32,
    heavy: f32,
) -> Biome {
    Biome {
        name,
        palette,
        count,
        dither,
        foliage,
        forest,
        colossi,
        wisps,
        inscriptions,
        wobble,
        sun,
        steps,
        amplitude,
        vol,
        murk,
        heavy,
    }
}

/// One biome per palette (indices match [`PALETTES`]). `bruise` (11) reproduces the current
/// house look (the "default"). Columns: palette, count, dither, foliage, forest, colossi, wisps,
/// inscriptions, wobble, sun, steps, amplitude, vol, murk, heavy.
pub const BIOMES: &[Biome] = &[
    b(
        "mono", 0, 4, 1.2, 0.8, 0.6, 1.0, 0.8, 1.0, 90.0, 0.15, 4.0, 1.0, 0.85, 0.55, 1.6,
    ),
    b(
        "verdant", 1, 5, 1.0, 1.5, 1.4, 0.6, 1.2, 0.8, 110.0, 0.25, 5.0, 1.1, 0.8, 0.7, 1.3,
    ),
    b(
        "ash", 2, 5, 1.3, 0.3, 0.2, 1.5, 0.7, 1.3, 70.0, 0.0, 4.0, 1.2, 0.85, 0.4, 1.9,
    ),
    b(
        "ember", 3, 5, 1.2, 0.5, 0.4, 1.1, 0.9, 1.0, 85.0, 0.1, 4.0, 1.1, 0.85, 0.5, 1.8,
    ),
    b(
        "dusk", 4, 5, 1.3, 0.9, 0.8, 0.9, 1.4, 1.0, 95.0, 0.0, 4.0, 1.0, 0.8, 0.6, 1.5,
    ),
    b(
        "mist", 5, 5, 1.5, 1.0, 0.7, 0.8, 1.8, 0.9, 80.0, 0.0, 3.0, 0.85, 0.8, 0.45, 1.4,
    ),
    b(
        "rust", 6, 5, 1.2, 0.4, 0.3, 1.6, 0.8, 1.4, 60.0, 0.0, 4.0, 1.15, 0.9, 0.4, 2.1,
    ),
    b(
        "neon", 7, 5, 1.4, 0.6, 0.5, 1.0, 1.6, 1.1, 75.0, 0.1, 4.0, 1.0, 0.8, 0.7, 1.5,
    ),
    b(
        "sodium", 8, 5, 1.3, 0.5, 0.4, 1.3, 1.0, 1.2, 85.0, 0.1, 4.0, 1.1, 0.85, 0.5, 1.7,
    ),
    b(
        "bog", 9, 5, 1.1, 1.4, 1.2, 0.7, 1.3, 0.9, 95.0, 0.05, 5.0, 0.95, 0.85, 0.55, 1.6,
    ),
    b(
        "oxide", 10, 5, 1.2, 0.8, 0.7, 1.2, 1.0, 1.1, 90.0, 0.15, 4.0, 1.05, 0.85, 0.55, 1.7,
    ),
    // bruise (11) = the current default look + mix.
    b(
        "bruise", 11, 5, 1.5, 1.0, 1.0, 1.0, 1.0, 1.0, 85.0, 0.0, 4.0, 1.0, 0.85, 0.7, 1.9,
    ),
    b(
        "abyss", 12, 5, 1.4, 0.4, 0.3, 1.0, 1.7, 1.0, 70.0, 0.0, 4.0, 1.2, 0.8, 0.35, 1.6,
    ),
    b(
        "venom", 13, 5, 1.2, 1.5, 1.3, 0.6, 1.3, 0.8, 100.0, 0.2, 5.0, 1.0, 0.8, 0.65, 1.4,
    ),
    b(
        "magma", 14, 5, 1.1, 0.1, 0.05, 1.4, 1.0, 1.2, 55.0, 0.1, 4.0, 1.4, 0.9, 0.5, 2.3,
    ),
    b(
        "tar", 15, 5, 1.3, 0.3, 0.2, 1.5, 0.9, 1.3, 65.0, 0.0, 4.0, 1.1, 0.9, 0.4, 2.0,
    ),
    b(
        "cobalt", 16, 5, 1.3, 0.7, 0.6, 1.2, 1.1, 1.1, 90.0, 0.15, 4.0, 1.1, 0.85, 0.55, 1.6,
    ),
    b(
        "slime", 17, 5, 1.2, 1.3, 1.1, 0.7, 1.4, 0.9, 95.0, 0.1, 5.0, 0.95, 0.8, 0.6, 1.5,
    ),
    b(
        "parchment",
        18,
        5,
        1.4,
        0.9,
        0.8,
        0.9,
        0.9,
        1.2,
        130.0,
        0.3,
        4.0,
        1.0,
        0.8,
        0.65,
        1.3,
    ),
    b(
        "frost", 19, 5, 1.3, 0.4, 0.3, 1.1, 1.0, 1.0, 100.0, 0.25, 4.0, 1.3, 0.85, 0.55, 1.5,
    ),
];

/// The fully-resolved (blended) biome settings at a point: blended palette ramp + all scalars,
/// plus the dominant biome name for the HUD.
#[derive(Clone, Copy)]
pub struct Blended {
    pub colors: [[f32; 3]; 5],
    pub count: u32,
    pub dither: f32,
    pub foliage: f32,
    pub forest: f32,
    pub colossi: f32,
    pub wisps: f32,
    pub inscriptions: f32,
    pub wobble: f32,
    pub sun: f32,
    pub steps: f32,
    pub amplitude: f32,
    pub vol: f32,
    pub murk: f32,
    pub heavy: f32,
    /// Rare **ethereal variant** amount `0..1`: in these scattered pockets the blueprint **ink**
    /// grid fades on and the drone turns less deep / more ethereal (already folded into `murk` +
    /// `heavy`). Smooth (fades in at the pocket edges), so it's a soft transition not a switch.
    pub ink: f32,
    pub name: &'static str,
    pub other: &'static str,
    pub frac: f32,
}

impl Blended {
    /// A label for the HUD: the dominant biome, or "a→b" mid-transition; an ethereal pocket is
    /// flagged with a trailing `*`.
    pub fn label(&self) -> String {
        let base = if self.frac > 0.18 && self.frac < 0.82 {
            format!("{}>{}", self.name, self.other)
        } else {
            self.name.to_string()
        };
        if self.ink > 0.5 {
            format!("{base}*")
        } else {
            base
        }
    }
}

/// Resample a palette's ramp to exactly 5 evenly-spaced colours (dark→light), so palettes of
/// different lengths can be blended slot-for-slot.
fn ramp5(palette: usize) -> [[f32; 3]; 5] {
    let cols = PALETTES[palette.min(PALETTES.len() - 1)].colors;
    let n = cols.len();
    let mut out = [[0.0f32; 3]; 5];
    for (i, slot) in out.iter_mut().enumerate() {
        let t = i as f32 / 4.0 * (n - 1) as f32;
        let lo = t.floor() as usize;
        let hi = (lo + 1).min(n - 1);
        let f = t.fract();
        for c in 0..3 {
            slot[c] = cols[lo][c] * (1.0 - f) + cols[hi][c] * f;
        }
    }
    out
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smooth value noise in `[0, 1)` (bilinear, smoothstep-interpolated) at integer-lattice scale.
fn vnoise(x: f32, z: f32, seed: u32) -> f32 {
    fn hash(xi: i32, zi: i32, seed: u32) -> f32 {
        let mut h = (xi as u32).wrapping_mul(0x8DA6_B343)
            ^ (zi as u32).wrapping_mul(0xD816_3841)
            ^ seed.wrapping_mul(0x9E37_79B1);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2C1B_3C6D);
        h ^= h >> 13;
        (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
    }
    let (x0, z0) = (x.floor(), z.floor());
    let (xi, zi) = (x0 as i32, z0 as i32);
    let (fx, fz) = (x - x0, z - z0);
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let n00 = hash(xi, zi, seed);
    let n10 = hash(xi + 1, zi, seed);
    let n01 = hash(xi, zi + 1, seed);
    let n11 = hash(xi + 1, zi + 1, seed);
    lerp(lerp(n00, n10, sx), lerp(n01, n11, sx), sz)
}

/// Two-octave fbm in roughly `[0, 1]`, spread a little so it uses the biome range.
fn fbm(x: f32, z: f32, seed: u32) -> f32 {
    let a = vnoise(x, z, seed);
    let b = vnoise(x * 2.03 + 11.3, z * 2.03 - 7.1, seed ^ 0x9E37);
    let v = a * 0.65 + b * 0.35;
    // Mild contrast stretch so regions reach the ends of the biome list.
    ((v - 0.5) * 1.35 + 0.5).clamp(0.0, 0.9999)
}

/// World-unit scale of the biome field (region size ≈ this many blocks across).
const SCALE: f32 = 1.0 / 700.0;
/// Scale of the rare ethereal-variant field (smaller → scattered pockets within biomes).
const VARIANT_SCALE: f32 = 1.0 / 420.0;

/// The two biomes and blend fraction at a world `(x, z)`. `frac` is the weight of the *second*.
pub fn field(x: f32, z: f32, seed: u32) -> (usize, usize, f32) {
    let n = BIOMES.len();
    let f = fbm(x * SCALE, z * SCALE, seed ^ 0xB10E_0001) * n as f32;
    let lo = (f.floor() as usize).min(n - 1);
    let hi = (lo + 1).min(n - 1);
    (lo, hi, f.fract())
}

/// The rare "ethereal variant" amount `0..1` at `(x, z)`: a separate low-frequency field, mostly
/// 0 with scattered pockets ramping to 1. In those pockets the ink grid fades on and the drone
/// turns ethereal. `smoothstep` gives soft pocket edges (a smooth transition, not a hard switch).
pub fn variant(x: f32, z: f32, seed: u32) -> f32 {
    let v = fbm(x * VARIANT_SCALE, z * VARIANT_SCALE, seed ^ 0xE7E7_0001);
    // Only the top sliver of the field becomes ethereal → rare.
    let t = ((v - 0.80) / 0.10).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Blend the two biomes at `(x, z)` into resolved [`Blended`] settings (palette ramp + scalars).
pub fn at(x: f32, z: f32, seed: u32) -> Blended {
    let (lo, hi, t) = field(x, z, seed);
    let (a, c) = (&BIOMES[lo], &BIOMES[hi]);
    let (ra, rc) = (ramp5(a.palette), ramp5(c.palette));
    let mut colors = [[0.0f32; 3]; 5];
    for i in 0..5 {
        for k in 0..3 {
            colors[i][k] = lerp(ra[i][k], rc[i][k], t);
        }
    }
    // Rare ethereal variant: fade the ink grid on, and push the drone "less deep, more
    // ethereal" — open the murk up and pull the heavy distortion down.
    let eth = variant(x, z, seed);
    let murk = lerp(lerp(a.murk, c.murk, t), 0.95, eth);
    let heavy = lerp(lerp(a.heavy, c.heavy, t), 0.55, eth);
    Blended {
        colors,
        count: lerp(a.count as f32, c.count as f32, t).round() as u32,
        dither: lerp(a.dither, c.dither, t),
        foliage: lerp(a.foliage, c.foliage, t),
        forest: lerp(a.forest, c.forest, t),
        colossi: lerp(a.colossi, c.colossi, t),
        wisps: lerp(a.wisps, c.wisps, t),
        inscriptions: lerp(a.inscriptions, c.inscriptions, t),
        wobble: lerp(a.wobble, c.wobble, t),
        sun: lerp(a.sun, c.sun, t),
        steps: lerp(a.steps, c.steps, t),
        amplitude: lerp(a.amplitude, c.amplitude, t),
        vol: lerp(a.vol, c.vol, t),
        murk,
        heavy,
        ink: eth,
        name: if t < 0.5 { a.name } else { c.name },
        other: if t < 0.5 { c.name } else { a.name },
        frac: if t < 0.5 { t } else { 1.0 - t },
    }
}

/// Just the spawn-density multipliers at a point (cheap; for per-chunk generation): returns
/// `(foliage, forest, colossi, inscriptions)`.
pub fn density(x: f32, z: f32, seed: u32) -> (f32, f32, f32, f32) {
    let (lo, hi, t) = field(x, z, seed);
    let (a, c) = (&BIOMES[lo], &BIOMES[hi]);
    (
        lerp(a.foliage, c.foliage, t),
        lerp(a.forest, c.forest, t),
        lerp(a.colossi, c.colossi, t),
        lerp(a.inscriptions, c.inscriptions, t),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_palette_is_a_biome() {
        assert_eq!(BIOMES.len(), PALETTES.len());
        for (i, b) in BIOMES.iter().enumerate() {
            assert_eq!(b.palette, i, "biome {} should map to palette {i}", b.name);
        }
    }

    #[test]
    fn field_is_deterministic_and_in_range() {
        let n = BIOMES.len();
        for &(x, z) in &[(0.0, 0.0), (1234.0, -567.0), (-9000.0, 4200.0)] {
            let (lo, hi, t) = field(x, z, 42);
            assert!(lo < n && hi < n);
            assert!((0.0..=1.0).contains(&t));
            assert_eq!(field(x, z, 42).0, lo, "deterministic");
        }
    }

    #[test]
    fn blend_stays_bounded_and_named() {
        let blend = at(321.0, 654.0, 7);
        for c in blend.colors {
            for ch in c {
                assert!((0.0..=1.0).contains(&ch), "colour out of range: {ch}");
            }
        }
        assert!(blend.count >= 1);
        assert!(!blend.label().is_empty());
        assert!(blend.foliage > 0.0 && blend.wobble > 0.0);
    }

    #[test]
    fn ethereal_variant_is_rare_and_bounded() {
        let (mut sum, mut hot) = (0.0f32, 0);
        let n = 4000;
        for i in 0..n {
            let v = variant(i as f32 * 13.0, (i * 7) as f32 * 9.0, 5);
            assert!((0.0..=1.0).contains(&v));
            sum += v;
            if v > 0.5 {
                hot += 1;
            }
        }
        // Mostly off: well under a quarter of samples are strongly ethereal.
        assert!(hot * 4 < n, "ethereal variant not rare enough: {hot}/{n}");
        assert!(sum > 0.0, "variant never fires at all");
    }

    #[test]
    fn transitions_are_continuous() {
        // Walking a line, the blended wobble shouldn't jump wildly between nearby samples.
        let mut prev = at(0.0, 0.0, 99).wobble;
        for i in 1..400 {
            let w = at(i as f32 * 3.0, 0.0, 99).wobble;
            assert!((w - prev).abs() < 30.0, "wobble jumped {prev}→{w}");
            prev = w;
        }
    }
}
