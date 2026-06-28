//! GG1 **save model** (full-port phase 3), persisted through the engine's [`brickmap::save::Store`]
//! seam. GG1 keeps *one* central **`collected`** map — `{<key>: {ts}}` — as the keystone of the
//! whole metagame: every unlock/achievement/item/event is just a key in it (a `mastery:<mode>` key,
//! an `init:<mode>` key, a `<collectibleId>`, an `event:<id>`, a `tier:<n>`…). Progression,
//! the collector ladder and the arena/event state are all *derived* from that one map, so the save
//! is small and forward-compatible: a new metagame feature is a new key prefix, not a schema bump.
//!
//! This is the durable shape (`collected` + `gold` + `last_mode`); reading it back rebuilds
//! [`progression::Progress`] via [`progression::Progress::from_collected`]. Everything serialises to
//! JSON and round-trips through any [`Store`] (a file per save-slot on native/Android,
//! `localStorage` on the web).

use crate::progression::{self, Progress};
use brickmap::save::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// The save slot key under which the blob lives in the [`Store`].
pub const SLOT: &str = "gg1";

/// One entry in the central `collected` map: a timestamp (epoch ms, as GG1 records it). The value
/// is an object (not a bare number) so future per-key fields can be added without a schema bump —
/// mirroring the live `{ts}` shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    /// When the key was first collected (epoch milliseconds).
    pub ts: u64,
}

/// One step of a finished round, in order — drives the per-question (solve/spark) awards and the
/// LIVE gold accrual (combo resets on a skip, so the order of solves vs skips matters).
#[derive(Clone, Debug, PartialEq)]
pub enum RoundStep {
    /// A clean solve of `prompt` taking `dt` seconds.
    Solve { prompt: String, dt: f64 },
    /// A skipped question (resets the gold combo).
    Skip,
}

/// The result of one finished round — what the results screen shows.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundOutcome {
    /// The rank tier reached this round (0..=22).
    pub rank_idx: usize,
    /// Its display name (e.g. "Archmage").
    pub rank_name: String,
    /// Collectible keys newly earned this round (for the "earned this run" list).
    pub newly: Vec<String>,
    /// Goblin Gold paid out this round (already accrued into the save).
    pub gold_earned: u64,
    /// Questions answered (= score, in the auto-accept drill).
    pub answered: u32,
    /// Questions in the round.
    pub total: u32,
    /// Total time across the round (seconds).
    pub total_time: f64,
}

/// The result of one Arena battle — what the Arena screen shows after a fight.
#[derive(Clone, Debug, PartialEq)]
pub struct ArenaOutcome {
    /// The tier fought (1-based).
    pub tier: u32,
    /// Did the party win?
    pub win: bool,
    /// Rounds the battle took.
    pub rounds: u32,
    /// Heroes still standing at the end.
    pub heroes_alive: usize,
    /// Gold paid out (0 on a loss; already accrued into the save on a win).
    pub gold_earned: u64,
    /// Loot ids granted this win (empty on a loss).
    pub loot: Vec<String>,
    /// Whether clearing this tier finished a region (a boss fell).
    pub region_cleared: bool,
}

/// The persisted game state: the central `collected` keystone plus the loose top-level fields.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Save {
    /// The keystone map — every unlock/achievement/item/event is a key here (see module docs).
    #[serde(default)]
    pub collected: BTreeMap<String, Stamp>,
    /// Gold balance. GG1 stores this as a string-float; we keep the numeric value and (de)serialise
    /// it through a string so the on-disk shape matches the live runtime.
    #[serde(default, with = "gold_string")]
    pub gold: f64,
    /// The last topic the player was on (restores the topic selection); `None` on a fresh save.
    #[serde(default, rename = "mode", skip_serializing_if = "Option::is_none")]
    pub last_mode: Option<String>,
    /// Total rounds played (the `games` running total gating the meta-milestones).
    #[serde(default)]
    pub games: u64,
    /// Per-topic **best total time** (seconds) — the lowest `total_time_secs` recorded across
    /// finished rounds of that mode. **D3**: feeds the Best Times screen, the home "<topic> · best…"
    /// detail line, and Practice's per-question best-time tiles. Mirrors web's
    /// `boardKey "per-mode best-time boards"` (we store just the headline scalar — sufficient for
    /// the three surfaces and stable under save migration). Empty for modes never finished.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub best_times: BTreeMap<String, f64>,
}

/// GG1 persists `gold` as a string-float (e.g. `"125.5"`); (de)serialise our `f64` through that
/// shape so the blob is byte-compatible with the live runtime's save.
mod gold_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(g: &f64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{g}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
        // Accept either a string ("125.5", the live shape) or a bare number, to be lenient on read.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StrOrNum {
            S(String),
            N(f64),
        }
        Ok(match StrOrNum::deserialize(d)? {
            StrOrNum::S(s) => s.trim().parse().unwrap_or(0.0),
            StrOrNum::N(n) => n,
        })
    }
}

impl Save {
    /// Is `key` collected?
    pub fn has(&self, key: &str) -> bool {
        self.collected.contains_key(key)
    }

    /// Mark `key` collected at `ts` (epoch ms), keeping the *earliest* timestamp if it's re-marked
    /// (a key is "first collected" once). Returns whether this was newly added.
    pub fn mark(&mut self, key: impl Into<String>, ts: u64) -> bool {
        use std::collections::btree_map::Entry;
        match self.collected.entry(key.into()) {
            Entry::Vacant(e) => {
                e.insert(Stamp { ts });
                true
            }
            Entry::Occupied(mut e) => {
                if ts < e.get().ts {
                    e.get_mut().ts = ts;
                }
                false
            }
        }
    }

    /// Rebuild [`progression::Progress`] from the `collected` keystone (the `init:`/`mastery:`
    /// keys). The single source of truth for "what's unlocked" lives in the save, not a side table.
    pub fn progress(&self) -> Progress {
        progression::Progress::from_collected(self.collected.keys().map(String::as_str))
    }

    /// The best total time (seconds) ever recorded for `mode_id`, or `None` if never finished
    /// flawlessly. **D3**: drives the Best Times screen + home "<topic> · best…" detail line.
    pub fn best_time(&self, mode_id: &str) -> Option<f64> {
        self.best_times.get(mode_id).copied()
    }

    /// Fold a finished round into the save: bump the games counter, run the earning rule
    /// ([`crate::earning::award`]) marking every awarded key (earliest `ts` wins), accrue the round's
    /// Goblin Gold, and return a [`RoundOutcome`] (rank + newly-earned keys + time + gold) for the
    /// results screen.
    ///
    /// `steps` is the round's ordered outcome — each [`RoundStep::Solve`] (with its time) or
    /// [`RoundStep::Skip`]. Solves drive the solve/spark awards; the full ordered list drives the
    /// **live** gold accrual (the combo resets on a skip, so order matters). In the auto-accept drill
    /// every accepted answer is clean (`miss == 0`) and `score == answered`.
    pub fn award_round(
        &mut self,
        mode: &progression::Mode,
        run: &progression::RunResult,
        steps: &[RoundStep],
        ts: u64,
    ) -> RoundOutcome {
        self.games += 1;
        let count_prefix =
            |s: &Save, p: &str| s.collected.keys().filter(|k| k.starts_with(p)).count() as u32;
        // Solve steps drive the solve/spark awards (every accepted answer is clean → miss 0).
        let qmap = steps
            .iter()
            .filter_map(|s| match s {
                RoundStep::Solve { prompt, dt } => Some(crate::earning::QSolve {
                    prompt: prompt.clone(),
                    miss: 0,
                    t: *dt,
                }),
                RoundStep::Skip => None,
            })
            .collect();
        let ctx = crate::earning::Ctx {
            mode_id: &mode.id,
            master_secs: mode.master_secs,
            total: run.total,
            answered: run.answered,
            score: run.answered,
            total_time: run.total_time_secs,
            qmap,
            stats: crate::earning::RunStats {
                games: self.games as u32,
                modes_cleared: count_prefix(self, "init:"),
                flawless: count_prefix(self, "flawless:"),
            },
        };
        let mut newly = Vec::new();
        for k in crate::earning::award(&ctx) {
            if self.mark(k.clone(), ts) {
                newly.push(k);
            }
        }

        // Rank for the round, and the Goblin Gold it pays out (accrued — gold only ever grows).
        let rank_idx = crate::earning::rank_index(run.answered, run.total, run.total_time_secs);
        let rank_name =
            crate::earning::rank_name(rank_idx).unwrap_or_else(|| "Unranked".to_string());
        // Gold multiplier from the owned-collectible counts (post-award). Heroes/tiers/bosses are 0
        // until the Arena is ported (T233b-combat).
        let items =
            crate::catalogue::earned(self.collected.keys().map(String::as_str)).len() as u32;
        let mult = crate::gold::gold_mult(items, count_prefix(self, "mastery:"), 0, 0, 0);
        // Accrue gold LIVE over the ordered steps (combo resets on skip — the parity fix).
        let plays: Vec<crate::gold::Play> = steps
            .iter()
            .map(|s| match s {
                RoundStep::Solve { dt, .. } => crate::gold::Play::Solve(*dt),
                RoundStep::Skip => crate::gold::Play::Skip,
            })
            .collect();
        let gold_earned = crate::gold::round_gold(
            mode.master_secs,
            mult,
            &plays,
            run.answered,
            rank_idx as u32,
        );
        self.gold += gold_earned as f64;

        // D3: track per-mode best (lowest) `total_time_secs` across finished rounds — feeds the
        // Best Times screen + home `<topic> · best…` + Practice qbest. Only count fully-answered
        // rounds (a partial skip doesn't beat a clean run on time).
        if run.skips() == 0 && run.total > 0 {
            let entry = self
                .best_times
                .entry(mode.id.clone())
                .or_insert(f64::INFINITY);
            if run.total_time_secs < *entry {
                *entry = run.total_time_secs;
            }
        }

        RoundOutcome {
            rank_idx,
            rank_name,
            newly,
            gold_earned,
            answered: run.answered,
            total: run.total,
            total_time: run.total_time_secs,
        }
    }

    /// Fight the **next** Arena tier (one past the highest cleared) with `party` (≤3 hero ids),
    /// resolving the battle via [`crate::combat::team_battle`]. On a win, grant `tier:<n>` + the
    /// tier's loot into the keystone and accrue the `tierGold` payoff; on a loss, nothing changes.
    /// Returns the outcome (`None` only if the tier has no enemy team — shouldn't happen in-range).
    ///
    /// The gold multiplier reuses the same owned-collectible policy as [`Save::award_round`]
    /// (`items` + `mastered`); the `heroes`/`tiers`/`bosses` contributions are a documented
    /// simplification pending a structured **hero-unlock** export (the export carries hero unlocks
    /// only as prose hints today). The win/loss itself is fully vector-proven in [`crate::combat`].
    pub fn resolve_arena(&mut self, party: &[&str], ts: u64) -> Option<ArenaOutcome> {
        let tier = crate::combat::next_tier(self.collected.keys().map(String::as_str));
        // Resolve the fight first (immutable borrow of `collected`), then grant (mutable).
        let result = {
            let keys: HashSet<&str> = self.collected.keys().map(String::as_str).collect();
            crate::combat::team_battle(party, tier, &keys)?
        };
        let (mut gold_earned, mut loot, mut region_cleared) = (0u64, Vec::new(), false);
        if result.win {
            self.mark(format!("tier:{tier}"), ts);
            for id in crate::combat::loot_for(tier) {
                self.mark(id.clone(), ts);
                loot.push(id);
            }
            region_cleared = crate::arena::is_boss(tier);
            // tierGold(n, goldMult(collected)) — post-grant counts; same mult policy as award_round.
            let mastered = self
                .collected
                .keys()
                .filter(|k| k.starts_with("mastery:"))
                .count() as u32;
            let items =
                crate::catalogue::earned(self.collected.keys().map(String::as_str)).len() as u32;
            let mult = crate::gold::gold_mult(items, mastered, 0, 0, 0);
            let g = crate::gold::tier_gold(tier, mult);
            gold_earned = g.floor() as u64;
            self.gold += g;
        }
        Some(ArenaOutcome {
            tier,
            win: result.win,
            rounds: result.rounds,
            heroes_alive: result.heroes_alive,
            gold_earned,
            loot,
            region_cleared,
        })
    }

    /// Fold a finished daily-event run into the save: grant the [`crate::event_play::event_tiers_earned`]
    /// keys for `eid` at `score`/`total` (always `event:<eid>`; `+:well` ≥ 0.7; `+:ace` on a flawless
    /// run). Events pay **no gold** — the reward IS the buff item. Returns the keys newly granted.
    pub fn award_event(&mut self, eid: &str, score: u32, total: u32, ts: u64) -> Vec<String> {
        let mut newly = Vec::new();
        for key in crate::event_play::event_tiers_earned(eid, score, total) {
            if self.mark(key.clone(), ts) {
                newly.push(key);
            }
        }
        newly
    }

    /// Load the save from `store` (slot [`SLOT`]); a fresh [`Save::default`] if absent or corrupt
    /// (a torn blob shouldn't brick the game — it starts over rather than refusing to launch).
    pub fn load(store: &dyn Store) -> Save {
        match store.load(SLOT) {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            None => Save::default(),
        }
    }

    /// Persist the save to `store` (slot [`SLOT`]) as JSON.
    pub fn store(&self, store: &dyn Store) -> std::io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        store.save(SLOT, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickmap::save::FileStore;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("gg1-save-{tag}-{nanos}"));
        p
    }

    #[test]
    fn round_trips_through_a_filestore() {
        let dir = temp_dir("rt");
        let store = FileStore::open(&dir).expect("open");

        // Fresh load on an empty store is the default.
        assert_eq!(Save::load(&store), Save::default());

        let mut s = Save::default();
        s.mark("init:halves", 1000);
        s.mark("mastery:addsub", 2000);
        s.mark("collector:10", 3000);
        s.gold = 125.5;
        s.last_mode = Some("times".into());
        s.store(&store).expect("store");

        let back = Save::load(&store);
        assert_eq!(back, s, "save must round-trip byte-for-byte through serde");
        assert_eq!(back.gold, 125.5);
        assert_eq!(back.last_mode.as_deref(), Some("times"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn progress_is_derived_from_the_collected_keystone() {
        let mut s = Save::default();
        s.mark("init:halves", 1); // played, not mastered
        s.mark("mastery:addsub", 2); // played AND mastered
        s.mark("collector:5", 3); // an item — ignored by progression
        s.mark("event:daily-7", 4); // an event — ignored by progression

        let p = s.progress();
        assert!(p.is_played("halves") && !p.is_mastered("halves"));
        assert!(p.is_played("addsub") && p.is_mastered("addsub"));
        assert!(!p.is_played("collector:5"), "non-progression keys ignored");
    }

    #[test]
    fn mark_keeps_the_earliest_timestamp_and_reports_newness() {
        let mut s = Save::default();
        assert!(s.mark("x", 100), "first mark is new");
        assert!(!s.mark("x", 50), "re-mark is not new");
        assert_eq!(s.collected["x"].ts, 50, "earliest ts wins");
        assert!(
            !s.mark("x", 999),
            "later re-mark is not new and doesn't move ts"
        );
        assert_eq!(s.collected["x"].ts, 50);
    }

    #[test]
    fn gold_persists_through_the_string_shape() {
        // GG1 stores gold as a string-float; the serialised blob must use that shape.
        let s = Save {
            gold: 42.0,
            ..Save::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(
            json.contains("\"gold\":\"42\""),
            "gold must serialise as a string: {json}"
        );
        // …and a string value (the live shape) reads back as the number.
        let parsed: Save = serde_json::from_str(r#"{"gold":"7.25"}"#).unwrap();
        assert_eq!(parsed.gold, 7.25);
        // Be lenient: a bare number also reads back.
        let parsed: Save = serde_json::from_str(r#"{"gold":3}"#).unwrap();
        assert_eq!(parsed.gold, 3.0);
    }

    #[test]
    fn award_round_marks_earned_keys_and_counts_games() {
        use crate::progression::{Mode, RunResult, Unlock};
        let mode = Mode {
            id: "halves".into(),
            name: "Halves".into(),
            master_secs: 5.0,
            unlock: Unlock::Always,
        };
        let mut s = Save::default();

        // A perfect, fast, clean round earns init/flawless/mastery + ranks + speed brackets, plus
        // solve/spark for the timed questions (a real halves prompt, solved fast).
        let steps = vec![RoundStep::Solve {
            prompt: "3".to_string(),
            dt: 0.5,
        }];
        let out = s.award_round(
            &mode,
            &RunResult {
                total: 10,
                answered: 10,
                total_time_secs: 0.0,
            },
            &steps,
            1000,
        );
        assert!(s.has("init:halves") && s.has("flawless:halves") && s.has("mastery:halves"));
        assert!(s.has("rank:goblin") && s.has("speed:halves:0"));
        assert!(
            s.has("solve:halves:3") && s.has("spark:halves:3"),
            "a clean, fast solve earns solve + spark"
        );
        assert_eq!(s.games, 1);
        assert!(out.newly.contains(&"init:halves".to_string()));
        // A perfect fast round is the top rank and pays gold (accrued into the save).
        assert_eq!(out.rank_idx, 22);
        assert!(out.gold_earned > 0 && s.gold >= out.gold_earned as f64);
        // The save is the source of truth for progression.
        assert!(s.progress().is_mastered("halves"));

        // Replaying bumps games but re-awards nothing already owned.
        let again = s.award_round(
            &mode,
            &RunResult {
                total: 10,
                answered: 10,
                total_time_secs: 0.0,
            },
            &steps,
            2000,
        );
        assert_eq!(s.games, 2);
        assert!(
            again.newly.is_empty(),
            "already-collected keys aren't re-awarded: {:?}",
            again.newly
        );
    }

    #[test]
    fn resolving_an_arena_win_grants_the_tier_loot_and_gold() {
        // A fully-collected party (every catalogue item owned → maxed effective stats) crushes the
        // first tier, so the win path is exercised: tier:1 + its loot are marked and gold accrues.
        let mut s = Save::default();
        for c in crate::catalogue::catalog() {
            s.mark(c.id, 1);
        }
        let before_gold = s.gold;
        let out = s
            .resolve_arena(&["mo", "roon", "zeph"], 1000)
            .expect("tier 1 has a team");
        assert_eq!(out.tier, 1);
        assert!(out.win, "a maxed party should clear tier 1");
        assert!(s.has("tier:1"), "the cleared tier is marked");
        for id in &out.loot {
            assert!(s.has(id), "granted loot {id} is marked");
        }
        assert_eq!(
            out.gold_earned > 0,
            s.gold > before_gold,
            "gold accrues on a win"
        );
        assert!(s.gold > before_gold, "a win pays gold");
        // The next fight targets the next tier (progression advances off the keystone).
        assert_eq!(
            crate::combat::next_tier(s.collected.keys().map(String::as_str)),
            2
        );
    }

    #[test]
    fn awarding_an_event_grants_its_tier_keys() {
        // A flawless run grants all three tiers; the keys persist and re-awarding adds nothing new.
        let mut s = Save::default();
        let newly = s.award_event("halving-moon", 13, 13, 5);
        assert_eq!(
            newly,
            vec![
                "event:halving-moon".to_string(),
                "event:halving-moon:well".to_string(),
                "event:halving-moon:ace".to_string(),
            ]
        );
        assert!(s.has("event:halving-moon:ace"));
        // A weaker re-run grants nothing new (keys already collected), and never *removes* a tier.
        let again = s.award_event("halving-moon", 3, 13, 6);
        assert!(again.is_empty(), "already-earned tiers aren't re-granted");
        assert!(
            s.has("event:halving-moon:ace"),
            "a worse run can't strip a tier"
        );
    }

    #[test]
    fn a_corrupt_blob_loads_as_default_rather_than_panicking() {
        let dir = temp_dir("corrupt");
        let store = FileStore::open(&dir).expect("open");
        store
            .save(SLOT, b"this is not json {{{")
            .expect("save garbage");
        assert_eq!(Save::load(&store), Save::default(), "torn blob → default");
        std::fs::remove_dir_all(&dir).ok();
    }
}
