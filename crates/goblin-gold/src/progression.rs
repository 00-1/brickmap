//! GG1 **unlock-chain + mastery** mechanics, re-implemented in Rust (full-port phase 2). The
//! topic graph and its gates come from the T229 `modes.json` (`unlock` + `masterSecs`); the
//! behaviour is re-authored from the live runtime (`isUnlocked` / the Initiation + Mastery
//! collectibles), not the JS:
//!
//! - **Initiation ("played"):** a topic is *initiated* once a run answers at least **half** its
//!   questions (`answered ≥ ⌈total·0.5⌉`). A spine topic (`unlock:{by:X}`) opens once **X is
//!   initiated**.
//! - **Mastery:** a run *masters* a topic with **no skips** AND **total time ≤ `masterSecs ×
//!   total`**. An off-spine topic (`unlock:{mastery:X}`) opens once **X is mastered**.
//! - **`is_unlocked`:** already-played → open; else the gate above; the first topic (`unlock:null`)
//!   is always open.
//!
//! The collector ladder / arena / events are later sub-phases; this is the core progression.

use serde::Deserialize;
use std::collections::HashSet;

/// The one-way-synced T229 modes export (id/name/masterSecs/unlock).
const MODES_JSON: &str = include_str!("../data/gg1/modes.json");

/// Answered at least this fraction of a round → the topic is "initiated" (unlocks its spine
/// successor). Mirrors the runtime `INIT_ANSWER_FRAC`.
const INIT_ANSWER_FRAC: f64 = 0.5;

/// How a topic unlocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unlock {
    /// Always open (the first topic).
    Always,
    /// Opens once the named topic is **initiated** (the spine `unlock:{by:X}`).
    Played(String),
    /// Opens once the named topic is **mastered** (the off-spine `unlock:{mastery:X}`).
    Mastered(String),
}

/// One topic's progression metadata.
#[derive(Clone, Debug)]
pub struct Mode {
    pub id: String,
    pub name: String,
    pub master_secs: f64,
    pub unlock: Unlock,
}

#[derive(Deserialize)]
struct UnlockRaw {
    #[serde(default)]
    by: Option<String>,
    #[serde(default)]
    mastery: Option<String>,
}

#[derive(Deserialize)]
struct ModeRaw {
    id: String,
    name: String,
    #[serde(rename = "masterSecs")]
    master_secs: f64,
    #[serde(default)]
    unlock: Option<UnlockRaw>,
}

/// The per-topic **prompt eyebrow** shown above each drill question (`modes.js eyebrow:`). Ported
/// straight from source — the field isn't in the export (presentation data, like `TOPIC_GLYPHS`).
/// Defaults to "solve ↓" for any unknown id (web's most-common eyebrow).
pub fn mode_eyebrow(id: &str) -> &'static str {
    match id {
        "halves" => "half of ↓",
        "times" => "product of ↓",
        "doubles" => "double ↓",
        "addsub" | "addsub2" => "solve ↓",
        "bonds" | "bonds2" => "fill the gap ↓",
        "placevalue" | "placevalue2" => "solve ↓",
        "fractionsof" | "fractionsof2" => "solve ↓",
        "percentages" | "percentages2" => "solve ↓",
        "fractions" | "fractions2" => "as a decimal ↓",
        "squares" => "square of ↓",
        "rounding" => "round ↓",
        "largermd" => "solve ↓",
        "metric" => "convert ↓",
        "sequences" => "continue the pattern ↓",
        "sequences2" => "evaluate the rule ↓",
        "scaling" => "same rule — in proportion ↓",
        "percentoff" | "partwhole" => "solve ↓",
        "balance" => "make both sides equal ↓",
        "lcmhcf" | "mean" => "solve ↓",
        "timegap" => "minutes between ↓",
        "ratioshare" => "solve ↓",
        "cubes" => "evaluate ↓",
        "money" => "answer in £ ↓",
        "digitsum" => "solve ↓",
        "roman" => "as a number ↓",
        "primes" => "next prime ↓",
        "pctup" => "new total ↓",
        "fdp" => "convert ↓",
        "bodmas" => "work it out ↓",
        "algebra" => "in → out ↓",
        "xtricks" => "use the trick ↓",
        "negatives" => "answer is 0 or more ↓",
        "area" => "work it out ↓",
        "volume" => "volume ↓",
        "angles" => "missing angle ↓",
        "mmr" => "find it ↓",
        "sdt" => "solve ↓",
        "factors" => "solve ↓",
        _ => "solve ↓",
    }
}

/// Load the topic graph (in `modes.json` order — importance/unlock order).
pub fn modes() -> Vec<Mode> {
    let raw: Vec<ModeRaw> = serde_json::from_str(MODES_JSON).expect("modes.json");
    raw.into_iter()
        .map(|m| {
            let unlock = match m.unlock {
                None => Unlock::Always,
                Some(u) => match (u.mastery, u.by) {
                    (Some(x), _) => Unlock::Mastered(x),
                    (_, Some(x)) => Unlock::Played(x),
                    _ => Unlock::Always,
                },
            };
            Mode {
                id: m.id,
                name: m.name,
                master_secs: m.master_secs,
                unlock,
            }
        })
        .collect()
}

/// The outcome of one drill round, enough to decide initiation + mastery.
#[derive(Clone, Copy, Debug)]
pub struct RunResult {
    /// Questions in the round.
    pub total: u32,
    /// Questions answered (not skipped).
    pub answered: u32,
    /// Sum of per-answer times (seconds).
    pub total_time_secs: f64,
}

impl RunResult {
    /// Skipped questions (`total − answered`).
    pub fn skips(&self) -> u32 {
        self.total.saturating_sub(self.answered)
    }

    /// Initiated = answered at least half the round (the spine-unlock trigger).
    pub fn initiated(&self) -> bool {
        self.answered >= (self.total as f64 * INIT_ANSWER_FRAC).ceil() as u32
    }

    /// Mastered = no skips AND total time within `master_secs × total` (the off-spine gate).
    pub fn masters(&self, master_secs: f64) -> bool {
        self.skips() == 0 && self.total_time_secs <= master_secs * self.total as f64
    }
}

/// Player progression: which topics are initiated ("played") and which are mastered.
#[derive(Default, Clone, Debug)]
pub struct Progress {
    played: HashSet<String>,
    mastered: HashSet<String>,
}

impl Progress {
    pub fn is_played(&self, id: &str) -> bool {
        self.played.contains(id)
    }
    pub fn is_mastered(&self, id: &str) -> bool {
        self.mastered.contains(id)
    }

    /// Is `m` unlocked? Already-played topics stay open; otherwise the unlock gate decides; the
    /// first topic is always open.
    pub fn is_unlocked(&self, m: &Mode) -> bool {
        if self.played.contains(&m.id) {
            return true;
        }
        match &m.unlock {
            Unlock::Always => true,
            Unlock::Played(x) => self.played.contains(x),
            Unlock::Mastered(x) => self.mastered.contains(x),
        }
    }

    /// Rebuild progression from the saved central `collected` map's keys (the GG1 keystone): a
    /// `mastery:<id>` key → mastered (+ played), an `init:<id>` key → played. Other keys (items,
    /// events, tiers) are ignored here. Bridges [`crate::save`] → progression on load.
    pub fn from_collected<'a>(keys: impl IntoIterator<Item = &'a str>) -> Progress {
        let mut p = Progress::default();
        for k in keys {
            if let Some(id) = k.strip_prefix("mastery:") {
                p.played.insert(id.to_string());
                p.mastered.insert(id.to_string());
            } else if let Some(id) = k.strip_prefix("init:") {
                p.played.insert(id.to_string());
            }
        }
        p
    }

    /// Fold a finished run into progression: initiation marks the topic played; a clean+fast run
    /// also marks it mastered (mastery implies no skips → all answered → initiated).
    pub fn record_run(&mut self, m: &Mode, run: &RunResult) {
        if run.initiated() {
            self.played.insert(m.id.clone());
        }
        if run.masters(m.master_secs) {
            self.played.insert(m.id.clone());
            self.mastered.insert(m.id.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(ms: &'a [Mode], id: &str) -> &'a Mode {
        ms.iter().find(|m| m.id == id).expect("mode")
    }

    #[test]
    fn loads_all_46_with_a_valid_unlock_graph() {
        let ms = modes();
        assert_eq!(ms.len(), 46);
        // halves is the always-open root.
        assert_eq!(get(&ms, "halves").unlock, Unlock::Always);
        // exactly one always-open topic (the single entry point).
        assert_eq!(ms.iter().filter(|m| m.unlock == Unlock::Always).count(), 1);
        // every gate points at a real topic (no dangling unlock).
        let ids: HashSet<&str> = ms.iter().map(|m| m.id.as_str()).collect();
        for m in &ms {
            match &m.unlock {
                Unlock::Played(x) | Unlock::Mastered(x) => {
                    assert!(
                        ids.contains(x.as_str()),
                        "{} unlock target {x} missing",
                        m.id
                    )
                }
                Unlock::Always => {}
            }
            assert!(m.master_secs > 0.0, "{} masterSecs", m.id);
        }
    }

    #[test]
    fn spine_gates_on_play_offspine_gates_on_mastery() {
        let ms = modes();
        let mut p = Progress::default();
        // From scratch only halves is open.
        assert!(p.is_unlocked(get(&ms, "halves")));
        assert!(!p.is_unlocked(get(&ms, "times"))); // unlock:{by:halves}
        assert!(!p.is_unlocked(get(&ms, "addsub2"))); // unlock:{mastery:addsub}

        // PLAY halves (init, but not fast) → its spine successor `times` opens…
        p.record_run(
            get(&ms, "halves"),
            &RunResult {
                total: 10,
                answered: 10,
                total_time_secs: 999.0,
            },
        );
        assert!(p.is_played("halves") && !p.is_mastered("halves"));
        assert!(p.is_unlocked(get(&ms, "times")), "spine opens on play");

        // …but an off-spine mastery gate needs MASTERY, not just play.
        // Play addsub (slowly): addsub2 still locked.
        p.record_run(
            get(&ms, "addsub"),
            &RunResult {
                total: 10,
                answered: 10,
                total_time_secs: 9999.0,
            },
        );
        assert!(p.is_played("addsub") && !p.is_mastered("addsub"));
        assert!(
            !p.is_unlocked(get(&ms, "addsub2")),
            "mastery gate not satisfied by mere play"
        );

        // Master addsub (clean + fast) → addsub2 opens.
        p.record_run(
            get(&ms, "addsub"),
            &RunResult {
                total: 10,
                answered: 10,
                total_time_secs: 0.0,
            },
        );
        assert!(p.is_mastered("addsub"));
        assert!(
            p.is_unlocked(get(&ms, "addsub2")),
            "mastery opens the off-spine topic"
        );
    }

    #[test]
    fn initiation_and_mastery_thresholds() {
        let m = Mode {
            id: "x".into(),
            name: "X".into(),
            master_secs: 4.0,
            unlock: Unlock::Always,
        };
        // init = answered >= ceil(total*0.5). total=7 → ceil(3.5)=4.
        assert!(!RunResult {
            total: 7,
            answered: 3,
            total_time_secs: 0.0
        }
        .initiated());
        assert!(RunResult {
            total: 7,
            answered: 4,
            total_time_secs: 0.0
        }
        .initiated());
        // mastery: no skips AND time <= masterSecs*total (4*10=40).
        assert!(RunResult {
            total: 10,
            answered: 10,
            total_time_secs: 40.0
        }
        .masters(m.master_secs));
        assert!(!RunResult {
            total: 10,
            answered: 10,
            total_time_secs: 40.01
        }
        .masters(m.master_secs)); // too slow
        assert!(!RunResult {
            total: 10,
            answered: 9,
            total_time_secs: 0.0
        }
        .masters(m.master_secs)); // a skip
    }

    #[test]
    fn the_whole_graph_is_satisfiable_no_deadlock() {
        // Repeatedly master every currently-unlocked topic (a perfect, instant run masters any
        // mode). If the graph is a well-formed DAG rooted at halves, this unlocks ALL 46.
        let ms = modes();
        let mut p = Progress::default();
        loop {
            let mut changed = false;
            for m in &ms {
                if p.is_unlocked(m) && !p.is_mastered(&m.id) {
                    p.record_run(
                        m,
                        &RunResult {
                            total: 10,
                            answered: 10,
                            total_time_secs: 0.0,
                        },
                    );
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        assert!(
            ms.iter().all(|m| p.is_unlocked(m)),
            "every topic must be reachable"
        );
        assert!(
            ms.iter().all(|m| p.is_mastered(&m.id)),
            "every topic must be masterable"
        );
    }
}
