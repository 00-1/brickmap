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
    /// G20: this ambient inscription is an instance of the world's recurring **formulaic
    /// frame** — `Some(frame_id)`. Its text spells the frame verbatim (in this cell's script),
    /// so distribution-watchers can spot the recurrence in raw glyphs. `None` for names,
    /// plain ambient noise, and ⟦erased⟧ cells (erased recovery is G21).
    pub frame: Option<u64>,
    /// G21: for a **worn** inscription, its full pre-weathered composition (same enclosure as
    /// `text`, so positions align char-for-char) — what rung 1's close reading recovers on foot.
    /// `None` for intact (nothing lost) and ⟦erased⟧ (that recovery is rung 2's, from
    /// [`hidden_text`], not from the surface).
    pub pristine: Option<String>,
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
/// [`name_text`] over the vocabulary, tolerant of lacuna marks (a worn name still reads as
/// its name from the surviving glyph positions). Drives the codex's live attestation rendering.
/// G20: seeded — names are per-world lexicon words now.
pub fn name_of_text(seed: u32, text: &str, script: Script) -> Option<crate::console::Block> {
    if is_erased_text(text) {
        return None;
    }
    let text = strip_reveal(text); // G21: a revealed erasure's marker is structure
    let text = strip_cartouche(text); // G20: the enclosure is structure, not content
                                      // G21: the Leiden restoration brackets are structure too — a recovered name (close
                                      // reading / a frame restore) still reads as its name.
    let text: String = text.chars().filter(|c| *c != '[' && *c != ']').collect();
    crate::console::Block::ALL.iter().copied().find(|b| {
        block_script(*b) == script && {
            let full = name_text(seed, *b);
            full.chars().count() == text.chars().count()
                && full
                    .chars()
                    .zip(text.chars())
                    .all(|(f, t)| t == crate::text::MARK_LACUNA || t == f)
        }
    })
}

/// G20: a block's player-facing **true name** — its seeded lexicon word (never the internal
/// English `name()`; that stays for codes/codecs/tests only). Per-world (Decision 3: each dead
/// world had its own tongue), deterministic, distinct across the vocabulary (lexicon-tested).
pub fn display_name(seed: u32, b: crate::console::Block) -> String {
    crate::lexicon::block_name(seed, b)
}

/// G20: enclose a glyph run in the **cartouche** mark pair — the visual name-enclosure (the
/// decipherment-research foothold: you learn to *spot* names before you can read anything).
/// The same marks wrap world name-inscriptions and console/codex name renders.
pub fn cartouche(inner: &str) -> String {
    format!(
        "{}{inner}{}",
        crate::text::MARK_CARTOUCHE_OPEN,
        crate::text::MARK_CARTOUCHE_CLOSE
    )
}

/// G20: the text inside a cartouche (or the text unchanged if it isn't cartouched) — the
/// content-reading inverse of [`cartouche`], used by [`name_of_text`].
pub fn strip_cartouche(text: &str) -> &str {
    text.strip_prefix(crate::text::MARK_CARTOUCHE_OPEN)
        .and_then(|t| t.strip_suffix(crate::text::MARK_CARTOUCHE_CLOSE))
        .unwrap_or(text)
}

// ---- G20: formulaic frames as cribs ---------------------------------------------------------

/// A frame's identity + skeleton, for matching worn instances (`None` = the varying slot).
/// The generic matcher takes a *list* of these (only ever the frames the player KNOWS), so the
/// unique-match rule below is real machinery, not a vacuous check.
pub struct FrameSkeleton {
    pub id: u64,
    pub words: Vec<Option<String>>,
}

/// The world's recurring frames (one today — G16's single emitter; the machinery is list-shaped
/// for when the corpus grows more).
pub fn world_frames(seed: u32) -> Vec<FrameSkeleton> {
    vec![FrameSkeleton {
        id: crate::lexicon::frame_id(seed),
        words: crate::lexicon::frame_skeleton(seed),
    }]
}

/// Do the surviving glyphs of `words` (a worn text split on spaces) fit frame `f` in `script`?
/// Fixed words must match length + every surviving glyph; the slot word is free content.
fn frame_matches(words: &[&str], f: &FrameSkeleton, script: Script) -> bool {
    words.len() == f.words.len()
        && words.iter().zip(&f.words).all(|(w, fw)| match fw {
            None => !w.is_empty(),
            Some(fixed) => {
                let expect = transliterate(fixed, script);
                w.chars().count() == expect.chars().count()
                    && w.chars()
                        .zip(expect.chars())
                        .all(|(c, e)| c == crate::text::MARK_LACUNA || c == e)
            }
        })
}

/// G20: try to **restore** a worn inscription's lost glyphs against the player's *known* frames
/// (Leiden `[abc]` — restoration, visually distinct from `[..]` lacunae). Returns the restored
/// text iff:
/// - the surviving glyphs match **exactly one** known frame (ambiguous → no false restorations,
///   Decision 4), with ≥1 surviving glyph in a *fixed* position actually pinning it (word-shape
///   alone doesn't restore), and
/// - **every** lacuna falls in a fixed (skeleton) position — the varying slot is not formulaic,
///   so a slot lacuna is unrecoverable and the whole text stays lacunae.
///
/// Restored runs are wrapped in ASCII `[` `]` (the Leiden restoration brackets — structural
/// punctuation, excluded from the data glyph count like every mark).
pub fn restore_worn(text: &str, script: Script, known: &[FrameSkeleton]) -> Option<String> {
    if !text.chars().any(|c| c == crate::text::MARK_LACUNA) {
        return None; // nothing lost, nothing to restore
    }
    let words: Vec<&str> = text.split(' ').collect();
    let mut fits = known.iter().filter(|f| frame_matches(&words, f, script));
    let f = fits.next()?;
    if fits.next().is_some() {
        return None; // ambiguous across known frames — no false restorations
    }
    // The match must be *pinned* by at least one surviving fixed-position glyph.
    let pinned = words
        .iter()
        .zip(&f.words)
        .any(|(w, fw)| fw.is_some() && w.chars().any(|c| c != crate::text::MARK_LACUNA));
    if !pinned {
        return None;
    }
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for (w, fw) in words.iter().zip(&f.words) {
        match fw {
            None => {
                if w.chars().any(|c| c == crate::text::MARK_LACUNA) {
                    return None; // the varying slot can't be restored from a formula
                }
                out.push((*w).to_string());
            }
            Some(fixed) => {
                let expect = transliterate(fixed, script);
                let mut restored = String::new();
                let mut open = false;
                for (c, e) in w.chars().zip(expect.chars()) {
                    if c == crate::text::MARK_LACUNA {
                        if !open {
                            restored.push('[');
                            open = true;
                        }
                        restored.push(e);
                    } else {
                        if open {
                            restored.push(']');
                            open = false;
                        }
                        restored.push(c);
                    }
                }
                if open {
                    restored.push(']');
                }
                out.push(restored);
            }
        }
    }
    Some(out.join(" "))
}

/// G21: **close reading** (rung 1) — recover a worn text's lost glyphs from its `pristine`
/// (pre-weathered) composition, wrapping each recovered run in the Leiden restoration brackets
/// `[` `]` (the same structural marks G20's frame restore uses — visually distinct from `[..]`
/// lacunae, excluded from the data glyph count, so the recovered text pays **full** yield).
/// Positions align char-for-char ([`weather_text`] preserves length; the caller applies the same
/// enclosure to both). `None` when nothing was lost or the texts can't align.
pub fn recover_worn(weathered: &str, pristine: &str) -> Option<String> {
    if weathered.chars().count() != pristine.chars().count() {
        return None;
    }
    if !weathered.chars().any(|c| c == crate::text::MARK_LACUNA) {
        return None; // nothing lost, nothing to recover
    }
    let mut out = String::new();
    let mut open = false;
    for (c, p) in weathered.chars().zip(pristine.chars()) {
        if c == crate::text::MARK_LACUNA {
            if !open {
                out.push('[');
                open = true;
            }
            out.push(p);
        } else {
            if open {
                out.push(']');
                open = false;
            }
            out.push(c);
        }
    }
    if open {
        out.push(']');
    }
    Some(out)
}

/// G21 rung 2: the **hidden content** of an ⟦erased⟧ inscription — composed deterministically
/// from the cell (it always existed; the gouge merely hid it), weighted **deep**: erasure was
/// deliberate, so what was struck out *mattered*. ~1/4 of gouges hide a **name-bearer** drawn
/// from a deep-weighted table over the gated vocabulary (RunFoot ×3 — the Relics-tier deep
/// operation — plus the Schematics nav words), cartouched like every name inscription
/// (collecting it discovers the block); the rest hide **Relics/Signals data** (Runic ~2 : ~1
/// Galactic script — the deep strata's currencies). Fresh-salt hash, independent of the
/// surface-condition and name-gate bits (the standing correlation discipline).
pub fn hidden_text(seed: u32, cell: (i32, i32)) -> (String, Script, Option<crate::console::Block>) {
    use crate::console::Block;
    let h = hash(
        cell.0 ^ 0x4831,
        cell.1 ^ 0x77DD,
        seed.wrapping_add(0x4849_4444), // "HIDD"
    );
    if (h >> 4).is_multiple_of(4) {
        // A deep name-bearer: the erasure struck out a name (the censored vocabulary).
        const TABLE: [Block; 6] = [
            Block::RunFoot,
            Block::RunFoot,
            Block::RunFoot,
            Block::Seek,
            Block::Circle,
            Block::Goto,
        ];
        let b = TABLE[(h >> 7) as usize % TABLE.len()];
        return (cartouche(&name_text(seed, b)), block_script(b), Some(b));
    }
    // Deep-strata data: a few words of Runic/Galactic glyphs (Relics/Signals yield).
    let script = if (h >> 6).is_multiple_of(3) {
        Script::Galactic
    } else {
        Script::Runic
    };
    let chars: Vec<char> = pool(script).chars().collect();
    let mut state = h | 1;
    let mut rng = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    let words = 1 + ((h >> 2) & 1);
    let mut s = String::new();
    for wi in 0..words {
        if wi > 0 {
            s.push(' ');
        }
        let len = 3 + (rng() % 4) as usize; // 3..=6 glyphs
        for _ in 0..len {
            s.push(chars[rng() as usize % chars.len()]);
        }
    }
    (s, script, None)
}

/// G21: the **revealed** form an erased site's hidden content is banked/logged as — the hidden
/// text behind one leading gouge mark, so the codex still reads "here stood an erasure" (the
/// resolved-gouge state) while the content pays normally (marks never pay — `glyph_count`).
pub fn revealed_text(hidden: &str) -> String {
    format!("{}{hidden}", crate::text::MARK_GOUGE)
}

/// G21: is this stored text a **revealed erasure** (the resolved-gouge codex state)? Distinct
/// from [`is_erased_text`] (all gouge, unresolved) — a reveal keeps exactly one leading gouge
/// mark in front of recovered glyphs.
pub fn is_revealed_text(text: &str) -> bool {
    text.starts_with(crate::text::MARK_GOUGE) && text.chars().any(|c| c != crate::text::MARK_GOUGE)
}

/// G21: the recovered content of a [`revealed_text`] (the text unchanged if it isn't one).
pub fn strip_reveal(text: &str) -> &str {
    text.strip_prefix(crate::text::MARK_GOUGE).unwrap_or(text)
}

/// G20: the exact glyph text a name-bearing inscription spells for block `b`: its true name
/// transliterated into its stratum script. The console renders the overlay form of this same
/// string — the world↔console recognition loop's single source.
pub fn name_text(seed: u32, b: crate::console::Block) -> String {
    transliterate(&display_name(seed, b), block_script(b))
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

/// G9: render romanized lexicon text into `script` as a **stable transliteration** — the same
/// glyphs every time for the same word, so a name reads as *a name* (recognisably recurring),
/// not fresh noise. Letter-wise: `a..z` maps positionally into the script's glyph pool (for the
/// Latin-glyph scripts — Latin/Galactic/Runic — that's the uppercase letters themselves; the
/// renderer draws them in the script's own glyph forms). Distinct across the vocabulary (tested).
/// G20: spaces survive (a multi-word *frame* keeps its word shape); everything else drops.
pub fn transliterate(name: &str, script: Script) -> String {
    let glyphs: Vec<char> = pool(script).chars().collect();
    name.chars()
        .filter(|c| c.is_ascii_alphabetic() || *c == ' ')
        .map(|c| {
            if c == ' ' {
                return ' ';
            }
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
/// *(G19: the table is **flat** over the gated vocabulary — the old RunFoot-×3 weighting made
/// the pacing probe's worst seeds Relics-first onboarding roulette (25–26 min to a first
/// comprehension) and stretched the per-block discovery tail to 63 min. Revisit the deep bias
/// when the Archive milestones grow real Relics/Signals-tier vocabulary.)*
pub fn colossus_label(seed: u32, p: &Placement) -> Inscription {
    use crate::console::Block;
    let h = p.seed ^ 0x00C0_1055;
    let (_text, _script, _h, color) = compose(h);
    const DEEP: [Block; 4] = [Block::Seek, Block::Circle, Block::Goto, Block::RunFoot];
    let block = DEEP[(h >> 9) as usize % DEEP.len()];
    let script = block_script(block);
    Inscription {
        cell: (
            (p.pos.x / CELL).floor() as i32,
            (p.pos.z / CELL).floor() as i32,
        ),
        // Float a few blocks above the giant's feet so it reads as a plaque at the base.
        pos: p.pos + Vec3::new(0.0, 3.0, 0.0),
        // G20: name-bearing text is always **cartouched** (the enclosure survives any wear).
        text: cartouche(&name_text(seed, block)),
        script,
        height: 1.8,
        color,
        name: Some(block),
        // G18: monuments stay intact — the deep names are cut monumentally, and the
        // every-name-findable coverage guarantee (no discovery softlock) keeps holding.
        condition: Condition::Intact,
        frame: None,
        pristine: None,
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
            // G9: ~1 in 6 cells spell a **block's name** (stratum-script, stable transliteration)
            // instead of ambient noise — the world's text becomes load-bearing. The ambient
            // majority is unchanged (same compose), keeping the melancholy noise. (G19: 1/4 → 1/6
            // — measured first discovery was 0.9 min against a 2–6 min envelope; names should be
            // finds, not wallpaper. The coverage test still guarantees every name findable.)
            let name = (h >> 5).is_multiple_of(6).then(|| name_pick(h));
            let (text, script) = match name {
                // G20: a name-bearer spells the block's seeded **true name** (lexicon, never
                // English) in its stratum script.
                Some(b) => (name_text(seed, b), block_script(b)),
                None => (text, script),
            };
            // G20: an ambient long cell that routes to the corpus' recurring **frame** shape
            // spells the frame *verbatim* in this cell's script — the recurrence is visible in
            // raw glyphs (the Grotefend foothold), not only after comprehension. The routing
            // keys off the *original* composed glyph count, the same input the translated
            // display feeds `lexicon::phrase`, so both layers agree on which cells are frames.
            let frame = (name.is_none() && {
                let g = text.chars().filter(|c| !c.is_whitespace()).count() as u32;
                crate::lexicon::is_frame_cell(seed, (cx, cz), g)
            })
            .then(|| crate::lexicon::frame_id(seed));
            let text = if frame.is_some() {
                transliterate(&crate::lexicon::frame(seed, (cx, cz)), script)
            } else {
                text
            };
            // G18: seeded condition off a **fresh-salt hash** — independent of the name-gate bits
            // of `h` (the G10 correlation lesson; the distribution test guards it). Worn keeps its
            // name (recoverable from partial glyphs, Decision 3); erased loses everything.
            let ch = hash(cx ^ 0x51AB, cz ^ 0x2E77, seed.wrapping_add(0x00C0_5D17));
            let glyphs = text.chars().filter(|c| !c.is_whitespace()).count();
            let condition = condition_pick(ch, glyphs);
            let full = text.clone(); // G21: the pre-weathered composition (close reading's source)
            let text = weather_text(&text, condition);
            // An erasure hides everything — its name AND its frame membership (G21's ladder).
            let (name, frame) = if condition == Condition::Erased {
                (None, None)
            } else {
                (name, frame)
            };
            // G20: a name-bearer's text is **cartouched** (after weathering, so the enclosure
            // survives wear — the frame outlives its glyphs). Ambient text never; an erasure
            // dropped its name above, so the gouge stays a bare gouge (G21's sensing tease).
            // G21: a worn cell keeps its `pristine` composition under the same enclosure, so
            // close reading's char-for-char recovery aligns.
            let (text, pristine) = {
                let worn = matches!(condition, Condition::Worn(_));
                if name.is_some() {
                    (cartouche(&text), worn.then(|| cartouche(&full)))
                } else {
                    (text, worn.then_some(full))
                }
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
                frame,
                pristine,
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
        let a = colossus_label(1337, &p);
        let b = colossus_label(1337, &p);
        assert_eq!(a.text, b.text); // same colossus → same inscription
        assert!(!a.text.is_empty());
        assert!(
            a.pos.y > p.pos.y,
            "label should float above the giant's feet"
        );
    }

    #[test]
    fn lexicon_names_stable_and_distinct_across_vocabulary() {
        use crate::console::Block;
        for seed in [0u32, 1, 42, 1337, 0xDEAD_BEEF] {
            // Deterministic: same block+seed → the same glyphs every time (a recurring *name*).
            for b in Block::ALL {
                assert_eq!(name_text(seed, b), name_text(seed, b));
                assert!(!name_text(seed, b).is_empty());
                // G20: the displayed name is a lexicon word, never the internal English name.
                assert_ne!(
                    display_name(seed, b),
                    b.name(),
                    "seed {seed}: display name must not spell the English key"
                );
            }
            // Distinct: no two DIFFERENT names collide after transliteration — each reads as ITS
            // name. (Parameterised families — scan items, spend faculties — intentionally share
            // one family name, so distinctness is over unique names.)
            let mut names: Vec<&str> = Block::ALL.iter().map(|b| b.name()).collect();
            names.sort_unstable();
            names.dedup();
            let all: Vec<String> = names
                .iter()
                .map(|n| {
                    let b = Block::ALL.iter().find(|b| b.name() == *n).unwrap();
                    name_text(seed, *b)
                })
                .collect();
            for i in 0..all.len() {
                for j in (i + 1)..all.len() {
                    assert_ne!(
                        all[i], all[j],
                        "seed {seed}: name collision: {:?} vs {:?}",
                        all[i], all[j]
                    );
                }
            }
        }
        // Per-world names (Decision 3): different seeds name at least *some* blocks differently.
        let a: Vec<String> = Block::ALL.iter().map(|b| display_name(1, *b)).collect();
        let b: Vec<String> = Block::ALL.iter().map(|b| display_name(2, *b)).collect();
        assert_ne!(a, b, "each world has its own tongue");
    }

    #[test]
    fn name_bearers_are_a_minority_and_match_their_block() {
        // ~1 in 6 inscriptions name-bearing (G19: down from 1/4 — first-discovery envelope); a
        // name-bearer's text is its block's transliteration in the block's stratum script;
        // ambient majority unchanged in spirit (no name).
        let g = |_x: f32, _z: f32| 0.0;
        let marks = inscriptions_near(1337, Vec3::ZERO, 1500.0, g);
        assert!(!marks.is_empty());
        let named = marks.iter().filter(|m| m.name.is_some()).count();
        let frac = named as f32 / marks.len() as f32;
        assert!(
            (0.08..=0.30).contains(&frac),
            "name fraction should be ~1/6, got {frac} ({named}/{})",
            marks.len()
        );
        for m in &marks {
            if let Some(b) = m.name {
                assert_eq!(
                    m.script,
                    block_script(b),
                    "name renders in its stratum script"
                );
                // G20: every name-bearer — intact or worn — keeps its cartouche enclosure.
                assert!(
                    m.text.starts_with(crate::text::MARK_CARTOUCHE_OPEN)
                        && m.text.ends_with(crate::text::MARK_CARTOUCHE_CLOSE),
                    "a name-bearer is cartouched: {:?}",
                    m.text
                );
                // G18: an intact bearer spells the exact transliteration; a worn one keeps it
                // recoverable from the surviving glyph positions (lacunae are wildcards).
                match m.condition {
                    Condition::Intact => assert_eq!(
                        m.text,
                        cartouche(&name_text(1337, b)),
                        "stable cartouched name text"
                    ),
                    // (Compared by *name*: parameterised families share one name, so the
                    // inverse read resolves to the family's first member.)
                    Condition::Worn(_) => assert_eq!(
                        name_of_text(1337, &m.text, m.script).map(|x| x.name()),
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
                if let Some(b) = colossus_label(seed, &p).name {
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
            if (h >> 5).is_multiple_of(6) {
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
        let mut seen: std::collections::HashSet<u8> = std::collections::HashSet::new();
        for p in &placements {
            let l = colossus_label(7, p);
            let b = l.name.expect("every monument label is name-bearing (G9)");
            assert!(
                b.required().is_some(),
                "monuments name the gated vocabulary"
            );
            assert_eq!(l.text, cartouche(&name_text(7, b)));
            seen.insert(b.code());
            // Deterministic per colossus.
            assert_eq!(colossus_label(7, p).name, l.name);
        }
        // G19: the label table is FLAT over the gated vocabulary (the RunFoot-×3 bias made
        // Relics-first onboarding roulette) — every gated name should turn up on monuments.
        for b in [Block::Seek, Block::Circle, Block::Goto, Block::RunFoot] {
            assert!(
                seen.contains(&b.code()),
                "the flat monument table should surface '{}' within 1200 units",
                b.name()
            );
        }
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
            if (h >> 5).is_multiple_of(6) {
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

    /// G18: `name_of_text` inverts [`name_text`] for every block, tolerates lacunae (a worn
    /// name still reads), and declines erased/ambient text. G20: over the seeded lexicon names.
    #[test]
    fn name_of_text_reads_intact_and_worn_names() {
        use crate::console::Block;
        for seed in [1u32, 1337] {
            for b in Block::ALL {
                let script = block_script(b);
                let full = name_text(seed, b);
                let read = name_of_text(seed, &full, script).expect("intact name reads");
                assert_eq!(read.name(), b.name(), "reads as its (family) name");
                // Wear the first glyph away: still reads (recoverable from partial glyphs, v1).
                let worn = weather_text(&full, Condition::Worn(1));
                assert_eq!(
                    name_of_text(seed, &worn, script).map(|x| x.name()),
                    Some(b.name())
                );
            }
        }
        // Erased text and ambient noise are not names.
        assert_eq!(
            name_of_text(1337, &weather_text("ABC", Condition::Erased), Script::Latin),
            None
        );
        assert_eq!(name_of_text(1337, "QQQQQQQQQ", Script::Latin), None);
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
            assert!(
                !m.text.contains(crate::text::MARK_CARTOUCHE_OPEN),
                "an erasure is a bare gouge — no cartouche (that tease is G21's)"
            );
        }
        // G20: ambient (non-name) text is never cartouched.
        for m in marks.iter().filter(|m| m.name.is_none()) {
            assert!(
                !m.text.contains(crate::text::MARK_CARTOUCHE_OPEN)
                    && !m.text.contains(crate::text::MARK_CARTOUCHE_CLOSE),
                "ambient text must not carry the name enclosure: {:?}",
                m.text
            );
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

    /// G20: the cartouche helpers — enclosure round-trip, strip is content-only, marks carry
    /// no yield, and a cartouched worn name still reads via `name_of_text`.
    #[test]
    fn cartouche_wraps_strips_and_never_pays() {
        use crate::console::Block;
        let inner = name_text(1337, Block::Collect);
        let wrapped = cartouche(&inner);
        assert!(wrapped.starts_with(crate::text::MARK_CARTOUCHE_OPEN));
        assert!(wrapped.ends_with(crate::text::MARK_CARTOUCHE_CLOSE));
        assert_eq!(strip_cartouche(&wrapped), inner);
        assert_eq!(
            strip_cartouche(&inner),
            inner,
            "uncartouched text passes through"
        );
        // The enclosure is structure: it adds nothing to the data glyph count (no yield).
        assert_eq!(
            crate::progress::glyph_count(&wrapped),
            crate::progress::glyph_count(&inner)
        );
        // A cartouched name reads; a cartouched *worn* name still reads.
        let script = block_script(Block::Collect);
        assert_eq!(
            name_of_text(1337, &wrapped, script).map(|b| b.name()),
            Some("collect")
        );
        let worn = cartouche(&weather_text(&inner, Condition::Worn(1)));
        assert_eq!(
            name_of_text(1337, &worn, script).map(|b| b.name()),
            Some("collect")
        );
        // Monument labels are cartouched too.
        let p = Placement {
            pos: glam::Vec3::new(40.0, 18.0, -12.0),
            yaw: 0.7,
            voxel: 1.3,
            seed: 12345,
            solid: false,
            human: false,
        };
        let l = colossus_label(1337, &p);
        assert!(
            l.text.starts_with(crate::text::MARK_CARTOUCHE_OPEN)
                && l.text.ends_with(crate::text::MARK_CARTOUCHE_CLOSE)
        );
    }

    /// G20: frame instances in the streamed field — ambient-only, spell the frame verbatim in
    /// their script, mutually exclusive with names, and erased cells lose frame membership.
    #[test]
    fn frame_cells_spell_the_frame_verbatim() {
        let g = |_x: f32, _z: f32| 0.0;
        let seed = 1337;
        let marks = inscriptions_near(seed, Vec3::ZERO, 2500.0, g);
        let frames: Vec<_> = marks.iter().filter(|m| m.frame.is_some()).collect();
        assert!(
            frames.len() >= 3,
            "a 2500-unit radius should hold several frame instances, got {}",
            frames.len()
        );
        let id = crate::lexicon::frame_id(seed);
        for m in &frames {
            assert_eq!(m.frame, Some(id), "one recurring frame per world (today)");
            assert!(
                m.name.is_none(),
                "a frame instance is ambient, never a name"
            );
            assert!(
                !m.text.contains(crate::text::MARK_CARTOUCHE_OPEN),
                "frames are not cartouched (the enclosure means NAME)"
            );
            match m.condition {
                Condition::Intact => assert_eq!(
                    m.text,
                    transliterate(&crate::lexicon::frame(seed, m.cell), m.script),
                    "an intact instance spells the frame verbatim in its script"
                ),
                Condition::Worn(_) => assert!(
                    m.text.chars().any(|c| c == crate::text::MARK_LACUNA),
                    "a worn instance shows its lacunae"
                ),
                Condition::Erased => panic!("an erased cell must have dropped its frame"),
            }
            assert!(
                m.text.split(' ').count() >= 3,
                "the frame keeps its word shape (spaces survive transliteration): {:?}",
                m.text
            );
        }
        // Erased cells never carry a frame id (checked across the whole field).
        for m in &marks {
            if m.condition == Condition::Erased {
                assert!(m.frame.is_none());
            }
        }
    }

    /// G20: Leiden restoration against known frames — skeleton lacunae restore bracketed and
    /// pay full; slot lacunae, ambiguity, unpinned matches, and mismatches never restore.
    #[test]
    fn restore_worn_restores_skeleton_lacunae_only_and_uniquely() {
        let seed = 1337;
        let script = Script::Greek;
        let known = world_frames(seed);
        let full = transliterate(&crate::lexicon::frame(seed, (2, 5)), script);
        let words: Vec<&str> = full.split(' ').collect();
        let skeleton = &known[0].words;
        assert_eq!(words.len(), skeleton.len());
        let lac = crate::text::MARK_LACUNA;
        // Helper: wear exactly the given (word, char) positions.
        let wear = |positions: &[(usize, usize)]| -> String {
            let mut out: Vec<String> = words.iter().map(|w| w.to_string()).collect();
            for &(wi, ci) in positions {
                out[wi] = out[wi]
                    .chars()
                    .enumerate()
                    .map(|(i, c)| if i == ci { lac } else { c })
                    .collect();
            }
            out.join(" ")
        };
        // (a) a lacuna in a FIXED word restores, bracketed, to the exact full text.
        let fixed_wi = skeleton.iter().position(|w| w.is_some()).unwrap();
        let worn = wear(&[(fixed_wi, 0)]);
        let restored = restore_worn(&worn, script, &known).expect("skeleton lacuna restores");
        assert!(restored.contains('['), "Leiden-bracketed restoration");
        assert!(!restored.contains(lac), "no lacunae remain");
        let unbracketed: String = restored
            .chars()
            .filter(|c| *c != '[' && *c != ']')
            .collect();
        assert_eq!(unbracketed, full, "restores the exact lost glyphs");
        assert_eq!(
            crate::progress::glyph_count(&restored),
            crate::progress::glyph_count(&full),
            "restored pays full (brackets are structure)"
        );
        // (b) a lacuna in the SLOT never restores (the formula doesn't fix the variable).
        let slot_wi = skeleton.iter().position(|w| w.is_none()).unwrap();
        assert_eq!(restore_worn(&wear(&[(slot_wi, 0)]), script, &known), None);
        // (c) nothing lost / not matching / no known frames → no restoration.
        assert_eq!(restore_worn(&full, script, &known), None);
        assert_eq!(restore_worn("ΑΒ ΓΔ", script, &known), None);
        assert_eq!(restore_worn(&worn, script, &[]), None);
        // (d) an AMBIGUOUS match (two known frames both fitting the survivors) never restores.
        //     Wear every fixed-word glyph away: shape alone can't pin a frame either (unpinned).
        let all_fixed: Vec<(usize, usize)> = skeleton
            .iter()
            .enumerate()
            .filter(|(_, w)| w.is_some())
            .flat_map(|(wi, _)| (0..words[wi].chars().count()).map(move |ci| (wi, ci)))
            .collect();
        let shape_only = wear(&all_fixed);
        assert_eq!(
            restore_worn(&shape_only, script, &known),
            None,
            "word-shape alone must not restore (unpinned)"
        );
        // Two same-shape frames, one surviving fixed glyph consistent with both → ambiguous.
        let sk = |a: &str, b: &str, c: &str| {
            vec![
                Some(a.to_string()),
                Some(b.to_string()),
                None,
                Some(c.to_string()),
            ]
        };
        let two = vec![
            FrameSkeleton {
                id: 1,
                words: sk("ki", "no", "ta"),
            },
            FrameSkeleton {
                id: 2,
                words: sk("ki", "su", "ta"),
            },
        ];
        let full2 = transliterate("ki no zapor ta", script);
        let w2: Vec<String> = full2.split(' ').map(String::from).collect();
        // Lose ALL of word 1 ("no"/"su" — the discriminating word): both frames fit.
        let ambiguous = {
            let mut out = w2.clone();
            out[1] = out[1].chars().map(|_| lac).collect();
            out.join(" ")
        };
        assert_eq!(
            restore_worn(&ambiguous, script, &two),
            None,
            "ambiguous across known frames must not restore"
        );
        // …but with only ONE of the two known, the same wear restores against it.
        let one = vec![FrameSkeleton {
            id: 1,
            words: sk("ki", "no", "ta"),
        }];
        let r = restore_worn(&ambiguous, script, &one).expect("unique match restores");
        let un: String = r.chars().filter(|c| *c != '[' && *c != ']').collect();
        assert_eq!(un, full2);
    }

    /// G21: close reading's `recover_worn` — every lacuna fills from the pristine composition,
    /// bracketed Leiden-style; full yield falls out (brackets are structure); a cartouched worn
    /// name recovers to its cartouched full name and still reads via `name_of_text`.
    #[test]
    fn recover_worn_fills_all_lacunae_bracketed_and_full_yield() {
        use crate::console::Block;
        let seed = 1337;
        // A generic worn text: positions 0 and 2 lost.
        let full = "ΑΒΓ ΔΕ";
        let worn = weather_text(full, Condition::Worn(0b101));
        let rec = recover_worn(&worn, full).expect("a worn text recovers");
        assert!(!rec.contains(crate::text::MARK_LACUNA), "no lacunae remain");
        assert!(rec.contains('[') && rec.contains(']'), "Leiden-bracketed");
        let unbracketed: String = rec.chars().filter(|c| *c != '[' && *c != ']').collect();
        assert_eq!(unbracketed, full, "recovers the exact lost glyphs");
        assert_eq!(
            crate::progress::glyph_count(&rec),
            crate::progress::glyph_count(full),
            "recovered pays full (brackets are structure)"
        );
        // Nothing lost / misaligned → no recovery.
        assert_eq!(recover_worn(full, full), None);
        assert_eq!(recover_worn(&worn, "ΑΒ"), None);
        // A cartouched worn name recovers under its enclosure and reads as its block.
        let name = name_text(seed, Block::Seek);
        let worn_name = cartouche(&weather_text(&name, Condition::Worn(1)));
        let pristine = cartouche(&name);
        let rec = recover_worn(&worn_name, &pristine).expect("worn name recovers");
        assert!(rec.starts_with(crate::text::MARK_CARTOUCHE_OPEN));
        assert_eq!(
            name_of_text(seed, &rec, block_script(Block::Seek)).map(|b| b.name()),
            Some("seek"),
            "a recovered name still reads (brackets are structure): {rec:?}"
        );
    }

    /// G21: the streamed field carries close reading's recovery source — every worn inscription
    /// has a `pristine` that aligns char-for-char and recovers; intact/erased carry none.
    #[test]
    fn worn_inscriptions_carry_an_aligned_pristine() {
        let g = |_x: f32, _z: f32| 0.0;
        let seed = 1337;
        let marks = inscriptions_near(seed, Vec3::ZERO, 2500.0, g);
        let mut worn_seen = 0;
        for m in &marks {
            match m.condition {
                Condition::Worn(_) => {
                    worn_seen += 1;
                    let p = m.pristine.as_deref().expect("worn carries pristine");
                    assert_eq!(
                        p.chars().count(),
                        m.text.chars().count(),
                        "pristine aligns char-for-char (same enclosure)"
                    );
                    assert!(!p.contains(crate::text::MARK_LACUNA), "pristine is whole");
                    let rec = recover_worn(&m.text, p).expect("every worn cell recovers");
                    assert!(!rec.contains(crate::text::MARK_LACUNA));
                    // Surviving glyphs agree with the pristine (recovery is honest, not invention).
                    for (c, pc) in m.text.chars().zip(p.chars()) {
                        if c != crate::text::MARK_LACUNA {
                            assert_eq!(c, pc, "survivors match the pristine: {:?}", m.text);
                        }
                    }
                    // Full yield: the recovered text pays what the intact composition would.
                    assert_eq!(
                        crate::progress::glyph_count(&rec),
                        crate::progress::glyph_count(p)
                    );
                }
                _ => assert!(
                    m.pristine.is_none(),
                    "only worn cells carry a recovery source"
                ),
            }
        }
        assert!(worn_seen > 5, "the field holds worn cells to check");
    }

    /// G21 rung 2: `hidden_text` is deterministic, **deep-weighted** (Runic/Galactic data —
    /// Relics/Signals yield — and a real share of deep name-bearers), independent of the
    /// surface-condition bits (the correlation discipline), and the reveal helpers round-trip.
    #[test]
    fn hidden_text_deterministic_deep_weighted_and_reveal_helpers() {
        use crate::console::Block;
        let seed = 1337u32;
        let (mut names, mut data, mut runfoot) = (0u32, 0u32, 0u32);
        let (mut named_by_cond, mut all_by_cond) = (0u32, 0u32);
        for i in 0..4000i32 {
            let cell = (i, -i * 7 + 3);
            let (text, script, name) = hidden_text(seed, cell);
            assert_eq!(hidden_text(seed, cell).0, text, "deterministic");
            assert!(!text.is_empty());
            match name {
                Some(b) => {
                    names += 1;
                    if b == Block::RunFoot {
                        runfoot += 1;
                    }
                    assert!(b.required().is_some(), "hidden names are gated vocabulary");
                    assert_eq!(text, cartouche(&name_text(seed, b)), "a real name-bearer");
                    assert_eq!(script, block_script(b));
                }
                None => {
                    data += 1;
                    assert!(
                        matches!(script, Script::Runic | Script::Galactic),
                        "hidden data is deep-strata script (Relics/Signals)"
                    );
                    assert!(crate::progress::glyph_count(&text) >= 3);
                }
            }
            // Independence probe: hidden-name presence among cells the CONDITION hash erases
            // must track the overall rate (fresh salt — no correlation with the surface bits).
            let ch = hash(
                cell.0 ^ 0x51AB,
                cell.1 ^ 0x2E77,
                seed.wrapping_add(0x00C0_5D17),
            );
            if condition_pick(ch, 6) == Condition::Erased {
                all_by_cond += 1;
                if name.is_some() {
                    named_by_cond += 1;
                }
            }
        }
        let name_rate = names as f32 / 4000.0;
        assert!(
            (0.15..=0.35).contains(&name_rate),
            "~1/4 of gouges hide a name, got {name_rate}"
        );
        assert!(
            data > 0 && runfoot > names / 3,
            "RunFoot dominates (deep ×3)"
        );
        if all_by_cond >= 50 {
            let gated = named_by_cond as f32 / all_by_cond as f32;
            assert!(
                (gated - name_rate).abs() < 0.15,
                "hidden bits must be independent of the condition bits ({gated} vs {name_rate})"
            );
        }
        // The reveal helpers: one leading gouge marker; content pays, marker doesn't.
        let rev = revealed_text("ᚠᚢᚦ");
        assert!(is_revealed_text(&rev));
        assert!(!is_revealed_text("ᚠᚢᚦ"), "plain text is not a reveal");
        let gouge: String = std::iter::repeat_n(crate::text::MARK_GOUGE, 3).collect();
        assert!(
            !is_revealed_text(&gouge),
            "an unresolved gouge is erased, not revealed"
        );
        assert!(is_erased_text(&gouge) && !is_erased_text(&rev));
        assert_eq!(strip_reveal(&rev), "ᚠᚢᚦ");
        assert_eq!(
            crate::progress::glyph_count(&rev),
            3,
            "the reveal marker is structure (no yield)"
        );
        // A revealed hidden NAME still reads as its block (marker + cartouche are structure).
        let (text, script, name) = (0..200i32)
            .map(|i| {
                let c = (i, i * 3 + 1);
                let (t, s, n) = hidden_text(seed, c);
                (revealed_text(&t), s, n)
            })
            .find(|(_, _, n)| n.is_some())
            .expect("some hidden name in 200 cells");
        assert_eq!(
            name_of_text(seed, &text, script).map(|b| b.name()),
            name.map(|b| b.name()),
            "a revealed name reads via name_of_text"
        );
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
