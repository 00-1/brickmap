//! GG1 daily **Events** — event-play (T233c), re-implemented from `events.js` + `main.js` and
//! **proven against `events-vectors.json`**. Pure logic; the event-play *screen* lives in
//! [`crate::app`]. (This is the richer T233c export — the deterministic gauntlet, the UTC-day
//! schedule, and the reward tiers — distinct from [`crate::events`], which derives the 14-event
//! reward keys from the collectibles catalogue.)
//!
//! - **schedule** — `epochDay = floor(now_ms / dayMs)`; `index = ((epochDay % 14) + 14) % 14`;
//!   `roster[index]` is live. Anchored at epochDay 0 = `roster[0]`, recurring every 14 days. `now` is
//!   injected (no clock baked in).
//! - **reward tiers** ([`event_tiers_earned`]) — always `event:<id>` (participation = completion);
//!   `+ :well` if `score/total ≥ wellFrac` (0.7); `+ :ace` on a flawless run (`total>0 && score==total`).
//!   Events pay **no gold** — the reward IS the buff item.
//! - **gauntlet** ([`build_gauntlet`]) — `seed = hashStr(id) ^ artSeed → mulberry32`; for each
//!   `questionMix {topic,n}` the topic's full pool ([`crate::transforms::generate`]) is sorted to a
//!   total order ([`gauntlet_cmp`] — JS `localeCompare` numeric collation, raw-string tiebreak),
//!   seed-shuffled, and the first `n` taken; the combined set is then seed-shuffled (the themed
//!   interleave). Fully deterministic — the same gauntlet every play and every 14-day recurrence.

use serde::Deserialize;

/// The synced T233c daily-events export.
const EVENTS_JSON: &str = include_str!("../data/gg1/events.json");

/// One slice of an event's question mix: `n` questions drawn from `topic`'s pool.
#[derive(Deserialize, Clone, Debug)]
pub struct Mix {
    pub topic: String,
    pub n: usize,
}

/// A daily event: its identity, theme/flavour, the question mix, the three reward tiers, and the
/// deterministic seeds.
#[derive(Deserialize, Clone, Debug)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub theme: String,
    pub blurb: String,
    #[serde(rename = "questionMix")]
    pub question_mix: Vec<Mix>,
    /// Participation reward (granted on completion).
    pub reward: String,
    /// "Well played" reward (≥ `wellFrac` accuracy).
    #[serde(rename = "rewardWell")]
    pub reward_well: String,
    /// Flawless-run reward.
    #[serde(rename = "rewardAce")]
    pub reward_ace: String,
    pub rarity: String,
    #[serde(rename = "artSeed")]
    pub art_seed: u32,
    #[serde(rename = "musicSeed")]
    pub music_seed: i64,
}

#[derive(Deserialize)]
struct EventsFile {
    #[serde(rename = "dayMs")]
    day_ms: i64,
    rotation: i64,
    #[serde(rename = "wellFrac")]
    well_frac: f64,
    roster: Vec<Event>,
}

fn parse() -> EventsFile {
    serde_json::from_str(EVENTS_JSON).expect("events.json")
}

/// The 14-event roster (rotation order).
pub fn roster() -> Vec<Event> {
    parse().roster
}

/// Milliseconds per UTC day (the schedule quantum).
pub fn day_ms() -> i64 {
    parse().day_ms
}

/// The roster index live on `epoch_day`: `((epochDay % rotation) + rotation) % rotation` (handles
/// negative days — the cycle extends backwards from the 1970-01-01 anchor).
pub fn live_index(epoch_day: i64) -> usize {
    let r = parse().rotation;
    (((epoch_day % r) + r) % r) as usize
}

/// The event live at `now_ms` (UTC epoch milliseconds). `epochDay = floor(now_ms / dayMs)`.
pub fn live_event(now_ms: i64) -> Event {
    let f = parse();
    // `div_euclid` is floor-division for a positive divisor (matches JS `Math.floor`, incl. negatives).
    let idx = (((now_ms.div_euclid(f.day_ms) % f.rotation) + f.rotation) % f.rotation) as usize;
    f.roster
        .into_iter()
        .nth(idx)
        .expect("roster index in range")
}

/// The collectible keys earned by finishing event `eid` with `score`/`total` solved: always
/// `event:<eid>`; `+ :well` at ≥ `wellFrac` accuracy; `+ :ace` on a flawless run.
pub fn event_tiers_earned(eid: &str, score: u32, total: u32) -> Vec<String> {
    let mut ids = vec![format!("event:{eid}")];
    if total > 0 {
        let frac = score as f64 / total as f64;
        if frac >= parse().well_frac {
            ids.push(format!("event:{eid}:well"));
        }
        if score == total {
            ids.push(format!("event:{eid}:ace"));
        }
    }
    ids
}

/// One built question in an event gauntlet: prompt, answer, and which topic it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct GauntletQ {
    pub p: String,
    pub a: f64,
    pub topic: String,
}

/// The ICU collation primary rank of a non-digit character, covering the GG1 prompt alphabet
/// (derived from JS `localeCompare(…,{numeric:true})`): `space < ? < . < / < % < + < ÷ < × < = < − <
/// digits < ² < letters`. Digit runs are compared numerically (see [`gauntlet_cmp`]); this rank only
/// orders *non-digit* characters (and a stray lone digit, via the `digit` tier). Reproducing this is
/// why we can't reuse the codepoint-ordered [`crate::earning::natural_cmp`] here.
fn char_rank(c: char) -> u32 {
    match c {
        ' ' => 0,
        '?' => 1,
        '.' => 2,
        '/' => 3,
        '%' => 4,
        '+' => 5,
        '÷' => 6,
        '×' => 7,
        '=' => 8,
        '−' => 9, // U+2212 MINUS SIGN
        '0'..='9' => 10,
        '²' => 11, // U+00B2 SUPERSCRIPT TWO (sorts after digits, before letters)
        c if c.is_ascii_alphabetic() => 100 + (c.to_ascii_lowercase() as u32 - b'a' as u32),
        // Any character outside the known alphabet sorts last, stably by codepoint (shouldn't occur
        // for the curated topic pools — the gauntlet test would flag it).
        c => 1000 + c as u32,
    }
}

/// Compare two prompts as JS `String.localeCompare` with `{numeric:true}` does over the GG1 alphabet:
/// maximal digit runs compared as numbers (leading zeros ignored), non-digit characters by ICU
/// [`char_rank`]. This is the total order `buildGauntlet` sorts the topic pool into before shuffling.
fn gauntlet_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    fn take_digits(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
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
    }
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) if ca.is_ascii_digit() && cb.is_ascii_digit() => {
                let (ra, rb) = (take_digits(&mut ai), take_digits(&mut bi));
                let (na, nb) = (ra.trim_start_matches('0'), rb.trim_start_matches('0'));
                let ord = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
                if ord != Ordering::Equal {
                    return ord;
                }
                // Numerically equal (incl. leading-zero variants) — continue with the next chunk.
            }
            (Some(ca), Some(cb)) => {
                let ord = char_rank(ca).cmp(&char_rank(cb));
                if ord != Ordering::Equal {
                    return ord;
                }
                ai.next();
                bi.next();
            }
        }
    }
}

/// A seeded Fisher–Yates shuffle (backward), consuming the `mulberry32` stream exactly as GG1's
/// `seededShuffle` does — one `rng()` per `i` from `len-1` down to `1`.
fn seeded_shuffle<T>(arr: &mut [T], rng: &mut crate::synth::Rng) {
    for i in (1..arr.len()).rev() {
        let j = (rng.next() * (i as f64 + 1.0)).floor() as usize;
        arr.swap(i, j);
    }
}

/// Build event `eid`'s deterministic gauntlet (the exact same question set + order every play and
/// every 14-day recurrence). Panics on an unknown event id.
pub fn build_gauntlet(eid: &str) -> Vec<GauntletQ> {
    let ev = roster()
        .into_iter()
        .find(|e| e.id == eid)
        .unwrap_or_else(|| panic!("unknown event {eid}"));
    let seed = crate::synth::hash_str(eid) ^ ev.art_seed;
    let mut rng = crate::synth::Rng::new(seed);
    let mut combined: Vec<GauntletQ> = Vec::new();
    for mix in &ev.question_mix {
        let pool = crate::transforms::generate(&mix.topic);
        // Sort the pool to a total order (numeric collation, raw-string tiebreak) so the shuffle is
        // reproducible regardless of the pool's natural build order.
        let mut idx: Vec<usize> = (0..pool.len()).collect();
        idx.sort_by(|&i, &j| {
            gauntlet_cmp(&pool[i].0, &pool[j].0).then_with(|| pool[i].0.cmp(&pool[j].0))
        });
        seeded_shuffle(&mut idx, &mut rng);
        for &i in idx.iter().take(mix.n) {
            combined.push(GauntletQ {
                p: pool[i].0.clone(),
                a: pool[i].1,
                topic: mix.topic.clone(),
            });
        }
    }
    // The themed interleave: shuffle the combined set with the same stream.
    seeded_shuffle(&mut combined, &mut rng);
    combined
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const VECTORS_JSON: &str = include_str!("../data/gg1/events-vectors.json");

    fn vectors() -> Value {
        serde_json::from_str(VECTORS_JSON).expect("events-vectors.json")
    }

    /// The roster is the 14-event rotation.
    #[test]
    fn roster_is_the_full_rotation() {
        assert_eq!(roster().len(), 14);
        assert_eq!(parse().rotation, 14);
    }

    /// The UTC-day schedule maps `epochDay`/`ms` → the live event (incl. negative days + far future).
    #[test]
    fn schedule_matches_vectors() {
        for s in vectors()["schedule"].as_array().unwrap() {
            let epoch_day = s["epochDay"].as_i64().unwrap();
            let ms = s["ms"].as_i64().unwrap();
            let index = s["index"].as_u64().unwrap() as usize;
            let live_id = s["liveId"].as_str().unwrap();
            assert_eq!(live_index(epoch_day), index, "index for day {epoch_day}");
            assert_eq!(roster()[index].id, live_id, "liveId for day {epoch_day}");
            assert_eq!(live_event(ms).id, live_id, "live_event(ms={ms})");
        }
    }

    /// The reward-tier keys (participation / well / ace) match across the score/total grid.
    #[test]
    fn reward_tiers_match_vectors() {
        for t in vectors()["tiers"].as_array().unwrap() {
            let eid = t["eid"].as_str().unwrap();
            let score = t["score"].as_u64().unwrap() as u32;
            let total = t["total"].as_u64().unwrap() as u32;
            let want: Vec<String> = t["ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            assert_eq!(
                event_tiers_earned(eid, score, total),
                want,
                "tiers for {eid} {score}/{total}"
            );
        }
    }

    /// The deterministic gauntlet reproduces every event's exact `{p,a,topic}` sequence — pinning the
    /// seed, the total-order sort, the seeded shuffle, and the topic pools all at once.
    #[test]
    fn gauntlet_matches_vectors() {
        let g = &vectors()["gauntlet"];
        for (eid, want) in g.as_object().unwrap() {
            let got = build_gauntlet(eid);
            let want_arr = want.as_array().unwrap();
            assert_eq!(got.len(), want_arr.len(), "gauntlet length for {eid}");
            for (i, (gq, wv)) in got.iter().zip(want_arr).enumerate() {
                assert_eq!(gq.p, wv["p"].as_str().unwrap(), "{eid}[{i}] prompt");
                assert_eq!(gq.a, wv["a"].as_f64().unwrap(), "{eid}[{i}] answer");
                assert_eq!(gq.topic, wv["topic"].as_str().unwrap(), "{eid}[{i}] topic");
            }
        }
    }
}
