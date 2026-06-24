//! GG1 **collector ladder** — the collect-N reward ladder of the metagame (full-port phase 3),
//! re-implemented in Rust over the T230/T232 data (`collectibles.json`). A player's *collected
//! count* earns ladder tiers at fixed thresholds; the top tier (the **capstone**) tracks the live
//! catalogue total and **must stay strictly below it to stay reachable** (GG1's `collector.test.js`
//! invariant — old unreachable tiers were dropped). We read the ladder from the export and verify
//! that invariant + the earning logic, rather than hardcoding the thresholds.
//!
//! The wider collectible *catalogue* (per-question Solved/Spark, ranks, milestones, events) is
//! generated/earned elsewhere; this module is the collect-N ladder specifically.

use serde::Deserialize;

/// The synced T230/T232 collectibles export (catalogue summary + the collector ladder).
const COLLECTIBLES_JSON: &str = include_str!("../data/gg1/collectibles.json");

/// One collect-N ladder tier: earned once the player's collected count reaches `n`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tier {
    pub id: String,
    pub n: u32,
    pub name: String,
    pub rarity: String,
}

#[derive(Deserialize)]
struct TierRaw {
    id: String,
    n: u32,
    name: String,
    rarity: String,
}

#[derive(Deserialize)]
struct LadderRaw {
    tiers: Vec<TierRaw>,
    // The export's own capstone/reachability fields — cross-checked against the derived values in
    // the tests (and reserved for the collector screen); not read by the lib API itself.
    #[allow(dead_code)]
    capstone: u32,
    #[serde(rename = "catalogTotal")]
    #[allow(dead_code)]
    catalog_total: u32,
    #[serde(rename = "capstoneReachable")]
    #[allow(dead_code)]
    capstone_reachable: bool,
}

#[derive(Deserialize)]
struct File {
    total: u32,
    #[serde(rename = "collectorLadder")]
    ladder: LadderRaw,
    // `categories` + `catalog` are present in the file but not needed here (ignored).
}

fn parse() -> File {
    serde_json::from_str(COLLECTIBLES_JSON).expect("collectibles.json")
}

/// The total number of collectibles in the catalogue (the capstone must stay below this).
pub fn catalogue_total() -> u32 {
    parse().total
}

/// The collect-N ladder, ascending by threshold `n`.
pub fn ladder() -> Vec<Tier> {
    let mut tiers: Vec<Tier> = parse()
        .ladder
        .tiers
        .into_iter()
        .map(|t| Tier {
            id: t.id,
            n: t.n,
            name: t.name,
            rarity: t.rarity,
        })
        .collect();
    tiers.sort_by_key(|t| t.n);
    tiers
}

/// The tiers a player with `collected` items has earned (those whose threshold `n ≤ collected`).
pub fn earned(collected: u32) -> Vec<Tier> {
    ladder().into_iter().filter(|t| t.n <= collected).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_loads_ascending_with_valid_rarities() {
        let l = ladder();
        assert!(!l.is_empty(), "the collector ladder must have tiers");
        // strictly ascending thresholds.
        for w in l.windows(2) {
            assert!(
                w[1].n > w[0].n,
                "ladder not ascending at {} → {}",
                w[0].n,
                w[1].n
            );
        }
        // each tier is a `collector:N` id matching its `n`, with a known rarity.
        for t in &l {
            assert_eq!(t.id, format!("collector:{}", t.n), "tier id/n mismatch");
            assert!(
                ["rare", "epic", "legendary", "mythic"].contains(&t.rarity.as_str()),
                "tier {} has odd rarity {}",
                t.n,
                t.rarity
            );
        }
    }

    // GG1's `collector.test.js` guard: the capstone (top tier) must stay STRICTLY BELOW the live
    // catalogue total, or it can never be earned. Verify the re-impl + the export agree.
    #[test]
    fn capstone_is_reachable_below_the_catalogue_total() {
        let f = parse();
        let total = f.total;
        let l = ladder();
        let capstone = l.last().expect("a capstone tier").n;
        assert_eq!(
            capstone, f.ladder.capstone,
            "capstone disagrees with the export"
        );
        assert_eq!(
            f.ladder.catalog_total, total,
            "catalogTotal must equal total"
        );
        assert!(
            capstone < total,
            "capstone {capstone} must be < catalogue total {total} (reachable)"
        );
        assert!(
            f.ladder.capstone_reachable,
            "export must flag the capstone reachable"
        );
    }

    #[test]
    fn earning_is_threshold_gated() {
        let l = ladder();
        let first = l.first().unwrap().n;
        let cap = l.last().unwrap().n;
        assert!(
            earned(first - 1).is_empty(),
            "below the first threshold → nothing earned"
        );
        assert_eq!(
            earned(first).len(),
            1,
            "exactly the first tier at its threshold"
        );
        assert_eq!(
            earned(u32::MAX).len(),
            l.len(),
            "everything earned past the capstone"
        );
        // The capstone is earned exactly at its threshold, not one short.
        assert!(!earned(cap - 1).iter().any(|t| t.n == cap));
        assert!(earned(cap).iter().any(|t| t.n == cap));
    }
}
