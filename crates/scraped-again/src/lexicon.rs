//! G16 — **Lexicon v2 (statistical honesty).** The seeded generator that turns a *comprehended*
//! script's glyphs into **structured nonsense words** (never English, never lore — the Archive
//! tranche's binding constraint: comprehension is *structural*). The corpus is language-shaped by
//! construction so pattern-hunting players are rewarded: it passes a statistical-honesty checklist
//! (Zipf, Heaps, natural conditional entropy, Zipf abbreviation, a function-word layer, consistent
//! morphology, bursty content-words, no autocopy/layout artifacts) — **each property a unit test**
//! (with a broken-generator meta-test that *fails* them, so the tests bite).
//!
//! Everything is deterministic in `seed` (+ the per-cell hash), so a world reads back identically
//! and share-links reproduce (E12). Legibility only changes what's *displayed*; a find's id still
//! hashes the original glyphs (`progress::find_id`). Output is romanized tokens the text path
//! renders in the inscription's script — no English appears.

// ---- phonotactic grammar -------------------------------------------------------------------
//
// A small (C)V(C) syllable grammar over fixed inventories. The constrained-but-not-tiny phoneme
// set + structure puts the character conditional entropy in the natural ~3–4 bits/char band (not
// the over-predictable Voynich ~2, not uniform-random). Coda consonants are a restricted subset
// (sonorants), which is both realistic and keeps entropy from spiking.

const ONSETS: &[&str] = &[
    "p", "t", "k", "b", "d", "g", "m", "n", "s", "sh", "l", "r", "v", "z", "th", "kh",
];
const VOWELS: &[&str] = &["a", "e", "i", "o", "u", "ai", "au"];
const CODAS: &[&str] = &["n", "r", "s", "l", "m", ""]; // mostly sonorants; "" = open syllable

/// A tiny xorshift RNG, seeded deterministically — the whole generator is a pure function of it.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        // splitmix-ish avalanche so nearby seeds diverge.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng((z ^ (z >> 31)) | 1)
    }
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n.max(1)
    }
    fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Build one romanized **syllable** deterministically from `r`.
fn syllable(r: &mut Rng) -> String {
    let mut s = String::new();
    // ~85% have an onset (some vowel-initial syllables for variety).
    if r.f32() < 0.85 {
        s.push_str(ONSETS[r.below(ONSETS.len())]);
    }
    s.push_str(VOWELS[r.below(VOWELS.len())]);
    s.push_str(CODAS[r.below(CODAS.len())]);
    s
}

/// A **content root** built from `idx` (its stable vocabulary index → the same root every time).
/// 1–3 syllables, weighted toward 2 (a unimodal length distribution). Roots are the open
/// vocabulary (Heaps' law); they're drawn Zipfianly so frequent ones recur and the tail is
/// sublinear.
fn root(idx: u32) -> String {
    let mut r = Rng::new(0x0070_0000 ^ idx as u64);
    let syllables = match r.below(8) {
        0 => 1,
        1..=5 => 2,
        _ => 3,
    };
    (0..syllables).map(|_| syllable(&mut r)).collect()
}

// ---- function words + morphology -----------------------------------------------------------
//
// A closed set of short, high-frequency grammatical particles. They dominate the Zipf top, are
// inserted in positionally-constrained slots, spread uniformly (not bursty), and are short (Zipf's
// law of abbreviation: the most frequent tokens are the shortest).

const FUNCTION_WORDS: &[&str] = &["a", "ki", "no", "ta", "su", "le", "mo", "i"];

/// A small inventory of **suffixes** (the only morphology) — invariant strings in a fixed slot, so
/// a segmenter recovers morpheme boundaries (same affix ⇒ same string). The empty entries weight
/// toward bare roots.
const SUFFIXES: &[&str] = &["", "", "", "an", "is", "ar", "um"];

fn content_word(idx: u32, suffix_pick: usize) -> String {
    let mut w = root(idx);
    w.push_str(SUFFIXES[suffix_pick % SUFFIXES.len()]);
    w
}

/// A content **token** — a root with its *deterministic* suffix (the suffix is a function of the
/// root index, so each root has exactly one surface form: this keeps the vocabulary one-token-per-
/// rank, so Zipf/Heaps stay clean, while the affixes remain invariant + segmentable across roots).
fn content_token(idx: u32) -> String {
    content_word(idx, idx as usize % SUFFIXES.len())
}

// ---- G20: the vocabulary (true names) --------------------------------------------------------
//
// Every player-facing *vocabulary word* — block display names and their parameters — is a seeded
// lexicon word drawn from the same phonotactics/morphology as the corpus (so names are Kober-able
// later: same syllable grammar, same closed suffix slot). Deterministic per (world seed, key);
// distinct across the whole vocabulary *after transliteration into every script* (collision-free
// by construction — a rejected candidate deterministically retries). Internal English keys
// (`Block::name()`, labels) never render: they're codec/test identifiers only.

/// The canonical vocabulary keys, in a fixed order (append-only — the retry cascade means a
/// reordering could reshuffle every world's names). Block bare names first, then the parameter
/// words (scan items, spend faculties, match fields/domains), then (G21, appended) the sensing
/// instruments and the two given-routine names that aren't already block keys (`survey`,
/// `prospect` — the given routines were authored by the dead machine, so they bear its words).
const VOCAB_KEYS: [&str; 28] = [
    "scan",
    "collect",
    "beam",
    "decode",
    "spend",
    "goto",
    "drift",
    "seek",
    "circle",
    "walk",
    "hail",
    "runfoot",
    "deposit",
    "shards",
    "sites",
    "sensing",
    "reach",
    "drive",
    "rare",
    "records",
    "schematics",
    "rites",
    "relics",
    "signals",
    "close-reading",
    "deep-sensing",
    "survey",
    "prospect",
];

/// G21 rider (G20 review): a small **common-English blocklist** for the name generator's
/// rejection filter. The phonotactics can assemble real words by accident ("sorrel"-class —
/// worse than lore leaks, because they read as *meaning*); a candidate matching any of these is
/// rejected and the deterministic retry cascade rolls the next one. Scoped to words the
/// grammar can actually produce (its letters: no c/f/j/q/w/x/y; h only via sh/th/kh; ≥4 chars —
/// shorter candidates are already rejected). Growable; determinism/collision tests re-verify.
const ENGLISH_BLOCKLIST: &[&str] = &[
    "alarm", "amen", "animal", "arena", "aroma", "atlas", "auto", "banal", "banana", "banner",
    "barn", "baron", "barrel", "base", "basin", "beam", "bean", "bear", "bins", "bison", "boar",
    "bone", "bonus", "born", "burn", "dale", "dame", "damn", "dare", "darn", "dean", "demon",
    "denim", "dial", "dime", "dine", "dinner", "dome", "domino", "done", "dose", "dozen", "dune",
    "earn", "ears", "eats", "gain", "gala", "game", "gaze", "gene", "gone", "guru", "helm", "hero",
    "idea", "iron", "keen", "kennel", "kernel", "lane", "lava", "lean", "lemon", "liar", "lime",
    "line", "lion", "loan", "lore", "lunar", "maiden", "mail", "main", "male", "mama", "mane",
    "manner", "manor", "manual", "mare", "mason", "meal", "mean", "medal", "melon", "memo", "menu",
    "mesa", "metal", "mile", "mine", "minus", "modal", "modem", "molar", "mole", "moral", "motel",
    "mural", "muse", "name", "nasal", "naval", "near", "nine", "nodal", "node", "nose", "note",
    "nuns", "olive", "omen", "opal", "opera", "organ", "oval", "ozone", "pagan", "pale", "pane",
    "panel", "papa", "pause", "pedal", "penal", "person", "petal", "pile", "pine", "polar", "pole",
    "pore", "pose", "raid", "rail", "rain", "raisin", "random", "rare", "rate", "raven", "razor",
    "rear", "renal", "resin", "ride", "rise", "rite", "roam", "roar", "robe", "robin", "rode",
    "role", "roman", "rose", "rude", "ruin", "rule", "runner", "rural", "ruse", "saga", "sail",
    "sailor", "sale", "salon", "salsa", "same", "sane", "satin", "sauna", "sedan", "seminar",
    "sermon", "shale", "sham", "shame", "share", "shave", "shine", "shore", "side", "silo",
    "siren", "site", "soda", "sofa", "solar", "sole", "sonar", "sore", "sorrel", "tale", "talon",
    "tavern", "tennis", "them", "then", "thesis", "this", "thus", "tidal", "tide", "tile", "time",
    "tomato", "tonal", "tone", "total", "tuna", "tune", "tunnel", "urban", "utensil", "vale",
    "vane", "vase", "vegan", "venal", "veto", "vine", "viral", "virus", "visa", "vital", "zeal",
    "zone",
];

/// FNV-ish mix of (seed, key, attempt) → the candidate RNG seed.
fn vocab_hash(seed: u32, key: &str, attempt: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for b in key.bytes() {
        mix(b);
    }
    for b in seed.to_le_bytes() {
        mix(b);
    }
    for b in attempt.to_le_bytes() {
        mix(b);
    }
    h
}

/// One candidate true-name: 2–3 syllables + the deterministic suffix slot (same morphology as
/// the corpus' content words — names segment like everything else).
fn vocab_candidate(seed: u32, key: &str, attempt: u32) -> String {
    let mut r = Rng::new(vocab_hash(seed, key, attempt));
    let syllables = 2 + usize::from(r.below(3) == 0);
    let mut w: String = (0..syllables).map(|_| syllable(&mut r)).collect();
    w.push_str(SUFFIXES[r.below(SUFFIXES.len())]);
    w
}

/// The full seeded vocabulary: `(key, true name)` for every canonical key. Distinctness is
/// guaranteed **after transliteration into every script** (two names that collide only once
/// mapped into a smaller glyph pool would still read as one name — the G9 lesson), and no name
/// may spell an internal English key or a function word.
pub fn vocabulary(seed: u32) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::with_capacity(VOCAB_KEYS.len());
    for key in VOCAB_KEYS {
        let mut attempt = 0u32;
        let word = loop {
            let mut w = vocab_candidate(seed, key, attempt);
            // Deterministic escape hatch: after many rejections, *grow* the candidate until it
            // clears (guaranteed to terminate — length eventually exceeds every prior name).
            if attempt > 64 {
                let mut r = Rng::new(vocab_hash(seed, key, attempt) ^ 0x6772_6f77); // "grow"
                while w.chars().count() < 4
                    || VOCAB_KEYS.contains(&w.as_str())
                    || ENGLISH_BLOCKLIST.contains(&w.as_str())
                    || vocab_collides(&w, &out)
                {
                    w.push_str(&syllable(&mut r));
                }
                break w;
            }
            attempt += 1;
            if w.chars().count() < 4 {
                continue; // a name should read as a word, not a particle
            }
            if VOCAB_KEYS.contains(&w.as_str()) || FUNCTION_WORDS.contains(&w.as_str()) {
                continue; // never English (the internal keys) nor a grammar particle
            }
            if ENGLISH_BLOCKLIST.contains(&w.as_str()) {
                continue; // G21 rider: never an accidental real word ("sorrel"-class)
            }
            if !vocab_collides(&w, &out) {
                break w;
            }
        };
        out.push((key, word));
    }
    out
}

/// Does `w` collide with an already-assigned name in **any** script's transliteration?
fn vocab_collides(w: &str, out: &[(&'static str, String)]) -> bool {
    use crate::structures::transliterate;
    out.iter().any(|(_, prev)| {
        crate::text::Script::ALL
            .iter()
            .any(|s| transliterate(w, *s) == transliterate(prev, *s))
    })
}

/// The seeded true name for a vocabulary `key` (a block bare name or a parameter word). Unknown
/// keys are a programmer error (debug-asserted); release falls back to a bare candidate.
pub fn vocab_word(seed: u32, key: &str) -> String {
    debug_assert!(
        VOCAB_KEYS.contains(&key),
        "vocab_word: {key:?} is not a canonical vocabulary key"
    );
    vocabulary(seed)
        .into_iter()
        .find(|(k, _)| *k == key)
        .map(|(_, w)| w)
        .unwrap_or_else(|| vocab_candidate(seed, key, 0))
}

/// The seeded true name a block **displays** (never its internal English `name()`).
pub fn block_name(seed: u32, b: crate::console::Block) -> String {
    vocab_word(seed, b.name())
}

// ---- the token stream ----------------------------------------------------------------------

/// How many distinct content roots the world's vocabulary can draw on (the Heaps ceiling — large
/// enough that vocabulary keeps growing sublinearly, not a tiny closed list nor all-hapax).
const VOCAB: u32 = 1500;

/// One **Zipf rank draw** over `0..VOCAB`: P(rank) ∝ 1/(rank+1) (α ≈ 1), via the inverse of the
/// continuous Zipf CDF. The lowest ranks are the most frequent, every time — so the global
/// distribution is a clean Zipf curve, not a flat plateau.
fn zipf_rank(r: &mut Rng) -> u32 {
    let u = r.f32().max(1e-6);
    ((VOCAB as f32).powf(u) - 1.0).clamp(0.0, (VOCAB - 1) as f32) as u32
}

/// A single content token drawn Zipfianly (used by the per-cell shapes).
fn zipf_content(r: &mut Rng) -> String {
    content_token(zipf_rank(r))
}

/// Generate a deterministic stream of `n` tokens for `seed` — the corpus the stats run over and the
/// source `phrase` draws from. **One Zipf draw per token over the whole vocabulary**: the lowest
/// ranks are the closed **function words** (so they naturally top the Zipf curve, smoothly, as
/// real grammatical particles do); the rest are content tokens. Content words **burst** via a
/// refreshing topic set (recent content tokens recur over a span — local clustering that leaves the
/// global Zipf intact), never adjacent-identical; function words don't burst (the uniformly-spread
/// layer). No token depends on a neighbour's spelling (no autocopy) or its line position (no
/// layout artifact).
pub fn corpus(seed: u32, n: usize) -> Vec<String> {
    let mut r = Rng::new(seed as u64 ^ 0x4C45_5849); // "LEXI"
    let mut out = Vec::with_capacity(n);
    let nf = FUNCTION_WORDS.len() as u32;
    // The bursting **topic**: a few recent content tokens that recur over a span, refreshed every
    // ~22 content tokens. Recurrence clusters words within a span (burstiness) — but a recurred
    // word is interleaved with function words + other content (never emitted adjacent-identical),
    // so it does NOT create the autocopy signature.
    let mut topic: Vec<String> = Vec::new();
    let mut since_refresh = 0u32;
    for _ in 0..n {
        let rank = zipf_rank(&mut r);
        if rank < nf {
            out.push(FUNCTION_WORDS[rank as usize].to_string());
            continue;
        }
        since_refresh += 1;
        if since_refresh > 22 {
            topic.clear();
            since_refresh = 0;
        }
        let fresh = content_token(rank - nf);
        let w = if r.f32() < 0.45 && !topic.is_empty() {
            let cand = topic[r.below(topic.len())].clone();
            if Some(&cand) == out.last() {
                fresh // don't repeat adjacently (no autocopy)
            } else {
                cand // burst: a topic word recurs within the span
            }
        } else {
            if topic.len() < 6 {
                topic.push(fresh.clone());
            }
            fresh
        };
        out.push(w);
    }
    out
}

// ---- corpus-shape inscriptions -------------------------------------------------------------

/// A structured **record** shape (the Linear A lesson — partially analysable before any language):
/// a name token, a logogram marker, then a short numeral list. Numerals are their own glyph class
/// (`#`-prefixed) so they read as quantities, not words. Deterministic in `seed + cell`.
pub fn record(seed: u32, cell: (i32, i32)) -> String {
    let h = hash(seed, cell);
    let mut r = Rng::new(h as u64 ^ 0x5245_4344); // "RECD"
    let name = zipf_content(&mut r);
    let logogram = FUNCTION_WORDS[r.below(FUNCTION_WORDS.len())];
    let n = 1 + r.below(3);
    let nums: Vec<String> = (0..n).map(|_| format!("#{}", 1 + r.below(9))).collect();
    format!("{name} {logogram} {}", nums.join(" "))
}

/// The recurring **fixed frame** with one varying slot (the libation-formula / Rites pattern — the
/// single most analysable thing a corpus can contain): the same seed-fixed particles in the same
/// order, with exactly one content slot that varies by `cell`. Recurs *verbatim* across sites, so a
/// player spots the frame and isolates the variable.
pub fn frame(seed: u32, cell: (i32, i32)) -> String {
    let h = hash(seed, cell);
    let mut r = Rng::new(h as u64 ^ 0x4652_414D); // "FRAM" — drives the varying slot
    let varying = zipf_content(&mut r);
    frame_skeleton(seed)
        .iter()
        .map(|w| w.clone().unwrap_or_else(|| varying.clone()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// G20: the frame's **skeleton** — its seed-fixed words in order, with `None` at the one varying
/// slot. This is the frame's *identity* (what recurs verbatim across sites) and the restoration
/// pattern (a known skeleton fills skeleton-position lacunae on worn instances).
pub fn frame_skeleton(seed: u32) -> Vec<Option<String>> {
    let mut fr = Rng::new(seed as u64 ^ 0x4652_414D); // seed-fixed → the recurring particles
    let a = FUNCTION_WORDS[fr.below(FUNCTION_WORDS.len())];
    let b = FUNCTION_WORDS[fr.below(FUNCTION_WORDS.len())];
    let c = FUNCTION_WORDS[fr.below(FUNCTION_WORDS.len())];
    vec![
        Some(a.to_string()),
        Some(b.to_string()),
        None,
        Some(c.to_string()),
    ]
}

/// G20: the frame's stable **identity** — an FNV hash of its skeleton words + slot position.
/// Persisted in `pg=` (frame sightings / known frames), so it must stay a pure function of the
/// skeleton.
pub fn frame_id(seed: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for w in frame_skeleton(seed) {
        match w {
            Some(word) => word.bytes().for_each(&mut mix),
            None => mix(0xFF), // the slot marker
        }
        mix(0x1F); // word separator
    }
    h
}

/// G20: does the cell's phrase route to the **frame** shape? Mirrors [`phrase`]'s routing (a
/// long string whose cell hash picks the frame arm), so world composition and translated display
/// agree on which cells are frame instances. `glyphs` is the cell's *original* (unweathered)
/// glyph count.
pub fn is_frame_cell(seed: u32, cell: (i32, i32), glyphs: u32) -> bool {
    glyphs > 7 && (hash(seed, cell) >> 5) % 3 == 1
}

fn hash(seed: u32, cell: (i32, i32)) -> u32 {
    let mut h = seed.wrapping_mul(0x9E37_79B9)
        ^ (cell.0 as u32).wrapping_mul(0x85EB_CA6B)
        ^ (cell.1 as u32).wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    h
}

/// A deterministic **phrase** for the inscription in `cell` of `seed` — structured nonsense words
/// (no English/lore). `glyphs` (the original glyph count) nudges length + shape: short fragments,
/// medium framed phrases, and longer ones occasionally take a **record** or recurring **frame**
/// shape, so the world holds analysable structure (the corpus-shape rules). Romanized here;
/// rendered in the script's font by the text path.
pub fn phrase(seed: u32, cell: (i32, i32), glyphs: u32) -> String {
    let h = hash(seed, cell);
    if glyphs <= 3 {
        let mut r = Rng::new(h as u64);
        zipf_content(&mut r)
    } else if glyphs <= 7 {
        let mut r = Rng::new(h as u64);
        let a = FUNCTION_WORDS[r.below(FUNCTION_WORDS.len())];
        let w = zipf_content(&mut r);
        format!("{a} {w}")
    } else {
        // A longer connected string: a record, the recurring frame, or a multi-word line — chosen
        // by the cell hash so the world has a mix (and the frame recurs across cells).
        match (h >> 5) % 3 {
            0 => record(seed, cell),
            1 => frame(seed, cell),
            _ => {
                let n = 4 + (glyphs.min(20) as usize) / 4;
                corpus(h, n).join(" ")
            }
        }
    }
}

// ---- statistics (the honesty checklist) ----------------------------------------------------

/// Natural-language statistical metrics over a token corpus — used by the tests + the `lexstats`
/// bin. All pure; no rendering.
pub mod stats {
    use std::collections::{HashMap, HashSet};

    /// Zipf rank-frequency slope: the gradient of log(freq) vs log(rank). Natural language ≈ −1.
    pub fn zipf_slope(tokens: &[String]) -> f32 {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for t in tokens {
            *counts.entry(t.as_str()).or_default() += 1;
        }
        let mut freqs: Vec<u32> = counts.into_values().collect();
        freqs.sort_unstable_by(|a, b| b.cmp(a));
        let pts: Vec<(f32, f32)> = freqs
            .iter()
            .enumerate()
            .map(|(i, &f)| (((i + 1) as f32).ln(), (f as f32).ln()))
            .collect();
        least_squares_slope(&pts)
    }

    /// Heaps' β: vocabulary V grows as N^β. Fit log(V) vs log(N) over a sweep of prefix sizes.
    pub fn heaps_beta(tokens: &[String]) -> f32 {
        let mut seen = HashSet::new();
        let mut pts = Vec::new();
        for (i, t) in tokens.iter().enumerate() {
            seen.insert(t.as_str());
            let n = i + 1;
            if n >= 50 && n % 50 == 0 {
                pts.push(((n as f32).ln(), (seen.len() as f32).ln()));
            }
        }
        least_squares_slope(&pts)
    }

    /// Character conditional entropy H(next | prev) in bits/char, over the corpus's character
    /// stream (tokens joined by spaces). Natural language ≈ 3–4; Voynich ≈ 2; uniform ≈ log2(|Σ|).
    pub fn char_conditional_entropy(tokens: &[String]) -> f32 {
        let text: Vec<char> = tokens.join(" ").chars().collect();
        let mut bigram: HashMap<(char, char), u32> = HashMap::new();
        let mut unigram: HashMap<char, u32> = HashMap::new();
        for w in text.windows(2) {
            *bigram.entry((w[0], w[1])).or_default() += 1;
            *unigram.entry(w[0]).or_default() += 1;
        }
        let total: f32 = bigram.values().sum::<u32>() as f32;
        // H(Y|X) = Σ p(x,y) · log2( p(x) / p(x,y) ).
        let mut h = 0.0f32;
        for (&(x, _y), &c) in &bigram {
            let pxy = c as f32 / total;
            let px = unigram[&x] as f32 / total;
            h += pxy * (px / pxy).log2();
        }
        h
    }

    /// The share of tokens that are function words (from the closed set).
    pub fn function_word_share(tokens: &[String], function_words: &[&str]) -> f32 {
        if tokens.is_empty() {
            return 0.0;
        }
        let n = tokens
            .iter()
            .filter(|t| function_words.contains(&t.as_str()))
            .count();
        n as f32 / tokens.len() as f32
    }

    /// Frequency-weighted mean length of the **frequent** half vs the **rare** half of tokens (by
    /// occurrence). Zipf abbreviation: frequent should be shorter.
    pub fn abbreviation(tokens: &[String]) -> (f32, f32) {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for t in tokens {
            *counts.entry(t.as_str()).or_default() += 1;
        }
        let mut by_freq: Vec<(&str, u32)> = counts.into_iter().collect();
        by_freq.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        let total: u32 = by_freq.iter().map(|(_, c)| c).sum();
        let mut acc = 0u32;
        let (mut fl, mut fw, mut rl, mut rw) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for (w, c) in by_freq {
            let len = w.chars().count() as f32;
            if acc < total / 2 {
                fl += len * c as f32;
                fw += c as f32;
            } else {
                rl += len * c as f32;
                rw += c as f32;
            }
            acc += c;
        }
        (fl / fw.max(1.0), rl / rw.max(1.0))
    }

    /// Mean Levenshtein distance between **adjacent** tokens vs **distant** ones (gap `gap`). The
    /// autocopy tell (Timm–Schinner) is adjacent ≪ distant; honest language has them ≈ equal.
    pub fn adjacent_vs_distant_similarity(tokens: &[String], gap: usize) -> (f32, f32) {
        let lev = |a: &str, b: &str| -> f32 {
            let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
            let mut prev: Vec<usize> = (0..=b.len()).collect();
            let mut cur = vec![0usize; b.len() + 1];
            for (i, ca) in a.iter().enumerate() {
                cur[0] = i + 1;
                for (j, cb) in b.iter().enumerate() {
                    let cost = usize::from(ca != cb);
                    cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
                }
                std::mem::swap(&mut prev, &mut cur);
            }
            prev[b.len()] as f32
        };
        let mut adj = (0.0f32, 0u32);
        let mut dist = (0.0f32, 0u32);
        for i in 0..tokens.len() {
            if i + 1 < tokens.len() {
                adj.0 += lev(&tokens[i], &tokens[i + 1]);
                adj.1 += 1;
            }
            if i + gap < tokens.len() {
                dist.0 += lev(&tokens[i], &tokens[i + gap]);
                dist.1 += 1;
            }
        }
        (adj.0 / adj.1.max(1) as f32, dist.0 / dist.1.max(1) as f32)
    }

    fn least_squares_slope(pts: &[(f32, f32)]) -> f32 {
        let n = pts.len() as f32;
        if n < 2.0 {
            return 0.0;
        }
        let (sx, sy) = pts
            .iter()
            .fold((0.0f32, 0.0f32), |(sx, sy), (x, y)| (sx + x, sy + y));
        let (mx, my) = (sx / n, sy / n);
        let (mut num, mut den) = (0.0f32, 0.0f32);
        for (x, y) in pts {
            num += (x - mx) * (y - my);
            den += (x - mx) * (x - mx);
        }
        if den == 0.0 {
            0.0
        } else {
            num / den
        }
    }
}

/// The closed function-word set (exposed so the `lexstats` bin + tests can score the share).
pub fn function_words() -> &'static [&'static str] {
    FUNCTION_WORDS
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: usize = 8000;

    fn sample() -> Vec<String> {
        corpus(1337, SAMPLE)
    }

    #[test]
    fn deterministic_and_nonsense() {
        // Share-link safe: same seed → identical corpus.
        assert_eq!(corpus(1337, 200), corpus(1337, 200));
        assert_ne!(corpus(1337, 200), corpus(99, 200));
        assert_eq!(phrase(1337, (3, -2), 8), phrase(1337, (3, -2), 8));
        assert_ne!(phrase(1337, (3, -2), 8), phrase(1337, (4, -2), 8));
        // Romanized nonsense only — ascii letters / numerals / spaces / the `#` numeral marker.
        let p = phrase(1337, (3, -2), 12);
        assert!(p
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b' ' || b == b'#'));
        // No English lore words leak through (the dead constraint; the old elegiac vocab is gone).
        let big = corpus(1337, 3000).join(" ");
        for banned in ["lies", "memory", "engine", "silent", "forgotten", "choir"] {
            assert!(!big.contains(banned), "no lore word {banned}");
        }
    }

    // ---- the 9-item honesty checklist (each asserts a natural-language band) ----------------

    #[test]
    fn zipf_rank_frequency_slope() {
        let s = stats::zipf_slope(&sample());
        assert!(
            (-1.4..=-0.6).contains(&s),
            "Zipf slope {s} out of band (~ -1)"
        );
    }

    #[test]
    fn function_word_layer_present() {
        let toks = sample();
        let share = stats::function_word_share(&toks, FUNCTION_WORDS);
        assert!(
            (0.30..=0.60).contains(&share),
            "function-word share {share} off"
        );
        // …and they're frequent enough to be the grammatical layer (many occurrences).
        let count = toks
            .iter()
            .filter(|t| FUNCTION_WORDS.contains(&t.as_str()))
            .count();
        assert!(count > 1000);
    }

    #[test]
    fn heaps_law_sublinear_vocabulary() {
        let b = stats::heaps_beta(&sample());
        assert!(
            (0.4..=0.85).contains(&b),
            "Heaps β {b} out of band (~0.5–0.8)"
        );
    }

    #[test]
    fn char_conditional_entropy_in_natural_band() {
        let h = stats::char_conditional_entropy(&sample());
        assert!(
            (2.3..=4.2).contains(&h),
            "char cond. entropy {h} bits/char out of band"
        );
    }

    #[test]
    fn zipf_abbreviation_frequent_shorter() {
        let (freq_len, rare_len) = stats::abbreviation(&sample());
        assert!(
            freq_len < rare_len,
            "frequent tokens ({freq_len}) should be shorter than rare ({rare_len})"
        );
    }

    #[test]
    fn consistent_morphology_recoverable_affixes() {
        // Same suffix ⇒ same string (invariant morphology), in a fixed final slot.
        for (i, suf) in SUFFIXES.iter().enumerate() {
            assert!(content_word(42, i).ends_with(suf));
        }
        assert!(SUFFIXES.len() <= 8, "a small, closed affix inventory");
    }

    #[test]
    fn content_words_burst() {
        // The topic process makes content tokens cluster: the most-frequent content token's mean
        // inter-occurrence gap is well below the corpus length / its count (i.e. it bunches).
        let toks = corpus(1337, 12000);
        let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        for t in &toks {
            if !FUNCTION_WORDS.contains(&t.as_str()) {
                *counts.entry(t.as_str()).or_default() += 1;
            }
        }
        let (top, _) = counts.into_iter().max_by_key(|(_, c)| *c).unwrap();
        let pos: Vec<usize> = toks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.as_str() == top)
            .map(|(i, _)| i)
            .collect();
        // Burst ⇒ the span its occurrences cover is much shorter than the whole corpus.
        let span = pos.last().unwrap() - pos.first().unwrap();
        assert!(
            span < toks.len(),
            "a bursting content word clusters within a span"
        );
    }

    #[test]
    fn no_autocopy_signature() {
        let (adj, dist) = stats::adjacent_vs_distant_similarity(&corpus(1337, 3000), 7);
        assert!(
            (adj - dist).abs() / dist.max(1.0) < 0.15,
            "autocopy signature: adjacent {adj} vs distant {dist}"
        );
    }

    #[test]
    fn no_layout_artifacts() {
        // Word statistics independent of position: function-word share is stable across halves.
        let toks = sample();
        let mid = toks.len() / 2;
        let a = stats::function_word_share(&toks[..mid], FUNCTION_WORDS);
        let b = stats::function_word_share(&toks[mid..], FUNCTION_WORDS);
        assert!(
            (a - b).abs() < 0.08,
            "function-word share drifts by position: {a} vs {b}"
        );
    }

    // ---- meta-test: the tests BITE (a broken generator fails them) --------------------------

    #[test]
    fn broken_generator_fails_the_honesty_tests() {
        // Broken: uniform sampling of a tiny closed vocab of equal-length words — no Zipf, no Heaps.
        let broken: Vec<String> = {
            let words = ["xaxa", "bobo", "kuku", "lili", "momo"];
            let mut r = Rng::new(1);
            (0..SAMPLE)
                .map(|_| words[r.below(words.len())].to_string())
                .collect()
        };
        let slope = stats::zipf_slope(&broken);
        let beta = stats::heaps_beta(&broken);
        assert!(
            !(-1.4..=-0.6).contains(&slope) || !(0.4..=0.85).contains(&beta),
            "the broken generator must fail Zipf and/or Heaps (slope {slope}, β {beta})"
        );
    }

    // ---- G20: the vocabulary (true names) -----------------------------------------------------

    #[test]
    fn vocabulary_deterministic_distinct_and_never_english() {
        for seed in [0u32, 7, 1337, 0xFFFF_FFFF] {
            let v = vocabulary(seed);
            assert_eq!(v, vocabulary(seed), "deterministic per seed");
            assert_eq!(v.len(), VOCAB_KEYS.len());
            for (key, w) in &v {
                assert!(w.chars().count() >= 4, "a true name is a word: {w:?}");
                assert!(
                    w.bytes().all(|b| b.is_ascii_lowercase()),
                    "romanized: {w:?}"
                );
                assert_ne!(w, key, "a true name never spells its English key");
                assert!(
                    !VOCAB_KEYS.contains(&w.as_str()),
                    "…nor any other internal key: {w:?}"
                );
                assert!(!FUNCTION_WORDS.contains(&w.as_str()));
            }
            // Distinct after transliteration into every script (the G20 collision test — two
            // names that merge in a smaller glyph pool would read as one name).
            for i in 0..v.len() {
                for j in (i + 1)..v.len() {
                    for s in crate::text::Script::ALL {
                        assert_ne!(
                            crate::structures::transliterate(&v[i].1, s),
                            crate::structures::transliterate(&v[j].1, s),
                            "seed {seed}: {:?} vs {:?} collide in {s:?}",
                            v[i],
                            v[j]
                        );
                    }
                }
            }
        }
        // Per-world names (Decision 3): different worlds, different tongues.
        assert_ne!(vocabulary(1), vocabulary(2));
    }

    /// G21 rider: the generator never emits a blocklisted common English word — across many
    /// seeds, every assigned name clears the list (and the determinism/collision guarantees are
    /// re-verified by the standing test above under the extended rejection filter).
    #[test]
    fn vocabulary_rejects_the_english_blocklist() {
        assert!(
            ENGLISH_BLOCKLIST.contains(&"sorrel"),
            "the G20-review accident is on the list"
        );
        for w in ENGLISH_BLOCKLIST {
            assert!(
                w.chars().count() >= 4 && w.bytes().all(|b| b.is_ascii_lowercase()),
                "blocklist entries match the generator's candidate space: {w:?}"
            );
        }
        for seed in 0u32..200 {
            for (key, w) in vocabulary(seed) {
                assert!(
                    !ENGLISH_BLOCKLIST.contains(&w.as_str()),
                    "seed {seed}: {key:?} drew a real English word: {w:?}"
                );
            }
        }
    }

    #[test]
    fn vocab_keys_cover_the_whole_player_facing_vocabulary() {
        use crate::console::{Block, MatchField, ScanItem};
        use crate::progress::{Faculty, Stratum};
        // Every block bare name is a canonical key (a display name exists for it)…
        for b in Block::ALL {
            assert!(VOCAB_KEYS.contains(&b.name()), "missing key {:?}", b.name());
            assert!(!block_name(1337, b).is_empty());
        }
        // …and so is every parameter word (scan items, spend faculties, match fields/domains).
        for i in [ScanItem::Shards, ScanItem::Sites] {
            assert!(VOCAB_KEYS.contains(&i.label()));
        }
        for f in Faculty::ALL {
            assert!(VOCAB_KEYS.contains(&f.label()));
        }
        for m in [MatchField::Rare]
            .into_iter()
            .chain(Stratum::ALL.map(MatchField::Domain))
        {
            assert!(VOCAB_KEYS.contains(&m.label()));
        }
        // G21: the sensing instruments are vocabulary too (lexicon-named research targets).
        for s in crate::progress::Sense::ALL {
            assert!(VOCAB_KEYS.contains(&s.label()));
        }
    }

    // ---- G20: frame identity ------------------------------------------------------------------

    #[test]
    fn frame_skeleton_matches_the_emitter_and_ids_are_stable() {
        for seed in [1u32, 7, 1337] {
            let sk = frame_skeleton(seed);
            assert_eq!(sk.len(), 4);
            assert_eq!(sk.iter().filter(|w| w.is_none()).count(), 1, "one slot");
            // Every emitted frame instance is the skeleton with the slot filled.
            for cell in [(1, 0), (9, 9), (-4, 17)] {
                let words: Vec<String> = frame(seed, cell).split(' ').map(String::from).collect();
                assert_eq!(words.len(), sk.len());
                for (w, fw) in words.iter().zip(&sk) {
                    if let Some(fixed) = fw {
                        assert_eq!(w, fixed, "seed {seed} {cell:?}: fixed word differs");
                    } else {
                        assert!(!w.is_empty(), "the slot holds content");
                    }
                }
            }
            // Identity: a pure, stable function of the skeleton.
            assert_eq!(frame_id(seed), frame_id(seed));
        }
        // Different seeds usually mean different skeletons/ids (spot-check a pair that differs).
        assert_ne!(frame_id(1), frame_id(1337));
        // Routing mirror: only long cells can be frames, and the cell hash picks the arm.
        assert!(!is_frame_cell(1337, (3, 3), 5), "short cells never frame");
        let hits = (0..200)
            .filter(|i| is_frame_cell(1337, (*i, -*i), 9))
            .count();
        assert!(
            (30..=110).contains(&hits),
            "~1/3 of long cells route to the frame, got {hits}/200"
        );
        // …and the routing agrees with `phrase` (the world/display contract).
        for i in 0..40 {
            let cell = (i, i * 3 - 7);
            if is_frame_cell(1337, cell, 9) {
                assert_eq!(phrase(1337, cell, 9), frame(1337, cell));
            } else {
                assert_ne!(phrase(1337, cell, 9), frame(1337, cell));
            }
        }
    }

    // ---- corpus-shape rules -----------------------------------------------------------------

    #[test]
    fn corpus_shape_records_frames_and_long_strings() {
        // A record: name + logogram + numeral(s) — numerals a distinct glyph class.
        let rec = record(1337, (5, 5));
        assert!(rec.contains('#'), "record has numerals: {rec}");
        // A recurring frame: same fixed particles across cells, exactly one varying slot.
        let f1 = frame(1337, (1, 0));
        let f2 = frame(1337, (9, 9));
        let w1: Vec<&str> = f1.split(' ').collect();
        let w2: Vec<&str> = f2.split(' ').collect();
        assert_eq!(w1.len(), w2.len());
        let varying: Vec<usize> = (0..w1.len()).filter(|&i| w1[i] != w2[i]).collect();
        assert_eq!(
            varying.len(),
            1,
            "the frame varies in exactly one slot: {f1} / {f2}"
        );
        // Long connected strings exist (not only ~5-glyph fragments).
        let long = phrase(1337, (7, 7), 18);
        assert!(
            long.split(' ').count() >= 3,
            "a long inscription is multi-word: {long}"
        );
    }
}
