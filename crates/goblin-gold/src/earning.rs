//! GG1 **earning** — the rule that turns a finished round (and the player's running totals) into the
//! set of collectible keys awarded (full-port phase 3). Re-implemented in Rust from the live logic
//! (`gg1/dev/collectibles.js`), with the tunables in the T233 export (`earning.json`) and the whole
//! `{ctx → awarded keys}` behaviour **proven against `earning-vectors.json`** (same contract as the
//! transforms-vs-parity test). Every awarded id is a key in the save's `collected` map (the
//! keystone — see [`crate::save`]).
//!
//! The pieces, mirroring `collectibles.js`:
//! - [`rank_index`] maps `(score, total, time)` to a rank tier — accuracy brackets while imperfect,
//!   average-seconds brackets once perfect. Reaching tier `i` grants every rank `0..=i`.
//! - [`award`] is the per-round evaluator: ranks, initiation (≥ half answered), flawless (no skips),
//!   speed brackets (clean + fast), mastery (clean + within `masterSecs`), per-question solve/spark,
//!   and the games/modes/flawless meta-milestones.
//! - the count/threshold evaluators ([`collector_awards`], [`topics_awards`], [`meta_awards`],
//!   [`gold_awards`], [`momentum_awards`]).
//!
//! GG1 ctx invariant (honoured here): **`mistakes == skipped == total − answered`**. A *wrong* answer
//! still counts as answered — it lowers `score` (→ rank) and marks the question missed in `qmap`
//! (→ no solve/spark), but it is **not** a skip, so it doesn't break flawless/mastery.

use serde::Deserialize;
use std::cmp::Ordering;

/// The T233 earning tunables export.
const EARNING_JSON: &str = include_str!("../data/gg1/earning.json");

#[derive(Clone, Deserialize)]
struct SpeedTier {
    avg: f64,
}

#[derive(Clone, Deserialize)]
struct RankDef {
    key: String,
    name: String,
}

#[derive(Clone, Deserialize)]
struct Earning {
    #[serde(rename = "initAnswerFrac")]
    init_answer_frac: f64,
    spark: f64,
    speed: Vec<SpeedTier>,
    ranks: Vec<RankDef>,
    #[serde(rename = "metaGames")]
    meta_games: Vec<u32>,
    #[serde(rename = "topicsUnlock")]
    topics_unlock: Vec<u32>,
    #[serde(rename = "collectorTiers")]
    collector_tiers: Vec<u32>,
    #[serde(rename = "goldThresholds")]
    gold_thresholds: Vec<u64>,
    #[serde(rename = "momentumThresholds")]
    momentum_thresholds: Vec<u32>,
}

fn earning() -> Earning {
    serde_json::from_str(EARNING_JSON).expect("earning.json")
}

/// Map a round to a rank tier (0..=22). Re-implemented verbatim from `collectibles.js`'s
/// `rankIndex`: while imperfect (`score < total`) the tier is the accuracy bracket; once perfect it
/// is the average-seconds-per-answer bracket (faster → higher). Proven against every `rankIndex`
/// vector.
pub fn rank_index(score: u32, total: u32, time: f64) -> usize {
    let f = if total > 0 {
        score as f64 / total as f64
    } else {
        0.0
    };
    let avg = if total > 0 { time / total as f64 } else { 99.0 };
    if f < 1.0 {
        // imperfect: rank by accuracy
        if f < 0.35 {
            0
        } else if f < 0.5 {
            1
        } else if f < 0.62 {
            2
        } else if f < 0.74 {
            3
        } else if f < 0.85 {
            4
        } else if f < 0.95 {
            5
        } else {
            6
        }
    } else {
        // perfect: rank by average seconds per answer
        if avg > 6.5 {
            7
        } else if avg > 5.5 {
            8
        } else if avg > 4.8 {
            9
        } else if avg > 4.2 {
            10
        } else if avg > 3.7 {
            11
        } else if avg > 3.2 {
            12
        } else if avg > 2.8 {
            13
        } else if avg > 2.4 {
            14
        } else if avg > 2.1 {
            15
        } else if avg > 1.8 {
            16
        } else if avg > 1.55 {
            17
        } else if avg > 1.35 {
            18
        } else if avg > 1.18 {
            19
        } else if avg > 1.02 {
            20
        } else if avg > 0.88 {
            21
        } else {
            22
        }
    }
}

/// The display name of the rank at tier `idx` (e.g. 22 → "God-Hand"), if in range.
pub fn rank_name(idx: usize) -> Option<String> {
    earning().ranks.into_iter().nth(idx).map(|r| r.name)
}

/// One question's outcome within a round, for solve/spark awards.
#[derive(Clone, Debug)]
pub struct QSolve {
    /// The question prompt (the `solve:<mode>:<prompt>` key stem).
    pub prompt: String,
    /// Misses on this question (0 = clean).
    pub miss: u32,
    /// Solve time in seconds.
    pub t: f64,
}

/// The player's running totals that gate the meta-milestones.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunStats {
    pub games: u32,
    pub modes_cleared: u32,
    pub flawless: u32,
}

/// Everything a finished round needs to decide its awards (the `ctx` of `collectibles.js`).
#[derive(Clone, Debug)]
pub struct Ctx<'a> {
    pub mode_id: &'a str,
    pub master_secs: f64,
    pub total: u32,
    pub answered: u32,
    pub score: u32,
    pub total_time: f64,
    pub qmap: Vec<QSolve>,
    pub stats: RunStats,
}

impl Ctx<'_> {
    fn avg(&self) -> f64 {
        if self.total > 0 {
            self.total_time / self.total as f64
        } else {
            99.0
        }
    }
    /// Skips = mistakes = total − answered.
    fn mistakes(&self) -> u32 {
        self.total.saturating_sub(self.answered)
    }
}

/// The per-round award set (the `evaluate` of `collectibles.js`): ranks, initiation, flawless,
/// speed, mastery, per-question solve/spark, and the games/modes/flawless meta-milestones. Returns
/// the awarded collectible ids (order-independent — they're `collected` keys).
pub fn award(ctx: &Ctx) -> Vec<String> {
    let e = earning();
    let mode_count = crate::progression::modes().len() as u32;
    let mut out = Vec::new();
    let avg = ctx.avg();
    let clean = ctx.mistakes() == 0;

    // Ranks: reaching tier i grants every rank 0..=i.
    let idx = rank_index(ctx.score, ctx.total, ctx.total_time);
    for r in e.ranks.iter().take(idx + 1) {
        out.push(format!("rank:{}", r.key));
    }

    // Initiation: answered at least `initAnswerFrac` of the round.
    let init_need = (ctx.total as f64 * e.init_answer_frac).ceil() as u32;
    if ctx.answered >= init_need {
        out.push(format!("init:{}", ctx.mode_id));
    }
    // Flawless: finished with no skips.
    if clean {
        out.push(format!("flawless:{}", ctx.mode_id));
    }
    // Speed brackets: clean + average under the bracket.
    for (i, tier) in e.speed.iter().enumerate() {
        if clean && avg < tier.avg {
            out.push(format!("speed:{}:{}", ctx.mode_id, i));
        }
    }
    // Mastery: clean + total time within masterSecs × total.
    if clean && ctx.total_time <= ctx.master_secs * ctx.total as f64 {
        out.push(format!("mastery:{}", ctx.mode_id));
    }
    // Per-question solve / spark.
    for q in &ctx.qmap {
        if q.miss == 0 {
            out.push(format!("solve:{}:{}", ctx.mode_id, q.prompt));
            if q.t < e.spark {
                out.push(format!("spark:{}:{}", ctx.mode_id, q.prompt));
            }
        }
    }
    // Meta-milestones gated on running totals.
    if ctx.stats.modes_cleared >= mode_count {
        out.push("meta:allmodes".to_string());
    }
    if ctx.stats.flawless >= mode_count {
        out.push("meta:allflawless".to_string());
    }
    for &n in &e.meta_games {
        if ctx.stats.games >= n {
            out.push(format!("meta:games{n}"));
        }
    }
    out
}

/// Collector tiers earned at a given owned-item count (`collector:<n>` where `count ≥ n`).
pub fn collector_awards(count: u32) -> Vec<String> {
    earning()
        .collector_tiers
        .into_iter()
        .filter(|&n| count >= n)
        .map(|n| format!("collector:{n}"))
        .collect()
}

/// Topic-completion milestones: `topics:unlock<n>` (unlocked ≥ n), `topics:one100` (≥1 at 100%),
/// `topics:all100` (every topic at 100%).
pub fn topics_awards(unlocked: u32, complete: u32, total: u32) -> Vec<String> {
    let e = earning();
    let mut out = Vec::new();
    for n in e.topics_unlock {
        if unlocked >= n {
            out.push(format!("topics:unlock{n}"));
        }
    }
    if complete >= 1 {
        out.push("topics:one100".to_string());
    }
    if total > 0 && complete >= total {
        out.push("topics:all100".to_string());
    }
    out
}

/// Hero/arena meta-milestones derivable from the live counts: `meta:allheroes` once every hero is
/// unlocked. (The `meta:tier<n>` markers gate on collected `tier:<n>` keys, handled at award time.)
pub fn meta_awards(heroes_unlocked: u32, heroes_total: u32) -> Vec<String> {
    if heroes_total > 0 && heroes_unlocked >= heroes_total {
        vec!["meta:allheroes".to_string()]
    } else {
        Vec::new()
    }
}

/// Wealth milestones earned at a gold balance (`gold:<g>` where `total ≥ g`).
pub fn gold_awards(total: u64) -> Vec<String> {
    earning()
        .gold_thresholds
        .into_iter()
        .filter(|&g| total >= g)
        .map(|g| format!("gold:{g}"))
        .collect()
}

/// Momentum milestones earned at a high-water momentum (`momentum:<n>` where `best ≥ n`).
pub fn momentum_awards(best: u32) -> Vec<String> {
    earning()
        .momentum_thresholds
        .into_iter()
        .filter(|&n| best >= n)
        .map(|n| format!("momentum:{n}"))
        .collect()
}

/// Collation class of a char, low-to-high, approximating the Unicode Collation Algorithm's primary
/// ordering enough for these prompts: whitespace < symbols/punctuation < digits < letters. (Plain
/// code-point order would wrongly sort a digit before a math symbol like `√`.)
fn collation_class(c: char) -> u8 {
    if c.is_whitespace() {
        0
    } else if c.is_ascii_digit() {
        2
    } else if c.is_alphabetic() {
        3
    } else {
        1 // symbols / punctuation (×, −, ÷, √, ², £, △, …)
    }
}

/// Natural (numeric-aware, collation-aware) comparison of two prompts, matching JS
/// `localeCompare(_, {numeric:true})` closely enough to pick a round's first prompt: digit runs
/// compare by value; otherwise by collation class (so symbols precede digits, as the UCA does), then
/// by code point. Used only to reconstruct the export's solve/spark scenarios.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                let (cla, clb) = (collation_class(ca), collation_class(cb));
                if cla != clb {
                    return cla.cmp(&clb);
                }
                if cla == 2 {
                    // Both digit runs: compare numerically (then by length for leading zeros).
                    let run = |it: &mut std::iter::Peekable<std::str::Chars>| {
                        let mut s = String::new();
                        while let Some(&c) = it.peek() {
                            if c.is_ascii_digit() {
                                s.push(c);
                                it.next();
                            } else {
                                break;
                            }
                        }
                        s
                    };
                    let ra = run(&mut ai);
                    let rb = run(&mut bi);
                    let na = ra.trim_start_matches('0');
                    let nb = rb.trim_start_matches('0');
                    let ord = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
                    if ord != Ordering::Equal {
                        return ord;
                    }
                } else {
                    if ca != cb {
                        return ca.cmp(&cb);
                    }
                    ai.next();
                    bi.next();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashSet;

    const VECTORS_JSON: &str = include_str!("../data/gg1/earning-vectors.json");

    fn vectors() -> Value {
        serde_json::from_str(VECTORS_JSON).expect("earning-vectors.json")
    }

    fn set(v: &Value) -> HashSet<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect()
    }

    // Every rankIndex vector: rank_index reproduces the export's `idx` exactly.
    #[test]
    fn rank_index_reproduces_every_vector() {
        let v = vectors();
        let arr = v["rankIndex"].as_array().unwrap();
        assert!(arr.len() >= 700, "expected the full rankIndex grid");
        for r in arr {
            let score = r["score"].as_u64().unwrap() as u32;
            let total = r["total"].as_u64().unwrap() as u32;
            let time = r["time"].as_f64().unwrap();
            let idx = r["idx"].as_u64().unwrap() as usize;
            assert_eq!(
                rank_index(score, total, time),
                idx,
                "rank_index({score},{total},{time}) mismatch"
            );
        }
    }

    // The count/threshold evaluators reproduce their vector families exactly (as sets).
    #[test]
    fn collector_topics_meta_gold_momentum_match_vectors() {
        let v = vectors();
        for c in v["collector"].as_array().unwrap() {
            let got: HashSet<String> = collector_awards(c["count"].as_u64().unwrap() as u32)
                .into_iter()
                .collect();
            assert_eq!(got, set(&c["awarded"]), "collector {}", c["count"]);
        }
        for t in v["topics"].as_array().unwrap() {
            let got: HashSet<String> = topics_awards(
                t["unlock"].as_u64().unwrap() as u32,
                t["complete"].as_u64().unwrap() as u32,
                t["total"].as_u64().unwrap() as u32,
            )
            .into_iter()
            .collect();
            assert_eq!(got, set(&t["awarded"]), "topics {t}");
        }
        for m in v["meta"].as_array().unwrap() {
            let got: HashSet<String> = meta_awards(
                m["heroesUnlocked"].as_u64().unwrap() as u32,
                m["heroesTotal"].as_u64().unwrap() as u32,
            )
            .into_iter()
            .collect();
            assert_eq!(got, set(&m["awarded"]), "meta {m}");
        }
        for g in v["gold"].as_array().unwrap() {
            let got: HashSet<String> = gold_awards(g["total"].as_u64().unwrap())
                .into_iter()
                .collect();
            assert_eq!(got, set(&g["awarded"]), "gold {}", g["total"]);
        }
        for mo in v["momentum"].as_array().unwrap() {
            let got: HashSet<String> = momentum_awards(mo["best"].as_u64().unwrap() as u32)
                .into_iter()
                .collect();
            assert_eq!(got, set(&mo["awarded"]), "momentum {}", mo["best"]);
        }
    }

    // The per-round evaluator reproduces all 46 modes × 13 scenarios, reconstructing each scenario's
    // ctx exactly as the export generator (`tools/earning-export.js`) does.
    #[test]
    fn award_reproduces_every_per_mode_scenario() {
        let v = vectors();
        let per_mode = v["perMode"].as_object().unwrap();
        let modes = crate::progression::modes();

        for m in &modes {
            // N = the mode's generated question count; q.p = its natural-first prompt.
            let qs = crate::transforms::generate(&m.id);
            let n = qs.len() as u32;
            let h = n.div_ceil(2);
            let first_prompt = qs
                .iter()
                .map(|(p, _)| p.clone())
                .min_by(|a, b| natural_cmp(a, b))
                .expect("a prompt");
            let ms = m.master_secs;

            // The 13 scenarios, verbatim from the generator's `scen` table.
            // (name, answered, score, avg, qmap, stats).
            type ScenRow = (&'static str, u32, u32, f64, Vec<QSolve>, RunStats);
            let scen: Vec<ScenRow> = vec![
                (
                    "allSkip",
                    0,
                    0,
                    99.0,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "halfAnswered",
                    h,
                    h,
                    2.0,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "fullCleanFast",
                    n,
                    n,
                    1.0,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "fullCleanMid",
                    n,
                    n,
                    ms * 0.99,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "fullCleanSlow",
                    n,
                    n,
                    ms + 1.0,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "oneWrongNoSkip",
                    n,
                    n.saturating_sub(1),
                    2.0,
                    vec![QSolve {
                        prompt: first_prompt.clone(),
                        miss: 1,
                        t: 2.0,
                    }],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "speedQuick",
                    n,
                    n,
                    2.1,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "speedSwift",
                    n,
                    n,
                    1.7,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "speedBlazing",
                    n,
                    n,
                    1.3,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "speedLightning",
                    n,
                    n,
                    1.0,
                    vec![],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "solveClean",
                    n,
                    n,
                    2.0,
                    vec![QSolve {
                        prompt: first_prompt.clone(),
                        miss: 0,
                        t: 2.0,
                    }],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "solveSpark",
                    n,
                    n,
                    2.0,
                    vec![QSolve {
                        prompt: first_prompt.clone(),
                        miss: 0,
                        t: 1.0,
                    }],
                    RunStats {
                        games: 1,
                        modes_cleared: 1,
                        flawless: 0,
                    },
                ),
                (
                    "metaGames100",
                    n,
                    n,
                    2.0,
                    vec![],
                    RunStats {
                        games: 100,
                        modes_cleared: modes.len() as u32,
                        flawless: modes.len() as u32,
                    },
                ),
            ];

            let want = &per_mode[&m.id];
            for (i, (name, answered, score, avg, qmap, stats)) in scen.into_iter().enumerate() {
                let ctx = Ctx {
                    mode_id: &m.id,
                    master_secs: ms,
                    total: n,
                    answered,
                    score,
                    total_time: avg * n as f64,
                    qmap,
                    stats,
                };
                let got: HashSet<String> = award(&ctx).into_iter().collect();
                let exp = set(&want[i]["awarded"]);
                assert_eq!(want[i]["scen"].as_str().unwrap(), name, "scenario order");
                assert_eq!(
                    got,
                    exp,
                    "mode {} scenario {name}: awarded mismatch\n got: {:?}\n exp: {:?}",
                    m.id,
                    {
                        let mut g: Vec<_> = got.iter().cloned().collect();
                        g.sort();
                        g
                    },
                    {
                        let mut e: Vec<_> = exp.iter().cloned().collect();
                        e.sort();
                        e
                    }
                );
            }
        }
    }
}
