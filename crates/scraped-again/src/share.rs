//! Shareable world+view state (E12) — encode/decode to a compact URL-fragment string,
//! so a link reproduces a world and a camera view. Deterministic generation means we
//! only need to carry the seed + a few view params, not any world data. Pure logic, no
//! `web-sys` — used by both the web (`location.hash`) and native (`--share`) paths.
//!
//! Format (readable, `&`-separated `k=v`, leading `#` tolerated):
//! `v=1&s=<seed>&x=&y=&z=<pos>&yaw=&pit=<rad>&w=<wobble>&d=<colour steps>&t=<toggles hex>`
//! Decode is lenient: unknown keys are ignored, missing keys take the default.

/// The shareable state: which world (`seed`) and where/how we're viewing it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ShareState {
    pub seed: u32,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub wobble: f32,
    pub color_steps: f32,
    /// Feature-toggle bitmask (see `gfx::TOGGLE_LABELS`).
    pub toggles: u32,
}

const VERSION: u32 = 1;

impl ShareState {
    /// Encode to the URL-fragment string (no leading `#`). Floats are trimmed to a
    /// short precision — camera placement is a hint, not bit-exact.
    pub fn encode(&self) -> String {
        format!(
            "v={}&s={}&x={:.2}&y={:.2}&z={:.2}&yaw={:.3}&pit={:.3}&w={:.0}&d={:.0}&t={:x}",
            VERSION,
            self.seed,
            self.pos[0],
            self.pos[1],
            self.pos[2],
            self.yaw,
            self.pitch,
            self.wobble,
            self.color_steps,
            self.toggles,
        )
    }

    /// Decode from a fragment string, falling back to `default` for any missing/invalid
    /// field. A leading `#` (as returned by `location.hash`) is tolerated. Returns
    /// `None` only if there's no recognizable `seed` (so callers can keep their world).
    pub fn decode(s: &str, default: ShareState) -> Option<ShareState> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.trim().is_empty() {
            return None;
        }
        let mut out = default;
        let mut saw_seed = false;
        for pair in s.split('&') {
            let Some((k, v)) = pair.split_once('=') else {
                continue;
            };
            match k {
                "s" => {
                    if let Ok(n) = v.parse::<u32>() {
                        out.seed = n;
                        saw_seed = true;
                    }
                }
                "x" => set_pos(&mut out.pos[0], v),
                "y" => set_pos(&mut out.pos[1], v),
                "z" => set_pos(&mut out.pos[2], v),
                "yaw" => set_f32(&mut out.yaw, v),
                "pit" => set_f32(&mut out.pitch, v),
                "w" => set_f32(&mut out.wobble, v),
                "d" => set_f32(&mut out.color_steps, v),
                "t" => {
                    if let Ok(n) = u32::from_str_radix(v, 16) {
                        out.toggles = n;
                    }
                }
                _ => {} // unknown / version key: ignore
            }
        }
        saw_seed.then_some(out)
    }
}

fn set_f32(slot: &mut f32, v: &str) {
    if let Ok(f) = v.parse::<f32>() {
        if f.is_finite() {
            *slot = f;
        }
    }
}

/// BUG1 (adversarial hunt, 2026-06-11): a crafted share link with an **extreme-but-finite**
/// coordinate (e.g. `z=3e11`) used to reach first-frame streaming, where `(z / CELL) as i32`
/// saturates to `i32::MAX` and the cell loops' `cc + reach` overflowed. The share link is a
/// trust boundary: clamp positions to a generous world bound here (no legitimate play ever
/// approaches it; the `*_near` streamers also saturate as defense-in-depth).
pub const POS_BOUND: f32 = 1.0e7;

fn set_pos(slot: &mut f32, v: &str) {
    if let Ok(f) = v.parse::<f32>() {
        if f.is_finite() {
            *slot = f.clamp(-POS_BOUND, POS_BOUND);
        }
    }
}

/// Turn a user-typed seed into a `u32`: a plain integer is used directly; anything else
/// is folded to a seed via the same avalanche mix the world uses (so text seeds are
/// stable + consistent across web and native). Empty → `None`.
pub fn seed_from_text(text: &str) -> Option<u32> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<u32>() {
        return Some(n);
    }
    // Fold the bytes through a multiply-xorshift mix (same shape as worldgen::hash).
    let mut h: u32 = 0x9e37_79b9;
    for &b in t.as_bytes() {
        h = h.wrapping_add(b as u32);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2c1b_3c6d);
        h ^= h >> 12;
        h = h.wrapping_mul(0x297a_2d39);
        h ^= h >> 15;
    }
    Some(h)
}

/// Deterministic "seed of the day" from a `YYYY-MM-DD` string (everyone gets the same
/// world on a given date, with no server).
pub fn seed_of_the_day(date: &str) -> u32 {
    seed_from_text(date).unwrap_or(0)
}

/// Format a Unix timestamp (seconds, UTC) as `YYYY-MM-DD`. Pure (no `chrono`), so
/// native `--daily` lands on the same UTC date string the web gets from JS `Date`,
/// and thus the same `seed_of_the_day`. Civil-from-days per Howard Hinnant's algorithm.
pub fn date_utc_from_unix_secs(secs: i64) -> String {
    let days = secs.div_euclid(86_400); // days since 1970-01-01 (can be negative)
    let z = days + 719_468; // shift epoch to 0000-03-01
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ShareState {
        ShareState {
            seed: 1337,
            pos: [12.5, 40.0, -8.25],
            yaw: 1.2,
            pitch: -0.3,
            wobble: 85.0,
            color_steps: 4.0,
            toggles: 0x7ff,
        }
    }

    fn default() -> ShareState {
        ShareState {
            seed: 0,
            pos: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            wobble: 85.0,
            color_steps: 4.0,
            toggles: 0x7ff,
        }
    }

    #[test]
    fn round_trips_within_precision() {
        let a = sample();
        let b = ShareState::decode(&a.encode(), default()).unwrap();
        assert_eq!(a.seed, b.seed);
        assert_eq!(a.toggles, b.toggles);
        for k in 0..3 {
            assert!((a.pos[k] - b.pos[k]).abs() < 0.01, "pos[{k}]");
        }
        assert!((a.yaw - b.yaw).abs() < 0.001);
        assert!((a.pitch - b.pitch).abs() < 0.001);
        assert_eq!(a.wobble, b.wobble);
        assert_eq!(a.color_steps, b.color_steps);
    }

    #[test]
    fn decode_tolerates_hash_prefix_and_missing_keys() {
        // Only a seed; everything else should fall back to the default.
        let d = default();
        let got = ShareState::decode("#s=42", d).unwrap();
        assert_eq!(got.seed, 42);
        assert_eq!(got.pos, d.pos);
        assert_eq!(got.wobble, d.wobble);
        assert_eq!(got.toggles, d.toggles);
    }

    #[test]
    fn decode_without_seed_is_none() {
        assert!(ShareState::decode("", default()).is_none());
        assert!(ShareState::decode("#x=1&y=2", default()).is_none());
    }

    #[test]
    fn decode_ignores_unknown_and_garbage() {
        let got = ShareState::decode("v=1&s=7&future=xyz&x=oops&t=ff", default()).unwrap();
        assert_eq!(got.seed, 7);
        assert_eq!(got.toggles, 0xff);
        assert_eq!(got.pos[0], default().pos[0]); // "oops" rejected → default
    }

    #[test]
    fn seed_from_text_numeric_vs_text() {
        assert_eq!(seed_from_text("1337"), Some(1337));
        assert_eq!(seed_from_text("  42 "), Some(42));
        assert_eq!(seed_from_text(""), None);
        // Text is deterministic and (almost surely) not the trivial value.
        let a = seed_from_text("aurora").unwrap();
        assert_eq!(a, seed_from_text("aurora").unwrap());
        assert_ne!(seed_from_text("aurora"), seed_from_text("nebula"));
    }

    #[test]
    fn seed_of_the_day_is_stable() {
        assert_eq!(seed_of_the_day("2026-06-05"), seed_of_the_day("2026-06-05"));
        assert_ne!(seed_of_the_day("2026-06-05"), seed_of_the_day("2026-06-06"));
    }

    #[test]
    fn date_utc_matches_known_days() {
        assert_eq!(date_utc_from_unix_secs(0), "1970-01-01");
        // 2026-06-05 00:00:00 UTC = 1_780_617_600 (20609 days × 86400).
        assert_eq!(date_utc_from_unix_secs(1_780_617_600), "2026-06-05");
        // Late in the same UTC day still reads the same date.
        assert_eq!(
            date_utc_from_unix_secs(1_780_617_600 + 86_399),
            "2026-06-05"
        );
        // A leap day.
        assert_eq!(date_utc_from_unix_secs(1_582_934_400), "2020-02-29");
    }

    /// BUG1 regression (adversarial hunt 2026-06-11): an extreme-but-finite share coordinate
    /// (z=3e11) used to overflow first-frame streaming. Positions are clamped at this boundary.
    #[test]
    fn extreme_share_coords_are_clamped() {
        let st = ShareState::decode("#v=1&s=1&z=300000000000", default()).expect("decodes");
        assert!(st.pos[2].abs() <= POS_BOUND, "z clamped: {}", st.pos[2]);
        let st = ShareState::decode("#v=1&s=1&x=-1e30&y=1e20", default()).expect("decodes");
        assert!(st.pos[0].abs() <= POS_BOUND && st.pos[1].abs() <= POS_BOUND);
        // Legitimate coords pass through untouched.
        let st = ShareState::decode("#v=1&s=1&x=123.5&z=-456", default()).expect("decodes");
        assert_eq!(st.pos[0], 123.5);
        assert_eq!(st.pos[2], -456.0);
    }
}
