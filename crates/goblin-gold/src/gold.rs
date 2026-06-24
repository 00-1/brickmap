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
//! [`round_gold`] composes these into a round payout (faithful to `finish()`): combo rises with each
//! clean solve (it never resets here — the auto-accept drill has no wrong submissions).

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

/// The hoard "fullness" 0..1: `clamp((log10(1+gold) − log10(EMPTY)) / (log10(FULL) − log10(EMPTY)))`.
pub fn hoard_level(gold: f64) -> f64 {
    let g = cfg();
    let gold = gold.max(0.0);
    let lo = (g.gold_empty).log10();
    let span = (g.gold_full).log10() - lo;
    (((1.0 + gold).log10() - lo) / span).clamp(0.0, 1.0)
}

/// A round's gold payout (faithful to `main.js` `finish()`): for each cleanly-solved question pay
/// `question_gold(master_secs, dt, combo, mult)` with `combo` rising 1,2,3… across the round, then
/// add `round_bonus_gold(score, rank_idx, mult)`, and round to a whole coin. `mult` is the global
/// multiplier (compute via [`gold_mult`]). NOTE: `main.js` recomputes `mult` per question as awards
/// drop mid-round; the live drill batches awards at round end, so a single `mult` is used — a
/// minor difference in a payout figure, not a vectored invariant.
pub fn round_gold(
    master_secs: f64,
    clean_dts: &[f64],
    score: u32,
    rank_idx: u32,
    mult: f64,
) -> u64 {
    let mut earn = 0.0;
    for (i, &dt) in clean_dts.iter().enumerate() {
        let combo = (i + 1) as u32; // combo++ before each award → first clean solve is combo 1
        earn += question_gold(master_secs, dt, combo, mult);
    }
    earn += round_bonus_gold(score, rank_idx, mult);
    earn.round().max(0.0) as u64
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

    #[test]
    fn round_gold_pays_per_question_plus_bonus() {
        // Two fast clean solves (master 5s, dt 0.5) at mult 1, score 2, rank 22.
        // q1: (2+round(4.5))*(1.1)*1 = (2+5)*1.1 = 7.7 ; q2 combo2: 7*1.2 = 8.4 ; bonus (2+44)*1 = 46.
        // total = 7.7 + 8.4 + 46 = 62.1 → round 62.
        let g = round_gold(5.0, &[0.5, 0.5], 2, 22, 1.0);
        assert_eq!(g, 62);
        // No solves, no score → no gold.
        assert_eq!(round_gold(5.0, &[], 0, 0, 1.0), 0);
    }
}
