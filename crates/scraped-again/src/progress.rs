//! Scraped Again — the **economy spine** (G1): five typed **data strata**, the **codex**
//! of finds, and a serializable **collect event** path (mirroring `edit::Edit`/`apply`).
//! Pure game logic over the engine's `Script`; no rendering. Save/restore rides the same
//! `k=v` share string as the camera view (its own `pg=` key, ignored by `ShareState`).
//!
//! Glyphs in the world (E17 inscriptions) belong to one of five writing systems; we map
//! **script → stratum** (game-mechanics §5), so the five scripts become five currencies.

use brickmap::text::Script;

/// The five data strata — typed currencies the tech tree will branch on (game-mechanics §5).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stratum {
    Records,    // Latin — mundane labels (common)
    Schematics, // Greek — engineering marks
    Rites,      // Hiragana — ritual inscriptions
    Relics,     // Runic — grave-marks (rare, deep lore)
    Signals,    // Galactic — alien signal (rarest)
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
}

/// All run progress: the banked strata + the codex of finds (with a de-dup set).
#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub strata: Strata,
    pub codex: Vec<CodexEntry>,
    seen: std::collections::HashSet<u64>,
}

impl Progress {
    pub fn has(&self, find_id: u64) -> bool {
        self.seen.contains(&find_id)
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
        }
    }

    /// Encode as a `pg=<hex>` share segment (binary blob → hex; unicode- and URL-safe). The
    /// blob carries the strata + every codex entry, so a reload restores both fully.
    pub fn encode(&self) -> String {
        let mut b = Vec::new();
        b.push(1u8); // version
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
    if *take(&mut p, 1)?.first()? != 1 {
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
    Some(Progress {
        strata,
        codex,
        seen,
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

/// Two progressions are equal when their strata + codex match (the `seen` set is derived).
impl PartialEq for Progress {
    fn eq(&self, other: &Self) -> bool {
        self.strata == other.strata && self.codex == other.codex
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
