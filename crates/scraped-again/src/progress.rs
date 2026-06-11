//! Scraped Again — the **economy spine** (G1): five typed **data strata**, the **codex**
//! of finds, and a serializable **collect event** path (mirroring `edit::Edit`/`apply`).
//! Pure game logic over the engine's `Script`; no rendering. Save/restore rides the same
//! `k=v` share string as the camera view (its own `pg=` key, ignored by `ShareState`).
//!
//! Glyphs in the world (E17 inscriptions) belong to one of five writing systems; we map
//! **script → stratum** (game-mechanics §5), so the five scripts become five currencies.

use brickmap::text::Script;

/// The five data strata — typed currencies the tech tree will branch on (game-mechanics §5).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Stratum {
    Records,    // Latin — mundane labels (common)
    Schematics, // Greek — engineering marks
    Rites,      // Hiragana — ritual inscriptions
    Relics,     // Runic — grave-marks (rare, deep lore)
    Signals,    // Galactic — alien signal (rarest)
}

impl Stratum {
    /// All five strata, in rarity order.
    pub const ALL: [Stratum; 5] = [
        Stratum::Records,
        Stratum::Schematics,
        Stratum::Rites,
        Stratum::Relics,
        Stratum::Signals,
    ];
    fn byte(self) -> u8 {
        match self {
            Stratum::Records => 0,
            Stratum::Schematics => 1,
            Stratum::Rites => 2,
            Stratum::Relics => 3,
            Stratum::Signals => 4,
        }
    }
    fn from_byte(b: u8) -> Stratum {
        Stratum::ALL[(b as usize).min(4)]
    }
}

/// Cost (in a stratum's own data) to `decode` / comprehend it (G6). Small + tunable.
pub const DECODE_COST: u64 = 12;

/// G10: the first **spend faculties** — passive, modest multipliers bought with shards.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Faculty {
    /// Scan radius +25% / level.
    Sensing,
    /// Collect/beam reach +20% / level.
    Reach,
    /// Cruise speed +15% / level.
    Drive,
}

impl Faculty {
    pub const ALL: [Faculty; 3] = [Faculty::Sensing, Faculty::Reach, Faculty::Drive];
    pub fn label(self) -> &'static str {
        match self {
            Faculty::Sensing => "sensing",
            Faculty::Reach => "reach",
            Faculty::Drive => "drive",
        }
    }
    pub fn idx(self) -> usize {
        match self {
            Faculty::Sensing => 0,
            Faculty::Reach => 1,
            Faculty::Drive => 2,
        }
    }
}

/// Shard cost of each faculty level (G10 pinned ladder — placeholder numbers, tuned at the
/// human pass). Level caps at [`MAX_FACULTY_LEVEL`].
pub const FACULTY_COSTS: [u64; 3] = [25, 75, 200];
pub const MAX_FACULTY_LEVEL: u8 = 3;

/// The live multipliers a faculty loadout grants (pure; applied at exactly three call sites:
/// scan range, collect/beam reach, cruise speed — charter §4: no plumbing sprawl).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FacultyMults {
    pub sensing: f32,
    pub reach: f32,
    pub drive: f32,
}

/// Multipliers for a set of faculty levels (pure + unit-tested).
pub fn faculty_mults(levels: [u8; 3]) -> FacultyMults {
    FacultyMults {
        sensing: 1.0 + 0.25 * levels[0] as f32,
        reach: 1.0 + 0.20 * levels[1] as f32,
        drive: 1.0 + 0.15 * levels[2] as f32,
    }
}

impl Stratum {
    /// Short label for the HUD readout.
    pub fn label(self) -> &'static str {
        match self {
            Stratum::Records => "REC",
            Stratum::Schematics => "SCH",
            Stratum::Rites => "RIT",
            Stratum::Relics => "REL",
            Stratum::Signals => "SIG",
        }
    }
}

/// Map an inscription's writing system to the stratum its data feeds. `Auto` (the fallback
/// chain, never a real inscription's script) is treated as Records.
pub fn stratum_of(script: Script) -> Stratum {
    match script {
        Script::Latin | Script::Auto => Stratum::Records,
        Script::Greek => Stratum::Schematics,
        Script::Hiragana => Stratum::Rites,
        Script::Runic => Stratum::Relics,
        Script::Galactic => Stratum::Signals,
    }
}

/// The inverse of [`stratum_of`]: the writing system a stratum's text is written in (G9 — a
/// block's name-inscription renders in **its stratum's script**).
pub fn script_for(stratum: Stratum) -> Script {
    match stratum {
        Stratum::Records => Script::Latin,
        Stratum::Schematics => Script::Greek,
        Stratum::Rites => Script::Hiragana,
        Stratum::Relics => Script::Runic,
        Stratum::Signals => Script::Galactic,
    }
}

/// Data yielded by collecting a glyph of `script` with `glyphs` non-space characters. A pure
/// function (rarer strata pay more per glyph); small integers, tuned later. `glyphs` is
/// clamped so a pathological string can't mint a fortune.
pub fn yield_amount(script: Script, glyphs: u32) -> u64 {
    let base: u64 = match stratum_of(script) {
        Stratum::Records => 1,
        Stratum::Schematics => 2,
        Stratum::Rites => 2,
        Stratum::Relics => 4,
        Stratum::Signals => 6,
    };
    base * (1 + glyphs.min(32) as u64)
}

/// The five strata counts.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Strata {
    pub records: u64,
    pub schematics: u64,
    pub rites: u64,
    pub relics: u64,
    pub signals: u64,
}

impl Strata {
    fn slot(&mut self, s: Stratum) -> &mut u64 {
        match s {
            Stratum::Records => &mut self.records,
            Stratum::Schematics => &mut self.schematics,
            Stratum::Rites => &mut self.rites,
            Stratum::Relics => &mut self.relics,
            Stratum::Signals => &mut self.signals,
        }
    }
    pub fn get(&self, s: Stratum) -> u64 {
        match s {
            Stratum::Records => self.records,
            Stratum::Schematics => self.schematics,
            Stratum::Rites => self.rites,
            Stratum::Relics => self.relics,
            Stratum::Signals => self.signals,
        }
    }
    /// Bank the yield of one collected glyph.
    pub fn add(&mut self, script: Script, glyphs: u32) {
        *self.slot(stratum_of(script)) += yield_amount(script, glyphs);
    }
    pub fn total(&self) -> u64 {
        self.records + self.schematics + self.rites + self.relics + self.signals
    }
}

/// One recorded find in the codex.
#[derive(Clone, Debug, PartialEq)]
pub struct CodexEntry {
    pub find_id: u64,
    pub script: Script,
    pub text: String,
    pub pos: [f32; 3],
}

/// A nearby, still-collectible inscription — what the app aims the collect pick at, and the
/// source it reconstructs a `CodexEntry` from. (Built each time inscriptions stream in.)
#[derive(Clone, Debug)]
pub struct Collectible {
    pub find_id: u64,
    pub script: Script,
    pub text: String,
    pub pos: [f32; 3],
    /// G9: this inscription **names a block** — collecting it discovers that block in the console.
    pub name: Option<crate::console::Block>,
}

/// A stable, deterministic id for an inscription find — a function of its grid cell + script +
/// text (all seed-deterministic), so the same world always yields the same id, across runs and
/// independent of visit order. Freeze per worldgen version (E12 policy).
pub fn find_id(cell: (i32, i32), script: Script, text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a
    let mut mix = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in cell
        .0
        .to_le_bytes()
        .iter()
        .chain(cell.1.to_le_bytes().iter())
    {
        mix(*byte);
    }
    mix(script_byte(script));
    for b in text.as_bytes() {
        mix(*b);
    }
    h
}

/// Glyph count (non-space) of an inscription's text — the yield/length input.
pub fn glyph_count(text: &str) -> u32 {
    text.chars().filter(|c| !c.is_whitespace()).count() as u32
}

/// A serializable world-mutating event (mirrors `edit::Edit`). The whole collection story
/// funnels through this + [`Progress::apply`], so it stays the deterministic, replayable,
/// shareable mutation path (multiplayer/undo groundwork, roadmap N1).
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Collect an inscription into its stratum + the codex.
    Collect {
        find_id: u64,
        script: Script,
        text: String,
        pos: [f32; 3],
    },
    /// G9: **discover** a block — its name was collected from a name-bearing inscription, so the
    /// console lists it (still stratum-locked until decoded).
    Discover { block: crate::console::Block },
    /// G10: collect a world **shard** — banks its rarity yield + counts it by domain×rarity.
    CollectShard {
        domain: Stratum,
        rarity: crate::shards::Rarity,
    },
    /// G10: **spend** the shard bank on a faculty level (gated on affordability + the level cap).
    Spend { faculty: Faculty },
}

/// All run progress: the banked strata + the codex of finds (with a de-dup set) + the set of
/// **scanned** (known-but-maybe-uncollected) sites — the G3 opportunity surface.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub strata: Strata,
    pub codex: Vec<CodexEntry>,
    seen: std::collections::HashSet<u64>,
    scanned: std::collections::HashSet<u64>,
    /// Comprehended strata (G6): spending `decode` makes a stratum's script legible + grows the
    /// console's block vocabulary.
    comprehended: std::collections::HashSet<Stratum>,
    /// Discovered blocks (G9): names collected from the world. Starters (no required stratum) are
    /// implicitly always discovered — see [`Progress::is_discovered`] — so this set only carries
    /// the gated vocabulary, and pre-G9 payloads load as starter-only.
    discovered: std::collections::HashSet<crate::console::Block>,
    /// G10: the spendable shard bank (Σ rarity yields of every shard collected, minus spends).
    shard_bank: u64,
    /// G10: lifetime shard pickup counts, domain × rarity (display + future domain-matched costs).
    shard_counts: [[u32; 3]; 5],
    /// G10: faculty levels (sensing / reach / drive), each capped at [`MAX_FACULTY_LEVEL`].
    faculties: [u8; 3],
}

impl Progress {
    pub fn has(&self, find_id: u64) -> bool {
        self.seen.contains(&find_id)
    }

    /// Mark a site **scanned/known** (G3) without collecting it. Returns `true` if newly known.
    pub fn scan(&mut self, find_id: u64) -> bool {
        self.scanned.insert(find_id)
    }

    pub fn is_scanned(&self, find_id: u64) -> bool {
        self.scanned.contains(&find_id)
    }

    /// How many sites are **known** (scanned or already collected) — the HUD readout.
    pub fn known_count(&self) -> usize {
        self.seen.union(&self.scanned).count()
    }

    /// How many sites are **collected** (the codex size).
    pub fn collected_count(&self) -> usize {
        self.codex.len()
    }

    /// G6: `decode` (spend) a stratum — if you can afford [`DECODE_COST`] of it and haven't
    /// already comprehended it, spend that data and mark it comprehended. Returns `true` on
    /// success (idempotent + affordability-gated).
    pub fn comprehend(&mut self, s: Stratum) -> bool {
        if self.comprehended.contains(&s) || self.strata.get(s) < DECODE_COST {
            return false;
        }
        *self.strata.slot(s) -= DECODE_COST;
        self.comprehended.insert(s);
        true
    }

    pub fn is_comprehended(&self, s: Stratum) -> bool {
        self.comprehended.contains(&s)
    }

    /// Is `script` legible? (Its stratum has been decoded — its inscriptions render translated.)
    pub fn is_legible(&self, script: Script) -> bool {
        self.comprehended.contains(&stratum_of(script))
    }

    /// The richest not-yet-comprehended stratum you can currently afford to `decode`, if any —
    /// the target for a one-click `decode` block (G6).
    pub fn decodable(&self) -> Option<Stratum> {
        Stratum::ALL
            .into_iter()
            .filter(|&s| !self.comprehended.contains(&s) && self.strata.get(s) >= DECODE_COST)
            .max_by_key(|&s| self.strata.get(s))
    }

    /// Is this block **discovered** (G9) — its name collected from the world? Starters (no
    /// required stratum) are always discovered, so the opening + given routines are untouched and
    /// pre-G9 payloads load as starter-only.
    pub fn is_discovered(&self, b: crate::console::Block) -> bool {
        b.required().is_none() || self.discovered.contains(&b)
    }

    /// The discovered gated blocks (for syncing the console's view of the vocabulary).
    pub fn discovered_blocks(&self) -> impl Iterator<Item = crate::console::Block> + '_ {
        self.discovered.iter().copied()
    }

    /// Apply an event. Returns `true` if it changed state (a new find), `false` on a
    /// duplicate collect (no-op — the de-dup that keeps strata/codex stable).
    pub fn apply(&mut self, ev: &Event) -> bool {
        match ev {
            Event::Collect {
                find_id,
                script,
                text,
                pos,
            } => {
                if !self.seen.insert(*find_id) {
                    return false; // already collected
                }
                self.strata.add(*script, glyph_count(text));
                self.codex.push(CodexEntry {
                    find_id: *find_id,
                    script: *script,
                    text: text.clone(),
                    pos: *pos,
                });
                true
            }
            // G9: a block name recovered. Idempotent — re-collecting an already-discovered name
            // is a normal collect (the Collect event above still banks its data).
            Event::Discover { block } => {
                if self.is_discovered(*block) {
                    return false;
                }
                self.discovered.insert(*block)
            }
            // G10: bank a shard (always changes state — shards are bulk currency, no dedup).
            Event::CollectShard { domain, rarity } => {
                self.shard_bank += rarity.yield_amount();
                self.shard_counts[domain.byte() as usize][rarity.idx()] += 1;
                true
            }
            // G10: buy the next level of a faculty — gated on the cap + affordability.
            Event::Spend { faculty } => {
                let lvl = self.faculties[faculty.idx()];
                if lvl >= MAX_FACULTY_LEVEL {
                    return false;
                }
                let cost = FACULTY_COSTS[lvl as usize];
                if self.shard_bank < cost {
                    return false;
                }
                self.shard_bank -= cost;
                self.faculties[faculty.idx()] += 1;
                true
            }
        }
    }

    /// G10: the spendable shard bank.
    pub fn shard_bank(&self) -> u64 {
        self.shard_bank
    }

    /// G10: lifetime pickups for one domain (summed over rarities — the HUD per-domain line).
    pub fn shard_count(&self, d: Stratum) -> u32 {
        self.shard_counts[d.byte() as usize].iter().sum()
    }

    /// G10: the current faculty levels (sensing / reach / drive).
    pub fn faculty_levels(&self) -> [u8; 3] {
        self.faculties
    }

    /// G10: the live faculty multipliers (see [`faculty_mults`]).
    pub fn faculties(&self) -> FacultyMults {
        faculty_mults(self.faculties)
    }

    /// Encode as a `pg=<hex>` share segment (binary blob → hex; unicode- and URL-safe). The
    /// blob carries the strata + every codex entry, so a reload restores both fully.
    pub fn encode(&self) -> String {
        let mut b = Vec::new();
        b.push(5u8); // version (…; 4 = + discovered, G9; 5 = + shards/faculties, G10)
        for s in [
            self.strata.records,
            self.strata.schematics,
            self.strata.rites,
            self.strata.relics,
            self.strata.signals,
        ] {
            b.extend_from_slice(&s.to_le_bytes());
        }
        b.extend_from_slice(&(self.codex.len() as u32).to_le_bytes());
        for e in &self.codex {
            b.extend_from_slice(&e.find_id.to_le_bytes());
            b.push(script_byte(e.script));
            for f in e.pos {
                b.extend_from_slice(&f.to_le_bytes());
            }
            let bytes = e.text.as_bytes();
            b.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            b.extend_from_slice(bytes);
        }
        // v2: the scanned (known-but-uncollected) site ids, sorted for a stable encoding.
        let mut scanned: Vec<u64> = self.scanned.iter().copied().collect();
        scanned.sort_unstable();
        b.extend_from_slice(&(scanned.len() as u32).to_le_bytes());
        for id in scanned {
            b.extend_from_slice(&id.to_le_bytes());
        }
        // v3: comprehended strata (sorted bytes for a stable encoding).
        let mut comp: Vec<u8> = self.comprehended.iter().map(|s| s.byte()).collect();
        comp.sort_unstable();
        b.push(comp.len() as u8);
        b.extend_from_slice(&comp);
        // v4: discovered blocks (G9; sorted codes for a stable encoding).
        let mut disc: Vec<u8> = self.discovered.iter().map(|x| x.code()).collect();
        disc.sort_unstable();
        b.push(disc.len() as u8);
        b.extend_from_slice(&disc);
        // v5: the shard economy (G10) — bank, 5×3 counts, 3 faculty levels.
        b.extend_from_slice(&self.shard_bank.to_le_bytes());
        for row in &self.shard_counts {
            for c in row {
                b.extend_from_slice(&c.to_le_bytes());
            }
        }
        b.extend_from_slice(&self.faculties);
        format!("pg={}", to_hex(&b))
    }

    /// Decode from a share string (scans for the `pg=` key; other keys ignored). Returns
    /// `Progress::default()` when absent or malformed (lenient, like `ShareState::decode`).
    pub fn decode(s: &str) -> Progress {
        let s = s.strip_prefix('#').unwrap_or(s);
        let Some(hex) = s.split('&').find_map(|p| p.strip_prefix("pg=")) else {
            return Progress::default();
        };
        let Some(b) = from_hex(hex) else {
            return Progress::default();
        };
        parse_blob(&b).unwrap_or_default()
    }
}

fn script_byte(s: Script) -> u8 {
    match s {
        Script::Auto => 0,
        Script::Latin => 1,
        Script::Greek => 2,
        Script::Hiragana => 3,
        Script::Galactic => 4,
        Script::Runic => 5,
    }
}

fn script_from_byte(b: u8) -> Script {
    match b {
        1 => Script::Latin,
        2 => Script::Greek,
        3 => Script::Hiragana,
        4 => Script::Galactic,
        5 => Script::Runic,
        _ => Script::Auto,
    }
}

/// Parse the binary blob produced by [`Progress::encode`]; `None` on any truncation/bad data.
fn parse_blob(b: &[u8]) -> Option<Progress> {
    let mut p = 0usize;
    let take = |p: &mut usize, n: usize| -> Option<&[u8]> {
        let s = b.get(*p..*p + n)?;
        *p += n;
        Some(s)
    };
    let u64at =
        |p: &mut usize| -> Option<u64> { Some(u64::from_le_bytes(take(p, 8)?.try_into().ok()?)) };
    let version = *take(&mut p, 1)?.first()?;
    if !(1..=5).contains(&version) {
        return None; // unknown version
    }
    let strata = Strata {
        records: u64at(&mut p)?,
        schematics: u64at(&mut p)?,
        rites: u64at(&mut p)?,
        relics: u64at(&mut p)?,
        signals: u64at(&mut p)?,
    };
    let count = u32::from_le_bytes(take(&mut p, 4)?.try_into().ok()?);
    let mut codex = Vec::with_capacity(count as usize);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..count {
        let find_id = u64at(&mut p)?;
        let script = script_from_byte(*take(&mut p, 1)?.first()?);
        let mut pos = [0.0f32; 3];
        for f in &mut pos {
            *f = f32::from_le_bytes(take(&mut p, 4)?.try_into().ok()?);
        }
        let tlen = u16::from_le_bytes(take(&mut p, 2)?.try_into().ok()?) as usize;
        let text = String::from_utf8(take(&mut p, tlen)?.to_vec()).ok()?;
        seen.insert(find_id);
        codex.push(CodexEntry {
            find_id,
            script,
            text,
            pos,
        });
    }
    // v2: the scanned set (absent in v1 → stays empty).
    let mut scanned = std::collections::HashSet::new();
    if version >= 2 {
        let sc = u32::from_le_bytes(take(&mut p, 4)?.try_into().ok()?);
        for _ in 0..sc {
            scanned.insert(u64at(&mut p)?);
        }
    }
    // v3: comprehended strata (absent in v1/v2 → empty).
    let mut comprehended = std::collections::HashSet::new();
    if version >= 3 {
        let cc = *take(&mut p, 1)?.first()?;
        for _ in 0..cc {
            comprehended.insert(Stratum::from_byte(*take(&mut p, 1)?.first()?));
        }
    }
    // v4: discovered blocks (G9; absent pre-v4 → starter-only). Unknown codes skipped (lenient).
    let mut discovered = std::collections::HashSet::new();
    if version >= 4 {
        let dc = *take(&mut p, 1)?.first()?;
        for _ in 0..dc {
            if let Some(blk) = crate::console::Block::from_code(*take(&mut p, 1)?.first()?) {
                discovered.insert(blk);
            }
        }
    }
    // v5: the shard economy (G10; absent pre-v5 → zeroed).
    let mut shard_bank = 0u64;
    let mut shard_counts = [[0u32; 3]; 5];
    let mut faculties = [0u8; 3];
    if version >= 5 {
        shard_bank = u64at(&mut p)?;
        for row in &mut shard_counts {
            for c in row.iter_mut() {
                *c = u32::from_le_bytes(take(&mut p, 4)?.try_into().ok()?);
            }
        }
        faculties.copy_from_slice(take(&mut p, 3)?);
    }
    Some(Progress {
        strata,
        codex,
        seen,
        scanned,
        comprehended,
        discovered,
        shard_bank,
        shard_counts,
        faculties,
    })
}

fn to_hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push(char::from_digit((x >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((x & 0xf) as u32, 16).unwrap());
    }
    s
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes = s.as_bytes();
    (0..bytes.len() / 2)
        .map(|i| {
            let hi = (bytes[2 * i] as char).to_digit(16)?;
            let lo = (bytes[2 * i + 1] as char).to_digit(16)?;
            Some(((hi << 4) | lo) as u8)
        })
        .collect()
}

/// Two progressions are equal when their strata + codex + scanned set match (`seen` is derived).
impl PartialEq for Progress {
    fn eq(&self, other: &Self) -> bool {
        self.strata == other.strata
            && self.codex == other.codex
            && self.scanned == other.scanned
            && self.comprehended == other.comprehended
            && self.discovered == other.discovered
            && self.shard_bank == other.shard_bank
            && self.shard_counts == other.shard_counts
            && self.faculties == other.faculties
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yield_is_pure_and_tiered() {
        // Same inputs → same output; rarer strata pay more per glyph.
        assert_eq!(
            yield_amount(Script::Latin, 3),
            yield_amount(Script::Latin, 3)
        );
        assert!(yield_amount(Script::Galactic, 3) > yield_amount(Script::Latin, 3));
        assert!(yield_amount(Script::Runic, 3) > yield_amount(Script::Greek, 3));
        // Longer words yield more; the clamp bounds a pathological length.
        assert!(yield_amount(Script::Latin, 5) > yield_amount(Script::Latin, 1));
        assert_eq!(
            yield_amount(Script::Latin, 100),
            yield_amount(Script::Latin, 32)
        );
        // Every script maps to a stratum (no panic / Auto handled).
        for s in [
            Script::Latin,
            Script::Greek,
            Script::Hiragana,
            Script::Runic,
            Script::Galactic,
            Script::Auto,
        ] {
            assert!(yield_amount(s, 4) > 0);
        }
    }

    fn ev(id: u64, script: Script, text: &str) -> Event {
        Event::Collect {
            find_id: id,
            script,
            text: text.into(),
            pos: [1.0, 2.0, 3.0],
        }
    }

    #[test]
    fn collect_banks_stratum_and_records_codex() {
        let mut p = Progress::default();
        assert!(p.apply(&ev(1, Script::Greek, "ΑΒ ΓΔΕ"))); // 5 glyphs → Schematics
        assert_eq!(p.strata.schematics, yield_amount(Script::Greek, 5));
        assert_eq!(p.codex.len(), 1);
        assert!(p.has(1));
    }

    #[test]
    fn collect_dedups() {
        let mut p = Progress::default();
        assert!(p.apply(&ev(7, Script::Latin, "ABC")));
        let after = p.strata;
        // Re-collecting the same find id is a no-op on strata + codex.
        assert!(!p.apply(&ev(7, Script::Latin, "ABC")));
        assert_eq!(p.strata, after);
        assert_eq!(p.codex.len(), 1);
    }

    #[test]
    fn find_id_is_stable_and_distinct() {
        let a = find_id((3, -2), Script::Runic, "ᚠᚢᚦ");
        assert_eq!(a, find_id((3, -2), Script::Runic, "ᚠᚢᚦ")); // stable
        assert_ne!(a, find_id((3, -1), Script::Runic, "ᚠᚢᚦ")); // cell matters
        assert_ne!(a, find_id((3, -2), Script::Greek, "ᚠᚢᚦ")); // script matters
        assert_ne!(a, find_id((3, -2), Script::Runic, "ᚠᚢ")); // text matters
    }

    #[test]
    fn progress_round_trips_through_the_share_segment() {
        let mut p = Progress::default();
        p.apply(&ev(1, Script::Latin, "ROAD"));
        p.apply(&ev(2, Script::Hiragana, "あい うえ"));
        p.apply(&ev(3, Script::Galactic, "XYZ"));
        // Round-trip through the encoded `pg=` segment (embedded in a fuller share string).
        let s = format!("v=1&s=42&{}&x=1.0", p.encode());
        let back = Progress::decode(&s);
        assert_eq!(back.strata, p.strata);
        assert_eq!(back.codex, p.codex);
        assert!(back.has(1) && back.has(2) && back.has(3));
        // Unicode survives the trip.
        assert_eq!(back.codex[1].text, "あい うえ");
    }

    #[test]
    fn scan_marks_known_not_collected() {
        let mut p = Progress::default();
        assert!(p.scan(10)); // newly known
        assert!(!p.scan(10)); // already known
        assert!(p.is_scanned(10));
        assert!(!p.has(10)); // scanned ≠ collected
        assert_eq!(p.known_count(), 1);
        assert_eq!(p.collected_count(), 0);
        // Collecting it adds to collected; still one known site, now one collected.
        p.apply(&ev(10, Script::Latin, "AB"));
        assert!(p.has(10));
        assert_eq!(p.known_count(), 1); // union of scanned + seen
        assert_eq!(p.collected_count(), 1);
    }

    #[test]
    fn scanned_set_round_trips_v2_and_tolerates_v1() {
        let mut p = Progress::default();
        p.apply(&ev(1, Script::Latin, "AB"));
        p.scan(5);
        p.scan(9);
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p);
        assert!(back.is_scanned(5) && back.is_scanned(9));
        assert!(back.has(1));
        // A v1 blob (no scanned set) still decodes — scanned just comes back empty.
        let v1_hex = {
            // strata=0, codex count=0, version 1.
            let mut b = vec![1u8];
            b.extend_from_slice(&[0u8; 40]); // 5 × u64 strata
            b.extend_from_slice(&0u32.to_le_bytes()); // codex count
            super::to_hex(&b)
        };
        let v1 = Progress::decode(&format!("pg={v1_hex}"));
        assert_eq!(v1, Progress::default());
    }

    #[test]
    fn decode_spends_and_comprehends() {
        let mut p = Progress::default();
        // Not enough data → can't decode.
        assert!(!p.comprehend(Stratum::Records));
        // Bank some Records, then decode.
        for _ in 0..6 {
            p.apply(&ev(0, Script::Latin, "ABCD")); // 4 glyphs each; dedup → only first counts
        }
        p.strata.records = DECODE_COST + 3; // ensure affordable for the test
        assert!(p.comprehend(Stratum::Records));
        assert!(p.is_comprehended(Stratum::Records));
        assert!(p.is_legible(Script::Latin)); // Latin → Records
        assert!(!p.is_legible(Script::Greek)); // others stay glyphs
        assert_eq!(p.strata.records, 3); // spent exactly DECODE_COST
        assert!(!p.comprehend(Stratum::Records)); // idempotent
    }

    #[test]
    fn comprehension_round_trips_v3() {
        let mut p = Progress::default();
        p.strata.relics = DECODE_COST;
        assert!(p.comprehend(Stratum::Relics));
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p);
        assert!(back.is_comprehended(Stratum::Relics));
    }

    #[test]
    fn discover_applies_idempotently_and_starters_are_implicit() {
        use crate::console::Block;
        let mut p = Progress::default();
        // Starters are always discovered (the opening is untouched); gated blocks aren't.
        assert!(p.is_discovered(Block::Collect));
        assert!(!p.is_discovered(Block::Seek));
        // Discovering a gated block applies once; the dupe is a no-op (a normal re-collect).
        assert!(p.apply(&Event::Discover { block: Block::Seek }));
        assert!(p.is_discovered(Block::Seek));
        assert!(!p.apply(&Event::Discover { block: Block::Seek }));
        // Discovering a starter is always a no-op (already known).
        assert!(!p.apply(&Event::Discover {
            block: Block::Collect
        }));
    }

    #[test]
    fn discoveries_round_trip_v4_and_v3_loads_starter_only() {
        use crate::console::Block;
        let mut p = Progress::default();
        p.apply(&ev(1, Script::Latin, "AB"));
        p.apply(&Event::Discover { block: Block::Seek });
        p.apply(&Event::Discover {
            block: Block::RunFoot,
        });
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p);
        assert!(back.is_discovered(Block::Seek) && back.is_discovered(Block::RunFoot));
        assert!(!back.is_discovered(Block::Goto));
        // A pre-G9 (v3) blob still loads — discoveries just come back starter-only.
        let v3_hex = {
            let mut b = vec![3u8];
            b.extend_from_slice(&[0u8; 40]); // strata
            b.extend_from_slice(&0u32.to_le_bytes()); // codex count
            b.extend_from_slice(&0u32.to_le_bytes()); // scanned count
            b.push(0); // comprehended count
            super::to_hex(&b)
        };
        let v3 = Progress::decode(&format!("pg={v3_hex}"));
        assert!(v3.is_discovered(Block::Collect)); // starters implicit
        assert!(!v3.is_discovered(Block::Seek)); // gated: undiscovered
    }

    #[test]
    fn shards_bank_spend_and_round_trip_v5() {
        use crate::shards::Rarity;
        let mut p = Progress::default();
        // Bank: 3 commons + 1 uncommon + 1 rare = 3·1 + 3 + 9 = 15; counts by domain×rarity.
        for _ in 0..3 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Records,
                rarity: Rarity::Common,
            });
        }
        p.apply(&Event::CollectShard {
            domain: Stratum::Relics,
            rarity: Rarity::Uncommon,
        });
        p.apply(&Event::CollectShard {
            domain: Stratum::Signals,
            rarity: Rarity::Rare,
        });
        assert_eq!(p.shard_bank(), 15);
        assert_eq!(p.shard_count(Stratum::Records), 3);
        assert_eq!(p.shard_count(Stratum::Signals), 1);
        // Spend: unaffordable (sensing costs 25 > 15) → no-op; bank some more and buy level 1.
        assert!(!p.apply(&Event::Spend {
            faculty: Faculty::Sensing
        }));
        for _ in 0..2 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Rites,
                rarity: Rarity::Rare,
            });
        }
        assert_eq!(p.shard_bank(), 33);
        assert!(p.apply(&Event::Spend {
            faculty: Faculty::Sensing
        }));
        assert_eq!(p.faculty_levels(), [1, 0, 0]);
        assert_eq!(p.shard_bank(), 33 - 25);
        // Round-trips through pg= v5; old v4-style payloads still load (zeroed economy).
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p);
        assert_eq!(back.shard_bank(), 8);
        assert_eq!(back.faculty_levels(), [1, 0, 0]);
    }

    #[test]
    fn faculty_mults_scale_per_level_and_costs_escalate() {
        let base = faculty_mults([0, 0, 0]);
        assert_eq!(base.sensing, 1.0);
        assert_eq!(base.reach, 1.0);
        assert_eq!(base.drive, 1.0);
        let maxed = faculty_mults([3, 3, 3]);
        assert!((maxed.sensing - 1.75).abs() < 1e-6);
        assert!((maxed.reach - 1.60).abs() < 1e-6);
        assert!((maxed.drive - 1.45).abs() < 1e-6);
        // The ladder escalates; the cap holds.
        assert!(FACULTY_COSTS[0] < FACULTY_COSTS[1] && FACULTY_COSTS[1] < FACULTY_COSTS[2]);
        let mut p = Progress::default();
        for _ in 0..400 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Records,
                rarity: crate::shards::Rarity::Rare,
            });
        }
        for _ in 0..5 {
            p.apply(&Event::Spend {
                faculty: Faculty::Drive,
            });
        }
        assert_eq!(p.faculty_levels()[2], MAX_FACULTY_LEVEL, "level caps at 3");
    }

    #[test]
    fn decode_is_lenient() {
        assert_eq!(Progress::decode(""), Progress::default());
        assert_eq!(Progress::decode("s=1&x=2"), Progress::default()); // no pg=
        assert_eq!(Progress::decode("pg=zzzz"), Progress::default()); // bad hex
        assert_eq!(Progress::decode("pg=00"), Progress::default()); // bad version
    }

    #[test]
    fn determinism_same_sequence_same_blob() {
        let seq = [
            ev(1, Script::Latin, "AB"),
            ev(2, Script::Runic, "ᚠᚢ"),
            ev(1, Script::Latin, "AB"), // dup, ignored
            ev(3, Script::Greek, "ΑΒΓ"),
        ];
        let mut a = Progress::default();
        let mut b = Progress::default();
        for e in &seq {
            a.apply(e);
        }
        for e in &seq {
            b.apply(e);
        }
        assert_eq!(a.encode(), b.encode());
    }
}
