//! The gold-**HOARD** coin-pile — a faithful port of `fxgl.js seedHoard` (constants pinned by the
//! export: `HOARD_CAP=480`, `HOARD_K=600`, `HOARD_MAX_H=1.0`, `HOARD_TIERS=8`, `GOLD_TONES`). It
//! places the **surface** coins of the pile (the "imply the bulk, render the surface" trick): each
//! coin carries a normalised position, a size, a squash aspect, and a gold tone. Deterministic from
//! `seed` and **stable under accumulation** (T196): a lower `level` is a byte-identical *prefix* of a
//! higher one — coins never teleport as the pile grows. Pure logic; the Home screen paints the coins.

/// Surface-coin ceiling at full level (≪ the 512 particle cap).
const HOARD_CAP: usize = 480;
/// Saturating-curve constant: `gold == K` → level 0.5.
const HOARD_K: f64 = 600.0;
/// A level-1.0 pile's wall-banked sides reach the top (the centre dips lower).
const HOARD_MAX_H: f64 = 1.0;
/// The gold tones (dark→light by luma is computed at use).
const GOLD_TONES: [[u8; 3]; 3] = [[255, 214, 110], [212, 158, 46], [120, 84, 22]];

/// One placed surface coin (normalised `x,y` ∈ [0,1]; `size` in px-ish units; `aspect` squashes it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coin {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub aspect: f64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// The saturating hoard level for a `gold` balance: `gold / (gold + K)` ∈ [0,1).
pub fn hoard_level(gold: f64) -> f64 {
    let gold = gold.max(0.0);
    gold / (gold + HOARD_K)
}

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
fn luma(c: [u8; 3]) -> f64 {
    (0.299 * c[0] as f64 + 0.587 * c[1] as f64 + 0.114 * c[2] as f64) / 255.0
}

/// `fxgl.js hash01i` — a small positional hash (organic micro-roughness, no sequential RNG).
fn hash01i(i: i64, salt: i64) -> f64 {
    let mut h = ((i as i32).wrapping_mul(374_761_393) as u32)
        .wrapping_add((salt as i32).wrapping_mul(668_265_263) as u32);
    h ^= h >> 13;
    h = h.wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) as f64) / 4_294_967_296.0
}

/// `fxgl.js makeRng` — a 32-bit xorshift returning a float in [0,1). Stateful per draw.
struct XorShift {
    s: i32,
}
impl XorShift {
    fn new(seed: i32) -> XorShift {
        XorShift {
            s: if seed == 0 {
                0x9e37_79b9u32 as i32
            } else {
                seed
            },
        }
    }
    fn next(&mut self) -> f64 {
        // JS: s ^= s<<13; s ^= s>>>17; s ^= s<<5; s |= 0; return ((s>>>8)&0xffffff)/0x1000000.
        let mut s = self.s;
        s ^= s << 13;
        s ^= ((s as u32) >> 17) as i32;
        s ^= s << 5;
        self.s = s;
        (((s as u32) >> 8) & 0x00ff_ffff) as f64 / 0x0100_0000 as f64
    }
}

/// `fxgl.js moundProfile` — the pile HEIGHT (0..`HOARD_MAX_H`) at normalised column `x`. Not a central
/// dome: coins bank against the side walls (higher toward x≈0/1, dipping in the middle), with seeded
/// drift + hashed roughness (tapered toward the walls so the banked sides climb cleanly).
fn mound_profile(x: f64, level: f64, seed: i32) -> f64 {
    let level = clamp01(level);
    if level <= 0.0 {
        return 0.0;
    }
    let s = if seed == 0 { 1 } else { seed };
    let wall = ((x - 0.5).abs() * 2.0).powf(1.5); // 0 centre → 1 at the walls
    let taper = 1.0 - 0.7 * wall;
    let drift = (0.10 * (x * 9.1 + (s.rem_euclid(17)) as f64).sin()
        + 0.07 * (x * 17.3 + (s.rem_euclid(23)) as f64).sin())
        * taper;
    let rough = (hash01i((x * 40.0).floor() as i64, (s ^ 0x9e37) as i64) - 0.5) * 0.10 * taper;
    let mut f = 0.44 + 0.58 * wall + drift + rough;
    f = f.max(0.12);
    clamp01(level * HOARD_MAX_H * f)
}

/// Seed the surface coins of the hoard at `level` (0..1), deterministic from `seed`. `palette` overrides
/// the gold tones (e.g. the home palette); empty → `GOLD_TONES`. Returns the coins for `level` — a
/// prefix of the full pile (stable accumulation). `cap` clamps the full-pile count (≤ `HOARD_CAP`).
pub fn seed_hoard(level: f64, seed: i32, palette: &[[u8; 3]], cap: usize) -> Vec<Coin> {
    let level = clamp01(level);
    let seed = if seed == 0 { 0x601d } else { seed };
    let ceil = cap.min(HOARD_CAP);
    let full = ceil.clamp(1, HOARD_CAP); // coins at level 1 (level-independent; reduced-motion off)
    let n = ((full as f64 * level).round() as usize).min(full);
    let mut rng = XorShift::new(seed);
    let pool: Vec<[u8; 3]> = if palette.is_empty() {
        GOLD_TONES.to_vec()
    } else {
        palette.to_vec()
    };
    // Order the pool dark→light so a coin's fill-rank `q` picks its tone (deep → dark, crest → light).
    let mut tone_ramp = pool.clone();
    tone_ramp.sort_by(|a, b| luma(*a).partial_cmp(&luma(*b)).unwrap());
    let tlast = tone_ramp.len() as i64 - 1;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let q = (i as f64 + 0.5) / full as f64; // fill rank (0..1), level-independent
        let mut x = rng.next();
        if rng.next() < 0.4 {
            x = if x < 0.5 {
                x * x
            } else {
                1.0 - (1.0 - x) * (1.0 - x)
            };
        }
        let h = mound_profile(x, q, seed);
        let surface_y = 1.0 - h;
        let band = 0.03 + 0.16 * h;
        let y = clamp01(surface_y + rng.next() * band);
        let pick = clamp01(q + (rng.next() - 0.5) * 0.9);
        let idx = (pick * tlast as f64).round().clamp(0.0, tlast as f64) as usize;
        let col = tone_ramp[idx];
        let size = lerp(6.0, 13.0, rng.next()) * (0.85 + 0.3 * q);
        let aspect = lerp(0.35, 0.95, rng.next());
        let _rot = rng.next(); // spin orientation (RectRun is axis-aligned; consumed for parity)
        let _glint = rng.next();
        out.push(Coin {
            x,
            y,
            size,
            aspect,
            r: col[0],
            g: col[1],
            b: col[2],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_scales_count_and_saturates() {
        assert_eq!(seed_hoard(0.0, 1, &[], HOARD_CAP).len(), 0);
        let full = seed_hoard(1.0, 1, &[], HOARD_CAP).len();
        assert_eq!(full, HOARD_CAP, "level 1 fills the cap");
        let half = seed_hoard(0.5, 1, &[], HOARD_CAP).len();
        assert_eq!(half, HOARD_CAP / 2, "level 0.5 ≈ half the coins");
        // hoardLevel: gold == K → 0.5.
        assert!((hoard_level(HOARD_K) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn accumulation_is_a_stable_prefix() {
        // T196: a lower level is a byte-identical PREFIX of a higher one (coins never teleport).
        let lo = seed_hoard(0.3, 0x601d, &[], HOARD_CAP);
        let hi = seed_hoard(0.7, 0x601d, &[], HOARD_CAP);
        assert!(hi.len() > lo.len());
        assert_eq!(&hi[..lo.len()], &lo[..], "lower level == prefix of higher");
    }

    #[test]
    fn deterministic_from_seed() {
        assert_eq!(
            seed_hoard(0.8, 42, &[], HOARD_CAP),
            seed_hoard(0.8, 42, &[], HOARD_CAP)
        );
        assert_ne!(
            seed_hoard(0.8, 1, &[], HOARD_CAP),
            seed_hoard(0.8, 2, &[], HOARD_CAP),
            "different seeds → different piles"
        );
        // Coins sit within the backdrop.
        for c in seed_hoard(1.0, 7, &[], HOARD_CAP) {
            assert!((0.0..=1.0).contains(&c.x) && (0.0..=1.0).contains(&c.y));
        }
    }
}
