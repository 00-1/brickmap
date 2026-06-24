//! GG1 **Arena** — the bestiary + hero roster of the metagame (full-port phase 3), re-implemented
//! in Rust over the T230/T232 tuning export (`balance.json`). The Arena is where collected items
//! pay off: every catalogue entry carries a [`crate::catalogue::Boost`], so a player's effective
//! hero stats are their **base stats plus the boosts of everything they've collected** — the bridge
//! from the collectible catalogue to the fight.
//!
//! What the export pins down (re-implemented + verified here):
//! - the **bestiary**: 120 enemy tiers (ascending `def`, one of three types), grouped into regions
//!   of `regionSize`, with a **boss** ending each region;
//! - the **roster**: 12 heroes, each with the four base stats and a type;
//! - **effective hero stats** = base + Σ boosts over the player's collected items.
//!
//! What it **doesn't** carry (so it's NOT re-implemented — data-seam rule, no fabrication): the
//! actual **combat resolution** (how power/guard/speed/focus and the type triangle turn into a
//! win/loss, and the gold-formula bodies) lives only in the JS, not the export. We model the
//! combatants and the boost economy; the fight math awaits an export of those rules or the JS.

use crate::catalogue;
use serde::Deserialize;

/// The synced T230/T232 tuning export.
const BALANCE_JSON: &str = include_str!("../data/gg1/balance.json");

/// A combatant type — the Arena's rock-paper-scissors triangle (shared by heroes + enemies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Kind {
    Brawn,
    Arcane,
    Cunning,
}

/// The four hero stats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Power,
    Guard,
    Speed,
    Focus,
}

impl Stat {
    /// Parse the export's lowercase stat name (`"power"`…).
    pub fn parse(s: &str) -> Option<Stat> {
        Some(match s {
            "power" => Stat::Power,
            "guard" => Stat::Guard,
            "speed" => Stat::Speed,
            "focus" => Stat::Focus,
            _ => return None,
        })
    }
}

/// A hero's four stats. `i64` so boost sums over the whole catalogue can't overflow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Stats {
    pub power: i64,
    pub guard: i64,
    pub speed: i64,
    pub focus: i64,
}

impl Stats {
    /// Add `amount` to one stat (used to fold in boosts).
    fn add(&mut self, stat: Stat, amount: i64) {
        match stat {
            Stat::Power => self.power += amount,
            Stat::Guard => self.guard += amount,
            Stat::Speed => self.speed += amount,
            Stat::Focus => self.focus += amount,
        }
    }
}

/// A roster hero: base stats + type + how it unlocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hero {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub base: Stats,
    pub unlock_hint: String,
}

/// A bestiary enemy at tier `n` (1-based).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enemy {
    pub n: u32,
    pub name: String,
    pub kind: Kind,
    pub def: i64,
}

#[derive(Deserialize)]
struct HeroRaw {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: Kind,
    base: Stats,
    #[serde(rename = "unlockHint", default)]
    unlock_hint: String,
}

#[derive(Deserialize)]
struct EnemyRaw {
    n: u32,
    name: String,
    #[serde(rename = "type")]
    kind: Kind,
    def: i64,
}

#[derive(Deserialize)]
struct Enemies {
    #[serde(rename = "tierCount")]
    tier_count: u32,
    #[serde(rename = "regionSize")]
    region_size: u32,
    tiers: Vec<EnemyRaw>,
}

#[derive(Deserialize)]
struct Heroes {
    roster: Vec<HeroRaw>,
}

#[derive(Deserialize)]
struct File {
    enemies: Enemies,
    heroes: Heroes,
}

fn parse() -> File {
    serde_json::from_str(BALANCE_JSON).expect("balance.json")
}

/// The hero roster (export order).
pub fn roster() -> Vec<Hero> {
    parse()
        .heroes
        .roster
        .into_iter()
        .map(|h| Hero {
            id: h.id,
            name: h.name,
            kind: h.kind,
            base: h.base,
            unlock_hint: h.unlock_hint,
        })
        .collect()
}

/// The full bestiary (120 tiers, ascending difficulty).
pub fn bestiary() -> Vec<Enemy> {
    parse()
        .enemies
        .tiers
        .into_iter()
        .map(|e| Enemy {
            n: e.n,
            name: e.name,
            kind: e.kind,
            def: e.def,
        })
        .collect()
}

/// Total enemy tiers.
pub fn tier_count() -> u32 {
    parse().enemies.tier_count
}

/// Tiers per region (a boss ends each region).
pub fn region_size() -> u32 {
    parse().enemies.region_size
}

/// Is tier `n` a region-ending **boss**? (Every `region_size`-th tier.)
pub fn is_boss(n: u32) -> bool {
    n > 0 && n.is_multiple_of(region_size())
}

/// The boss tiers (one per region).
pub fn bosses() -> Vec<Enemy> {
    bestiary().into_iter().filter(|e| is_boss(e.n)).collect()
}

/// A hero's **effective** stats: their base plus the boosts of every collected item that targets
/// them. This is the catalogue → Arena bridge — collecting items literally powers up your heroes.
/// `collected` is the save's key set; unknown hero ids in `hero_id` yield `None`.
pub fn hero_stats<'a>(
    hero_id: &str,
    collected: impl IntoIterator<Item = &'a str>,
) -> Option<Stats> {
    let hero = roster().into_iter().find(|h| h.id == hero_id)?;
    let mut stats = hero.base;
    for item in catalogue::earned(collected) {
        if let Some(b) = item.boost {
            if b.hero == hero_id {
                if let Some(stat) = Stat::parse(&b.stat) {
                    stats.add(stat, b.amount);
                }
            }
        }
    }
    Some(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // The bestiary: exactly `tierCount` tiers, numbered 1..=tierCount in order, with non-decreasing
    // `def` (difficulty ramps), a known type each, and `regionSize` dividing the tier count.
    #[test]
    fn bestiary_is_a_well_formed_difficulty_ramp() {
        let b = bestiary();
        let tc = tier_count();
        let rs = region_size();
        assert_eq!(b.len() as u32, tc, "bestiary size == tierCount");
        assert_eq!(tc, 120);
        assert!(
            rs > 0 && tc.is_multiple_of(rs),
            "regionSize {rs} must divide tierCount {tc}"
        );
        for (i, e) in b.iter().enumerate() {
            assert_eq!(e.n, i as u32 + 1, "tiers numbered 1..=n in order");
            assert!(e.def > 0, "tier {} def must be positive", e.n);
        }
        for w in b.windows(2) {
            assert!(
                w[1].def >= w[0].def,
                "def must not decrease ({} → {})",
                w[0].def,
                w[1].def
            );
        }
    }

    // The roster: 12 distinct heroes, every base stat positive, a known type, balanced across the
    // three types (4 each).
    #[test]
    fn roster_is_twelve_balanced_heroes() {
        let r = roster();
        assert_eq!(r.len(), 12);
        let mut ids = HashSet::new();
        let (mut brawn, mut arcane, mut cunning) = (0, 0, 0);
        for h in &r {
            assert!(ids.insert(h.id.clone()), "duplicate hero id {}", h.id);
            let s = h.base;
            assert!(
                s.power > 0 && s.guard > 0 && s.speed > 0 && s.focus > 0,
                "{} must have all positive base stats",
                h.id
            );
            match h.kind {
                Kind::Brawn => brawn += 1,
                Kind::Arcane => arcane += 1,
                Kind::Cunning => cunning += 1,
            }
        }
        assert_eq!((brawn, arcane, cunning), (4, 4, 4), "four heroes per type");
    }

    // Bosses: exactly one per region, each at a multiple of regionSize, and `is_boss` agrees.
    #[test]
    fn bosses_end_each_region() {
        let tc = tier_count();
        let rs = region_size();
        let bosses = bosses();
        assert_eq!(bosses.len() as u32, tc / rs, "one boss per region");
        for b in &bosses {
            assert!(is_boss(b.n) && b.n.is_multiple_of(rs));
        }
        // The final tier is a boss; tier 1 is not.
        assert!(is_boss(tc));
        assert!(!is_boss(1));
        assert!(!is_boss(0));
    }

    // Effective stats: base with no items; collecting an item folds its boost into the targeted
    // hero's stat, and an item targeting a different hero leaves this one at base. Data-driven —
    // whatever the first boosting item happens to target.
    #[test]
    fn collected_items_boost_their_target_heros_stats() {
        // The first catalogue item that carries a boost (every entry does, but stay robust).
        let item = catalogue::catalog()
            .into_iter()
            .find(|c| c.boost.is_some())
            .expect("a boosting item exists");
        let b = item.boost.clone().unwrap();
        let stat = Stat::parse(&b.stat).expect("a known stat");

        let base = hero_stats(&b.hero, std::iter::empty()).expect("the boost's hero exists");
        assert_eq!(
            base,
            roster().into_iter().find(|h| h.id == b.hero).unwrap().base,
            "no items ⇒ base stats"
        );

        // Collecting the item lifts exactly its target stat by its amount; the others are untouched.
        let boosted = hero_stats(&b.hero, [item.id.as_str()]).unwrap();
        let mut want = base;
        want.add(stat, b.amount);
        assert_eq!(boosted, want, "the boost lifts {} by {}", b.stat, b.amount);

        // The same item does nothing for a hero it doesn't target.
        let other = roster().into_iter().find(|h| h.id != b.hero).unwrap();
        let other_stats = hero_stats(&other.id, [item.id.as_str()]).unwrap();
        assert_eq!(
            other_stats, other.base,
            "a {}-boost doesn't touch {}",
            b.hero, other.id
        );
    }

    #[test]
    fn unknown_hero_has_no_stats() {
        assert!(hero_stats("nobody", std::iter::empty()).is_none());
    }
}
