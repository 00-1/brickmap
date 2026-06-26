//! GG1 **Arena** combat resolution (T233b-combat, **redesigned model** `b49e62b`) — the 3v3 team
//! battle, re-implemented in Rust from `main.js`/`Enemies.teamBattle` and **proven against
//! `combat-vectors.json`**. Pure logic; the Arena *screen* + on-win grant live in [`crate::app`] /
//! [`crate::save`].
//!
//! Data (`combat.json`) is consumed as-is per the data-seam rule — the tier ladder, the per-tier
//! enemy combatants (pre-calibrated `{pow,grd,spd,foc,hp,type}` from the foe-budget curve), the loot
//! and the loot-stat boosts; the `constants.combat` block + `_resolution` doc are the authoritative
//! recipe. We reproduce the **resolution** (all four stats now have one distinct role):
//!
//! - **`effective_stats(hero, collected)`** = base + Σ catalogue boosts
//!   ([`crate::arena::hero_stats`]) + Σ combat loot boosts for owned `loot:*` ids (loot ids aren't in
//!   the catalogue → no double-count).
//! - **`hero_combatant(stats)`** → `{pow:power, grd:guard, spd:speed, foc:focus, hp:HP_FLAT}` — every
//!   hero has the same flat HP; the stats drive damage/mitigation/speed instead.
//! - **stat roles:** PWR = `round(pow·matchup)` typed damage · FOC = `round(foc·FOC_FLAT)` flat
//!   damage (matchup-independent floor) · GRD = per-hit mitigation `round(grd·MIT)` (min 1 gets
//!   through) · SPD = a one-time **opening strike** `round(spd·SPD_ALPHA·matchup)` for any HERO that
//!   outspeeds its target, *before* the rounds.
//! - **`simulate`** — ords party `0..2` / foes `100..102`; order = spd desc, ord asc (fixed). (1)
//!   opening strikes in order; (2) rounds: every living actor targets the other side by **max
//!   matchup → lowest hp → lowest ord**, dealing `max(1, round(pow·mu)+round(foc·FOC_FLAT) −
//!   round(tgt.grd·MIT))`. Repeat until a side is empty (4000-round guard). Result `{win, heroes_alive,
//!   foes_alive, rounds}` (win = any hero alive; `rounds` counts the main rounds, not the opening).

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use crate::arena::{self, Kind, Stat, Stats};

/// The synced 3v3 Arena export (ladder / enemy teams / loot / loot boosts / constants).
const COMBAT_JSON: &str = include_str!("../data/gg1/combat.json");

/// JS `Math.round` (round half **up**, toward +∞) — matches the export's `round(…)`.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[derive(Deserialize, Clone)]
struct MatchupMul {
    same: f64,
    advantage: f64,
    disadvantage: f64,
}

/// The redesigned combat constants (`constants.combat`).
#[derive(Deserialize, Clone)]
struct CombatConsts {
    #[serde(rename = "HP_FLAT")]
    hp_flat: f64,
    #[serde(rename = "MIT")]
    mit: f64,
    #[serde(rename = "FOC_FLAT")]
    foc_flat: f64,
    #[serde(rename = "SPD_ALPHA")]
    spd_alpha: f64,
}

#[derive(Deserialize, Clone)]
struct Constants {
    #[serde(rename = "tierCount")]
    tier_count: u32,
    #[serde(rename = "regionSize")]
    region_size: u32,
    matchup: MatchupMul,
    combat: CombatConsts,
}

#[derive(Deserialize)]
struct LootBoost {
    id: String,
    hero: String,
    stat: String,
    amount: i64,
}

/// A pre-calibrated enemy combatant (consumed as-is from `enemyTeams`; `grd`/`foc` are 0).
#[derive(Deserialize, Clone)]
struct FoeRaw {
    pow: f64,
    grd: f64,
    spd: f64,
    foc: f64,
    hp: f64,
    #[serde(rename = "type")]
    kind: Kind,
}

/// A hero's unlock predicate over the save's `collected` keystone (`compileUnlock`): a single key, a
/// minimum count of keys with a prefix, or a minimum count of keys matching `prefix…suffix` with at
/// least one character between (the `speed:<mode>:3` Lightning bracket).
#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Unlock {
    HasKey {
        key: String,
    },
    CountPrefix {
        prefix: String,
        min: usize,
    },
    KeyMatch {
        prefix: String,
        suffix: String,
        min: usize,
    },
}

impl Unlock {
    /// Is this predicate satisfied by `collected`?
    fn satisfied(&self, collected: &HashSet<&str>) -> bool {
        match self {
            Unlock::HasKey { key } => collected.contains(key.as_str()),
            Unlock::CountPrefix { prefix, min } => {
                collected.iter().filter(|k| k.starts_with(prefix)).count() >= *min
            }
            Unlock::KeyMatch {
                prefix,
                suffix,
                min,
            } => {
                collected
                    .iter()
                    .filter(|k| {
                        k.starts_with(prefix.as_str())
                            && k.ends_with(suffix.as_str())
                            // ≥1 char between prefix and suffix (and they can't overlap).
                            && k.len() > prefix.len() + suffix.len()
                    })
                    .count()
                    >= *min
            }
        }
    }
}

/// A roster hero's id + its unlock predicate (the rest of its stats live in `balance.json` via
/// [`crate::arena`]; `combat.json` carries only the id + unlock here).
#[derive(Deserialize)]
struct HeroRaw {
    id: String,
    unlock: Unlock,
}

#[derive(Deserialize)]
struct CombatFile {
    constants: Constants,
    heroes: Vec<HeroRaw>,
    #[serde(rename = "lootBoosts")]
    loot_boosts: Vec<LootBoost>,
    #[serde(rename = "enemyTeams")]
    enemy_teams: BTreeMap<String, Vec<FoeRaw>>,
    loot: BTreeMap<String, Vec<String>>,
}

fn parse() -> CombatFile {
    serde_json::from_str(COMBAT_JSON).expect("combat.json")
}

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

/// The enemy types fielded at tier `n` (for the Arena screen's tier header).
pub fn foe_kinds(n: u32) -> Vec<Kind> {
    parse()
        .enemy_teams
        .get(&n.to_string())
        .map(|team| team.iter().map(|f| f.kind).collect())
        .unwrap_or_default()
}

/// The enemy team at tier `n` as `(type, pow, hp)` triples — for the Arena screen's foe cards.
pub fn tier_foes(n: u32) -> Vec<(Kind, f64, f64)> {
    parse()
        .enemy_teams
        .get(&n.to_string())
        .map(|team| team.iter().map(|f| (f.kind, f.pow, f.hp)).collect())
        .unwrap_or_default()
}

/// The region index for tier `n` (`(n-1)/regionSize`) — selects the scenery backdrop + region label.
pub fn tier_region(n: u32) -> u32 {
    (n.max(1) - 1) / region_size()
}

/// The next tier to fight = one past the highest cleared `tier:n` key (capped at the ladder top).
pub fn next_tier<'a>(collected: impl IntoIterator<Item = &'a str>) -> u32 {
    let max_cleared = collected
        .into_iter()
        .filter_map(|k| k.strip_prefix("tier:").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    (max_cleared + 1).min(tier_count())
}

/// Is hero `hero_id` unlocked given the player's `collected` keys? (Unknown id → `false`.)
pub fn is_hero_unlocked(hero_id: &str, collected: &HashSet<&str>) -> bool {
    parse()
        .heroes
        .iter()
        .find(|h| h.id == hero_id)
        .map(|h| h.unlock.satisfied(collected))
        .unwrap_or(false)
}

/// The hero ids unlocked for `collected`, in roster order — the Arena fields only these (no more
/// "all 12" interim). The unlock predicates are `compileUnlock`, proven vs the `heroUnlock` battery.
pub fn unlocked_roster(collected: &HashSet<&str>) -> Vec<String> {
    parse()
        .heroes
        .into_iter()
        .filter(|h| h.unlock.satisfied(collected))
        .map(|h| h.id)
        .collect()
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

/// A hero's combatant numbers: the four stats verbatim plus the flat HP. (Foes use the same five
/// fields but with `grd = foc = 0` and budget-derived `pow`/`hp`.)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeroCombatant {
    pub pow: f64,
    pub grd: f64,
    pub spd: f64,
    pub foc: f64,
    pub hp: f64,
}

/// Map a hero's effective stats to its combatant (the redesigned `constants.combat.hero`): the four
/// stats drive damage/mitigation/speed; HP is the flat `HP_FLAT`.
pub fn hero_combatant(stats: Stats) -> HeroCombatant {
    let hp = parse().constants.combat.hp_flat;
    HeroCombatant {
        pow: stats.power as f64,
        grd: stats.guard as f64,
        spd: stats.speed as f64,
        foc: stats.focus as f64,
        hp,
    }
}

/// The type-triangle multiplier for `attacker` hitting `target` — Brawn▸Cunning▸Arcane▸Brawn (each
/// type has the `advantage` over the next; the reverse is `disadvantage`; same type is neutral).
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

/// The type-triangle multiplier for `attacker` vs `target`, using the loaded matchup constants —
/// for the Arena's per-hero matchup badge (`>1` advantage · `1.0` neutral · `<1` disadvantage).
pub fn matchup_mult(attacker: Kind, target: Kind) -> f64 {
    matchup(attacker, target, &parse().constants.matchup)
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

/// A live combatant: type + the five numbers + its turn-order key (`ord`). `hp` is mutated in place.
#[derive(Clone, Debug)]
struct Unit {
    kind: Kind,
    pow: f64,
    grd: f64,
    spd: f64,
    foc: f64,
    hp: f64,
    ord: u32,
}

/// One strike in the battle playout (the `teamBattleLog` shape) — for the per-strike parity replay
/// and animation callouts. `round` is 0 for opening strikes; `t_hp` is the target's hp after the hit
/// (clamped at 0); `adv` = matchup > 1; `blocked` = the mitigation absorbed ≥ half the raw damage.
#[derive(Clone, Debug, PartialEq)]
struct LogEntry {
    round: u32,
    open: bool,
    a_side: u8,
    a_ord: u32,
    t_side: u8,
    t_ord: u32,
    dmg: f64,
    t_hp: f64,
    ko: bool,
    adv: bool,
    blocked: bool,
}

/// Resolve a fight between `party` (ords 0..2) and `foes` (ords 100..102). Mutates a working copy and
/// records the per-strike `log` (the same playout `teamBattleLog` holds).
fn simulate(
    mut party: Vec<Unit>,
    mut foes: Vec<Unit>,
    c: &Constants,
) -> (BattleResult, Vec<LogEntry>) {
    let m = &c.matchup;
    let cc = &c.combat;
    let mut log: Vec<LogEntry> = Vec::new();
    // Fixed turn order: spd desc, ord asc, over all units (side 0 = party, 1 = foes).
    let mut order: Vec<(u8, usize)> = party
        .iter()
        .enumerate()
        .map(|(i, _)| (0u8, i))
        .chain(foes.iter().enumerate().map(|(i, _)| (1u8, i)))
        .collect();
    order.sort_by(|&a, &b| {
        let (ua, ub) = (unit(&party, &foes, a), unit(&party, &foes, b));
        ub.spd
            .partial_cmp(&ua.spd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(ua.ord.cmp(&ub.ord))
    });

    // (1) OPENING STRIKES: each HERO (side 0) that outspeeds its chosen target lands one hit first.
    for &(side, idx) in &order {
        if side != 0 {
            continue; // only heroes open
        }
        let actor = party[idx].clone();
        if actor.hp <= 0.0 {
            continue;
        }
        let Some(t) = pick_target(actor.kind, &foes, m) else {
            break; // no foes left
        };
        if actor.spd > foes[t].spd {
            let mu = matchup(actor.kind, foes[t].kind, m);
            let raw = js_round(actor.spd * cc.spd_alpha * mu);
            let mitigation = js_round(foes[t].grd * cc.mit);
            let dmg = (raw - mitigation).max(1.0);
            let tgt_ord = foes[t].ord;
            foes[t].hp -= dmg;
            let hp_after = foes[t].hp;
            log.push(LogEntry {
                round: 0,
                open: true,
                a_side: 0,
                a_ord: actor.ord,
                t_side: 1,
                t_ord: tgt_ord,
                dmg,
                t_hp: hp_after.max(0.0),
                ko: hp_after <= 0.0,
                adv: mu > 1.0,
                blocked: mitigation >= raw / 2.0,
            });
        }
    }

    // (2) ROUNDS — every living actor in order strikes; repeat until a side is empty.
    let mut rounds = 0u32;
    'battle: loop {
        let party_live = party.iter().any(|u| u.hp > 0.0);
        let foes_live = foes.iter().any(|u| u.hp > 0.0);
        if !party_live || !foes_live {
            break; // a side was emptied (possibly by the openings → rounds stays 0)
        }
        rounds += 1;
        for &(side, idx) in &order {
            let actor = unit(&party, &foes, (side, idx)).clone();
            if actor.hp <= 0.0 {
                continue;
            }
            let targets = if side == 0 { &foes } else { &party };
            let Some(t) = pick_target(actor.kind, targets, m) else {
                break 'battle; // the other side is empty mid-round
            };
            let tgt = &targets[t];
            let mu = matchup(actor.kind, tgt.kind, m);
            let raw = js_round(actor.pow * mu) + js_round(actor.foc * cc.foc_flat);
            let mitigation = js_round(tgt.grd * cc.mit);
            let dmg = (raw - mitigation).max(1.0);
            let tgt_ord = tgt.ord;
            let defenders = if side == 0 { &mut foes } else { &mut party };
            defenders[t].hp -= dmg;
            let hp_after = defenders[t].hp;
            log.push(LogEntry {
                round: rounds,
                open: false,
                a_side: side,
                a_ord: actor.ord,
                t_side: 1 - side,
                t_ord: tgt_ord,
                dmg,
                t_hp: hp_after.max(0.0),
                ko: hp_after <= 0.0,
                adv: mu > 1.0,
                blocked: mitigation >= raw / 2.0,
            });
        }
        if rounds >= ROUND_GUARD {
            break;
        }
    }

    let heroes_alive = party.iter().filter(|u| u.hp > 0.0).count();
    let foes_alive = foes.iter().filter(|u| u.hp > 0.0).count();
    (
        BattleResult {
            win: heroes_alive > 0,
            heroes_alive,
            foes_alive,
            rounds,
        },
        log,
    )
}

/// Borrow the combatant addressed by `(side, idx)`.
fn unit<'a>(party: &'a [Unit], foes: &'a [Unit], (side, idx): (u8, usize)) -> &'a Unit {
    if side == 0 {
        &party[idx]
    } else {
        &foes[idx]
    }
}

/// Choose a living target: **max matchup → lowest hp → lowest ord**. `None` if all are dead.
fn pick_target(attacker: Kind, targets: &[Unit], m: &MatchupMul) -> Option<usize> {
    targets
        .iter()
        .enumerate()
        .filter(|(_, t)| t.hp > 0.0)
        .max_by(|(_, a), (_, b)| {
            let (ma, mb) = (matchup(attacker, a.kind, m), matchup(attacker, b.kind, m));
            ma.partial_cmp(&mb)
                .unwrap_or(std::cmp::Ordering::Equal)
                // higher matchup wins; then LOWER hp; then LOWER ord (so reverse those two).
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
    let party_units: Vec<Unit> = party
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let kind = roster.iter().find(|h| h.id == *id)?.kind;
            let hc = hero_combatant(effective_stats(id, collected)?);
            Some(Unit {
                kind,
                pow: hc.pow,
                grd: hc.grd,
                spd: hc.spd,
                foc: hc.foc,
                hp: hc.hp,
                ord: i as u32,
            })
        })
        .collect();
    let foe_units: Vec<Unit> = foes_raw
        .iter()
        .enumerate()
        .map(|(i, f)| Unit {
            kind: f.kind,
            pow: f.pow,
            grd: f.grd,
            spd: f.spd,
            foc: f.foc,
            hp: f.hp,
            ord: 100 + i as u32,
        })
        .collect();
    Some(simulate(party_units, foe_units, &data.constants).0)
}

/// Like [`team_battle`] but also returns the per-strike playout log (the `teamBattleLog` shape) — for
/// the per-strike parity replay. Builds the same combatants the headline path does.
#[cfg(test)]
fn team_battle_logged(
    party: &[&str],
    tier: u32,
    collected: &HashSet<&str>,
) -> Option<(BattleResult, Vec<LogEntry>)> {
    let data = parse();
    let foes_raw = data.enemy_teams.get(&tier.to_string())?;
    let roster = arena::roster();
    let party_units: Vec<Unit> = party
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let kind = roster.iter().find(|h| h.id == *id)?.kind;
            let hc = hero_combatant(effective_stats(id, collected)?);
            Some(Unit {
                kind,
                pow: hc.pow,
                grd: hc.grd,
                spd: hc.spd,
                foc: hc.foc,
                hp: hc.hp,
                ord: i as u32,
            })
        })
        .collect();
    let foe_units: Vec<Unit> = foes_raw
        .iter()
        .enumerate()
        .map(|(i, f)| Unit {
            kind: f.kind,
            pow: f.pow,
            grd: f.grd,
            spd: f.spd,
            foc: f.foc,
            hp: f.hp,
            ord: 100 + i as u32,
        })
        .collect();
    Some(simulate(party_units, foe_units, &data.constants))
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
    /// (every drill/metagame collectible — loot ids aren't in it), `full` = catalogue + every loot id.
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

    /// `hero_combatant` maps stats → `{pow,grd,spd,foc,hp}` exactly (5 vectors).
    #[test]
    fn hero_combatant_matches_vectors() {
        for hc in vectors()["heroCombatant"].as_array().unwrap() {
            let got = hero_combatant(as_stats(&hc["stats"]));
            let want = &hc["combatant"];
            assert_eq!(got.pow, want["pow"].as_f64().unwrap(), "pow");
            assert_eq!(got.grd, want["grd"].as_f64().unwrap(), "grd");
            assert_eq!(got.spd, want["spd"].as_f64().unwrap(), "spd");
            assert_eq!(got.foc, want["foc"].as_f64().unwrap(), "foc");
            assert_eq!(got.hp, want["hp"].as_f64().unwrap(), "hp");
        }
    }

    /// `effective_stats` (base + catalogue + loot) reproduces all 36 effectiveStats vectors across
    /// the empty / drillAll / full owned-sets.
    #[test]
    fn effective_stats_matches_vectors() {
        for es in vectors()["effectiveStats"].as_array().unwrap() {
            let hero = es["hero"].as_str().unwrap();
            let owned = owned_set(es["own"].as_str().unwrap());
            let keys: HashSet<&str> = owned.iter().map(String::as_str).collect();
            let got = effective_stats(hero, &keys).expect("known hero");
            assert_eq!(got, as_stats(&es["stats"]), "effective_stats({hero})");
        }
    }

    /// The full sim reproduces every headline `teamBattle` vector (240): win / heroesAlive /
    /// foesAlive / rounds — exercising the opening strikes + the redesigned damage model.
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
    /// headline result — the per-turn opening/damage math is what makes the headline match.
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

    /// Hero-unlock predicates (`hasKey` / `countPrefix` / `keyMatch`) reproduce the 18-state
    /// `heroUnlock` battery exactly — incl. the count boundaries and the `keyMatch` rejects.
    #[test]
    fn hero_unlock_matches_vectors() {
        for st in vectors()["heroUnlock"].as_array().unwrap() {
            let label = st["label"].as_str().unwrap();
            let owned: Vec<String> = st["collected"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            let keys: HashSet<&str> = owned.iter().map(String::as_str).collect();
            let want: Vec<String> = st["unlocked"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            assert_eq!(unlocked_roster(&keys), want, "heroUnlock state {label}");
        }
    }

    /// Per-strike replay of all **5** diverse `teamBattleLogs` (opening strikes · a loss · single
    /// hero · region 5 · the boss) — every strike's round/sides/ords/dmg/tHp/ko/adv/blocked matches,
    /// so a per-strike bug (targeting, order, mitigation rounding) can't hide behind a right headline.
    #[test]
    fn team_battle_logs_replay_per_strike() {
        for tl in vectors()["teamBattleLogs"].as_array().unwrap() {
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
            let (res, log) = team_battle_logged(&party_refs, tier, &keys).expect("tier team");
            let want = tl["log"].as_array().unwrap();
            assert_eq!(log.len(), want.len(), "log length {party:?} t{tier}");
            for (i, (e, w)) in log.iter().zip(want).enumerate() {
                let at = format!("{party:?} t{tier} strike {i}");
                assert_eq!(e.round, w["round"].as_u64().unwrap() as u32, "round @ {at}");
                assert_eq!(e.open, w["open"].as_bool().unwrap(), "open @ {at}");
                assert_eq!(e.a_side, w["aSide"].as_u64().unwrap() as u8, "aSide @ {at}");
                assert_eq!(e.a_ord, w["aOrd"].as_u64().unwrap() as u32, "aOrd @ {at}");
                assert_eq!(e.t_side, w["tSide"].as_u64().unwrap() as u8, "tSide @ {at}");
                assert_eq!(e.t_ord, w["tOrd"].as_u64().unwrap() as u32, "tOrd @ {at}");
                assert_eq!(e.dmg, w["dmg"].as_f64().unwrap(), "dmg @ {at}");
                assert!(
                    (e.t_hp - w["tHp"].as_f64().unwrap()).abs() < 1e-9,
                    "tHp @ {at}: {} vs {}",
                    e.t_hp,
                    w["tHp"]
                );
                assert_eq!(e.ko, w["ko"].as_bool().unwrap(), "ko @ {at}");
                assert_eq!(e.adv, w["adv"].as_bool().unwrap(), "adv @ {at}");
                assert_eq!(e.blocked, w["blocked"].as_bool().unwrap(), "blocked @ {at}");
            }
            // …and the headline agrees with the replayed log.
            assert_eq!(
                res.win,
                tl["win"].as_bool().unwrap(),
                "win {party:?} t{tier}"
            );
            assert_eq!(res.rounds, tl["rounds"].as_u64().unwrap() as u32, "rounds");
        }
    }

    /// Sanity: the ladder is complete (every tier 1..=tierCount has an enemy team).
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
