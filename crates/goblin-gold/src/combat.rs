//! GG1 **Arena** combat resolution (T233b-combat) — the 3v3 team battle, re-implemented in Rust
//! from `main.js`'s `finishBattle` ← `Enemies.teamBattle` and **proven against
//! `combat-vectors.json`**. Pure logic (serde_json only); the Arena *screen* + the on-win grant live
//! in [`crate::app`].
//!
//! The data (`combat.json`) is consumed as-is per the data-seam rule — the tier ladder, the
//! per-tier enemy combatants (pre-calibrated `{atk,hp,spd,type}`), the loot, and the loot-stat
//! boosts. We do NOT re-derive the `FOE_BUDGET` calibration; we reproduce the **resolution**:
//!
//! - **`effective_stats(hero, collected)`** = base + Σ catalogue boosts
//!   ([`crate::arena::hero_stats`]) + Σ combat loot boosts for owned `loot:*` ids. (Loot ids are NOT
//!   in the catalogue, so there's no double-count — verified: `full` = base + all-catalogue +
//!   all-loot, `drillAll` = base + all-catalogue.)
//! - **`hero_combatant(stats)`** → `{atk = power + 0.8·focus, hp = HB + guard·HG + power·HPP, spd =
//!   speed}` (the constants from `combat.json`).
//! - **`matchup(a, t)`** — the type triangle Brawn▸Cunning▸Arcane▸Brawn (each beats the next):
//!   `advantage`/`same`/`disadvantage` multipliers from the constants. The *cycle direction* isn't a
//!   constant in the export, but the turn-by-turn `teamBattleLog` pins it exactly (and the test
//!   proves it).
//! - **`simulate`** — ords: party `0..2`, foes `100..102`; turn order = spd desc, ord asc (fixed for
//!   the battle); each round every still-living actor in that order picks a target on the other side
//!   by **max matchup → lowest hp → lowest ord** and deals `max(1, round(atk·matchup))`; repeat until
//!   one side is empty (or a 4000-round guard). Result: `{win, heroes_alive, foes_alive, rounds}`.

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use crate::arena::{self, Kind, Stat, Stats};

/// The synced 3v3 Arena export (ladder / enemy teams / loot / loot boosts / constants).
const COMBAT_JSON: &str = include_str!("../data/gg1/combat.json");

/// JS `Math.round` (round half **up**, toward +∞) — matches the export's `round(atk·matchup)`.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[derive(Deserialize)]
struct MatchupMul {
    same: f64,
    advantage: f64,
    disadvantage: f64,
}

#[derive(Deserialize)]
struct HeroConsts {
    #[serde(rename = "HB")]
    hb: f64,
    #[serde(rename = "HG")]
    hg: f64,
    #[serde(rename = "HPP")]
    hpp: f64,
}

#[derive(Deserialize)]
struct Constants {
    #[serde(rename = "tierCount")]
    tier_count: u32,
    #[serde(rename = "regionSize")]
    region_size: u32,
    matchup: MatchupMul,
    hero: HeroConsts,
}

#[derive(Deserialize)]
struct LootBoost {
    id: String,
    hero: String,
    stat: String,
    amount: i64,
}

/// A pre-calibrated enemy combatant (consumed as-is from `enemyTeams`).
#[derive(Deserialize, Clone)]
struct FoeRaw {
    atk: f64,
    hp: f64,
    spd: f64,
    #[serde(rename = "type")]
    kind: Kind,
}

#[derive(Deserialize)]
struct CombatFile {
    constants: Constants,
    #[serde(rename = "lootBoosts")]
    loot_boosts: Vec<LootBoost>,
    #[serde(rename = "enemyTeams")]
    enemy_teams: BTreeMap<String, Vec<FoeRaw>>,
    loot: BTreeMap<String, Vec<String>>,
}

fn parse() -> CombatFile {
    serde_json::from_str(COMBAT_JSON).expect("combat.json")
}

/// The hero stat-atk weight on focus (from `constants.hero.atk = "power + 0.8*focus"`).
const FOCUS_ATK_WEIGHT: f64 = 0.8;

/// Total tiers in the ladder.
pub fn tier_count() -> u32 {
    parse().constants.tier_count
}

/// Tiers per region (a boss ends each region).
pub fn region_size() -> u32 {
    parse().constants.region_size
}

/// The loot ids granted for clearing tier `n` (1-based), if any.
pub fn loot_for(n: u32) -> Vec<String> {
    parse()
        .loot
        .get(&n.to_string())
        .cloned()
        .unwrap_or_default()
}

/// A hero's **effective** stats for the Arena: base + catalogue boosts (the existing
/// [`crate::arena::hero_stats`] bridge) + the combat loot boosts of every owned `loot:*` id.
/// `None` for an unknown hero id.
pub fn effective_stats(hero_id: &str, collected: &HashSet<&str>) -> Option<Stats> {
    let mut stats = arena::hero_stats(hero_id, collected.iter().copied())?;
    for lb in parse().loot_boosts {
        if lb.hero == hero_id && collected.contains(lb.id.as_str()) {
            if let Some(stat) = Stat::parse(&lb.stat) {
                add_stat(&mut stats, stat, lb.amount);
            }
        }
    }
    Some(stats)
}

fn add_stat(s: &mut Stats, stat: Stat, amount: i64) {
    match stat {
        Stat::Power => s.power += amount,
        Stat::Guard => s.guard += amount,
        Stat::Speed => s.speed += amount,
        Stat::Focus => s.focus += amount,
    }
}

/// A unit in the fight: its type, attack, *current* hp, speed, and turn-order key (`ord`).
#[derive(Clone, Debug)]
struct Combatant {
    kind: Kind,
    atk: f64,
    hp: f64,
    spd: f64,
    ord: u32,
}

/// Turn a hero's effective stats into a combatant (the `constants.hero` formulas).
pub fn hero_combatant(stats: Stats) -> (f64, f64, f64) {
    let c = parse().constants.hero;
    let atk = stats.power as f64 + FOCUS_ATK_WEIGHT * stats.focus as f64;
    let hp = c.hb + stats.guard as f64 * c.hg + stats.power as f64 * c.hpp;
    let spd = stats.speed as f64;
    (atk, hp, spd)
}

/// The type-triangle multiplier for `attacker` hitting `target` — Brawn▸Cunning▸Arcane▸Brawn (each
/// type has the `advantage` over the next in the cycle; the reverse is `disadvantage`; same type is
/// neutral). Multipliers from `combat.json` constants.
fn matchup(attacker: Kind, target: Kind, m: &MatchupMul) -> f64 {
    use Kind::*;
    if attacker == target {
        m.same
    } else if matches!(
        (attacker, target),
        (Brawn, Cunning) | (Cunning, Arcane) | (Arcane, Brawn)
    ) {
        m.advantage
    } else {
        m.disadvantage
    }
}

/// The outcome of a resolved battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattleResult {
    pub win: bool,
    pub heroes_alive: usize,
    pub foes_alive: usize,
    pub rounds: u32,
}

/// The 4000-round guard from the export (a deadlock backstop; real fights end in a few rounds).
const ROUND_GUARD: u32 = 4000;

/// Resolve a fight between `party` (ords 0..2) and `foes` (ords 100..102). Both are consumed by
/// value (their hp is mutated as the fight runs). Pure — no questions, no RNG.
fn simulate(mut party: Vec<Combatant>, mut foes: Vec<Combatant>, m: &MatchupMul) -> BattleResult {
    // Fixed turn order: spd desc, ord asc. Stored as (side, index) so we can reach the live hp.
    // side 0 = party, 1 = foes.
    let mut order: Vec<(u8, usize)> = party
        .iter()
        .enumerate()
        .map(|(i, _)| (0u8, i))
        .chain(foes.iter().enumerate().map(|(i, _)| (1u8, i)))
        .collect();
    order.sort_by(|&a, &b| {
        let (sa, sb) = (unit(&party, &foes, a), unit(&party, &foes, b));
        sb.spd
            .partial_cmp(&sa.spd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(sa.ord.cmp(&sb.ord))
    });

    let mut rounds = 0u32;
    'battle: loop {
        rounds += 1;
        for &(side, idx) in &order {
            // The actor must still be alive to act.
            let actor = unit(&party, &foes, (side, idx)).clone();
            if actor.hp <= 0.0 {
                continue;
            }
            // Pick the best target on the OTHER side; if none live, the battle is over.
            let targets = if side == 0 { &foes } else { &party };
            let Some(tgt) = pick_target(actor.kind, targets, m) else {
                break 'battle;
            };
            let dmg = js_round(actor.atk * matchup(actor.kind, targets[tgt].kind, m)).max(1.0);
            let defenders = if side == 0 { &mut foes } else { &mut party };
            defenders[tgt].hp -= dmg;
        }
        let party_live = party.iter().any(|u| u.hp > 0.0);
        let foes_live = foes.iter().any(|u| u.hp > 0.0);
        if !party_live || !foes_live || rounds >= ROUND_GUARD {
            break;
        }
    }

    let heroes_alive = party.iter().filter(|u| u.hp > 0.0).count();
    let foes_alive = foes.iter().filter(|u| u.hp > 0.0).count();
    BattleResult {
        win: heroes_alive > 0,
        heroes_alive,
        foes_alive,
        rounds,
    }
}

/// Borrow the combatant addressed by `(side, idx)`.
fn unit<'a>(
    party: &'a [Combatant],
    foes: &'a [Combatant],
    (side, idx): (u8, usize),
) -> &'a Combatant {
    if side == 0 {
        &party[idx]
    } else {
        &foes[idx]
    }
}

/// Choose a living target: **max matchup → lowest hp → lowest ord**. `None` if all are dead.
fn pick_target(attacker: Kind, targets: &[Combatant], m: &MatchupMul) -> Option<usize> {
    targets
        .iter()
        .enumerate()
        .filter(|(_, t)| t.hp > 0.0)
        .max_by(|(_, a), (_, b)| {
            let (ma, mb) = (matchup(attacker, a.kind, m), matchup(attacker, b.kind, m));
            ma.partial_cmp(&mb)
                .unwrap_or(std::cmp::Ordering::Equal)
                // higher matchup wins; then LOWER hp; then LOWER ord (so reverse those).
                .then(b.hp.partial_cmp(&a.hp).unwrap_or(std::cmp::Ordering::Equal))
                .then(b.ord.cmp(&a.ord))
        })
        .map(|(i, _)| i)
}

/// Resolve the full Arena battle: `party` (≤3 hero ids) vs the enemy team at `tier` (1-based), with
/// the player's `collected` keys driving each hero's effective stats. `None` if `tier` has no team.
pub fn team_battle(party: &[&str], tier: u32, collected: &HashSet<&str>) -> Option<BattleResult> {
    let data = parse();
    let foes_raw = data.enemy_teams.get(&tier.to_string())?;
    let roster = arena::roster();
    let party_units: Vec<Combatant> = party
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let kind = roster.iter().find(|h| h.id == *id)?.kind;
            let stats = effective_stats(id, collected)?;
            let (atk, hp, spd) = hero_combatant(stats);
            Some(Combatant {
                kind,
                atk,
                hp,
                spd,
                ord: i as u32,
            })
        })
        .collect();
    let foe_units: Vec<Combatant> = foes_raw
        .iter()
        .enumerate()
        .map(|(i, f)| Combatant {
            kind: f.kind,
            atk: f.atk,
            hp: f.hp,
            spd: f.spd,
            ord: 100 + i as u32,
        })
        .collect();
    Some(simulate(party_units, foe_units, &data.constants.matchup))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const VECTORS_JSON: &str = include_str!("../data/gg1/combat-vectors.json");

    fn vectors() -> Value {
        serde_json::from_str(VECTORS_JSON).expect("combat-vectors.json")
    }

    /// The labelled owned-sets the vectors use: `empty` = nothing, `drillAll` = the whole catalogue
    /// (every drill/metagame collectible — loot ids aren't in it), `full` = the catalogue + every
    /// combat loot id. Returns the owned key set for a label.
    fn owned_set(label: &str) -> HashSet<String> {
        match label {
            "empty" => HashSet::new(),
            "drillAll" => crate::catalogue::catalog()
                .into_iter()
                .map(|c| c.id)
                .collect(),
            "full" => {
                let mut s: HashSet<String> = crate::catalogue::catalog()
                    .into_iter()
                    .map(|c| c.id)
                    .collect();
                for lb in parse().loot_boosts {
                    s.insert(lb.id);
                }
                s
            }
            other => panic!("unknown owned-set label {other}"),
        }
    }

    fn as_stats(v: &Value) -> Stats {
        Stats {
            power: v["power"].as_i64().unwrap(),
            guard: v["guard"].as_i64().unwrap(),
            speed: v["speed"].as_i64().unwrap(),
            focus: v["focus"].as_i64().unwrap(),
        }
    }

    /// `hero_combatant` reproduces the constants' atk/hp/spd formulas exactly (5 vectors).
    #[test]
    fn hero_combatant_matches_vectors() {
        for hc in vectors()["heroCombatant"].as_array().unwrap() {
            let stats = as_stats(&hc["stats"]);
            let (atk, hp, spd) = hero_combatant(stats);
            assert_eq!(atk, hc["atk"].as_f64().unwrap(), "atk for {stats:?}");
            assert_eq!(hp, hc["hp"].as_f64().unwrap(), "hp for {stats:?}");
            assert_eq!(spd, hc["spd"].as_f64().unwrap(), "spd for {stats:?}");
        }
    }

    /// `effective_stats` (base + catalogue + loot) reproduces all 36 effectiveStats vectors across
    /// the empty / drillAll / full owned-sets — proving the boost composition + no double-count.
    #[test]
    fn effective_stats_matches_vectors() {
        for es in vectors()["effectiveStats"].as_array().unwrap() {
            let hero = es["hero"].as_str().unwrap();
            let label = es["own"].as_str().unwrap();
            let owned = owned_set(label);
            let keys: HashSet<&str> = owned.iter().map(String::as_str).collect();
            let got = effective_stats(hero, &keys).expect("known hero");
            assert_eq!(
                got,
                as_stats(&es["stats"]),
                "effective_stats({hero}, {label})"
            );
        }
    }

    /// The full sim reproduces every headline `teamBattle` vector (240): win / heroesAlive /
    /// foesAlive / rounds. This also pins the matchup-cycle direction.
    #[test]
    fn team_battle_matches_all_vectors() {
        for tb in vectors()["teamBattle"].as_array().unwrap() {
            let party: Vec<String> = tb["party"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect();
            let party_refs: Vec<&str> = party.iter().map(String::as_str).collect();
            let tier = tb["tier"].as_u64().unwrap() as u32;
            let owned = owned_set(tb["own"].as_str().unwrap());
            let keys: HashSet<&str> = owned.iter().map(String::as_str).collect();
            let r = team_battle(&party_refs, tier, &keys).expect("tier has a team");
            assert_eq!(r.win, tb["win"].as_bool().unwrap(), "win {party:?} t{tier}");
            assert_eq!(
                r.heroes_alive,
                tb["heroesAlive"].as_u64().unwrap() as usize,
                "heroesAlive {party:?} t{tier}"
            );
            assert_eq!(
                r.foes_alive,
                tb["foesAlive"].as_u64().unwrap() as usize,
                "foesAlive {party:?} t{tier}"
            );
            assert_eq!(
                r.rounds,
                tb["rounds"].as_u64().unwrap() as u32,
                "rounds {party:?} t{tier}"
            );
        }
    }

    /// The turn-by-turn log fixture: re-running its exact party/tier/owned-set reproduces the
    /// headline result (win/alive/rounds) — the per-turn damage is what pins the matchup direction,
    /// and the headline can only match if every turn did.
    #[test]
    fn team_battle_log_fixture_matches() {
        let tl = &vectors()["teamBattleLog"];
        let party: Vec<String> = tl["party"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        let party_refs: Vec<&str> = party.iter().map(String::as_str).collect();
        let tier = tl["tier"].as_u64().unwrap() as u32;
        let owned = owned_set(tl["own"].as_str().unwrap());
        let keys: HashSet<&str> = owned.iter().map(String::as_str).collect();
        let r = team_battle(&party_refs, tier, &keys).expect("tier team");
        assert_eq!(r.win, tl["win"].as_bool().unwrap());
        assert_eq!(r.heroes_alive, tl["heroesAlive"].as_u64().unwrap() as usize);
        assert_eq!(r.foes_alive, tl["foesAlive"].as_u64().unwrap() as usize);
        assert_eq!(r.rounds, tl["rounds"].as_u64().unwrap() as u32);
    }

    /// Sanity: the ladder + loot are well-formed (every tier 1..=tierCount has an enemy team).
    #[test]
    fn ladder_is_complete() {
        let n = tier_count();
        assert_eq!(n, 120);
        for t in 1..=n {
            assert!(
                parse().enemy_teams.contains_key(&t.to_string()),
                "tier {t} missing an enemy team"
            );
        }
    }
}
