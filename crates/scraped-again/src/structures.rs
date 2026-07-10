//! World structures (E18) — large, seed-driven features placed *independently of chunk
//! terrain*: the first kind is the **fallen colossi** (giant collapsed figures from `body`).
//! A structure is positioned deterministically from the world seed on a coarse cell grid, so
//! the same seed always strews the same giants across the (infinite, streamed) world. More
//! structure kinds can join later behind the same placement scheme.

use glam::Vec3;

use crate::relic::Placement;
use crate::text::Script;

/// World-unit spacing of the candidate grid (one possible colossus per cell).
const CELL: f32 = 200.0;
/// Fraction of cells that actually hold a colossus (kept sparse so they feel monumental).
const PRESENCE: f32 = 0.4;

/// Hash a cell `(cx, cz)` + seed → a well-mixed `u32`.
fn hash(cx: i32, cz: i32, seed: u32) -> u32 {
    let mut h = (cx as u32).wrapping_mul(0x8DA6_B343)
        ^ (cz as u32).wrapping_mul(0xD8163841)
        ^ seed.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 13;
    h
}

/// All colossus placements whose anchor is within `radius` (world units, in the XZ plane) of
/// `cam`. Deterministic in `seed`; `ground(x, z)` drops each onto the terrain surface. Used by
/// the live app to stream giants in/out around the camera.
pub fn colossi_near(
    seed: u32,
    cam: Vec3,
    radius: f32,
    ground: impl Fn(f32, f32) -> f32,
) -> Vec<Placement> {
    let reach = (radius / CELL).ceil() as i32 + 1;
    // BUG1 defense-in-depth: extreme cam coords saturate the float→i32 cast, so the loop
    // bounds saturate too (share.rs clamps the share-link boundary; this guards the rest).
    let (ccx, ccz) = ((cam.x / CELL).floor() as i32, (cam.z / CELL).floor() as i32);
    let mut out = Vec::new();
    for cz in ccz.saturating_sub(reach)..=ccz.saturating_add(reach) {
        for cx in ccx.saturating_sub(reach)..=ccx.saturating_add(reach) {
            let h = hash(cx, cz, seed);
            // Jitter the anchor within the cell so they don't sit on a visible lattice.
            let jx = ((h >> 16) & 0xFF) as f32 / 255.0;
            let jz = ((h >> 24) & 0xFF) as f32 / 255.0;
            let x = cx as f32 * CELL + jx * CELL;
            let z = cz as f32 * CELL + jz * CELL;
            // Biome scales how many giants stand here (E10): ruined/barren biomes throng with
            // them, lush ones are sparse. Continuous field → density eases across borders.
            let pres = (PRESENCE * crate::biome::density(x, z, seed).2).min(0.95);
            if (h & 0xFFFF) as f32 / 65536.0 >= pres {
                continue;
            }
            let dx = x - cam.x;
            let dz = z - cam.z;
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            let p = hash(cx ^ 0x5A5A, cz ^ 0x3C3C, seed);
            // E18: a minority of giants are the fallen *human* figure (a fresh salt, so the
            // existing tube-tech placements are undisturbed). Humans render ethereal (points).
            let human =
                hash(cx ^ 0x6E11, cz ^ 0x4D22, seed.wrapping_add(0x4855_4D4E)).is_multiple_of(4);
            out.push(Placement {
                pos: Vec3::new(x, ground(x, z), z),
                yaw: (p % 6283) as f32 / 1000.0,
                voxel: 1.15 + ((p >> 5) % 70) as f32 / 100.0, // ~95–155 world units across
                seed: p | 1,
                // Solid (explorable) vs ethereal (points): ~half of humans are solid/landable;
                // ~1 in 3 of the tube-tech relics. The rest drift as ethereal point clouds.
                solid: if human {
                    (p >> 9).is_multiple_of(2)
                } else {
                    (p >> 12).is_multiple_of(3)
                },
                human,
            });
        }
    }
    out
}

/// A stable key for a placement's cell, so the live app can detect when the in-range set
/// changes (and only then rebuild the giants' point buffer).
pub fn cell_key(p: &Placement) -> (i32, i32) {
    (
        (p.pos.x / CELL).floor() as i32,
        (p.pos.z / CELL).floor() as i32,
    )
}

/// World-unit spacing of the inscription grid (E17) — denser than the colossi so glowing
/// text turns up fairly often as you fly.
const ICELL: f32 = 82.0;
/// Fraction of inscription cells that actually hold a marker.
const IPRESENCE: f32 = 0.5;

/// One seed-placed in-world inscription (E17): where it floats, what it says, in which script.
pub struct Inscription {
    /// Grid cell (the change key, so the app only rebuilds label textures when the set changes).
    pub cell: (i32, i32),
    pub pos: Vec3,
    pub text: String,
    pub script: Script,
    pub height: f32,
    pub color: [f32; 3],
    /// G9: this inscription spells a **block's name** — collecting it discovers the block.
    /// `None` = ambient glyph-noise (the melancholy majority). An ⟦erased⟧ inscription never
    /// carries a name (its content — name or noise — is unrecoverable, G18).
    pub name: Option<crate::console::Block>,
    /// G18: the inscription's seeded material **condition** (already applied to `text`).
    pub condition: Condition,
}

/// G18: an inscription's seeded material condition. `Worn(mask)` — bit `i` set ⇒ the `i`-th
/// non-space glyph position is lost to a lacuna (rendered as the engine's generic lacuna mark;
/// data yield drops with the surviving glyphs). `Erased` — deliberately struck out: the billboard
/// shows a gouge cluster, collecting yields nothing but logs the event (the G20 sensing-ladder
/// hook). Deterministic per cell from a **fresh-salt hash, independent of the name-gate bits**
/// (the G10 correlation lesson — tested).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Condition {
    Intact,
    Worn(u32),
    Erased,
}

/// G18: pick a condition from an (independent) hash for a text of `glyphs` non-space positions.
/// Mix: ~70% intact / ~27% worn / ~3% erased (Decision 2 placeholders; the feel pass tunes).
/// A worn mask always loses ≥1 glyph and keeps ≥1 survivor (a 1-glyph text can't be part-worn,
/// so it stays intact rather than degenerate to fully-lost).
fn condition_pick(ch: u32, glyphs: usize) -> Condition {
    match ch % 100 {
        0..=69 => Condition::Intact,
        70..=96 => {
            let n = glyphs.min(32) as u32;
            if n < 2 {
                return Condition::Intact;
            }
            // ~1/3 of positions lost, via a per-position xorshift stream off the hash.
            let mut s = (ch >> 7) | 1;
            let mut mask = 0u32;
            for i in 0..n {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                if s.is_multiple_of(3) {
                    mask |= 1 << i;
                }
            }
            // Clamp to a strict, non-empty subset: ≥1 lacuna, ≥1 survivor.
            if mask == 0 {
                mask |= 1 << ((ch >> 9) % n);
            }
            let full = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };
            if mask == full {
                mask &= !(1 << ((ch >> 13) % n));
            }
            Condition::Worn(mask)
        }
        _ => Condition::Erased,
    }
}

/// G18: the number of gouge cells an ⟦erased⟧ billboard renders — a constant-width strike so even
/// the original *length* is hidden (the erasure hides everything until G20's sensing ladder).
const GOUGE_CELLS: usize = 3;

/// G18: apply a [`Condition`] to composed text. Worn replaces the masked non-space glyph
/// positions with the engine's generic lacuna mark (spaces survive — word shape stays legible);
/// erased replaces the whole text with a constant gouge cluster.
pub fn weather_text(text: &str, cond: Condition) -> String {
    match cond {
        Condition::Intact => text.to_string(),
        Condition::Worn(mask) => {
            let mut gi = 0usize;
            text.chars()
                .map(|c| {
                    if c.is_whitespace() {
                        c
                    } else {
                        let i = gi;
                        gi += 1;
                        if i < 32 && mask & (1 << i) != 0 {
                            crate::text::MARK_LACUNA
                        } else {
                            c
                        }
                    }
                })
                .collect()
        }
        Condition::Erased => std::iter::repeat_n(crate::text::MARK_GOUGE, GOUGE_CELLS).collect(),
    }
}

/// G18: is this stored text an ⟦erased⟧ record (all gouge cells)? Used by the codex to render
/// the erasure event as `⟦——⟧` and by tests.
pub fn is_erased_text(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c == crate::text::MARK_GOUGE)
}

/// G18: which block (if any) a stored inscription text **names** — the inverse of
/// [`transliterate`] over the vocabulary, tolerant of lacuna marks (a worn name still reads as
/// its name from the surviving glyph positions). Drives the codex's live attestation rendering.
pub fn name_of_text(text: &str, script: Script) -> Option<crate::console::Block> {
    if is_erased_text(text) {
        return None;
    }
    crate::console::Block::ALL.iter().copied().find(|b| {
        block_script(*b) == script && {
            let full = transliterate(b.name(), script);
            full.chars().count() == text.chars().count()
                && full
                    .chars()
                    .zip(text.chars())
                    .all(|(f, t)| t == crate::text::MARK_LACUNA || t == f)
        }
    })
}

/// Per-script character pool to assemble abstract "words" from (Latin/Galactic/Runic all draw
/// from Latin letters — Galactic/Runic remap them to their own glyphs).
fn pool(script: Script) -> &'static str {
    match script {
        Script::Greek => "αβγδεζηθικλμνξοπρστυφχψω",
        Script::Hiragana => "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをん",
        _ => "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    }
}

/// G9: render a block's bare name into `script` as a **stable transliteration** — the same
/// glyphs every time for the same block, so a name reads as *a name* (recognisably recurring),
/// not fresh noise. Letter-wise: `a..z` maps positionally into the script's glyph pool (for the
/// Latin-glyph scripts — Latin/Galactic/Runic — that's the uppercase letters themselves; the
/// renderer draws them in the script's own glyph forms). Distinct across the vocabulary (tested).
pub fn transliterate(name: &str, script: Script) -> String {
    let glyphs: Vec<char> = pool(script).chars().collect();
    name.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| {
            let idx = (c.to_ascii_lowercase() as u8 - b'a') as usize;
            glyphs[idx % glyphs.len()]
        })
        .collect()
}

/// G9: the script a block's name is written in — its required stratum's script (starters →
/// Records/Latin).
pub fn block_script(b: crate::console::Block) -> Script {
    use crate::progress::{script_for, Stratum};
    script_for(b.required().unwrap_or(Stratum::Records))
}

/// G9: pick the block a name-bearing cell spells, **stratum-rarity-weighted** (common strata
/// often, the deep vocabulary rare — but everything findable; the coverage test guards it).
fn name_pick(h: u32) -> crate::console::Block {
    use crate::console::Block;
    use crate::progress::Stratum;
    // Weighted table: starters ×3 (common Records-tier chatter), Schematics ×2, deeper ×1.
    let mut table: Vec<Block> = Vec::new();
    for b in Block::ALL {
        let w = match b.required() {
            None => 3,
            Some(Stratum::Schematics) => 2,
            _ => 1,
        };
        for _ in 0..w {
            table.push(b);
        }
    }
    // Remix so the pick is independent of the name-gate bits (the gate fixes bits of `h`;
    // without the remix some table residues — whole blocks — were unreachable).
    let hh = h.wrapping_mul(0x9E37_79B9) >> 7;
    table[hh as usize % table.len()]
}

/// Compose a deterministic abstract inscription (1–2 short "words") from a cell hash: a script,
/// a few glyphs, a glowing tint, and a small world height ("a few words attached to a voxel").
fn compose(h: u32) -> (String, Script, f32, [f32; 3]) {
    let script = Script::ALL[(h % Script::ALL.len() as u32) as usize];
    let chars: Vec<char> = pool(script).chars().collect();
    let mut state = h | 1;
    let mut rng = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    let words = 1 + ((h >> 2) & 1); // 1 or 2 words
    let mut s = String::new();
    for wi in 0..words {
        if wi > 0 {
            s.push(' ');
        }
        let len = 2 + (rng() % 4) as usize; // 2..=5 glyphs
        for _ in 0..len {
            s.push(chars[rng() as usize % chars.len()]);
        }
    }
    // Dim, glowing tints (recoloured anyway by the palette); kept emissive so they bloom.
    const TINTS: [[f32; 3]; 4] = [
        [0.95, 0.55, 0.20], // amber
        [0.40, 0.85, 0.95], // cyan
        [0.55, 0.95, 0.45], // green
        [0.75, 0.50, 0.95], // violet
    ];
    let color = TINTS[((h >> 7) % 4) as usize];
    let height = 0.9 + ((h >> 9) % 4) as f32 * 0.15; // ~0.9–1.35 world units tall
    (s, script, height, color)
}

/// A monument inscription for a colossus (E17×E18): a glowing label at the giant's base, so the
/// fallen giants read as ancient *labelled* monuments. **G9: always name-bearing** — the giants
/// are *named after* the deep operations, so the rare vocabulary lives at the rare landmarks.
/// *(Brief said Relics/Signals-tier; the vocabulary currently has only one such block (`runfoot`),
/// so labels draw from the full gated set biased deep — runfoot half the time — for variety.
/// Recorded deviation; revisit when G10 grows the deep vocabulary.)*
pub fn colossus_label(p: &Placement) -> Inscription {
    use crate::console::Block;
    let h = p.seed ^ 0x00C0_1055;
    let (_text, _script, _h, color) = compose(h);
    const DEEP: [Block; 6] = [
        Block::RunFoot, // ×3: the deep Relics-tier name dominates the monuments
        Block::RunFoot,
        Block::RunFoot,
        Block::Seek,
        Block::Circle,
        Block::Goto,
    ];
    let block = DEEP[(h >> 9) as usize % DEEP.len()];
    let script = block_script(block);
    Inscription {
        cell: (
            (p.pos.x / CELL).floor() as i32,
            (p.pos.z / CELL).floor() as i32,
        ),
        // Float a few blocks above the giant's feet so it reads as a plaque at the base.
        pos: p.pos + Vec3::new(0.0, 3.0, 0.0),
        text: transliterate(block.name(), script),
        script,
        height: 1.8,
        color,
        name: Some(block),
        // G18: monuments stay intact — the deep names are cut monumentally, and the
        // every-name-findable coverage guarantee (no discovery softlock) keeps holding.
        condition: Condition::Intact,
    }
}

/// All inscription markers within `radius` of `cam`, deterministic in `seed`; `ground(x, z)`
/// floats each just above the surface. Mirrors [`colossi_near`]'s coarse-grid scheme.
pub fn inscriptions_near(
    seed: u32,
    cam: Vec3,
    radius: f32,
    ground: impl Fn(f32, f32) -> f32,
) -> Vec<Inscription> {
    let reach = (radius / ICELL).ceil() as i32 + 1;
    // BUG1 defense-in-depth: saturating bounds (see `colossi_near`).
    let (ccx, ccz) = (
        (cam.x / ICELL).floor() as i32,
        (cam.z / ICELL).floor() as i32,
    );
    let mut out = Vec::new();
    for cz in ccz.saturating_sub(reach)..=ccz.saturating_add(reach) {
        for cx in ccx.saturating_sub(reach)..=ccx.saturating_add(reach) {
            let h = hash(cx ^ 0x1111, cz ^ 0x2222, seed.wrapping_add(0x7E47_0000));
            let jx = ((h >> 16) & 0xFF) as f32 / 255.0;
            let jz = ((h >> 24) & 0xFF) as f32 / 255.0;
            let x = cx as f32 * ICELL + jx * ICELL;
            let z = cz as f32 * ICELL + jz * ICELL;
            // Biome scales inscription density too (E10).
            let pres = (IPRESENCE * crate::biome::density(x, z, seed).3).min(0.95);
            if (h & 0xFFFF) as f32 / 65536.0 >= pres {
                continue;
            }
            if (x - cam.x).powi(2) + (z - cam.z).powi(2) > radius * radius {
                continue;
            }
            let (text, script, height, color) = compose(h);
            // G9: ~1 in 4 cells spell a **block's name** (stratum-script, stable transliteration)
            // instead of ambient noise — the world's text becomes load-bearing. The ambient
            // majority is unchanged (same compose), keeping the melancholy noise.
            let name = (h >> 5).is_multiple_of(4).then(|| name_pick(h));
            let (text, script) = match name {
                Some(b) => {
                    let s = block_script(b);
                    (transliterate(b.name(), s), s)
                }
                None => (text, script),
            };
            // G18: seeded condition off a **fresh-salt hash** — independent of the name-gate bits
            // of `h` (the G10 correlation lesson; the distribution test guards it). Worn keeps its
            // name (recoverable from partial glyphs, Decision 3); erased loses everything.
            let ch = hash(cx ^ 0x51AB, cz ^ 0x2E77, seed.wrapping_add(0x00C0_5D17));
            let glyphs = text.chars().filter(|c| !c.is_whitespace()).count();
            let condition = condition_pick(ch, glyphs);
            let text = weather_text(&text, condition);
            let name = if condition == Condition::Erased {
                None
            } else {
                name
            };
            out.push(Inscription {
                cell: (cx, cz),
                // Float just above the surface — a small label tethered to the ground voxel.
                pos: Vec3::new(x, ground(x, z) + 1.3 + height * 0.5, z),
                text,
                script,
                height,
                color,
                name,
                condition,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_within_radius() {
        let g = |_x: f32, _z: f32| 0.0;
        let cam = Vec3::new(123.0, 0.0, -77.0);
        let a = colossi_near(1337, cam, 500.0, g);
        let b = colossi_near(1337, cam, 500.0, g);
        assert_eq!(a.len(), b.len());
        assert!(!a.is_empty(), "some colossi should be in a 500-unit radius");
        for (p, q) in a.iter().zip(&b) {
            assert_eq!(cell_key(p), cell_key(q));
            let d2 = (p.pos.x - cam.x).powi(2) + (p.pos.z - cam.z).powi(2);
            assert!(d2 <= 500.0 * 500.0 + 1.0, "placement outside radius");
        }
    }

    #[test]
    fn different_seeds_place_differently() {
        let g = |_x: f32, _z: f32| 0.0;
        let cam = Vec3::ZERO;
        let a = colossi_near(1, cam, 600.0, g);
        let b = colossi_near(2, cam, 600.0, g);
        let keys_a: Vec<_> = a.iter().map(cell_key).collect();
        let keys_b: Vec<_> = b.iter().map(cell_key).collect();
        assert_ne!(keys_a, keys_b);
    }

    #[test]
    fn colossus_label_floats_above_feet_and_is_deterministic() {
        let p = Placement {
            pos: Vec3::new(40.0, 18.0, -12.0),
            yaw: 0.7,
            voxel: 1.3,
            seed: 12345,
            solid: false,
            human: false,
        };
        let a = colossus_label(&p);
        let b = colossus_label(&p);
        assert_eq!(a.text, b.text); // same colossus → same inscription
        assert!(!a.text.is_empty());
        assert!(
            a.pos.y > p.pos.y,
            "label should float above the giant's feet"
        );
    }

    #[test]
    fn transliteration_stable_and_distinct_across_vocabulary() {
        use crate::console::Block;
        // Deterministic: same block+script → the same glyphs every time (a recurring *name*).
        for b in Block::ALL {
            let s = block_script(b);
            assert_eq!(transliterate(b.name(), s), transliterate(b.name(), s));
            assert!(!transliterate(b.name(), s).is_empty());
        }
        // Distinct: no two DIFFERENT names collide after transliteration — each reads as ITS
        // name. (Parameterised families — scan items, spend faculties — intentionally share one
        // family name, so distinctness is over unique names.)
        let mut names: Vec<&str> = Block::ALL.iter().map(|b| b.name()).collect();
        names.sort_unstable();
        names.dedup();
        let all: Vec<String> = names
            .iter()
            .map(|n| {
                let b = Block::ALL.iter().find(|b| b.name() == *n).unwrap();
                transliterate(n, block_script(*b))
            })
            .collect();
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(
                    all[i], all[j],
                    "name collision: {:?} vs {:?}",
                    all[i], all[j]
                );
            }
        }
    }

    #[test]
    fn name_bearers_are_a_minority_and_match_their_block() {
        // ~1 in 4 inscriptions name-bearing; a name-bearer's text is its block's transliteration
        // in the block's stratum script; ambient majority unchanged in spirit (no name).
        let g = |_x: f32, _z: f32| 0.0;
        let marks = inscriptions_near(1337, Vec3::ZERO, 1500.0, g);
        assert!(!marks.is_empty());
        let named = marks.iter().filter(|m| m.name.is_some()).count();
        let frac = named as f32 / marks.len() as f32;
        assert!(
            (0.10..=0.45).contains(&frac),
            "name fraction should be ~1/4, got {frac} ({named}/{})",
            marks.len()
        );
        for m in &marks {
            if let Some(b) = m.name {
                assert_eq!(
                    m.script,
                    block_script(b),
                    "name renders in its stratum script"
                );
                // G18: an intact bearer spells the exact transliteration; a worn one keeps it
                // recoverable from the surviving glyph positions (lacunae are wildcards).
                match m.condition {
                    Condition::Intact => assert_eq!(
                        m.text,
                        transliterate(b.name(), m.script),
                        "stable name text"
                    ),
                    // (Compared by *name*: parameterised families share one name, so the
                    // inverse read resolves to the family's first member.)
                    Condition::Worn(_) => assert_eq!(
                        name_of_text(&m.text, m.script).map(|x| x.name()),
                        Some(b.name()),
                        "a worn name still reads as its block: {:?}",
                        m.text
                    ),
                    Condition::Erased => panic!("an erased inscription can't carry a name"),
                }
            }
        }
    }

    #[test]
    fn every_block_name_is_findable_nearby() {
        // Coverage guarantee (no discovery softlock): the full vocabulary's names occur within a
        // bounded radius of the start, across seeds. 2500 units ≈ a modest autopilot wander.
        use crate::console::Block;
        let g = |_x: f32, _z: f32| 0.0;
        for seed in [1u32, 42, 1337] {
            let mut found: std::collections::HashSet<u8> = std::collections::HashSet::new();
            for m in inscriptions_near(seed, Vec3::ZERO, 2500.0, g) {
                if let Some(b) = m.name {
                    found.insert(b.code());
                }
            }
            // Colossus monuments also surface deep names.
            for p in colossi_near(seed, Vec3::ZERO, 2500.0, g) {
                if let Some(b) = colossus_label(&p).name {
                    found.insert(b.code());
                }
            }
            for b in Block::ALL {
                assert!(
                    found.contains(&b.code()),
                    "seed {seed}: block '{}' (code {}) has no findable name within 2500 units",
                    b.name(),
                    b.code()
                );
            }
        }
    }

    #[test]
    fn name_pick_distribution_matches_weights() {
        // Babysitter suggestion (G10 review): guard the correlation bug-class directly — the
        // picked-block distribution over many hashes must track the rarity weights (a stuck
        // residue/correlation would skew it loudly), not just 3-seed reachability.
        use crate::console::Block;
        use crate::progress::Stratum;
        let n = 40_000u32;
        let mut counts: std::collections::HashMap<u8, u32> = std::collections::HashMap::new();
        for i in 0..n {
            // Drive with the same gate the live path applies: only gate-passing hashes pick.
            let h = hash(i as i32, -(i as i32) * 7 + 3, 0xABCD);
            if (h >> 5).is_multiple_of(4) {
                *counts.entry(name_pick(h).code()).or_default() += 1;
            }
        }
        let total: u32 = counts.values().sum();
        assert!(total > 5_000, "need a large gated sample, got {total}");
        // Expected share per code = its table weight / total weight (families share a code).
        let weight = |b: Block| match b.required() {
            None => 3u32,
            Some(Stratum::Schematics) => 2,
            _ => 1,
        };
        let total_w: u32 = Block::ALL.iter().map(|b| weight(*b)).sum();
        let mut code_w: std::collections::HashMap<u8, u32> = std::collections::HashMap::new();
        for b in Block::ALL {
            *code_w.entry(b.code()).or_default() += weight(b);
        }
        for (code, w) in code_w {
            let expect = w as f32 / total_w as f32;
            let got = counts.get(&code).copied().unwrap_or(0) as f32 / total as f32;
            assert!(
                (got - expect).abs() < expect * 0.35 + 0.005,
                "code {code}: distribution skew — got {got:.3}, expected ≈{expect:.3}"
            );
        }
    }

    #[test]
    fn colossus_labels_name_deep_blocks() {
        use crate::console::Block;
        let g = |_x: f32, _z: f32| 12.0;
        let placements = colossi_near(7, Vec3::ZERO, 1200.0, g);
        assert!(!placements.is_empty());
        let mut saw_runfoot = false;
        for p in &placements {
            let l = colossus_label(p);
            let b = l.name.expect("every monument label is name-bearing (G9)");
            assert!(
                b.required().is_some(),
                "monuments name the gated vocabulary"
            );
            assert_eq!(l.text, transliterate(b.name(), l.script));
            saw_runfoot |= b == Block::RunFoot;
            // Deterministic per colossus.
            assert_eq!(colossus_label(p).name, l.name);
        }
        assert!(
            saw_runfoot,
            "the deep Relics-tier name should dominate the monuments"
        );
    }

    #[test]
    fn inscriptions_deterministic_and_nonempty() {
        let g = |_x: f32, _z: f32| 12.0;
        let cam = Vec3::new(60.0, 0.0, -40.0);
        let a = inscriptions_near(2024, cam, 300.0, g);
        let b = inscriptions_near(2024, cam, 300.0, g);
        assert!(
            !a.is_empty(),
            "some inscriptions should be in a 300-unit radius"
        );
        assert_eq!(a.len(), b.len());
        for (m, n) in a.iter().zip(&b) {
            assert_eq!(m.cell, n.cell);
            assert_eq!(m.text, n.text); // same seed → same words/script
            assert!(!m.text.is_empty());
            // Floated above the supplied ground height.
            assert!(m.pos.y > 12.0);
        }
    }

    /// G18: the condition mix is deterministic, lands near the pinned ~70/27/3 ratios, and its
    /// bits are **independent of the name-gate bits** (the G9/G10 correlation bug-class, as a
    /// test): among gate-passing cells the condition distribution must match the overall one.
    #[test]
    fn condition_distribution_and_independence_from_the_name_gate() {
        let n = 40_000i32;
        let (mut all, mut named) = ([0u32; 3], [0u32; 3]);
        let slot = |c: Condition| match c {
            Condition::Intact => 0usize,
            Condition::Worn(_) => 1,
            Condition::Erased => 2,
        };
        for i in 0..n {
            let (cx, cz) = (i, -i * 7 + 3);
            let seed = 0xABCDu32;
            // The same two hashes the live path derives (name gate off `h`, condition off `ch`).
            let h = hash(cx ^ 0x1111, cz ^ 0x2222, seed.wrapping_add(0x7E47_0000));
            let ch = hash(cx ^ 0x51AB, cz ^ 0x2E77, seed.wrapping_add(0x00C0_5D17));
            let c = condition_pick(ch, 6);
            assert_eq!(c, condition_pick(ch, 6), "deterministic");
            all[slot(c)] += 1;
            if (h >> 5).is_multiple_of(4) {
                named[slot(c)] += 1;
            }
        }
        let named_total: u32 = named.iter().sum();
        assert!(named_total > 5_000, "need a large gated sample");
        for (i, expect) in [(0usize, 0.70f32), (1, 0.27), (2, 0.03)] {
            let overall = all[i] as f32 / n as f32;
            let gated = named[i] as f32 / named_total as f32;
            assert!(
                (overall - expect).abs() < 0.02,
                "condition {i}: overall {overall:.3} vs pinned {expect:.2}"
            );
            assert!(
                (gated - overall).abs() < expect * 0.15 + 0.01,
                "condition {i}: name-gated {gated:.3} diverges from overall {overall:.3} — \
                 the bits are correlated (the G10 bug-class)"
            );
        }
    }

    /// G18: worn weathering keeps length/spaces and is a strict, non-empty lacuna subset;
    /// erasure hides everything behind a constant gouge cluster.
    #[test]
    fn weathering_masks_glyphs_and_erasure_hides_all() {
        // Worn: every mask this generator produces loses ≥1 glyph and keeps ≥1 survivor.
        for ch in (70..97).chain([170u32, 1234, 99_170]) {
            let text = "ΑΒΓ ΔΕΖ";
            if let Condition::Worn(mask) = condition_pick(ch * 100 + 85, 6) {
                let worn = weather_text(text, Condition::Worn(mask));
                assert_eq!(worn.chars().count(), text.chars().count(), "length kept");
                assert_eq!(
                    worn.chars().position(|c| c == ' '),
                    text.chars().position(|c| c == ' '),
                    "spaces survive"
                );
                let lost = worn
                    .chars()
                    .filter(|c| *c == crate::text::MARK_LACUNA)
                    .count();
                assert!(lost >= 1, "a worn text lost at least one glyph");
                assert!(lost < 6, "a worn text keeps at least one survivor");
            }
        }
        // A 1-glyph text can't be part-worn — it stays intact rather than fully lost.
        assert_eq!(condition_pick(85, 1), Condition::Intact);
        // Erased: constant-width gouge cells, recognised by the codex helper.
        let erased = weather_text("ΑΒΓΔ", Condition::Erased);
        assert!(erased.chars().all(|c| c == crate::text::MARK_GOUGE));
        assert!(is_erased_text(&erased));
        assert!(!is_erased_text("ΑΒΓ"));
        assert!(!is_erased_text(""));
        // Intact is the identity.
        assert_eq!(weather_text("AB C", Condition::Intact), "AB C");
    }

    /// G18: `name_of_text` inverts `transliterate` for every block, tolerates lacunae (a worn
    /// name still reads), and declines erased/ambient text.
    #[test]
    fn name_of_text_reads_intact_and_worn_names() {
        use crate::console::Block;
        for b in Block::ALL {
            let script = block_script(b);
            let full = transliterate(b.name(), script);
            let read = name_of_text(&full, script).expect("intact name reads");
            assert_eq!(read.name(), b.name(), "reads as its (family) name");
            // Wear the first glyph away: still reads (recoverable from partial glyphs, v1).
            let worn = weather_text(&full, Condition::Worn(1));
            assert_eq!(
                name_of_text(&worn, script).map(|x| x.name()),
                Some(b.name())
            );
        }
        // Erased text and ambient noise are not names.
        assert_eq!(
            name_of_text(&weather_text("ABC", Condition::Erased), Script::Latin),
            None
        );
        assert_eq!(name_of_text("QQQQQQQQQ", Script::Latin), None);
    }

    /// G18: an ⟦erased⟧ cell strips its name (content unrecoverable — no discovery from a
    /// gouge), while worn name-bearers keep theirs; erased billboards render the gouge cluster.
    #[test]
    fn erased_inscriptions_carry_no_name_and_render_the_gouge() {
        let g = |_x: f32, _z: f32| 0.0;
        let marks = inscriptions_near(1337, Vec3::ZERO, 2500.0, g);
        let erased: Vec<_> = marks
            .iter()
            .filter(|m| m.condition == Condition::Erased)
            .collect();
        assert!(
            !erased.is_empty(),
            "~3% of inscriptions in a 2500-unit radius should be erased"
        );
        for m in &erased {
            assert!(m.name.is_none(), "an erasure discovers nothing");
            assert!(is_erased_text(&m.text), "the billboard shows the gouge");
        }
        // Worn inscriptions exist and render lacuna marks in-world.
        let worn: Vec<_> = marks
            .iter()
            .filter(|m| matches!(m.condition, Condition::Worn(_)))
            .collect();
        assert!(!worn.is_empty(), "~27% should be worn");
        for m in &worn {
            assert!(
                m.text.chars().any(|c| c == crate::text::MARK_LACUNA),
                "a worn billboard renders its lacunae"
            );
        }
    }

    /// BUG1 regression: huge cam coords must not overflow the colossi/inscription cell loops.
    #[test]
    fn extreme_cam_coords_dont_overflow_structures() {
        let g = |_x: f32, _z: f32| 0.0;
        for v in [3.0e11_f32, -3.0e11, f32::MAX, f32::MIN] {
            let cam = Vec3::new(v, 40.0, v);
            let _ = colossi_near(1337, cam, 420.0, g);
            let _ = inscriptions_near(1337, cam, 90.0, g);
        }
    }
}
