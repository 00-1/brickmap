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
    let (ccx, ccz) = ((cam.x / CELL).floor() as i32, (cam.z / CELL).floor() as i32);
    let mut out = Vec::new();
    for cz in (ccz - reach)..=(ccz + reach) {
        for cx in (ccx - reach)..=(ccx + reach) {
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
    /// `None` = ambient glyph-noise (the melancholy majority).
    pub name: Option<crate::console::Block>,
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
    let (ccx, ccz) = (
        (cam.x / ICELL).floor() as i32,
        (cam.z / ICELL).floor() as i32,
    );
    let mut out = Vec::new();
    for cz in (ccz - reach)..=(ccz + reach) {
        for cx in (ccx - reach)..=(ccx + reach) {
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
            out.push(Inscription {
                cell: (cx, cz),
                // Float just above the surface — a small label tethered to the ground voxel.
                pos: Vec3::new(x, ground(x, z) + 1.3 + height * 0.5, z),
                text,
                script,
                height,
                color,
                name,
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
                assert_eq!(
                    m.text,
                    transliterate(b.name(), m.script),
                    "stable name text"
                );
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
}
