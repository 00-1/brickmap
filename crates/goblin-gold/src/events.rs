//! GG1 **daily events** — the 14-event rotation of the metagame (full-port phase 3), re-implemented
//! in Rust over the T230/T232 export. There are **14 events**, each a themed challenge with three
//! reward tiers — *participation* (`event:<id>`), *well* (`event:<id>:well`) and *ace*
//! (`event:<id>:ace`) — all of which are keys in the save's `collected` map (the keystone). A new
//! event runs each day on a **14-day UTC cycle**.
//!
//! What the export pins down (re-implemented + verified here): the 14 events (id and title), their
//! three reward keys each (the 42 `Events` collectibles), and the 14-day rotation structure.
//!
//! What it does NOT carry (data-seam rule, never fabricated): the per-event content — which
//! topics/transforms it draws and the well/ace score thresholds — and the cycle's canonical anchor
//! and order live only in the JS, not the export. So `scheduled` rotates over the 14 in a stable
//! (catalogue) order by `day mod 14`: structurally faithful, but not the canonical phase, which
//! awaits a schedule export. The reward keys and earned-state are fully data-backed.

use crate::catalogue::{self, Category};

/// How well a player did in an event — the three reward tiers, in ascending order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Completed it (`event:<id>`).
    Participation,
    /// Did well (`event:<id>:well`).
    Well,
    /// Aced it (`event:<id>:ace`).
    Ace,
}

impl Tier {
    /// The id suffix this tier adds to the base `event:<id>` key.
    fn suffix(self) -> &'static str {
        match self {
            Tier::Participation => "",
            Tier::Well => ":well",
            Tier::Ace => ":ace",
        }
    }

    /// All three tiers, ascending.
    pub fn all() -> [Tier; 3] {
        [Tier::Participation, Tier::Well, Tier::Ace]
    }
}

/// One daily event: a stable id and its display title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// The event id (e.g. `bondfire-night`) — the stem of its reward keys.
    pub id: String,
    /// The display title (e.g. `Bondfire Night`), parsed from the participation reward's blurb.
    pub name: String,
}

impl Event {
    /// This event's reward key for a given tier — exactly a `collected` map key.
    pub fn reward_key(&self, tier: Tier) -> String {
        format!("event:{}{}", self.id, tier.suffix())
    }
}

/// Strip an `event:<id>[:well|:ace]` collectible id down to its bare event id.
fn event_id_of(collectible_id: &str) -> Option<String> {
    let rest = collectible_id.strip_prefix("event:")?;
    let stem = rest
        .strip_suffix(":well")
        .or_else(|| rest.strip_suffix(":ace"))
        .unwrap_or(rest);
    Some(stem.to_string())
}

/// The event title from a participation blurb (`"Reward for completing <Title>."`); falls back to
/// the id if the blurb isn't in the expected shape.
fn title_from_desc(desc: &str, id: &str) -> String {
    desc.strip_prefix("Reward for completing ")
        .and_then(|s| s.strip_suffix('.'))
        .map(|s| s.to_string())
        .unwrap_or_else(|| id.to_string())
}

/// The 14 daily events, in a stable (catalogue) order. Derived from the `Events` collectibles: each
/// event's participation entry (`event:<id>`, no tier suffix) gives its id + title.
pub fn events() -> Vec<Event> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for c in catalogue::catalog() {
        if c.cat != Category::Events {
            continue;
        }
        // The participation entry is the bare `event:<id>` (no `:well` / `:ace`).
        if c.id.ends_with(":well") || c.id.ends_with(":ace") {
            continue;
        }
        if let Some(id) = event_id_of(&c.id) {
            if seen.insert(id.clone()) {
                out.push(Event {
                    name: title_from_desc(&c.desc, &id),
                    id,
                });
            }
        }
    }
    out
}

/// The event scheduled on a given **UTC day index** (e.g. days since the Unix epoch): the 14-day
/// rotation, `day mod 14` over [`events`]. Caller supplies the day index (kept pure — no clock here),
/// so it's deterministic and testable. NOTE: the rotation *order/anchor* isn't in the export (see
/// the module docs); this is a faithful 14-day cycle over the catalogue order, not the canonical
/// phase.
pub fn scheduled(utc_day_index: u64) -> Event {
    let all = events();
    let i = (utc_day_index % all.len() as u64) as usize;
    all.into_iter().nth(i).expect("14 events")
}

/// The events a player has any reward for: those with at least one `event:<id>*` key collected.
pub fn touched<'a>(collected: impl IntoIterator<Item = &'a str>) -> Vec<Event> {
    let keys: std::collections::HashSet<&str> = collected.into_iter().collect();
    events()
        .into_iter()
        .filter(|e| {
            Tier::all()
                .iter()
                .any(|&t| keys.contains(e.reward_key(t).as_str()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn there_are_fourteen_distinct_named_events() {
        let evs = events();
        assert_eq!(evs.len(), 14, "the daily rotation is 14 events");
        let mut ids = HashSet::new();
        for e in &evs {
            assert!(ids.insert(e.id.clone()), "duplicate event id {}", e.id);
            assert!(!e.name.is_empty(), "{} has no title", e.id);
            assert_ne!(e.name, e.id, "{} title should be the prose name", e.id);
        }
    }

    // Every event's three reward keys are real `Events` collectibles, and those 42 keys are exactly
    // the Events category — nothing missing, nothing extra.
    #[test]
    fn reward_keys_are_exactly_the_events_category() {
        let event_keys: HashSet<String> = events()
            .iter()
            .flat_map(|e| Tier::all().map(|t| e.reward_key(t)))
            .collect();
        assert_eq!(event_keys.len(), 14 * 3, "14 events × 3 tiers");

        let catalogue_event_ids: HashSet<String> = catalogue::catalog()
            .into_iter()
            .filter(|c| c.cat == Category::Events)
            .map(|c| c.id)
            .collect();
        assert_eq!(
            event_keys, catalogue_event_ids,
            "the generated reward keys must match the Events category exactly"
        );
    }

    #[test]
    fn the_schedule_is_a_fourteen_day_cycle() {
        let evs = events();
        // Period 14: the same day-of-cycle yields the same event.
        for d in 0u64..30 {
            assert_eq!(scheduled(d), scheduled(d + 14), "cycle period must be 14");
        }
        // One full cycle visits every event exactly once.
        let cycle: Vec<String> = (0..14).map(|d| scheduled(d).id).collect();
        assert_eq!(
            cycle.iter().collect::<HashSet<_>>().len(),
            14,
            "a 14-day window covers all events once"
        );
        assert_eq!(cycle[0], evs[0].id, "day 0 is the first event");
    }

    #[test]
    fn touched_maps_collected_keys_back_to_events() {
        let evs = events();
        let first = &evs[0];
        let third = &evs[2];
        let collected = [
            first.reward_key(Tier::Participation),
            third.reward_key(Tier::Ace),
        ];
        let keys: Vec<&str> = collected.iter().map(|s| s.as_str()).collect();
        let got: HashSet<String> = touched(keys).into_iter().map(|e| e.id).collect();
        assert_eq!(got, HashSet::from([first.id.clone(), third.id.clone()]));
    }
}
