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
use std::collections::BTreeMap;

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

    /// Fold a finished round into the save: bump the games counter, run the earning rule
    /// ([`crate::earning::award`]), and mark every awarded key (keeping the earliest `ts`). Returns
    /// the keys **newly** collected this round (for a "you earned…" toast).
    ///
    /// Per-question solve/spark aren't awarded here yet — the live drill doesn't time individual
    /// questions, so `qmap` is empty; everything else (ranks, init, flawless, speed, mastery, and the
    /// games/modes/flawless meta) is awarded from the round aggregates. In the auto-accept drill
    /// every accepted answer is correct, so `score == answered` and `mistakes == total − answered`.
    pub fn award_round(
        &mut self,
        mode: &progression::Mode,
        run: &progression::RunResult,
        ts: u64,
    ) -> Vec<String> {
        self.games += 1;
        let count_prefix =
            |p: &str| self.collected.keys().filter(|k| k.starts_with(p)).count() as u32;
        let ctx = crate::earning::Ctx {
            mode_id: &mode.id,
            master_secs: mode.master_secs,
            total: run.total,
            answered: run.answered,
            score: run.answered,
            total_time: run.total_time_secs,
            qmap: Vec::new(),
            stats: crate::earning::RunStats {
                games: self.games as u32,
                modes_cleared: count_prefix("init:"),
                flawless: count_prefix("flawless:"),
            },
        };
        let mut added = Vec::new();
        for k in crate::earning::award(&ctx) {
            if self.mark(k.clone(), ts) {
                added.push(k);
            }
        }
        added
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

        // A perfect, fast, clean round earns init/flawless/mastery + ranks + speed brackets.
        let added = s.award_round(
            &mode,
            &RunResult {
                total: 10,
                answered: 10,
                total_time_secs: 0.0,
            },
            1000,
        );
        assert!(s.has("init:halves") && s.has("flawless:halves") && s.has("mastery:halves"));
        assert!(s.has("rank:goblin") && s.has("speed:halves:0"));
        assert_eq!(s.games, 1);
        assert!(added.contains(&"init:halves".to_string()));
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
            2000,
        );
        assert_eq!(s.games, 2);
        assert!(
            again.is_empty(),
            "already-collected keys aren't re-awarded: {again:?}"
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
