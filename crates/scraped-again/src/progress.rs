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

/// Shard cost of each faculty level. Level caps at [`MAX_FACULTY_LEVEL`]. G15b: this is a
/// *research* cost, not a bank-then-buy price (filled by allocated shard intake, like a block).
/// G19: trued against the measured any-domain yield (≈16/min under autopilot,
/// `docs/pacing-analysis.md`) — L1 ≈ 9.4 min, the full 9-level ladder ≈ 1.6 h cumulative
/// (the old `[25, 75, 200]` maxed everything in ~35 min, 10–30× too fast).
pub const FACULTY_COSTS: [u64; 3] = [150, 450, 900];
pub const MAX_FACULTY_LEVEL: u8 = 3;

/// G17: the walker's **carry cap** — the total shards it can hold in transit before `deposit`
/// (the first real per-agent scarcity; the ship is the uncapped hauler). Placeholder; the feel
/// pass tunes it (brief Decision 4).
pub const CARRY_CAP: u32 = 8;

/// G21: the **sensing ladder**'s researched instruments — recovered faculties of the dead machine
/// that let the *walker* read what survives only as damage (the real recovery ladder: raking
/// light → multispectral → penetrating). Binary unlocks, one level each (v1):
/// - rung 1, **close reading** (Rites-gated): a worn inscription collected *on foot* recovers
///   fully — full text, full yield, frame credit;
/// - rung 2, **deep sensing** (Signals-gated + the standing 8-rare gate, its first natural
///   object): an ⟦erased⟧ inscription collected on foot reveals its hidden content, and
///   palimpsest under-texts become readable.
///
/// Discovered by the **frustration events** (first worn collect / first erased log — the damage
/// teaches the need; the console then offers the remedy), never by name-bearers. Lexicon-named +
/// glyph-rendered like all vocabulary (`Sense::label` is the internal key only).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Sense {
    CloseReading,
    DeepSensing,
}

impl Sense {
    pub const ALL: [Sense; 2] = [Sense::CloseReading, Sense::DeepSensing];
    /// Internal vocabulary key (codec/tests; the display name is the seeded lexicon word).
    pub fn label(self) -> &'static str {
        match self {
            Sense::CloseReading => "close-reading",
            Sense::DeepSensing => "deep-sensing",
        }
    }
    pub fn idx(self) -> usize {
        match self {
            Sense::CloseReading => 0,
            Sense::DeepSensing => 1,
        }
    }
    /// The stratum gating this instrument's research (domain-matched fill, like a block):
    /// close reading fills the empty **Rites** tier, deep sensing the empty **Signals** tier.
    pub fn stratum(self) -> Stratum {
        match self {
            Sense::CloseReading => Stratum::Rites,
            Sense::DeepSensing => Stratum::Signals,
        }
    }
}

/// G20: how many **intact sightings** of a recurring frame teach it (codex-known). Placeholder
/// (brief Decision 1); the feel pass tunes.
pub const FRAME_KNOWN_SIGHTINGS: u8 = 3;

/// G15: what research can target — a discovered **block** (→ comprehend it, G15a), a **faculty**
/// (→ level it, G15b), or (G21) a discovered **sensing instrument** (→ comprehend the ladder
/// rung). The unified research pipe (no separate bank-then-spend subsystem).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ResearchTarget {
    Block(crate::console::Block),
    Faculty(Faculty),
    Sense(Sense),
}

impl ResearchTarget {
    /// A stable single-byte key for the in-progress fill map + `pg=` codec. Block codes are
    /// 0..=15; faculties live at 0xF0.., senses (G21) at 0xE0.. (all disjoint), so a tagged
    /// round-trip is unambiguous.
    fn rkey(self) -> u8 {
        match self {
            ResearchTarget::Block(b) => b.code(),
            ResearchTarget::Faculty(f) => 0xF0 + f.idx() as u8,
            ResearchTarget::Sense(s) => 0xE0 + s.idx() as u8,
        }
    }
    /// Player-facing label: a block by its **glyphs** (G12, unreadable); a faculty by its seeded
    /// vocabulary word rendered glyph (G20 — the same word `spend(…)` shows, in the spend block's
    /// Records/Latin script, so the research bar never re-leaks the English the palette hides);
    /// a sensing instrument (G21) by its seeded word in **its gating stratum's script** (a
    /// recovered instrument of that tier).
    pub fn glyphs(self, seed: u32) -> String {
        match self {
            ResearchTarget::Block(b) => b.glyphs(seed),
            ResearchTarget::Faculty(f) => {
                let word = crate::lexicon::vocab_word(seed, f.label());
                crate::text::to_overlay(
                    &crate::structures::transliterate(&word, Script::Latin),
                    Script::Latin,
                )
            }
            ResearchTarget::Sense(s) => {
                let script = script_for(s.stratum());
                let word = crate::lexicon::vocab_word(seed, s.label());
                crate::text::to_overlay(&crate::structures::transliterate(&word, script), script)
            }
        }
    }
    /// Resolve a `pg=` key byte back to a target (lenient — unknown → `None`).
    fn from_rkey(k: u8) -> Option<ResearchTarget> {
        if (0xF0..0xF0 + 3).contains(&k) {
            Some(ResearchTarget::Faculty(Faculty::ALL[(k - 0xF0) as usize]))
        } else if (0xE0..0xE0 + 2).contains(&k) {
            Some(ResearchTarget::Sense(Sense::ALL[(k - 0xE0) as usize]))
        } else {
            crate::console::Block::from_code(k).map(ResearchTarget::Block)
        }
    }
}

/// G19: how many **rare-tier shard pickups** a research target demands before it can complete
/// (over and above its `filled ≥ cost` bar) — the second, previously unimplemented half of the
/// human's 2026-06-11 decision "rarer blocks demand rarer shards". The deep strata demand rare
/// evidence — Relics 4, Signals 8 — while everything shallower (and every faculty: general
/// machine instrumentation) needs none. Only rare pickups of the target's **own domain**, made
/// while it is the active target, count (the same credit rule as the fill). Placeholder numbers;
/// the *mechanism* is the decision.
pub fn rare_requirement(t: ResearchTarget) -> u32 {
    let deep = |s: Stratum| match s {
        Stratum::Relics => 4,
        Stratum::Signals => 8,
        _ => 0,
    };
    match t {
        ResearchTarget::Block(b) => b.required().map(deep).unwrap_or(0),
        ResearchTarget::Faculty(_) => 0,
        // G21: deep sensing is Signals-gated — the standing 8-rare gate's first natural object.
        ResearchTarget::Sense(s) => deep(s.stratum()),
    }
}

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

/// G18: how far a discovered block's **reading** is trusted — the uncertainty layer. A name
/// collected once is a *hypothesis* (`Provisional`); it's `Confirmed` by a **second sighting**
/// (another inscription bearing the same name) or **behaviorally** (the first successful
/// execution of the block after comprehension — the machine answering IS the confirmation).
/// Display + one gentle lit-goal nudge only: nothing mechanical gates on `Provisional`
/// (Decision 1 — the no-softlock invariant; allocation/research stay open).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Attestation {
    Provisional,
    Confirmed,
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
    /// G21: the inscription's grid cell — the deterministic key the sensing ladder's composed
    /// recoveries (hidden/under texts) derive from.
    pub cell: (i32, i32),
    pub script: Script,
    pub text: String,
    pub pos: [f32; 3],
    /// G9: this inscription **names a block** — collecting it discovers that block in the console.
    pub name: Option<crate::console::Block>,
    /// G18: an ⟦erased⟧ inscription — collecting it yields nothing but **logs the erasure event**
    /// in the codex (content unrecoverable below rung 2 of G21's sensing ladder).
    pub erased: bool,
    /// G20: this inscription is an instance of the recurring formulaic **frame** (`frame_id`).
    /// Intact instances teach the frame (sightings → known); worn instances of a *known* frame
    /// can be restored at collect time.
    pub frame: Option<u64>,
    /// G21: a worn inscription's pre-weathered composition (close reading's recovery source).
    pub pristine: Option<String>,
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

/// Glyph count (non-space) of an inscription's text — the yield/length input. G18: the generic
/// damage marks (a lacuna where a glyph eroded away, the gouge of a deliberate erasure) carry no
/// data, so a **worn** inscription yields proportionally less (only its surviving glyphs pay).
/// G20: the cartouche enclosure marks and the Leiden restoration brackets `[` `]` are
/// structure, not content — they never pay either (a *restored* glyph inside them does).
pub fn glyph_count(text: &str) -> u32 {
    text.chars()
        .filter(|c| {
            !c.is_whitespace()
                && *c != crate::text::MARK_LACUNA
                && *c != crate::text::MARK_GOUGE
                && *c != crate::text::MARK_CARTOUCHE_OPEN
                && *c != crate::text::MARK_CARTOUCHE_CLOSE
                && *c != '['
                && *c != ']'
        })
        .count() as u32
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
    /// console lists it (still stratum-locked until decoded). G18: applying it to an
    /// already-discovered block is a **second sighting** — it upgrades the reading
    /// Provisional → Confirmed (the "dupes yield normally" path gained one line).
    Discover { block: crate::console::Block },
    /// G18: collect an ⟦erased⟧ inscription — **logs the erasure event** in the codex (site,
    /// stratum, the gouge) but banks nothing (content unrecoverable until G20's sensing ladder).
    CollectErased {
        find_id: u64,
        script: Script,
        text: String,
        pos: [f32; 3],
    },
    /// G10: collect a world **shard** — banks its rarity yield + counts it by domain×rarity.
    CollectShard {
        domain: Stratum,
        rarity: crate::shards::Rarity,
    },
    /// G10: **spend** the shard bank on a faculty level (gated on affordability + the level cap).
    Spend { faculty: Faculty },
    /// G20: an **intact sighting** of a recurring formulaic frame (a collect on a frame-matching
    /// inscription). At [`FRAME_KNOWN_SIGHTINGS`] the frame becomes *known*.
    SightFrame { frame_id: u64 },
}

/// All run progress: the banked strata + the codex of finds (with a de-dup set) + the set of
/// **scanned** (known-but-maybe-uncollected) sites — the G3 opportunity surface.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub strata: Strata,
    pub codex: Vec<CodexEntry>,
    seen: std::collections::HashSet<u64>,
    scanned: std::collections::HashSet<u64>,
    /// Comprehended strata (G6 legibility): a stratum's script renders **translated** once
    /// legible. G15: this is no longer set by a `decode`-stratum action (removed) — it's a
    /// *side-effect of research*: comprehending (researching) any block of a stratum marks that
    /// stratum legible. So the decipherment lore-spine survives, funded by the same research pipe.
    comprehended: std::collections::HashSet<Stratum>,
    /// Discovered blocks (G9): names collected from the world. Starters (no required stratum) are
    /// implicitly always discovered — see [`Progress::is_discovered`] — so this set only carries
    /// the gated vocabulary, and pre-G9 payloads load as starter-only.
    discovered: std::collections::HashSet<crate::console::Block>,
    /// G18: discovered blocks whose reading is **confirmed** (second sighting or first use after
    /// comprehension). A discovered block absent here is *provisional* — see
    /// [`Progress::attestation`]. Starters are implicitly confirmed (their names were never
    /// hypotheses), so this only carries the gated vocabulary. Display-only state (Decision 1).
    confirmed: std::collections::HashSet<crate::console::Block>,
    /// G15: blocks whose **research filled** → comprehended (usable). Replaces G9's
    /// decode-a-stratum-unlocks-all gate with per-block research. Starters are implicitly
    /// comprehended (see [`Progress::is_block_comprehended`]); this set carries the gated blocks
    /// the player has researched to completion.
    comprehended_blocks: std::collections::HashSet<crate::console::Block>,
    /// G15: the player's single **active research target** (Decision 1) — a discovered-but-locked
    /// block or an un-maxed faculty. Auto-collected shards fill it (allocate-and-fill).
    active_research: Option<ResearchTarget>,
    /// G15: per-target accumulated **domain-matched** shard yield (`filled`); on `filled ≥ cost`
    /// the block comprehends. Keyed by the block's `code()` so it survives `co=`/`pg=` round-trips.
    research_filled: std::collections::HashMap<u8, u64>,
    /// G19: per-target count of **rare-tier** shard pickups credited while it was the active
    /// target (own-domain only, like the fill). A deep target completes only once this reaches
    /// [`rare_requirement`] — see that fn for the decision it implements. Same key as
    /// `research_filled`; cleared with it on completion.
    research_rare: std::collections::HashMap<u8, u32>,
    /// G10: the spendable shard bank (Σ rarity yields of every shard collected, minus spends).
    shard_bank: u64,
    /// G10: lifetime shard pickup counts, domain × rarity (display + future domain-matched costs).
    shard_counts: [[u32; 3]; 5],
    /// G10: faculty levels (sensing / reach / drive), each capped at [`MAX_FACULTY_LEVEL`].
    faculties: [u8; 3],
    /// G17: the **expedition handshake** — shards *in transit*, held outside `shard_bank`/research
    /// until the ship drains the cache home (Decision 2: value lands on ship pickup). `carry` is
    /// what the walker holds (capped at [`CARRY_CAP`] total — its `deposit` moves it to `cache`);
    /// `cache` is the per-site drop-point the walker fills and the ship empties. Both are
    /// domain×rarity counts (same shape as `shard_counts`). One cache v1 (Decision 3).
    carry: [[u32; 3]; 5],
    cache: [[u32; 3]; 5],
    /// G20: intact sightings per recurring frame (by `frame_id`) — the crib counter.
    frame_sightings: std::collections::HashMap<u64, u8>,
    /// G20: frames the player **knows** (≥ [`FRAME_KNOWN_SIGHTINGS`] sightings): the codex shows
    /// their skeleton, and worn instances that uniquely match one are *restored* (full yield).
    frames_known: std::collections::HashSet<u64>,
    /// G21: sensing instruments **discovered** by the frustration events (first worn collect →
    /// close reading; first erased log → deep sensing). Discovery makes them research targets;
    /// append-only knowledge (nothing un-discovers a need).
    senses_discovered: std::collections::HashSet<Sense>,
    /// G21: sensing instruments **comprehended** (research filled + rare-gated) — the ladder
    /// rungs the walker actually holds. Consulted by the collect paths (on foot only).
    senses: std::collections::HashSet<Sense>,
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

    pub fn is_comprehended(&self, s: Stratum) -> bool {
        self.comprehended.contains(&s)
    }

    /// Is `script` legible? G15: a stratum's script renders **translated** once any of its blocks
    /// has been **researched** (comprehended) — the decipherment spine, now funded by research
    /// rather than a standalone `decode` action.
    pub fn is_legible(&self, script: Script) -> bool {
        self.comprehended.contains(&stratum_of(script))
    }

    // ---- G15: comprehension-as-research -----------------------------------------------------

    /// G15: research cost for a target. **Block** (G15a): a base **doubling** with the gating
    /// stratum's depth — 25/50/100/200/400 (deeper = dearer, Decision 3; G19 trued the old
    /// `30 + 20·depth` line against the measured ~3.2 domain-yield/min: SCH ≈ 15 min of intake,
    /// REL ≈ 1 h, SIG ≈ 2 h — a real arc instead of a flat ramp). **Faculty** (G15b): the next
    /// level's cost from [`FACULTY_COSTS`] (capped — a maxed faculty is effectively infinite).
    /// Starters cost 0 (pre-comprehended).
    pub fn research_cost(&self, t: ResearchTarget) -> u64 {
        match t {
            ResearchTarget::Block(b) => match b.required() {
                None => 0,
                Some(s) => 25u64 << s.byte(),
            },
            ResearchTarget::Faculty(f) => {
                let lvl = self.faculties[f.idx()];
                if lvl >= MAX_FACULTY_LEVEL {
                    u64::MAX
                } else {
                    FACULTY_COSTS[lvl as usize]
                }
            }
            // G21: a sensing instrument prices like a block of its gating stratum (the same
            // deeper-is-dearer doubling): close reading (Rites) 100, deep sensing (Signals) 400.
            ResearchTarget::Sense(s) => 25u64 << s.stratum().byte(),
        }
    }

    /// G15: is this block **comprehended** (usable)? Starters always; a gated block once its
    /// research filled. (Replaces the per-stratum decode gate — no path unlocks a whole stratum.)
    pub fn is_block_comprehended(&self, b: crate::console::Block) -> bool {
        b.required().is_none() || self.comprehended_blocks.contains(&b)
    }

    /// G15: the comprehended gated blocks (for syncing the console's unlock view).
    pub fn comprehended_blocks(&self) -> impl Iterator<Item = crate::console::Block> + '_ {
        self.comprehended_blocks.iter().copied()
    }

    /// G15: set the active research target (allocate-and-fill, player-directed — Decision 1). A
    /// **block** must be discovered, gated, not-yet-comprehended; a **faculty** must be below its
    /// level cap. Returns `true` if it became active.
    pub fn allocate(&mut self, t: ResearchTarget) -> bool {
        let valid = match t {
            ResearchTarget::Block(b) => {
                b.required().is_some() && self.is_discovered(b) && !self.is_block_comprehended(b)
            }
            ResearchTarget::Faculty(f) => self.faculties[f.idx()] < MAX_FACULTY_LEVEL,
            // G21: a sensing instrument must be discovered (the frustration taught the need)
            // and not yet comprehended.
            ResearchTarget::Sense(s) => {
                self.is_sense_discovered(s) && !self.is_sense_comprehended(s)
            }
        };
        if valid {
            self.active_research = Some(t);
        }
        valid
    }

    /// G15: the active research target (the player's chosen "what next"), if any.
    pub fn active_research(&self) -> Option<ResearchTarget> {
        self.active_research
    }

    /// G15: `(filled, cost)` for a target's research bar.
    pub fn research_progress(&self, t: ResearchTarget) -> (u64, u64) {
        (
            self.research_filled.get(&t.rkey()).copied().unwrap_or(0),
            self.research_cost(t),
        )
    }

    /// G19: `(rare pickups credited, rare pickups required)` — the research bar's second gauge
    /// (rendered alongside the fill, e.g. `172/200 · r 1/4`). `(_, 0)` for targets that demand
    /// no rare evidence (shallow strata, faculties).
    pub fn research_rare_progress(&self, t: ResearchTarget) -> (u32, u32) {
        (
            self.research_rare.get(&t.rkey()).copied().unwrap_or(0),
            rare_requirement(t),
        )
    }

    /// G15: the discovered-but-not-yet-comprehended blocks — the block research targets (for the UI).
    pub fn research_targets(&self) -> impl Iterator<Item = crate::console::Block> + '_ {
        self.discovered
            .iter()
            .copied()
            .filter(|b| !self.is_block_comprehended(*b))
    }

    /// G15: credit shard `amount` to the active target; on `filled ≥ cost` **and** (G19) the
    /// target's rare-pickup gauge reaching [`rare_requirement`], a **block** comprehends (usable)
    /// and its stratum turns legible (the fold-in), a **faculty** levels up. Clears the active
    /// target (a faculty re-arms for its next level until capped). Returns the completed target.
    /// (Called from `CollectShard`: a block draws its own domain; a faculty draws any.)
    fn credit_research(&mut self, amount: u64) -> Option<ResearchTarget> {
        let t = self.active_research?;
        let key = t.rkey();
        let cost = self.research_cost(t);
        let filled = {
            let e = self.research_filled.entry(key).or_default();
            *e += amount;
            *e
        };
        let rares = self.research_rare.get(&key).copied().unwrap_or(0);
        if filled < cost || rares < rare_requirement(t) {
            return None;
        }
        match t {
            ResearchTarget::Block(b) => {
                self.comprehended_blocks.insert(b);
                if let Some(s) = b.required() {
                    self.comprehended.insert(s); // legibility fold-in
                }
            }
            ResearchTarget::Faculty(f) => {
                if self.faculties[f.idx()] < MAX_FACULTY_LEVEL {
                    self.faculties[f.idx()] += 1;
                }
            }
            // G21: a comprehended sensing instrument is vocabulary of its tier — the legibility
            // fold-in applies exactly as for a block (researching the Signals-tier instrument is
            // the first thing that can turn Galactic legible).
            ResearchTarget::Sense(s) => {
                self.senses.insert(s);
                self.comprehended.insert(s.stratum());
            }
        }
        self.research_filled.remove(&key);
        self.research_rare.remove(&key);
        // A faculty re-arms for its next level (keep feeding for levels) until capped; a block clears.
        self.active_research = match t {
            ResearchTarget::Faculty(f) if self.faculties[f.idx()] < MAX_FACULTY_LEVEL => Some(t),
            _ => None,
        };
        Some(t)
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

    // ---- G18: the uncertainty layer (attestation) ---------------------------------------------

    /// G18: how far this block's reading is trusted. `None` = not discovered yet; starters are
    /// implicitly `Confirmed` (their vocabulary was never a field hypothesis); a gated block is
    /// `Provisional` from its first name-collect until a second sighting or first use confirms it.
    pub fn attestation(&self, b: crate::console::Block) -> Option<Attestation> {
        if b.required().is_none() {
            return Some(Attestation::Confirmed);
        }
        if !self.discovered.contains(&b) {
            return None;
        }
        Some(if self.confirmed.contains(&b) {
            Attestation::Confirmed
        } else {
            Attestation::Provisional
        })
    }

    /// G18: **behavioral confirmation** — the first successful *execution* of a comprehended
    /// block confirms its reading (the world-as-oracle rule: the machine answering IS the
    /// confirmation). Called from the app's dispatch sites; idempotent. Returns `true` on the
    /// Provisional → Confirmed edge.
    pub fn confirm_block_use(&mut self, b: crate::console::Block) -> bool {
        if b.required().is_none() || !self.discovered.contains(&b) || !self.is_block_comprehended(b)
        {
            return false;
        }
        self.confirmed.insert(b)
    }

    /// G18: the confirmed gated blocks (for syncing the console's attestation view).
    pub fn confirmed_blocks(&self) -> impl Iterator<Item = crate::console::Block> + '_ {
        self.confirmed.iter().copied()
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
                // G21: the first WORN collect (lacunae banked, glyphs lost) is the frustration
                // event that discovers **close reading** — the damage teaches the need.
                if text.chars().any(|c| c == crate::text::MARK_LACUNA) {
                    self.senses_discovered.insert(Sense::CloseReading);
                }
                true
            }
            // G9: a block name recovered — first sighting discovers (a provisional reading).
            // G18: a *second* sighting of an already-discovered name confirms it (still a normal
            // collect for yield — the Collect event above banks its data either way). A starter's
            // name is never a hypothesis, so it stays a full no-op.
            Event::Discover { block } => {
                if block.required().is_none() {
                    return false;
                }
                if self.discovered.insert(*block) {
                    true // first sighting → Provisional
                } else {
                    self.confirmed.insert(*block) // second sighting → Confirmed (once)
                }
            }
            // G18: an ⟦erased⟧ inscription — log the erasure event (site, stratum, the gouge)
            // in the codex; nothing banks (its content is unrecoverable until G20).
            Event::CollectErased {
                find_id,
                script,
                text,
                pos,
            } => {
                if !self.seen.insert(*find_id) {
                    return false; // already logged
                }
                self.codex.push(CodexEntry {
                    find_id: *find_id,
                    script: *script,
                    text: text.clone(),
                    pos: *pos,
                });
                // G21: the first erased log is the frustration event that discovers **deep
                // sensing** — the gouge that yields nothing teaches what's still missing.
                self.senses_discovered.insert(Sense::DeepSensing);
                true
            }
            // G10/G15: bank a shard (lifetime tally, no dedup) AND — G15 allocate-and-fill — credit
            // it to the active research target. A **block** draws its own stratum's domain
            // (Decision 2, own-domain only); a **faculty** (G15b) is general machine instrumentation
            // and draws **any** domain. The bank is now just a displayed lifetime total.
            Event::CollectShard { domain, rarity } => {
                self.shard_bank += rarity.yield_amount();
                self.shard_counts[domain.byte() as usize][rarity.idx()] += 1;
                let credit = match self.active_research {
                    Some(ResearchTarget::Block(b)) => b.required() == Some(*domain),
                    Some(ResearchTarget::Faculty(_)) => true, // any domain funds a faculty
                    // G21: a sensing instrument draws its gating stratum's domain (like a block).
                    Some(ResearchTarget::Sense(s)) => s.stratum() == *domain,
                    None => false,
                };
                if credit {
                    // G19: a credited **rare** pickup also advances the target's rare gauge
                    // (targets with a rare requirement only — blocks and, G21, deep sensing).
                    if let (crate::shards::Rarity::Rare, Some(t)) = (*rarity, self.active_research)
                    {
                        if rare_requirement(t) > 0 {
                            *self.research_rare.entry(t.rkey()).or_default() += 1;
                        }
                    }
                    self.credit_research(rarity.yield_amount());
                }
                true
            }
            // G15b: bank-then-spend is **retired** — faculties are research targets now (allocate
            // → fill → level up, via `CollectShard`). This event is a no-op kept only so any
            // serialized old event log still applies cleanly.
            Event::Spend { .. } => false,
            // G20: count an intact frame sighting; the third teaches the frame (append-only
            // knowledge — nothing un-knows a frame).
            Event::SightFrame { frame_id } => {
                if self.frames_known.contains(frame_id) {
                    return false; // already cracked — recurrence is just recurrence now
                }
                let n = self.frame_sightings.entry(*frame_id).or_default();
                *n = n.saturating_add(1);
                if *n >= FRAME_KNOWN_SIGHTINGS {
                    self.frames_known.insert(*frame_id);
                }
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

    /// G10/G11: lifetime shard pickups across every domain (telemetry item accounting).
    pub fn shard_total_count(&self) -> u32 {
        self.shard_counts.iter().flatten().sum()
    }

    /// G10: the current faculty levels (sensing / reach / drive).
    pub fn faculty_levels(&self) -> [u8; 3] {
        self.faculties
    }

    /// G10: the live faculty multipliers (see [`faculty_mults`]).
    pub fn faculties(&self) -> FacultyMults {
        faculty_mults(self.faculties)
    }

    // ---- G17: the expedition handshake (carry → deposit → cache → ship drain) ----------------

    /// How many shards the walker is currently carrying (in transit, not yet banked).
    pub fn carry_count(&self) -> u32 {
        self.carry.iter().flatten().sum()
    }

    /// How many shards sit in the site cache (deposited, awaiting the ship).
    pub fn cache_count(&self) -> u32 {
        self.cache.iter().flatten().sum()
    }

    /// G17: the carry as a **percentage** of [`CARRY_CAP`] (for `when(carry ≥ %)`). Clamps at 100.
    pub fn carry_pct(&self) -> u32 {
        (self.carry_count() * 100 / CARRY_CAP.max(1)).min(100)
    }

    /// G17: is the walker's carry full? (A further `collect` is then honestly `blocked: carry full`.)
    pub fn carry_is_full(&self) -> bool {
        self.carry_count() >= CARRY_CAP
    }

    /// G17: the walker collects one shard into its **carry** (does **not** bank — value lands on the
    /// ship's cache drain, Decision 2). Returns `false` when the carry is full (the honest block),
    /// so the caller can report `blocked: carry full` and leave the shard in the world.
    pub fn carry_shard(&mut self, domain: Stratum, rarity: crate::shards::Rarity) -> bool {
        if self.carry_is_full() {
            return false;
        }
        self.carry[domain.byte() as usize][rarity.idx()] += 1;
        true
    }

    /// G17: `deposit` — move the walker's whole carry into the site cache (carry → cache; clears
    /// carry). Returns the number of shards moved (0 if the carry was empty — an honest no-op).
    pub fn deposit(&mut self) -> u32 {
        let mut moved = 0;
        for d in 0..5 {
            for r in 0..3 {
                self.cache[d][r] += self.carry[d][r];
                moved += self.carry[d][r];
                self.carry[d][r] = 0;
            }
        }
        moved
    }

    /// G17: drain the cache to a flat list of `(domain, rarity)` shards (ship pickup); clears the
    /// cache. The caller applies one canonical [`Event::CollectShard`] per shard, so the bank,
    /// research fill, and routine credit all flow through the existing path (D11 covers it).
    pub fn drain_cache(&mut self) -> Vec<(Stratum, crate::shards::Rarity)> {
        let mut out = Vec::new();
        for d in 0..5 {
            for r in 0..3 {
                for _ in 0..self.cache[d][r] {
                    out.push((Stratum::from_byte(d as u8), crate::shards::Rarity::ALL[r]));
                }
                self.cache[d][r] = 0;
            }
        }
        out
    }

    // ---- G21: the sensing ladder ---------------------------------------------------------------

    /// G21: has this sensing instrument been **discovered** (its frustration event fired)?
    /// Discovery lists it as a research target — the console offering the remedy.
    pub fn is_sense_discovered(&self, s: Sense) -> bool {
        self.senses_discovered.contains(&s)
    }

    /// G21: is this sensing instrument **comprehended** (the ladder rung held)? Consulted by the
    /// on-foot collect paths; the ship stays rung 0 whatever is researched.
    pub fn is_sense_comprehended(&self, s: Sense) -> bool {
        self.senses.contains(&s)
    }

    /// G21: the discovered-but-not-yet-comprehended sensing instruments (research targets, UI).
    pub fn sense_targets(&self) -> impl Iterator<Item = Sense> + '_ {
        Sense::ALL
            .into_iter()
            .filter(|s| self.is_sense_discovered(*s) && !self.is_sense_comprehended(*s))
    }

    // ---- G20: formulaic frames as cribs -------------------------------------------------------

    /// G20: is this frame **known** (its skeleton cracked — sightings reached the bar)?
    pub fn frame_known(&self, frame_id: u64) -> bool {
        self.frames_known.contains(&frame_id)
    }

    /// G20: intact sightings recorded for a frame (display; clamps at the known bar).
    pub fn frame_sightings(&self, frame_id: u64) -> u8 {
        if self.frames_known.contains(&frame_id) {
            return FRAME_KNOWN_SIGHTINGS;
        }
        self.frame_sightings.get(&frame_id).copied().unwrap_or(0)
    }

    /// Encode as a `pg=<hex>` share segment (binary blob → hex; unicode- and URL-safe). The
    /// blob carries the strata + every codex entry, so a reload restores both fully.
    pub fn encode(&self) -> String {
        let mut b = Vec::new();
        b.push(11u8); // version (…; 9 = + rare gates G19; 10 = + frames G20; 11 = + senses G21)
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
        // v6: the G15 research economy — comprehended blocks, the active target, in-progress fills.
        let mut comp_b: Vec<u8> = self.comprehended_blocks.iter().map(|x| x.code()).collect();
        comp_b.sort_unstable();
        b.push(comp_b.len() as u8);
        b.extend_from_slice(&comp_b);
        b.push(self.active_research.map(|x| x.rkey()).unwrap_or(0xFF));
        let mut fills: Vec<(u8, u64)> =
            self.research_filled.iter().map(|(&k, &v)| (k, v)).collect();
        fills.sort_unstable_by_key(|x| x.0);
        b.push(fills.len() as u8);
        for (code, amt) in fills {
            b.push(code);
            b.extend_from_slice(&amt.to_le_bytes());
        }
        // v7: the G17 expedition handshake — shards in transit (carry, then cache), 5×3 counts each.
        for store in [&self.carry, &self.cache] {
            for row in store {
                for c in row {
                    b.extend_from_slice(&c.to_le_bytes());
                }
            }
        }
        // v8: the G18 attestation — confirmed block codes (sorted for a stable encoding).
        let mut conf: Vec<u8> = self.confirmed.iter().map(|x| x.code()).collect();
        conf.sort_unstable();
        b.push(conf.len() as u8);
        b.extend_from_slice(&conf);
        // v9: the G19 rare-gate gauges — per-target rare-pickup counts (sorted for stability).
        let mut rares: Vec<(u8, u32)> = self.research_rare.iter().map(|(&k, &v)| (k, v)).collect();
        rares.sort_unstable_by_key(|x| x.0);
        b.push(rares.len() as u8);
        for (code, n) in rares {
            b.push(code);
            b.extend_from_slice(&n.to_le_bytes());
        }
        // v10: the G20 frame cribs — per-frame sighting counts, then the known set (sorted for
        // a stable encoding; append-only — old payloads load with no frames known).
        let mut sightings: Vec<(u64, u8)> =
            self.frame_sightings.iter().map(|(&k, &v)| (k, v)).collect();
        sightings.sort_unstable_by_key(|x| x.0);
        b.push(sightings.len() as u8);
        for (id, n) in sightings {
            b.extend_from_slice(&id.to_le_bytes());
            b.push(n);
        }
        let mut knowns: Vec<u64> = self.frames_known.iter().copied().collect();
        knowns.sort_unstable();
        b.push(knowns.len() as u8);
        for id in knowns {
            b.extend_from_slice(&id.to_le_bytes());
        }
        // v11: the G21 sensing ladder — discovered senses, then comprehended (sorted idx bytes;
        // append-only — old payloads load with both undiscovered).
        for set in [&self.senses_discovered, &self.senses] {
            let mut xs: Vec<u8> = set.iter().map(|s| s.idx() as u8).collect();
            xs.sort_unstable();
            b.push(xs.len() as u8);
            b.extend_from_slice(&xs);
        }
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
    if !(1..=11).contains(&version) {
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
    // v6: the G15 research economy (absent pre-v6 → migration default: nothing researched
    // in-progress; starters stay implicitly comprehended). Unknown codes skipped (lenient).
    let mut comprehended_blocks = std::collections::HashSet::new();
    let mut active_research = None;
    let mut research_filled = std::collections::HashMap::new();
    if version >= 6 {
        let cb = *take(&mut p, 1)?.first()?;
        for _ in 0..cb {
            if let Some(blk) = crate::console::Block::from_code(*take(&mut p, 1)?.first()?) {
                comprehended_blocks.insert(blk);
            }
        }
        let act = *take(&mut p, 1)?.first()?;
        if act != 0xFF {
            active_research = ResearchTarget::from_rkey(act).filter(|t| match t {
                ResearchTarget::Block(b) => b.required().is_some(),
                ResearchTarget::Faculty(_) => true,
                ResearchTarget::Sense(_) => true,
            });
        }
        let fc = *take(&mut p, 1)?.first()?;
        for _ in 0..fc {
            let code = *take(&mut p, 1)?.first()?;
            let amt = u64at(&mut p)?;
            research_filled.insert(code, amt);
        }
    }
    // v7: the G17 carry + cache stores (absent pre-v7 → empty; the handshake just starts fresh).
    let mut carry = [[0u32; 3]; 5];
    let mut cache = [[0u32; 3]; 5];
    if version >= 7 {
        for store in [&mut carry, &mut cache] {
            for row in store.iter_mut() {
                for c in row.iter_mut() {
                    *c = u32::from_le_bytes(take(&mut p, 4)?.try_into().ok()?);
                }
            }
        }
    }
    // v8: the G18 attestation (confirmed block codes; unknown codes skipped — lenient). Pre-v8
    // migration: a comprehended block loads **Confirmed** (its use already answered), a
    // merely-discovered one **Provisional** — i.e. confirmed = discovered ∩ comprehended.
    let mut confirmed = std::collections::HashSet::new();
    if version >= 8 {
        let cc = *take(&mut p, 1)?.first()?;
        for _ in 0..cc {
            if let Some(blk) = crate::console::Block::from_code(*take(&mut p, 1)?.first()?) {
                confirmed.insert(blk);
            }
        }
    } else {
        confirmed = discovered
            .intersection(&comprehended_blocks)
            .copied()
            .collect();
    }
    // v9: the G19 rare-gate gauges (absent pre-v9 → 0 — an in-progress deep research migrates
    // owing its **full** rare requirement, up to 4 rares for Relics; accepted + documented).
    let mut research_rare = std::collections::HashMap::new();
    if version >= 9 {
        let rc = *take(&mut p, 1)?.first()?;
        for _ in 0..rc {
            let code = *take(&mut p, 1)?.first()?;
            let n = u32::from_le_bytes(take(&mut p, 4)?.try_into().ok()?);
            research_rare.insert(code, n);
        }
    }
    // v10: the G20 frame cribs (absent pre-v10 → nothing known; the corpus re-teaches).
    let mut frame_sightings = std::collections::HashMap::new();
    let mut frames_known = std::collections::HashSet::new();
    if version >= 10 {
        let sc = *take(&mut p, 1)?.first()?;
        for _ in 0..sc {
            let id = u64at(&mut p)?;
            let n = *take(&mut p, 1)?.first()?;
            frame_sightings.insert(id, n);
        }
        let kc = *take(&mut p, 1)?.first()?;
        for _ in 0..kc {
            frames_known.insert(u64at(&mut p)?);
        }
    }
    // v11: the G21 sensing ladder (absent pre-v11 → undiscovered; the frustration events
    // re-teach on the next worn collect / erased log). Unknown idx bytes skipped (lenient).
    let mut senses_discovered = std::collections::HashSet::new();
    let mut senses = std::collections::HashSet::new();
    if version >= 11 {
        for set in [&mut senses_discovered, &mut senses] {
            let n = *take(&mut p, 1)?.first()?;
            for _ in 0..n {
                let idx = *take(&mut p, 1)?.first()? as usize;
                if let Some(s) = Sense::ALL.get(idx) {
                    set.insert(*s);
                }
            }
        }
    }
    Some(Progress {
        strata,
        codex,
        seen,
        scanned,
        comprehended,
        discovered,
        confirmed,
        shard_bank,
        shard_counts,
        faculties,
        comprehended_blocks,
        active_research,
        research_filled,
        research_rare,
        carry,
        cache,
        frame_sightings,
        frames_known,
        senses_discovered,
        senses,
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
            && self.confirmed == other.confirmed
            && self.shard_bank == other.shard_bank
            && self.shard_counts == other.shard_counts
            && self.faculties == other.faculties
            && self.comprehended_blocks == other.comprehended_blocks
            && self.active_research == other.active_research
            && self.research_filled == other.research_filled
            && self.research_rare == other.research_rare
            && self.carry == other.carry
            && self.cache == other.cache
            && self.frame_sightings == other.frame_sightings
            && self.frames_known == other.frames_known
            && self.senses_discovered == other.senses_discovered
            && self.senses == other.senses
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

    /// G20: three intact sightings teach a frame; the state rides `pg=` v10; and a pre-v10
    /// payload (the same blob with the frame section stripped, re-versioned v9) loads with no
    /// frames known — append-only migration.
    #[test]
    fn frame_sightings_teach_at_three_and_ride_pg_v10() {
        let mut p = Progress::default();
        let id = 0xF00D_F00Du64;
        assert!(!p.frame_known(id));
        assert!(p.apply(&Event::SightFrame { frame_id: id }));
        assert!(p.apply(&Event::SightFrame { frame_id: id }));
        assert_eq!(p.frame_sightings(id), 2);
        assert!(!p.frame_known(id), "two sightings are not enough");
        assert!(p.apply(&Event::SightFrame { frame_id: id }));
        assert!(p.frame_known(id), "the third sighting cracks the frame");
        assert_eq!(p.frame_sightings(id), FRAME_KNOWN_SIGHTINGS);
        // Post-known sightings are no-ops (recurrence is just recurrence now).
        assert!(!p.apply(&Event::SightFrame { frame_id: id }));
        // A different frame tracks independently.
        assert!(p.apply(&Event::SightFrame { frame_id: 7 }));
        assert!(!p.frame_known(7));
        // v10 round-trip.
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p);
        assert!(back.frame_known(id));
        assert_eq!(back.frame_sightings(7), 1);
        // Pre-v10 migration: strip the appended frame section (two trailing zero-count bytes on
        // a fresh blob) + stamp v9 → decodes, nothing known.
        let fresh = Progress::default().encode();
        let hex = fresh.strip_prefix("pg=").unwrap();
        let mut blob = super::from_hex(hex).unwrap();
        assert_eq!(&blob[blob.len() - 2..], &[0, 0], "empty frame section");
        blob.truncate(blob.len() - 2);
        blob[0] = 9;
        let v9 = Progress::decode(&format!("pg={}", super::to_hex(&blob)));
        assert!(!v9.frame_known(id));
        assert_eq!(v9, Progress::default());
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

    /// G15: comprehension comes from **research** now (decode removed); comprehending a block via
    /// its domain shards makes that stratum legible, and the comprehension round-trips through
    /// `pg=`. (Replaces the old decode-spend tests.)
    #[test]
    fn research_comprehension_and_legibility_round_trip() {
        use crate::console::Block;
        use crate::shards::Rarity;
        let mut p = Progress::default();
        p.apply(&Event::Discover {
            block: Block::RunFoot,
        }); // Relics-gated (Runic)
        assert!(!p.is_legible(Script::Runic));
        p.allocate(ResearchTarget::Block(Block::RunFoot));
        let mut guard = 0;
        while !p.is_block_comprehended(Block::RunFoot) && guard < 10_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Relics,
                rarity: Rarity::Rare,
            });
            guard += 1;
        }
        assert!(p.is_block_comprehended(Block::RunFoot));
        assert!(
            p.is_legible(Script::Runic),
            "comprehending a Relics block → Runic legible"
        );
        assert!(!p.is_legible(Script::Greek), "untouched strata stay glyphs");
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p, "research comprehension round-trips");
        assert!(back.is_legible(Script::Runic));
    }

    #[test]
    fn discover_applies_idempotently_and_starters_are_implicit() {
        use crate::console::Block;
        let mut p = Progress::default();
        // Starters are always discovered (the opening is untouched); gated blocks aren't.
        assert!(p.is_discovered(Block::Collect));
        assert!(!p.is_discovered(Block::Seek));
        // Discovering a gated block applies once (→ a provisional reading, G18)…
        assert!(p.apply(&Event::Discover { block: Block::Seek }));
        assert!(p.is_discovered(Block::Seek));
        assert_eq!(p.attestation(Block::Seek), Some(Attestation::Provisional));
        // …the second sighting applies once more (→ confirms the reading)…
        assert!(p.apply(&Event::Discover { block: Block::Seek }));
        assert_eq!(p.attestation(Block::Seek), Some(Attestation::Confirmed));
        // …and any further sighting is a plain no-op (a normal re-collect).
        assert!(!p.apply(&Event::Discover { block: Block::Seek }));
        // Discovering a starter is always a no-op (already known, never a hypothesis).
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
        assert_eq!(p.shard_bank(), 15); // lifetime tally (no longer spent — G15b)
        assert_eq!(p.shard_count(Stratum::Records), 3);
        assert_eq!(p.shard_count(Stratum::Signals), 1);
        // G15b: faculties are research now — allocate Sensing, fill it from any-domain shards →
        // level 1 (bank-then-spend retired; the bank is just a displayed total).
        assert!(p.allocate(ResearchTarget::Faculty(Faculty::Sensing)));
        let mut guard = 0;
        while p.faculty_levels()[0] == 0 && guard < 10_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Rites,
                rarity: Rarity::Rare,
            });
            guard += 1;
        }
        assert_eq!(p.faculty_levels(), [1, 0, 0]);
        // Round-trips through pg= v6; old v4/v5-style payloads still load (zeroed economy).
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p);
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
        // G15b: research the Drive faculty to its cap via shard intake; it stops at MAX.
        let mut p = Progress::default();
        p.allocate(ResearchTarget::Faculty(Faculty::Drive));
        let mut guard = 0;
        while p.faculty_levels()[2] < MAX_FACULTY_LEVEL && guard < 100_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Records,
                rarity: crate::shards::Rarity::Rare,
            });
            guard += 1;
        }
        assert_eq!(p.faculty_levels()[2], MAX_FACULTY_LEVEL, "level caps at 3");
        assert_eq!(
            p.active_research(),
            None,
            "a maxed faculty clears the active target"
        );
    }

    #[test]
    fn decode_is_lenient() {
        assert_eq!(Progress::decode(""), Progress::default());
        assert_eq!(Progress::decode("s=1&x=2"), Progress::default()); // no pg=
        assert_eq!(Progress::decode("pg=zzzz"), Progress::default()); // bad hex
        assert_eq!(Progress::decode("pg=00"), Progress::default()); // bad version
    }

    /// G15: allocate a discovered gated block → its **domain** shards fill its research → it
    /// comprehends (becomes usable); off-domain shards don't fill it (Decision 2). Starters are
    /// comprehended from the start (opening parity).
    #[test]
    fn research_fills_from_domain_shards_and_comprehends() {
        use crate::console::Block;
        use crate::shards::Rarity;
        let mut p = Progress::default();
        // Opening parity: a starter is usable from the start; a gated block is not.
        assert!(p.is_block_comprehended(Block::Collect)); // starter (required None)
        let target = Block::Seek; // gated on Schematics
        assert!(!p.is_block_comprehended(target));
        p.apply(&Event::Discover { block: target });
        assert!(
            p.allocate(ResearchTarget::Block(target)),
            "a discovered gated block is a valid research target"
        );
        let cost = p.research_cost(ResearchTarget::Block(target));
        assert!(cost > 0);
        // Off-domain shards (Records) don't fill a Schematics target.
        for _ in 0..50 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Records,
                rarity: Rarity::Rare,
            });
        }
        assert_eq!(
            p.research_progress(ResearchTarget::Block(target)).0,
            0,
            "off-domain shards don't fill (Decision 2)"
        );
        // Domain-matched shards fill it; enough → comprehended + active target cleared.
        let mut guard = 0;
        while !p.is_block_comprehended(target) && guard < 10_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Schematics,
                rarity: Rarity::Common,
            });
            guard += 1;
        }
        assert!(
            p.is_block_comprehended(target),
            "domain shards filled research → comprehended"
        );
        assert_eq!(
            p.active_research(),
            None,
            "active target cleared on completion"
        );
        // Legibility folded in: comprehending a Schematics block makes Greek legible.
        assert!(p.is_legible(Script::Greek));
    }

    /// G15: research state (active target + partial fill + comprehended set) round-trips through
    /// `pg=` v6; old (pre-v6) payloads load with the migration default (no research).
    #[test]
    fn research_state_round_trips_and_old_payloads_migrate() {
        use crate::console::Block;
        use crate::shards::Rarity;
        let mut p = Progress::default();
        p.apply(&Event::Discover { block: Block::Seek });
        p.allocate(ResearchTarget::Block(Block::Seek));
        p.apply(&Event::CollectShard {
            domain: Stratum::Schematics,
            rarity: Rarity::Common,
        });
        assert!(
            p.research_progress(ResearchTarget::Block(Block::Seek)).0 > 0,
            "partial fill recorded"
        );
        assert_eq!(
            p.active_research(),
            Some(ResearchTarget::Block(Block::Seek))
        );
        let back = Progress::decode(&p.encode());
        assert_eq!(back, p, "v6 research state round-trips");
        // A fresh (default) progress has no research — the migration default old payloads load to.
        let fresh = Progress::default();
        assert_eq!(fresh.active_research(), None);
        assert!(!fresh.is_block_comprehended(Block::Seek));
    }

    /// G19: the rare gate — a Relics-gated target demands **4 rare-tier own-domain pickups** on
    /// top of its fill (Signals 8; shallow strata + faculties 0); commons alone can overfill the
    /// bar without completing it; off-domain rares don't count; the gauge round-trips through
    /// `pg=` v9 and a pre-v9 payload migrates to a zero gauge (owing its full requirement).
    #[test]
    fn rare_gate_holds_deep_research_until_rare_pickups() {
        use crate::console::Block;
        use crate::shards::Rarity;
        let t = ResearchTarget::Block(Block::RunFoot); // Relics-gated
        assert_eq!(rare_requirement(t), 4);
        assert_eq!(rare_requirement(ResearchTarget::Block(Block::Seek)), 0);
        assert_eq!(
            rare_requirement(ResearchTarget::Faculty(Faculty::Sensing)),
            0,
            "faculties are general instrumentation — no rare evidence demanded"
        );
        let mut p = Progress::default();
        p.apply(&Event::Discover {
            block: Block::RunFoot,
        });
        assert!(p.allocate(t));
        assert_eq!(p.research_rare_progress(t), (0, 4));
        // Overfill with commons: fill ≥ cost, zero rares → must NOT complete (the gate).
        let cost = p.research_cost(t);
        for _ in 0..(cost * 2) {
            p.apply(&Event::CollectShard {
                domain: Stratum::Relics,
                rarity: Rarity::Common,
            });
        }
        assert!(p.research_progress(t).0 >= cost, "bar overfilled");
        assert!(
            !p.is_block_comprehended(Block::RunFoot),
            "filled ≥ cost alone does not complete a Relics target"
        );
        // Three rares: still one short.
        for _ in 0..3 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Relics,
                rarity: Rarity::Rare,
            });
        }
        assert_eq!(p.research_rare_progress(t).0, 3);
        assert!(!p.is_block_comprehended(Block::RunFoot));
        // The in-progress gauge round-trips through pg= v9…
        let back = Progress::decode(&p.encode());
        assert_eq!(back, p, "v9 rare gauge round-trips");
        assert_eq!(back.research_rare_progress(t).0, 3);
        // …and a pre-v9 (v8) payload migrates append-only: the gauge loads zeroed, so the
        // in-progress deep research owes its full rare requirement again (accepted + noted).
        let v8_hex = {
            let hex = p.encode();
            let hex = hex.strip_prefix("pg=").unwrap();
            let mut bytes = super::from_hex(hex).unwrap();
            bytes.truncate(bytes.len() - 1 - 5); // the v9 tail: count byte + one (key, u32) entry
            bytes[0] = 8;
            super::to_hex(&bytes)
        };
        let v8 = Progress::decode(&format!("pg={v8_hex}"));
        assert_eq!(v8.research_rare_progress(t).0, 0, "pre-v9 → zero gauge");
        assert_eq!(v8.research_progress(t).0, p.research_progress(t).0);
        // An off-domain rare doesn't count (the same own-domain rule as the fill)…
        p.apply(&Event::CollectShard {
            domain: Stratum::Records,
            rarity: Rarity::Rare,
        });
        assert_eq!(p.research_rare_progress(t).0, 3);
        // …the 4th own-domain rare satisfies the gate and (the bar already full) completes.
        p.apply(&Event::CollectShard {
            domain: Stratum::Relics,
            rarity: Rarity::Rare,
        });
        assert!(p.is_block_comprehended(Block::RunFoot));
        assert_eq!(
            p.research_rare_progress(t).0,
            0,
            "the gauge clears with the fill on completion"
        );
    }

    /// G15b: a **faculty** is an ordinary research target — allocate it, **any-domain** shards fill
    /// it (faculties are general instrumentation), it levels up on fill (the tested multiplier
    /// applies), re-arms for the next level, and stops at the cap. Round-trips through `pg=`.
    #[test]
    fn faculty_research_levels_up_from_any_domain_shards() {
        use crate::shards::Rarity;
        let mut p = Progress::default();
        let f = Faculty::Sensing;
        assert_eq!(p.faculty_levels()[f.idx()], 0);
        assert!(p.allocate(ResearchTarget::Faculty(f)));
        // Off-domain (Records) shards still fund a faculty (any domain) — unlike a block.
        let mut guard = 0;
        while p.faculty_levels()[f.idx()] == 0 && guard < 10_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Records,
                rarity: Rarity::Common,
            });
            guard += 1;
        }
        assert_eq!(
            p.faculty_levels()[f.idx()],
            1,
            "any-domain shards leveled the faculty"
        );
        assert!(
            p.faculties().sensing > 1.0,
            "the multiplier applies at level 1"
        );
        // It re-arms for the next level (keep feeding for levels).
        assert_eq!(p.active_research(), Some(ResearchTarget::Faculty(f)));
        // Drive it to the cap; the active target then clears (nothing left to research).
        let mut guard = 0;
        while p.faculty_levels()[f.idx()] < MAX_FACULTY_LEVEL && guard < 100_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Signals,
                rarity: Rarity::Rare,
            });
            guard += 1;
        }
        assert_eq!(p.faculty_levels()[f.idx()], MAX_FACULTY_LEVEL);
        assert_eq!(
            p.active_research(),
            None,
            "a maxed faculty clears the active target"
        );
        // The faculty research state round-trips.
        let mut p2 = Progress::default();
        p2.allocate(ResearchTarget::Faculty(Faculty::Reach));
        p2.apply(&Event::CollectShard {
            domain: Stratum::Rites,
            rarity: Rarity::Common,
        });
        let back = Progress::decode(&p2.encode());
        assert_eq!(back, p2, "faculty research round-trips through pg=");
    }

    /// G17: the walker carries shards (capped), `deposit` moves carry → cache, the ship drain
    /// returns the cached shards for banking. Value is held *in transit* (not banked) until drain.
    #[test]
    fn carry_deposit_cache_and_drain() {
        use crate::shards::Rarity;
        let mut p = Progress::default();
        assert_eq!(p.carry_count(), 0);
        assert!(!p.carry_is_full());
        // Carry up to the cap; the cap then blocks further carry (the honest `carry full`).
        for i in 0..CARRY_CAP {
            assert!(p.carry_shard(Stratum::Relics, Rarity::Common), "carry #{i}");
        }
        assert!(p.carry_is_full());
        assert_eq!(p.carry_pct(), 100);
        assert!(
            !p.carry_shard(Stratum::Relics, Rarity::Common),
            "a full carry blocks further collect"
        );
        // Carry is in transit — nothing banked yet (Decision 2: value lands on ship pickup).
        assert_eq!(p.shard_bank(), 0);
        assert_eq!(p.shard_total_count(), 0);
        // Deposit moves the whole carry into the cache; carry empties.
        assert_eq!(p.deposit(), CARRY_CAP);
        assert_eq!(p.carry_count(), 0);
        assert_eq!(p.cache_count(), CARRY_CAP);
        assert_eq!(p.deposit(), 0, "an empty carry deposits nothing");
        // The ship drains the cache → canonical CollectShard events bank + count it.
        let drained = p.drain_cache();
        assert_eq!(drained.len() as u32, CARRY_CAP);
        assert_eq!(p.cache_count(), 0);
        for (domain, rarity) in drained {
            p.apply(&Event::CollectShard { domain, rarity });
        }
        assert_eq!(
            p.shard_bank(),
            CARRY_CAP as u64 * Rarity::Common.yield_amount()
        );
        assert_eq!(p.shard_count(Stratum::Relics), CARRY_CAP);
    }

    /// G17: carry + cache survive the `pg=` v7 round-trip; pre-v7 payloads load with empty stores.
    #[test]
    fn handshake_round_trips_v7_and_old_payloads_load_empty() {
        use crate::shards::Rarity;
        let mut p = Progress::default();
        p.carry_shard(Stratum::Records, Rarity::Common);
        p.carry_shard(Stratum::Signals, Rarity::Rare);
        p.deposit(); // → cache
        p.carry_shard(Stratum::Schematics, Rarity::Uncommon); // some still in carry
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p, "carry + cache round-trip through pg= v7");
        assert_eq!(back.carry_count(), 1);
        assert_eq!(back.cache_count(), 2);
        // A pre-v7 (v6) blob still loads — the handshake stores just come back empty.
        let mut q = Progress::default();
        q.apply(&Event::CollectShard {
            domain: Stratum::Records,
            rarity: Rarity::Common,
        });
        let v6_hex = {
            let mut b = q.encode().strip_prefix("pg=").unwrap().to_string();
            // Rebuild as a v6 blob by truncating the v9 rare tail (1 byte, empty) + the v8
            // confirmed tail (1 byte, empty) + the v7 carry/cache tail (120 bytes) = 244 hex.
            b.truncate(b.len() - 244);
            let mut bytes = super::from_hex(&b).unwrap();
            bytes[0] = 6; // stamp version 6
            super::to_hex(&bytes)
        };
        let v6 = Progress::decode(&format!("pg={v6_hex}"));
        assert_eq!(v6.carry_count(), 0);
        assert_eq!(v6.cache_count(), 0);
        assert_eq!(
            v6.shard_count(Stratum::Records),
            1,
            "v6 economy still loads"
        );
    }

    /// G18: attestation transitions — provisional on first sighting; confirmed by a second
    /// sighting OR by first use after comprehension (never by comprehension alone); idempotent;
    /// starters implicitly confirmed; **no mechanical gate** on provisional (Decision 1).
    #[test]
    fn attestation_both_confirmation_paths_and_no_gate() {
        use crate::console::Block;
        use crate::shards::Rarity;
        let mut p = Progress::default();
        // Starters were never hypotheses; undiscovered gated blocks have no reading at all.
        assert_eq!(p.attestation(Block::Collect), Some(Attestation::Confirmed));
        assert_eq!(p.attestation(Block::Seek), None);
        assert!(!p.confirm_block_use(Block::Collect), "starters no-op");
        assert!(
            !p.confirm_block_use(Block::Seek),
            "undiscovered can't attest"
        );
        // Path (a): second sighting. First collect → Provisional…
        p.apply(&Event::Discover { block: Block::Seek });
        assert_eq!(p.attestation(Block::Seek), Some(Attestation::Provisional));
        // …and NO gate: a provisional block allocates as a research target (no softlock).
        assert!(
            p.allocate(ResearchTarget::Block(Block::Seek)),
            "provisional blocks research freely (Decision 1)"
        );
        p.apply(&Event::Discover { block: Block::Seek });
        assert_eq!(p.attestation(Block::Seek), Some(Attestation::Confirmed));
        // Path (b): behavioral. Discover + comprehend another block; comprehension alone does
        // NOT confirm — the first *use* does (the machine answering is the confirmation).
        let b = Block::RunFoot;
        p.apply(&Event::Discover { block: b });
        assert!(
            !p.confirm_block_use(b),
            "uncomprehended use attests nothing"
        );
        p.allocate(ResearchTarget::Block(b));
        let mut guard = 0;
        while !p.is_block_comprehended(b) && guard < 10_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Relics,
                rarity: Rarity::Rare,
            });
            guard += 1;
        }
        assert_eq!(
            p.attestation(b),
            Some(Attestation::Provisional),
            "comprehension alone leaves the reading provisional"
        );
        assert!(p.confirm_block_use(b), "first use confirms");
        assert_eq!(p.attestation(b), Some(Attestation::Confirmed));
        assert!(!p.confirm_block_use(b), "idempotent (edge, not level)");
    }

    /// G18: attestation round-trips through `pg=` v8; a pre-v8 payload migrates append-only —
    /// comprehended → Confirmed, merely-discovered → Provisional.
    #[test]
    fn attestation_round_trips_v8_and_v7_payloads_migrate() {
        use crate::console::Block;
        use crate::shards::Rarity;
        let mut p = Progress::default();
        p.apply(&Event::Discover { block: Block::Seek }); // provisional
        p.apply(&Event::Discover {
            block: Block::Circle,
        });
        p.apply(&Event::Discover {
            block: Block::Circle,
        }); // confirmed by sighting
        p.apply(&Event::Discover {
            block: Block::RunFoot,
        });
        p.allocate(ResearchTarget::Block(Block::RunFoot));
        let mut guard = 0;
        while !p.is_block_comprehended(Block::RunFoot) && guard < 10_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Relics,
                rarity: Rarity::Rare,
            });
            guard += 1;
        }
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p, "v8 attestation round-trips");
        assert_eq!(
            back.attestation(Block::Seek),
            Some(Attestation::Provisional)
        );
        assert_eq!(
            back.attestation(Block::Circle),
            Some(Attestation::Confirmed)
        );
        // Rebuild the same payload as v7 (strip the appended confirmed tail, restamp): the
        // migration confirms the comprehended block, leaves the merely-discovered provisional.
        let v7_hex = {
            let hex = p.encode();
            let hex = hex.strip_prefix("pg=").unwrap();
            let mut bytes = super::from_hex(hex).unwrap();
            let conf = p.confirmed.len();
            // Strip the v9 rare tail (count byte; empty — RunFoot's gauge cleared on
            // completion) then the v8 tail (count byte + codes).
            assert!(p.research_rare.is_empty());
            bytes.truncate(bytes.len() - 1 - 1 - conf);
            bytes[0] = 7;
            super::to_hex(&bytes)
        };
        let v7 = Progress::decode(&format!("pg={v7_hex}"));
        assert_eq!(
            v7.attestation(Block::RunFoot),
            Some(Attestation::Confirmed),
            "pre-v8 comprehended → Confirmed"
        );
        assert_eq!(
            v7.attestation(Block::Seek),
            Some(Attestation::Provisional),
            "pre-v8 merely-discovered → Provisional"
        );
    }

    /// G18: the damage marks carry no data — a worn text yields only its surviving glyphs; the
    /// erased-collect event logs the gouge in the codex without banking anything.
    #[test]
    fn worn_yields_less_and_erased_logs_without_yield() {
        let full = "ΑΒΓΔΕΖ";
        let worn: String = format!(
            "ΑΒ{}Δ{}Ζ",
            crate::text::MARK_LACUNA,
            crate::text::MARK_LACUNA
        );
        assert_eq!(glyph_count(full), 6);
        assert_eq!(glyph_count(&worn), 4, "lacunae don't count");
        assert!(
            yield_amount(Script::Greek, glyph_count(&worn))
                < yield_amount(Script::Greek, glyph_count(full)),
            "worn yield drops with the lost glyphs"
        );
        // Erased: the event dedups, logs a codex entry, banks nothing.
        let mut p = Progress::default();
        let gouge: String = std::iter::repeat_n(crate::text::MARK_GOUGE, 3).collect();
        assert_eq!(glyph_count(&gouge), 0);
        let ev = Event::CollectErased {
            find_id: 99,
            script: Script::Runic,
            text: gouge,
            pos: [1.0, 2.0, 3.0],
        };
        assert!(p.apply(&ev));
        assert_eq!(p.strata.total(), 0, "an erasure banks nothing");
        assert_eq!(p.collected_count(), 1, "…but the event is logged");
        assert!(p.has(99), "the site is spent");
        assert!(!p.apply(&ev), "logging dedups like any collect");
        // The logged erasure survives the share round-trip.
        let back = Progress::decode(&p.encode());
        assert_eq!(back, p);
        assert!(crate::structures::is_erased_text(&back.codex[0].text));
    }

    /// G21: the sensing instruments are ordinary research targets — Rites/Signals-gated,
    /// domain-matched fill, deep-sensing behind the standing 8-rare gate — and only allocatable
    /// once **discovered** (the frustration events).
    #[test]
    fn sense_targets_gate_cost_rare_and_fill() {
        use crate::shards::Rarity;
        // Costs price like blocks of their gating stratum; deep sensing carries the 8-rare gate.
        let p0 = Progress::default();
        let cr = ResearchTarget::Sense(Sense::CloseReading);
        let ds = ResearchTarget::Sense(Sense::DeepSensing);
        assert_eq!(p0.research_cost(cr), 100, "Rites-priced (25 << 2)");
        assert_eq!(p0.research_cost(ds), 400, "Signals-priced (25 << 4)");
        assert_eq!(rare_requirement(cr), 0, "close reading demands no rares");
        assert_eq!(
            rare_requirement(ds),
            8,
            "deep sensing is the Signals rare gate's first natural object"
        );
        // Undiscovered → not allocatable (the need must be felt first).
        let mut p = Progress::default();
        assert!(!p.allocate(cr), "undiscovered senses can't be allocated");
        // First WORN collect (banked lacunae) discovers close reading.
        let worn = format!("AB{}D", crate::text::MARK_LACUNA);
        p.apply(&Event::Collect {
            find_id: 1,
            script: Script::Latin,
            text: worn,
            pos: [0.0; 3],
        });
        assert!(p.is_sense_discovered(Sense::CloseReading));
        assert!(
            !p.is_sense_discovered(Sense::DeepSensing),
            "an erased log, not a worn collect, teaches deep sensing"
        );
        assert_eq!(p.sense_targets().collect::<Vec<_>>(), [Sense::CloseReading]);
        // Allocate + fill: own-domain (Rites) shards only.
        assert!(p.allocate(cr));
        for _ in 0..50 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Records,
                rarity: Rarity::Rare,
            });
        }
        assert_eq!(p.research_progress(cr).0, 0, "off-domain doesn't fill");
        let mut guard = 0;
        while !p.is_sense_comprehended(Sense::CloseReading) && guard < 10_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Rites,
                rarity: Rarity::Common,
            });
            guard += 1;
        }
        assert!(p.is_sense_comprehended(Sense::CloseReading));
        assert_eq!(p.active_research(), None, "completion clears the target");
        assert!(
            p.is_comprehended(Stratum::Rites),
            "the instrument folds in its tier's legibility (like a block)"
        );
        assert!(!p.allocate(cr), "a held rung can't re-allocate");
        // First ERASED log discovers deep sensing; the 8-rare Signals gate then holds it.
        p.apply(&Event::CollectErased {
            find_id: 2,
            script: Script::Runic,
            text: std::iter::repeat_n(crate::text::MARK_GOUGE, 3).collect(),
            pos: [0.0; 3],
        });
        assert!(p.is_sense_discovered(Sense::DeepSensing));
        assert!(p.allocate(ds));
        for _ in 0..800 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Signals,
                rarity: Rarity::Common,
            });
        }
        assert!(
            !p.is_sense_comprehended(Sense::DeepSensing),
            "an overfilled bar without 8 own-domain rares must not complete"
        );
        for _ in 0..8 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Signals,
                rarity: Rarity::Rare,
            });
        }
        assert!(
            p.is_sense_comprehended(Sense::DeepSensing),
            "8 Signals rares satisfy the gate"
        );
        assert!(p.is_comprehended(Stratum::Signals), "Signals turns legible");
    }

    /// G21: the sensing ladder rides `pg=` **v11** (discovered + comprehended + an in-progress
    /// fill under the sense rkey); a pre-v11 payload loads with everything undiscovered
    /// (append-only migration — the frustration events re-teach).
    #[test]
    fn senses_ride_pg_v11_and_old_payloads_load_undiscovered() {
        use crate::shards::Rarity;
        let mut p = Progress::default();
        p.apply(&Event::Collect {
            find_id: 1,
            script: Script::Greek,
            text: format!("Α{}Γ", crate::text::MARK_LACUNA),
            pos: [0.0; 3],
        });
        p.apply(&Event::CollectErased {
            find_id: 2,
            script: Script::Runic,
            text: std::iter::repeat_n(crate::text::MARK_GOUGE, 3).collect(),
            pos: [0.0; 3],
        });
        assert!(p.allocate(ResearchTarget::Sense(Sense::CloseReading)));
        p.apply(&Event::CollectShard {
            domain: Stratum::Rites,
            rarity: Rarity::Common,
        });
        let back = Progress::decode(&format!("s=1&{}", p.encode()));
        assert_eq!(back, p, "v11 round-trips the sensing ladder");
        assert!(back.is_sense_discovered(Sense::DeepSensing));
        assert_eq!(
            back.active_research(),
            Some(ResearchTarget::Sense(Sense::CloseReading)),
            "an in-progress sense research round-trips (rkey 0xE0)"
        );
        assert!(
            back.research_progress(ResearchTarget::Sense(Sense::CloseReading))
                .0
                > 0
        );
        // Pre-v11 migration: strip the two appended sense sections + restamp v10 → undiscovered.
        let v10_hex = {
            let hex = p.encode();
            let hex = hex.strip_prefix("pg=").unwrap();
            let mut bytes = super::from_hex(hex).unwrap();
            // Tail: [discovered: count + 2 idx][comprehended: count(0)] = 3 + 1 bytes.
            bytes.truncate(bytes.len() - 4);
            bytes[0] = 10;
            super::to_hex(&bytes)
        };
        let v10 = Progress::decode(&format!("pg={v10_hex}"));
        assert!(
            !v10.is_sense_discovered(Sense::CloseReading)
                && !v10.is_sense_discovered(Sense::DeepSensing),
            "pre-v11 → undiscovered"
        );
        assert_eq!(v10.collected_count(), p.collected_count(), "codex intact");
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
