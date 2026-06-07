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

/// A Tier-0 block — one recovered console operation (game-system §11).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Block {
    Scan,     // scan(shards): sense sites in the forward cone (G3)
    Collect,  // collect the aimed/nearest in-reach site (G1)
    FireBeam, // cast the survey-beam (G2)
    Spend,    // spend a shard on a faculty (inert until G6 gives targets)
    Goto,     // direct travel to a picked map target (inert until G5's picker)
    Drift,    // aimless cinematic wander — today's default autopilot
    OnScan,   // trigger: fires when a scan finds something (not a clickable action)
}

impl Block {
    pub fn label(self) -> &'static str {
        match self {
            Block::Scan => "scan(shards)",
            Block::Collect => "collect",
            Block::FireBeam => "fire-beam",
            Block::Spend => "spend",
            Block::Goto => "goto(area)",
            Block::Drift => "drift",
            Block::OnScan => "on-scan",
        }
    }
    /// Wired to real behaviour in G4? (`spend`/`goto` are vocabulary stubs for now.)
    pub fn wired(self) -> bool {
        matches!(
            self,
            Block::Scan | Block::Collect | Block::FireBeam | Block::Drift
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
                Block::Spend,
                Block::Goto,
                Block::Drift,
            ],
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
            let tag = if b.wired() { "" } else { "  (—)" };
            s.push_str(&format!("{cur} {}{}\n", b.label(), tag));
            row += 1;
        }
        s.push_str("[↑↓ select · Enter run/toggle]");
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
    }
}
