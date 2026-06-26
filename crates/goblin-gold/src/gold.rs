//! GG1 **Goblin Gold** economy (full-port phase 3) — the pure numeric reward formulas, re-implemented
//! from `main.js` and **proven against `gold-vectors.json`** (T233b-gold). Gold only ever *accrues*
//! (there's no spending): a finished round pays out per cleanly-solved question plus a round bonus,
//! and the running total crosses the wealth milestones (handled by [`crate::earning::gold_awards`]).
//!
//! The formulas (verbatim from `main.js`, tunables in `gold.json`):
//! - [`question_gold`] `(2 + max(0, round(target − dt))) · (1 + combo·0.1) · mult` — per clean
//!   question; `target` is the mode's `masterSecs`, `combo` the running clean-solve streak.
//! - [`round_bonus_gold`] `(score + rankIdx·2) · mult`.
//! - [`tier_gold`] `round(10·(1 + n/10)) · mult` — the Arena per-tier payoff (combat deferred).
//! - [`gold_mult`] `(1 + items·0.05 + mastered·0.5 + heroes·0.5 + tiers·1) · HOARD_G^bosses`.
//! - [`hoard_level`] a 0..1 log-scaled wealth fraction (for the hoard visuals).
//!
//! [`round_gold`] composes these into a round payout (faithful to `finish()`), accruing **live** over
//! the ordered round steps: combo rises on each clean solve and **resets to 0 on a skip** — so it
//! must be accumulated as the round plays, not derived post-hoc from the solved times (which would
//! over-pay after a mid-run skip). See [`Play`] / [`accrue_round`].

use serde::Deserialize;

/// The T233b gold tunables export.
const GOLD_JSON: &str = include_str!("../data/gg1/gold.json");

#[derive(Deserialize)]
struct MultWeights {
    item: f64,
    mastery: f64,
    hero: f64,
    tier: f64,
}

#[derive(Deserialize)]
struct Gold {
    #[serde(rename = "HOARD_G")]
    hoard_g: f64,
    #[serde(rename = "GOLD_EMPTY")]
    gold_empty: f64,
    #[serde(rename = "GOLD_FULL")]
    gold_full: f64,
    #[serde(rename = "multWeights")]
    mult_weights: MultWeights,
}

fn cfg() -> Gold {
    serde_json::from_str(GOLD_JSON).expect("gold.json")
}

/// Per clean question: `(2 + max(0, round(target − dt))) · (1 + combo·0.1) · mult`.
pub fn question_gold(target: f64, dt: f64, combo: u32, mult: f64) -> f64 {
    let speed = (target - dt).round().max(0.0);
    (2.0 + speed) * (1.0 + combo as f64 * 0.1) * mult
}

/// The end-of-round bonus: `(score + rankIdx·2) · mult`.
pub fn round_bonus_gold(score: u32, rank_idx: u32, mult: f64) -> f64 {
    (score as f64 + rank_idx as f64 * 2.0) * mult
}

/// The Arena per-tier payoff: `round(10·(1 + n/10)) · mult`. (Combat is deferred — kept for parity.)
pub fn tier_gold(n: u32, mult: f64) -> f64 {
    (10.0 * (1.0 + n as f64 / 10.0)).round() * mult
}

/// The global gold multiplier from owned-collectible counts:
/// `(1 + items·item + mastered·mastery + heroes·hero + tiers·tier) · HOARD_G^bosses`.
pub fn gold_mult(items: u32, mastered: u32, heroes: u32, tiers: u32, bosses: u32) -> f64 {
    let g = cfg();
    let w = &g.mult_weights;
    let base = 1.0
        + items as f64 * w.item
        + mastered as f64 * w.mastery
        + heroes as f64 * w.hero
        + tiers as f64 * w.tier;
    base * g.hoard_g.powi(bosses as i32)
}

/// Short-scale magnitude suffixes for [`fmt_gold`] (`main.js GOLD_SUFFIX`).
const GOLD_SUFFIX: [&str; 16] = [
    "", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", "Dc", "Ud", "Dd", "Td", "Qad",
];

/// Format a gold balance the way the HUD does (`main.js fmtGold`): `< 1000` verbatim, else a
/// 1000-power tier with a suffix and 2/1/0 decimals by magnitude (e.g. `987654321 → "988M"`).
pub fn fmt_gold(n: f64) -> String {
    let mut n = if !n.is_finite() || n < 0.0 { 0.0 } else { n };
    n = n.floor();
    if n < 1000.0 {
        return format!("{}", n as u64);
    }
    let mut tier = (n.log10() / 3.0).floor() as usize;
    if tier >= GOLD_SUFFIX.len() {
        tier = GOLD_SUFFIX.len() - 1;
    }
    let s = n / 1000f64.powi(tier as i32);
    let body = if s >= 100.0 {
        format!("{s:.0}")
    } else if s >= 10.0 {
        format!("{s:.1}")
    } else {
        format!("{s:.2}")
    };
    format!("{body}{}", GOLD_SUFFIX[tier])
}

/// The hoard "fullness" 0..1: `clamp((log10(1+gold) − log10(EMPTY)) / (log10(FULL) − log10(EMPTY)))`.
/// This is the canonical gold→pile-fraction the Home backdrop feeds to `seedHoard` (vs `hoard.rs`'s
/// distinct `gold/(gold+K)` fxgl helper).
pub fn hoard_level(gold: f64) -> f64 {
    let g = cfg();
    let gold = gold.max(0.0);
    let lo = (g.gold_empty).log10();
    let span = (g.gold_full).log10() - lo;
    (((1.0 + gold).log10() - lo) / span).clamp(0.0, 1.0)
}

/// One step of a round, for **live** gold accrual: a clean solve (with its time) or a skip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Play {
    /// A clean solve taking `dt` seconds — `combo++` then earns `question_gold`.
    Solve(f64),
    /// A skip — resets the combo streak to 0 and earns nothing.
    Skip,
}

/// Accrue a round's per-question gold **live**, faithful to `main.js`: `combo` starts 0, rises by 1
/// on each [`Play::Solve`] (earning `question_gold(target, dt, combo, mult)`), and **resets to 0** on
/// each [`Play::Skip`]. This is the corrected model (T233b-gold `roundGold` vectors): combo *must* be
/// accumulated live — derived post-hoc from the solved times it would over-pay after a mid-run skip,
/// since the skip-reset is unrecoverable. Returns the per-question sum (the vectors' `total`; the
/// round bonus is added by [`round_gold`]).
pub fn accrue_round(target: f64, mult: f64, plays: &[Play]) -> f64 {
    let mut combo = 0u32;
    let mut earn = 0.0;
    for p in plays {
        match p {
            Play::Solve(dt) => {
                combo += 1;
                earn += question_gold(target, *dt, combo, mult);
            }
            Play::Skip => combo = 0,
        }
    }
    earn
}

/// A round's total payout: the live per-question accrual ([`accrue_round`]) plus
/// `round_bonus_gold(score, rank_idx, mult)`, rounded to a whole coin. `score` = number of solves.
/// `mult` is the global multiplier (see [`gold_mult`]); our drill batches awards at round end so a
/// single round-`mult` is used (the col doesn't mutate mid-round here).
pub fn round_gold(target: f64, mult: f64, plays: &[Play], score: u32, rank_idx: u32) -> u64 {
    (accrue_round(target, mult, plays) + round_bonus_gold(score, rank_idx, mult))
        .round()
        .max(0.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const VECTORS_JSON: &str = include_str!("../data/gg1/gold-vectors.json");

    fn vectors() -> Value {
        serde_json::from_str(VECTORS_JSON).expect("gold-vectors.json")
    }

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn fmt_gold_matches_the_hud() {
        assert_eq!(fmt_gold(0.0), "0");
        assert_eq!(fmt_gold(999.0), "999");
        assert_eq!(fmt_gold(1000.0), "1.00K");
        assert_eq!(fmt_gold(12_345.0), "12.3K");
        assert_eq!(fmt_gold(987_654_321.0), "988M"); // the capture-state balance
        assert_eq!(fmt_gold(-5.0), "0");
    }

    #[test]
    fn question_gold_reproduces_every_vector() {
        let v = vectors();
        let arr = v["questionGold"].as_array().unwrap();
        assert!(arr.len() >= 400);
        for r in arr {
            let got = question_gold(
                r["target"].as_f64().unwrap(),
                r["dt"].as_f64().unwrap(),
                r["combo"].as_u64().unwrap() as u32,
                r["mult"].as_f64().unwrap(),
            );
            assert!(
                approx(got, r["gold"].as_f64().unwrap(), 1e-9),
                "questionGold {r} → {got}"
            );
        }
    }

    #[test]
    fn round_bonus_tier_and_hoard_reproduce_their_vectors() {
        let v = vectors();
        for r in v["roundBonusGold"].as_array().unwrap() {
            let got = round_bonus_gold(
                r["score"].as_u64().unwrap() as u32,
                r["rankIdx"].as_u64().unwrap() as u32,
                r["mult"].as_f64().unwrap(),
            );
            assert!(
                approx(got, r["gold"].as_f64().unwrap(), 1e-9),
                "roundBonus {r}"
            );
        }
        for r in v["tierGold"].as_array().unwrap() {
            let got = tier_gold(r["n"].as_u64().unwrap() as u32, r["mult"].as_f64().unwrap());
            assert!(
                approx(got, r["gold"].as_f64().unwrap(), 1e-9),
                "tierGold {r}"
            );
        }
        for r in v["hoardLevel"].as_array().unwrap() {
            let got = hoard_level(r["gold"].as_f64().unwrap());
            assert!(
                approx(got, r["level"].as_f64().unwrap(), 1e-9),
                "hoardLevel {r}"
            );
        }
    }

    #[test]
    fn gold_mult_reproduces_every_vector() {
        let v = vectors();
        for r in v["goldMult"].as_array().unwrap() {
            let c = &r["counts"];
            let got = gold_mult(
                c["items"].as_u64().unwrap() as u32,
                c["mastered"].as_u64().unwrap() as u32,
                c["heroes"].as_u64().unwrap() as u32,
                c["tiers"].as_u64().unwrap() as u32,
                c["bosses"].as_u64().unwrap() as u32,
            );
            let want = r["goldMult"].as_f64().unwrap();
            // goldMult uses HOARD_G^bosses (powi vs Math.pow): allow 1e-6 relative.
            assert!(
                approx(got, want, want.abs() * 1e-6 + 1e-9),
                "goldMult {} → {got} (want {want})",
                r["label"]
            );
        }
    }

    // The corrected LIVE accrual: every `roundGold` composition vector (combo resets on skip).
    #[test]
    fn accrue_round_reproduces_every_round_gold_vector() {
        let v = vectors();
        let arr = v["roundGold"].as_array().unwrap();
        assert!(
            arr.len() >= 20,
            "expected the roundGold composition vectors"
        );
        for r in arr {
            let plays: Vec<Play> = r["seq"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| match e.as_f64() {
                    Some(dt) => Play::Solve(dt),
                    None => Play::Skip, // the string "skip"
                })
                .collect();
            let got = accrue_round(
                r["target"].as_f64().unwrap(),
                r["mult"].as_f64().unwrap(),
                &plays,
            );
            assert!(
                approx(got, r["total"].as_f64().unwrap(), 1e-9),
                "roundGold {r} → {got}"
            );
        }
    }

    #[test]
    fn round_gold_resets_combo_on_skip_then_adds_bonus() {
        // Solve, SKIP, solve (master 5s, dt 0.5, mult 1): combo 1 then reset then 1 again.
        // q1 combo1: (2+round(4.5))*1.1 = 7*1.1 = 7.7 ; skip: 0 ; q3 combo1: 7.7 ; accrual = 15.4.
        // score = 2 solves, rank 22 → bonus (2+44)*1 = 46 ; total 61.4 → round 61.
        let plays = [Play::Solve(0.5), Play::Skip, Play::Solve(0.5)];
        assert_eq!(round_gold(5.0, 1.0, &plays, 2, 22), 61);
        // No plays, no score → no gold.
        assert_eq!(round_gold(5.0, 1.0, &[], 0, 0), 0);
    }
}
