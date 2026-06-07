//! G6 — the decipherment **lexicon**: a tiny seeded grammar that turns a *comprehended*
//! script's glowing glyphs into fragmentary **elegiac words** (game-mechanics §9;
//! procedural-poetic per §6). Deterministic in `seed + cell`, so a world always reads back the
//! same; no authored lore — the grief is in the *register*, composed from a small word-bank.
//!
//! Note: legibility only changes what's **displayed**. A find's id still hashes the original
//! glyphs (so collecting stays stable across comprehension) — see `progress::find_id`.

const OPENERS: &[&str] = &[
    "here lies",
    "the warden of",
    "we kept",
    "in memory of",
    "the last of",
    "beneath this",
    "do not wake",
    "they sang to",
];
const SUBJECTS: &[&str] = &[
    "the drowned light",
    "a sleeping engine",
    "the ninth choir",
    "the salt road",
    "the iron dark",
    "the tide-clock",
    "the grey sleepers",
    "the long signal",
];
const CODAS: &[&str] = &[
    "who slept",
    "and were forgotten",
    "still humming",
    "unanswered",
    "until the cold",
    "that we made",
    "now silent",
];

fn hash(seed: u32, cell: (i32, i32)) -> u32 {
    let mut h = seed.wrapping_mul(0x9E37_79B9)
        ^ (cell.0 as u32).wrapping_mul(0x85EB_CA6B)
        ^ (cell.1 as u32).wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    h
}

/// A deterministic translated phrase for the inscription in `cell` of `seed`. `glyphs` (the
/// original glyph count) nudges length: short inscriptions resolve to a fragment, longer ones
/// to a fuller line.
pub fn phrase(seed: u32, cell: (i32, i32), glyphs: u32) -> String {
    let h = hash(seed, cell);
    let opener = OPENERS[(h % OPENERS.len() as u32) as usize];
    let subject = SUBJECTS[((h >> 8) % SUBJECTS.len() as u32) as usize];
    if glyphs <= 3 {
        subject.to_string()
    } else if glyphs <= 6 {
        format!("{opener} {subject}")
    } else {
        let coda = CODAS[((h >> 16) % CODAS.len() as u32) as usize];
        format!("{opener} {subject}, {coda}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_is_deterministic_and_varied() {
        assert_eq!(phrase(1337, (3, -2), 8), phrase(1337, (3, -2), 8)); // stable
        assert_ne!(phrase(1337, (3, -2), 8), phrase(1337, (4, -2), 8)); // cell matters
                                                                        // Length tiers compose differently.
        assert!(!phrase(1337, (3, -2), 2).contains(',')); // short → bare subject, no coda
        assert!(phrase(1337, (3, -2), 8).contains(',')); // long → has a coda
                                                         // Only ASCII words (render in the Latin font when legible).
        assert!(phrase(7, (1, 1), 8).is_ascii());
    }
}
