//! GG1 **collectible catalogue** — the full 2352-item reward set of the metagame (full-port phase
//! 3), re-implemented in Rust over the T230/T232 export (`collectibles.json`). The keystone insight
//! (see [`crate::save`]): a collectible's **`id` is exactly the key stored in the save's `collected`
//! map** — so "is it earned?" is simply "is its id collected?", and *awarding* one is inserting its
//! id. This module models the catalogue, proves its shape against the export's own
//! `categories`/`total` (mirroring GG1's `collectibles.test.js`), and re-implements the earning
//! rules **that the export actually carries**.
//!
//! What the export *does* pin down (so we re-implement earning, verified vs the catalogue):
//! - **Initiation** (`init:<mode>`, 46) — a run that answered ≥ half the round.
//! - **Flawless** (`flawless:<mode>`, 46) — a finished round with **no skips**.
//! - **Mastery** (`mastery:<mode>`, 46) — no skips **and** within `masterSecs × total` (progression).
//! - **Collector** (`collector:<n>`, 15) — by collected count (see [`crate::collector`]).
//! - **Milestone / gold** (`gold:<n>`) and **momentum** (`momentum:<n>`) — numeric thresholds the
//!   entries carry (`gold` / `momentum` fields).
//!
//! What it **doesn't** carry (count/structure-verified only here — earning awaits a thresholds
//! export or the JS, NOT fabricated): **Speed** tiers and **Rank** thresholds live only in the
//! `desc` prose, and **Solved/Spark/Events** plus the `meta`/`topics` milestones are awarded by
//! gameplay events the export doesn't quantify. Per the data-seam rule we never invent those
//! numbers; the catalogue still tracks the items (id ∈ `collected` decides earned).

use crate::progression::{Mode, RunResult};
use serde::Deserialize;
use std::collections::BTreeMap;

/// The synced T230/T232 collectibles export.
const COLLECTIBLES_JSON: &str = include_str!("../data/gg1/collectibles.json");

/// A collectible's gameplay category (the export's `cat`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub enum Category {
    Rank,
    Initiation,
    Flawless,
    Speed,
    Mastery,
    Solved,
    Spark,
    Milestone,
    Collector,
    Events,
}

/// A collectible's Arena boost: owning it lifts one hero's one stat by `amount` (see
/// [`crate::arena`]). Every catalogue entry carries one.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Boost {
    pub hero: String,
    pub stat: String,
    pub amount: i64,
}

/// One catalogue entry. `id` doubles as the save's `collected` key (the keystone).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Collectible {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub cat: Category,
    #[serde(rename = "modeId", default)]
    pub mode_id: Option<String>,
    pub desc: String,
    /// Collect-N threshold (Collector tiers only).
    #[serde(default)]
    pub n: Option<u32>,
    /// Gold threshold (gold-milestone entries only).
    #[serde(default)]
    pub gold: Option<u64>,
    /// Momentum threshold (momentum-milestone entries only).
    #[serde(default)]
    pub momentum: Option<u32>,
    /// The Arena stat boost this item grants (every entry has one).
    #[serde(default)]
    pub boost: Option<Boost>,
}

#[derive(Deserialize)]
struct File {
    total: u32,
    categories: BTreeMap<String, u32>,
    catalog: Vec<Collectible>,
}

fn parse() -> File {
    serde_json::from_str(COLLECTIBLES_JSON).expect("collectibles.json")
}

/// The whole catalogue (all 2352 entries, export order).
pub fn catalog() -> Vec<Collectible> {
    parse().catalog
}

/// The declared catalogue total (the export's `total`).
pub fn total() -> u32 {
    parse().total
}

/// The export's declared per-category counts (the data contract the catalogue must satisfy).
pub fn category_counts() -> BTreeMap<String, u32> {
    parse().categories
}

/// The keys a single finished run **awards**, among the categories the export pins down: a run that
/// answered ≥ half its round earns `init:<mode>`; a no-skip finish also earns `flawless:<mode>`; a
/// no-skip, within-`masterSecs` finish also earns `mastery:<mode>`. (Speed/Rank are intentionally
/// absent — their thresholds aren't in the export; see the module docs.) Each returned key is a real
/// catalogue id, so the caller can drop them straight into the save's `collected` map.
pub fn run_award_keys(mode: &Mode, run: &RunResult) -> Vec<String> {
    let mut keys = Vec::new();
    if run.initiated() {
        keys.push(format!("init:{}", mode.id));
    }
    // Flawless = the round was finished without skipping a question.
    if run.skips() == 0 {
        keys.push(format!("flawless:{}", mode.id));
    }
    if run.masters(mode.master_secs) {
        keys.push(format!("mastery:{}", mode.id));
    }
    keys
}

/// The gold-milestone keys earned at a given gold balance (`gold:<n>` entries whose threshold `≤
/// gold`). The threshold is the entry's own `gold` field, so this stays data-driven.
pub fn gold_milestones_earned(gold: u64) -> Vec<String> {
    catalog()
        .into_iter()
        .filter(|c| c.gold.is_some_and(|t| t <= gold))
        .map(|c| c.id)
        .collect()
}

/// The momentum-milestone keys earned at a given momentum (`momentum:<n>` entries whose threshold `≤
/// momentum`).
pub fn momentum_milestones_earned(momentum: u32) -> Vec<String> {
    catalog()
        .into_iter()
        .filter(|c| c.momentum.is_some_and(|t| t <= momentum))
        .map(|c| c.id)
        .collect()
}

/// The catalogue entries a player has earned: those whose `id` is in their `collected` keys (the
/// keystone — the save's map is the single source of truth).
pub fn earned<'a>(collected: impl IntoIterator<Item = &'a str>) -> Vec<Collectible> {
    let keys: std::collections::HashSet<&str> = collected.into_iter().collect();
    catalog()
        .into_iter()
        .filter(|c| keys.contains(c.id.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progression;
    use std::collections::HashSet;

    fn of_cat(cat: Category) -> Vec<Collectible> {
        catalog().into_iter().filter(|c| c.cat == cat).collect()
    }

    // The data contract (GG1's `collectibles.test.js`): the catalogue is exactly `total` entries,
    // the per-category tally matches the export's own `categories`, and every id is unique.
    #[test]
    fn catalogue_matches_its_declared_total_and_category_counts() {
        let all = catalog();
        let total = total();
        assert_eq!(all.len() as u32, total, "catalogue size must equal total");
        assert_eq!(total, 2352, "the synced export's known total");

        // Per-category counts must equal the declared `categories` map.
        let declared = category_counts();
        let mut tally: BTreeMap<String, u32> = BTreeMap::new();
        for c in &all {
            *tally.entry(format!("{:?}", c.cat)).or_default() += 1;
        }
        assert_eq!(tally, declared, "per-category counts must match the export");
        assert_eq!(
            declared.values().sum::<u32>(),
            total,
            "category counts must sum to total"
        );

        // Unique ids (each is a distinct save key).
        let mut ids = HashSet::new();
        for c in &all {
            assert!(
                ids.insert(c.id.clone()),
                "duplicate collectible id {}",
                c.id
            );
        }
    }

    // Initiation / Flawless / Mastery: exactly one per mode (46), ids `<prefix>:<modeId>`, every
    // modeId a real topic. These are the per-run awards we re-implement.
    #[test]
    fn per_mode_categories_cover_every_topic_once() {
        let mode_ids: HashSet<String> = progression::modes().into_iter().map(|m| m.id).collect();
        assert_eq!(mode_ids.len(), 46);
        for (cat, prefix) in [
            (Category::Initiation, "init"),
            (Category::Flawless, "flawless"),
            (Category::Mastery, "mastery"),
        ] {
            let entries = of_cat(cat);
            assert_eq!(entries.len(), 46, "{cat:?} must be one per mode");
            let mut covered = HashSet::new();
            for c in &entries {
                let m = c.mode_id.as_deref().expect("per-mode entry has a modeId");
                assert_eq!(c.id, format!("{prefix}:{m}"), "{cat:?} id/modeId mismatch");
                assert!(mode_ids.contains(m), "{cat:?} points at unknown mode {m}");
                assert!(covered.insert(m.to_string()), "{cat:?} duplicates mode {m}");
            }
            assert_eq!(
                covered, mode_ids,
                "{cat:?} must cover every topic exactly once"
            );
        }
    }

    // The Collector category = the numeric collect-N ladder (`collector.rs`) PLUS the named special
    // collectibles (the menagerie capstones). Cross-check: every ladder tier is a Collector entry,
    // and the only Collector entries beyond the ladder are the non-numeric specials.
    #[test]
    fn collector_category_is_the_ladder_plus_named_specials() {
        let entries = of_cat(Category::Collector);
        let ladder = crate::collector::ladder();
        let entry_ids: HashSet<&str> = entries.iter().map(|c| c.id.as_str()).collect();
        let ladder_ids: HashSet<&str> = ladder.iter().map(|t| t.id.as_str()).collect();

        // Every numeric ladder tier appears in the Collector category.
        assert!(
            ladder_ids.is_subset(&entry_ids),
            "ladder tiers must all be Collector entries"
        );
        // The extras are exactly the named (non-`collector:<n>`) specials.
        let extras: HashSet<&str> = entry_ids.difference(&ladder_ids).copied().collect();
        assert!(
            extras.iter().all(|id| id
                .strip_prefix("collector:")
                .is_some_and(|s| s.parse::<u32>().is_err())),
            "Collector extras beyond the ladder must be named specials, got {extras:?}"
        );
        assert_eq!(
            entries.len(),
            ladder.len() + extras.len(),
            "Collector = ladder tiers + named specials"
        );
    }

    // A perfect run awards init+flawless+mastery; a half-answered slow run only init; an all-skip
    // run awards nothing (and crucially NOT flawless). Every awarded key is a real catalogue id.
    #[test]
    fn run_awards_are_export_keys_and_gated_correctly() {
        let ids: HashSet<String> = catalog().into_iter().map(|c| c.id).collect();
        let m = Mode {
            id: "halves".into(),
            name: "Halves".into(),
            master_secs: 5.0,
            unlock: progression::Unlock::Always,
        };

        // Perfect, fast, clean run → all three.
        let perfect = run_award_keys(
            &m,
            &RunResult {
                total: 10,
                answered: 10,
                total_time_secs: 0.0,
            },
        );
        assert_eq!(
            perfect.iter().cloned().collect::<HashSet<_>>(),
            HashSet::from([
                "init:halves".to_string(),
                "flawless:halves".to_string(),
                "mastery:halves".to_string(),
            ])
        );
        for k in &perfect {
            assert!(
                ids.contains(k),
                "awarded key {k} is not a real catalogue id"
            );
        }

        // Clean but slow → init + flawless, no mastery.
        let slow = run_award_keys(
            &m,
            &RunResult {
                total: 10,
                answered: 10,
                total_time_secs: 9999.0,
            },
        );
        assert!(slow.contains(&"init:halves".to_string()));
        assert!(slow.contains(&"flawless:halves".to_string()));
        assert!(!slow.contains(&"mastery:halves".to_string()));

        // Half-answered (with skips) → init only (initiation needs ≥ half; flawless needs zero skips).
        let half = run_award_keys(
            &m,
            &RunResult {
                total: 10,
                answered: 5,
                total_time_secs: 0.0,
            },
        );
        assert_eq!(half, vec!["init:halves".to_string()]);

        // All skipped → nothing (and not flawless).
        let none = run_award_keys(
            &m,
            &RunResult {
                total: 10,
                answered: 0,
                total_time_secs: 0.0,
            },
        );
        assert!(none.is_empty(), "an all-skip round earns nothing");
    }

    // Gold + momentum milestones carry numeric thresholds and gate on them (data-driven).
    #[test]
    fn threshold_milestones_gate_on_their_export_values() {
        // Every gold milestone has a gold threshold; every momentum one a momentum threshold.
        for c in of_cat(Category::Milestone) {
            if c.id.starts_with("gold:") {
                assert!(c.gold.is_some(), "{} lacks a gold threshold", c.id);
            }
            if c.id.starts_with("momentum:") {
                assert!(c.momentum.is_some(), "{} lacks a momentum threshold", c.id);
            }
        }
        // Earning is monotonic in the balance: more gold ⇒ a superset of milestones.
        let some = gold_milestones_earned(5_000);
        let more = gold_milestones_earned(1_000_000);
        let some_set: HashSet<&String> = some.iter().collect();
        assert!(
            more.iter().collect::<HashSet<_>>().is_superset(&some_set),
            "a bigger balance must earn a superset of gold milestones"
        );
        assert!(more.len() >= some.len());
        // Below the smallest threshold, nothing is earned.
        assert!(
            gold_milestones_earned(0).is_empty(),
            "no gold ⇒ no gold milestones"
        );
    }

    // The keystone: `earned` is exactly the catalogue entries whose id is in the collected set.
    #[test]
    fn earned_is_the_collected_ids_intersected_with_the_catalogue() {
        let collected = [
            "init:halves",
            "mastery:addsub",
            "collector:25",
            "not-a-real-key",
        ];
        let got: HashSet<String> = earned(collected).into_iter().map(|c| c.id).collect();
        assert!(got.contains("init:halves"));
        assert!(got.contains("mastery:addsub"));
        assert!(got.contains("collector:25"));
        assert!(
            !got.contains("not-a-real-key"),
            "phantom keys aren't in the catalogue"
        );
        assert_eq!(got.len(), 3);
    }
}
