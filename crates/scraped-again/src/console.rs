//! G4 — the operations **console**: the game's actions surfaced as visible, **clickable
//! blocks**, with a couple of **given routines** a minimal runtime runs ([`game-system.md`]).
//! No typing — a cursor + a confirm button (controller/phone-first). This module is the pure
//! model (the block vocabulary, the given routines, cursor/toggle, the terminal render); the
//! app dispatches each block's effect through the **existing G1–G3 paths** (behaviour parity).
//!
//! Assumptions / decisions taken solo (G4): (1) `collect` is the single G1 `collect_aimed`
//! behaviour for both a manual click and the routine's `on-scan → collect` — so at cruise
//! altitude the given auto-collect harvests ~nothing (sites sit below the aim ray's reach),
//! keeping net autopilot behaviour ≈ today's; it bites as you fly/scan low (and as reach grows
//! in G6). (2) Console interaction is keyboard/pad **cursor + confirm**; per-row *mouse* hit-
//! testing on the text path is deferred to G5. (3) `spend`/`goto` are in the Tier-0 vocabulary
//! but inert in G4 (no spend targets / no map picker yet → G5/G6).

/// A generic filter field for `match(field)` (game-system §11 Tier 1). v1 fields, both
/// comprehensible from G1's `script→stratum→rarity` (the collectible set is already
/// uncollected, so the useful filter is by *rarity*); more fields arrive in G6.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MatchField {
    /// Only rare strata (Relics + Signals) — "save the buffer for the good stuff".
    Rare,
}

impl MatchField {
    pub fn label(self) -> &'static str {
        match self {
            MatchField::Rare => "rare",
        }
    }
}

/// A block — one recovered console operation (game-system §11). G4 = Tier 0; G5 adds the nav
/// vocabulary (`seek`/`circle`) and the generic `match` filter.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Block {
    Scan,              // scan(shards): sense sites in the forward cone (G3)
    Collect,           // collect the aimed/nearest in-reach site (G1)
    FireBeam,          // cast the survey-beam (G2)
    Decode,            // decode/comprehend the richest affordable stratum (G6)
    Spend,             // spend a shard on a faculty (inert until G6 gives targets)
    Goto,              // direct travel to a picked map target (inert until a map picker)
    Drift,             // aimless cinematic wander — today's default autopilot
    Seek,              // head to the nearest known-uncollected site (G5)
    Circle,            // loiter / orbit the current area (G5)
    Match(MatchField), // generic filter in an on-scan pipeline (G5)
    OnScan,            // trigger: fires when a scan finds something (not a clickable action)
}

impl Block {
    pub fn label(self) -> &'static str {
        match self {
            Block::Scan => "scan(shards)",
            Block::Collect => "collect",
            Block::FireBeam => "fire-beam",
            Block::Decode => "decode",
            Block::Spend => "spend",
            Block::Goto => "goto(area)",
            Block::Drift => "drift",
            Block::Seek => "seek(uncollected)",
            Block::Circle => "circle",
            Block::Match(f) => match f {
                MatchField::Rare => "match(rare)",
            },
            Block::OnScan => "on-scan",
        }
    }
    /// The stratum whose **comprehension** (G6 `decode`) recovers this block — `None` = a
    /// starter (Tier 0). This is the "tree": decoding grows the vocabulary (game-system §4).
    pub fn required(self) -> Option<crate::progress::Stratum> {
        use crate::progress::Stratum;
        match self {
            Block::Seek | Block::Circle | Block::Goto => Some(Stratum::Schematics),
            Block::Match(_) => Some(Stratum::Rites),
            _ => None, // scan/collect/fire-beam/decode/drift/spend/on-scan are starters
        }
    }

    /// Wired to real behaviour? (`spend`/`goto` are vocabulary stubs for now.)
    pub fn wired(self) -> bool {
        matches!(
            self,
            Block::Scan
                | Block::Collect
                | Block::FireBeam
                | Block::Decode
                | Block::Drift
                | Block::Seek
                | Block::Circle
                | Block::Match(_)
        )
    }
}

/// A given routine (no player editor in G4): actions run **every tick** while enabled, plus
/// actions fired on the **on-scan** event. The two givens (drift; scan→on-scan→collect) fit
/// this shape; G5's editor will generalise it.
#[derive(Clone, Debug)]
pub struct Routine {
    pub name: &'static str,
    pub enabled: bool,
    pub continuous: Vec<Block>,
    pub on_scan: Vec<Block>,
}

/// What the cursor is currently on.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sel {
    Routine(usize),
    Block(Block),
}

/// The console: the given routines + the clickable block palette + cursor/open state.
pub struct Console {
    pub open: bool,
    pub cursor: usize,
    pub routines: Vec<Routine>,
    pub palette: Vec<Block>,
    /// Comprehended strata (synced from `progress` each frame) — gates the block vocabulary (G6).
    pub unlocked: std::collections::HashSet<crate::progress::Stratum>,
}

impl Default for Console {
    fn default() -> Self {
        Console {
            open: false,
            cursor: 0,
            // The onboarding artifact: two working routines shown as their blocks (game-system §8).
            routines: vec![
                Routine {
                    name: "drift",
                    enabled: true,
                    continuous: vec![Block::Drift],
                    on_scan: vec![],
                },
                Routine {
                    name: "survey",
                    enabled: true,
                    continuous: vec![Block::Scan],
                    on_scan: vec![Block::Collect],
                },
            ],
            palette: vec![
                Block::Scan,
                Block::Collect,
                Block::FireBeam,
                Block::Decode,
                Block::Goto,
                Block::Drift,
            ],
            unlocked: std::collections::HashSet::new(),
        }
    }
}

impl Console {
    /// Total selectable rows: routines (toggle) then palette blocks (run).
    pub fn rows(&self) -> usize {
        self.routines.len() + self.palette.len()
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let n = self.rows().max(1) as i32;
        self.cursor = (((self.cursor as i32 + delta) % n + n) % n) as usize;
    }

    /// Resolve the cursor to a routine-toggle or a block-trigger.
    pub fn selected(&self) -> Sel {
        if self.cursor < self.routines.len() {
            Sel::Routine(self.cursor)
        } else {
            Sel::Block(self.palette[self.cursor - self.routines.len()])
        }
    }

    /// Toggle the routine under the cursor (if it's a routine row). Returns its new state.
    pub fn toggle_routine(&mut self, i: usize) -> bool {
        let r = &mut self.routines[i];
        r.enabled = !r.enabled;
        r.enabled
    }

    fn routine(&self, name: &str) -> Option<&Routine> {
        self.routines.iter().find(|r| r.name == name)
    }

    /// Is the `drift` (autopilot) routine enabled? Gates the auto-wander.
    pub fn drift_enabled(&self) -> bool {
        self.routine("drift").is_some_and(|r| r.enabled)
    }

    /// Is the `survey` routine (scan + on-scan→collect) enabled? Gates auto-scan/collect.
    pub fn survey_enabled(&self) -> bool {
        self.routine("survey").is_some_and(|r| r.enabled)
    }

    /// Does the `survey` routine auto-collect on scan? (on-scan → collect wired.)
    pub fn survey_autocollects(&self) -> bool {
        self.routine("survey")
            .is_some_and(|r| r.enabled && r.on_scan.contains(&Block::Collect))
    }

    fn routine_mut(&mut self, name: &str) -> Option<&mut Routine> {
        self.routines.iter_mut().find(|r| r.name == name)
    }

    /// Has this block been recovered? (Starters always; gated blocks need their stratum decoded.)
    pub fn is_unlocked(&self, b: Block) -> bool {
        b.required().is_none_or(|s| self.unlocked.contains(&s))
    }

    /// The current nav block of the `drift` routine (Drift / Seek / Circle).
    pub fn nav_block(&self) -> Block {
        self.routine("drift")
            .and_then(|r| r.continuous.first().copied())
            .unwrap_or(Block::Drift)
    }

    /// The `survey` routine's collect filter, if any (the `match` step before `collect`).
    pub fn filter(&self) -> Option<MatchField> {
        self.routine("survey").and_then(|r| {
            r.on_scan.iter().find_map(|b| match b {
                Block::Match(f) => Some(*f),
                _ => None,
            })
        })
    }

    /// G5 picker: step the routine under the cursor's parameter — the nav block for `drift`,
    /// the collect filter for `survey` (no typing; ←/→ cycle).
    pub fn cycle_param(&mut self, i: i32) {
        match self.selected() {
            Sel::Routine(r) if self.routines[r].name == "drift" => {
                // Only cycle to nav blocks that have been recovered (Seek/Circle need Schematics).
                let nav: Vec<Block> = [Block::Drift, Block::Seek, Block::Circle]
                    .into_iter()
                    .filter(|b| self.is_unlocked(*b))
                    .collect();
                let cur = nav.iter().position(|b| *b == self.nav_block()).unwrap_or(0) as i32;
                let n = nav.len() as i32;
                let next = nav[(((cur + i) % n + n) % n) as usize];
                if let Some(d) = self.routine_mut("drift") {
                    d.continuous = vec![next];
                }
            }
            Sel::Routine(r) if self.routines[r].name == "survey" => {
                // Cycle the filter: none ↔ match(rare) — only once `match` is recovered (Rites).
                if !self.is_unlocked(Block::Match(MatchField::Rare)) {
                    return;
                }
                let on = self.filter().is_some();
                if let Some(s) = self.routine_mut("survey") {
                    s.on_scan = if on {
                        vec![Block::Collect]
                    } else {
                        vec![Block::Match(MatchField::Rare), Block::Collect]
                    };
                }
            }
            _ => {}
        }
    }

    /// Encode the editable routine state (nav + filter) as a `co=` share segment.
    pub fn encode(&self) -> String {
        let nav = match self.nav_block() {
            Block::Seek => 1,
            Block::Circle => 2,
            _ => 0,
        };
        let filter = u8::from(self.filter().is_some());
        format!("co={nav}.{filter}")
    }

    /// Restore the nav + filter from a `co=` segment (lenient; absent → the given defaults).
    pub fn restore(&mut self, s: &str) {
        let s = s.strip_prefix('#').unwrap_or(s);
        let Some(v) = s.split('&').find_map(|p| p.strip_prefix("co=")) else {
            return;
        };
        let mut it = v.split('.');
        if let Some(nav) = it.next().and_then(|n| n.parse::<u8>().ok()) {
            if let Some(d) = self.routine_mut("drift") {
                d.continuous = vec![match nav {
                    1 => Block::Seek,
                    2 => Block::Circle,
                    _ => Block::Drift,
                }];
            }
        }
        if let Some(filter) = it.next().and_then(|n| n.parse::<u8>().ok()) {
            if let Some(s) = self.routine_mut("survey") {
                s.on_scan = if filter == 1 {
                    vec![Block::Match(MatchField::Rare), Block::Collect]
                } else {
                    vec![Block::Collect]
                };
            }
        }
    }

    /// The terminal-styled console text for the HUD/text overlay.
    pub fn render(&self) -> String {
        let mut s = String::from("OPERATIONS CONSOLE   [O close]\nroutines (Enter toggles):\n");
        let mut row = 0usize;
        for r in &self.routines {
            let cur = if row == self.cursor { ">" } else { " " };
            let on = if r.enabled { "on " } else { "off" };
            let pipe: Vec<&str> = r
                .continuous
                .iter()
                .chain(r.on_scan.iter())
                .map(|b| b.label())
                .collect();
            s.push_str(&format!(
                "{cur} [{on}] {:<7}: {}\n",
                r.name,
                pipe.join(" → ")
            ));
            row += 1;
        }
        s.push_str("blocks (Enter runs):\n");
        for b in &self.palette {
            let cur = if row == self.cursor { ">" } else { " " };
            // Locked blocks read as a recovered-but-un-comprehended man-page entry (decode to gain).
            let tag = match b.required() {
                Some(s2) if !self.unlocked.contains(&s2) => {
                    format!("  (locked: decode {})", s2.label())
                }
                _ if !b.wired() => "  (—)".to_string(),
                _ => String::new(),
            };
            s.push_str(&format!("{cur} {}{}\n", b.label(), tag));
            row += 1;
        }
        s.push_str("[↑↓ select · ←→ change · Enter run/toggle]");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_routines_are_the_onboarding_artifact() {
        let c = Console::default();
        assert!(c.drift_enabled());
        assert!(c.survey_enabled());
        assert!(c.survey_autocollects());
        // The survey routine reads as scan(shards) → on-scan → collect.
        let survey = c.routines.iter().find(|r| r.name == "survey").unwrap();
        assert_eq!(survey.continuous, vec![Block::Scan]);
        assert_eq!(survey.on_scan, vec![Block::Collect]);
    }

    #[test]
    fn cursor_wraps_and_resolves() {
        let mut c = Console::default();
        assert_eq!(c.selected(), Sel::Routine(0)); // drift
        c.move_cursor(-1);
        assert_eq!(c.cursor, c.rows() - 1); // wrapped to the last block
        assert!(matches!(c.selected(), Sel::Block(_)));
        c.move_cursor(1);
        assert_eq!(c.cursor, 0);
    }

    #[test]
    fn toggling_a_routine_gates_it() {
        let mut c = Console::default();
        c.move_cursor(1); // survey row
        assert!(!c.toggle_routine(1));
        assert!(!c.survey_enabled());
        assert!(c.toggle_routine(1));
        assert!(c.survey_enabled());
    }

    #[test]
    fn vocabulary_marks_wired_vs_stub() {
        assert!(Block::Collect.wired() && Block::Scan.wired() && Block::FireBeam.wired());
        assert!(!Block::Spend.wired() && !Block::Goto.wired());
        assert!(
            Block::Seek.wired() && Block::Circle.wired() && Block::Match(MatchField::Rare).wired()
        );
    }

    #[test]
    fn cycle_nav_steps_drift_seek_circle() {
        let mut c = Console::default();
        c.unlocked.insert(crate::progress::Stratum::Schematics); // recover seek/circle
        assert_eq!(c.nav_block(), Block::Drift); // cursor on drift (row 0)
        c.cycle_param(1);
        assert_eq!(c.nav_block(), Block::Seek);
        c.cycle_param(1);
        assert_eq!(c.nav_block(), Block::Circle);
        c.cycle_param(1);
        assert_eq!(c.nav_block(), Block::Drift); // wraps
        c.cycle_param(-1);
        assert_eq!(c.nav_block(), Block::Circle);
    }

    #[test]
    fn cycle_filter_toggles_match_rare() {
        let mut c = Console::default();
        c.unlocked.insert(crate::progress::Stratum::Rites); // recover match()
        c.move_cursor(1); // survey row
        assert_eq!(c.filter(), None);
        c.cycle_param(1);
        assert_eq!(c.filter(), Some(MatchField::Rare));
        assert!(c.survey_autocollects()); // still collects, now filtered
        c.cycle_param(1);
        assert_eq!(c.filter(), None);
    }

    #[test]
    fn vocabulary_is_gated_by_comprehension() {
        use crate::progress::Stratum;
        let mut c = Console::default();
        // Starters are always available; seek/match are locked until their stratum is decoded.
        assert!(c.is_unlocked(Block::Scan) && c.is_unlocked(Block::Decode));
        assert!(!c.is_unlocked(Block::Seek) && !c.is_unlocked(Block::Match(MatchField::Rare)));
        // Cycling nav can't reach Seek while locked.
        c.cycle_param(1);
        assert_eq!(c.nav_block(), Block::Drift);
        // Comprehend Schematics → seek unlocks + becomes reachable.
        c.unlocked.insert(Stratum::Schematics);
        assert!(c.is_unlocked(Block::Seek));
        c.cycle_param(1);
        assert_eq!(c.nav_block(), Block::Seek);
        // The filter stays locked until Rites.
        c.move_cursor(1); // survey
        c.cycle_param(1);
        assert_eq!(c.filter(), None);
        c.unlocked.insert(Stratum::Rites);
        c.cycle_param(1);
        assert_eq!(c.filter(), Some(MatchField::Rare));
    }

    #[test]
    fn routine_state_round_trips_through_co_segment() {
        use crate::progress::Stratum;
        let mut c = Console::default();
        c.unlocked.insert(Stratum::Schematics);
        c.unlocked.insert(Stratum::Rites);
        c.cycle_param(1); // drift → seek
        c.move_cursor(1);
        c.cycle_param(1); // survey filter → rare
        let s = format!("s=1&{}&x=2", c.encode());
        let mut back = Console::default();
        back.restore(&s);
        assert_eq!(back.nav_block(), Block::Seek);
        assert_eq!(back.filter(), Some(MatchField::Rare));
        // Lenient: no co= leaves the givens.
        let mut d = Console::default();
        d.restore("s=1&x=2");
        assert_eq!(d.nav_block(), Block::Drift);
        assert_eq!(d.filter(), None);
    }
}
