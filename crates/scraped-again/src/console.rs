//! G7 — the operations **console**: a real, **interpreted** routine model + a no-typing
//! **free-form editor** ([`game-system.md`] §2). The game's actions are visible **blocks**;
//! routines are **data** (a trigger + a linear body of steps) that an **interpreter** runs each
//! tick. The two (now three) given routines are *ordinary instances* of that model — there are
//! **no per-name branches** anywhere. The player can create / delete routines and
//! insert / remove / reorder / re-param their blocks from the console with **no typing**
//! (cursor + discrete buttons, controller/phone-first); authored routines persist (`co=`).
//!
//! The app reads behaviour **from the interpreter** ([`Console::tick`] / [`Console::on_scan_acts`])
//! — nav steering, a scan request, and one-shot acts — and dispatches each block's effect through
//! the **existing G1–G6 effect paths** (behaviour parity). This module is the pure model: the
//! block vocabulary, the routine/step types, the interpreter, the editor ops, and the terminal
//! render.
//!
//! Decisions taken solo (G7), recorded per the unattended-run norm:
//! 1. **Body shape.** A routine is `trigger + linear Vec<Step>`; control flow is expressed by
//!    *prefix-modifier* steps — `match(field)` filters the Collect(s) that follow it, and
//!    `repeat(n)` multiplies the **next** `Do`. This is the simplest shape that expresses the
//!    givens *and* `when`/`repeat`/match-gated collect (brief Decision 1 default: "linear body
//!    with optional if/repeat, not a node graph") without the navigation cost of nested
//!    `Vec<Step>` in a no-typing editor.
//! 2. **The given "survey" splits into two data routines** — `survey` (continuous `scan`) +
//!    `collect` (on-scan `collect`) — exactly the brief's "`{OnScan,[Do(Collect)]}` + a continuous
//!    `Do(Scan)`" decomposition. So the home screen now shows **three** givens, all plain data.
//! 3. **Persistence stays in the console's own `co=` segment** (full routine list), not `pg=`.
//!    The console owns its serialization (it already round-tripped through `co=`); what the brief
//!    requires is that authored routines *persist + round-trip*, which `co=` satisfies.

use std::collections::HashSet;

use crate::progress::Stratum;

/// The item a `scan` block senses. v1 has only `shards` (the data sites); the parameterised
/// form (`scan(item)`) is here so the model carries block params uniformly (G7 cleanup).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ScanItem {
    Shards,
}

impl ScanItem {
    pub fn label(self) -> &'static str {
        match self {
            ScanItem::Shards => "shards",
        }
    }
}

/// A generic filter field for a `match(field)` step (game-system §11 Tier 1). v1: by *rarity*
/// (the collectible set is already uncollected, so the useful filter is "save the buffer for the
/// good stuff"). Recovered by comprehending **Rites**.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MatchField {
    /// Only rare strata (Relics + Signals).
    Rare,
}

impl MatchField {
    pub fn label(self) -> &'static str {
        match self {
            MatchField::Rare => "rare",
        }
    }
    /// The stratum whose comprehension recovers the `match` step.
    pub fn required(self) -> Stratum {
        Stratum::Rites
    }
}

/// Which agent a routine drives (game-system §7). Routines are **per-agent**, drawing on a shared
/// block library; the *insertable* vocabulary is context-scoped to the agent (+ shared blocks).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Agent {
    /// The cruiser — flies, scans, routes (the autopilot half).
    Ship,
    /// The walker — on-foot survey-beam, descent, close collection (the exploration half).
    Foot,
}

impl Agent {
    pub fn label(self) -> &'static str {
        match self {
            Agent::Ship => "ship",
            Agent::Foot => "foot",
        }
    }
    pub fn other(self) -> Agent {
        match self {
            Agent::Ship => Agent::Foot,
            Agent::Foot => Agent::Ship,
        }
    }
}

/// A block — one recovered console **action** (game-system §11). Triggers (`on-scan`, `when`) and
/// the `match`/`repeat` modifiers are *not* blocks: they live on the routine ([`Trigger`]) or as
/// modifier [`Step`]s, so this enum is exactly "things an agent can *do*".
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Block {
    Scan(ScanItem), // sense sites of `item` in the forward cone (G3)
    Collect,        // collect the aimed/nearest in-reach site (G1)
    FireBeam,       // cast the survey-beam (G2)
    Decode,         // decode/comprehend the richest affordable stratum (G6)
    Spend,          // spend a shard on a faculty (vocabulary stub — no targets yet)
    Goto,           // direct travel to a picked map target (vocabulary stub — no map picker yet)
    Drift,          // aimless cinematic wander — the default autopilot
    Seek,           // head to the nearest known-uncollected site (ship nav)
    Circle,         // loiter / orbit the current area (ship nav)
    Walk,           // on foot, walk to the nearest known site (foot nav — G8c)
    Hail,           // recall the autonomous/parked ship to the walker (G8a — foot/shared)
}

impl Block {
    pub fn label(self) -> &'static str {
        match self {
            Block::Scan(i) => match i {
                ScanItem::Shards => "scan(shards)",
            },
            Block::Collect => "collect",
            Block::FireBeam => "fire-beam",
            Block::Decode => "decode",
            Block::Spend => "spend",
            Block::Goto => "goto(area)",
            Block::Drift => "drift",
            Block::Seek => "seek(uncollected)",
            Block::Circle => "circle",
            Block::Walk => "walk(uncollected)",
            Block::Hail => "hail",
        }
    }

    /// The stratum whose **comprehension** (G6 `decode`) recovers this block — `None` = a starter
    /// (Tier 0). This is the "tree": decoding grows the vocabulary (game-system §4).
    pub fn required(self) -> Option<Stratum> {
        match self {
            Block::Seek | Block::Circle | Block::Goto => Some(Stratum::Schematics),
            _ => None, // scan/collect/fire-beam/decode/drift/spend are starters
        }
    }

    /// Wired to a real effect? (`spend`/`goto` are vocabulary stubs for now.)
    pub fn wired(self) -> bool {
        !matches!(self, Block::Spend | Block::Goto)
    }

    /// Is this a navigation block (it steers an agent rather than firing an effect)?
    pub fn is_nav(self) -> bool {
        matches!(
            self,
            Block::Drift | Block::Seek | Block::Circle | Block::Walk
        )
    }

    /// Which agent this block belongs to — `None` = **shared** (usable by either agent). Ship
    /// blocks fly/scan/route; foot blocks are the survey-beam + `walk`; collect/decode/spend/hail
    /// are shared (game-system §7).
    pub fn agent(self) -> Option<Agent> {
        match self {
            Block::Scan(_) | Block::Drift | Block::Seek | Block::Circle | Block::Goto => {
                Some(Agent::Ship)
            }
            Block::FireBeam | Block::Walk => Some(Agent::Foot),
            Block::Collect | Block::Decode | Block::Spend | Block::Hail => None, // shared
        }
    }

    /// Is this block available to `agent`? (Its own agent, or shared.)
    pub fn for_agent(self, agent: Agent) -> bool {
        self.agent().is_none_or(|a| a == agent)
    }
}

impl PartialEq for Routine {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

/// One step in a routine body. A linear sequence of these is the routine's program;
/// `Match`/`Repeat` are *prefix modifiers* (see the module decision note).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Step {
    /// Do a block.
    Do(Block),
    /// Filter the Collect(s) that follow it in this body to only matching sites.
    Match(MatchField),
    /// Repeat the **next** `Do` this many times (1..=9).
    Repeat(u8),
}

impl Step {
    pub fn label(self) -> String {
        match self {
            Step::Do(b) => b.label().to_string(),
            Step::Match(f) => format!("match({})", f.label()),
            Step::Repeat(n) => format!("repeat ×{n}"),
        }
    }
    /// The stratum gating this step (so a locked step can't be inserted/cycled into).
    fn required(self) -> Option<Stratum> {
        match self {
            Step::Do(b) => b.required(),
            Step::Match(f) => Some(f.required()),
            Step::Repeat(_) => None,
        }
    }
}

/// Is `step` available to `agent`? `Do(block)` follows the block's agent; the `Match`/`Repeat`
/// modifiers are shared (available to either agent).
fn step_for_agent(step: Step, agent: Agent) -> bool {
    match step {
        Step::Do(b) => b.for_agent(agent),
        Step::Match(_) | Step::Repeat(_) => true,
    }
}

/// A state the `when` trigger can test. v1: the **total banked data** (all strata).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum State {
    Data,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Data => "data",
        }
    }
}

/// A `when(state ≥ min)` condition.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Cond {
    pub state: State,
    pub min: u32,
}

impl Cond {
    fn holds(self, value: u32) -> bool {
        match self.state {
            State::Data => value >= self.min,
        }
    }
}

/// When a routine's body runs. The interpreter fires `Continuous` every tick, `OnScan` on a scan
/// that finds something, `When` once on the rising edge of its condition, and `OnArrive` once when
/// the agent reaches the site it's heading to (the expedition skeleton — game-system §7/§11).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Trigger {
    Continuous,
    OnScan,
    When(Cond),
    OnArrive,
}

impl Trigger {
    pub fn label(self) -> String {
        match self {
            Trigger::Continuous => "every tick".to_string(),
            Trigger::OnScan => "on scan".to_string(),
            Trigger::When(c) => format!("when {} ≥ {}", c.state.label(), c.min),
            Trigger::OnArrive => "on arrive".to_string(),
        }
    }
}

/// A routine: **data** the interpreter runs. No per-name behaviour — the givens are just the
/// default instances. (`armed` is transient runtime edge-state, excluded from equality below.)
#[derive(Clone, Debug)]
pub struct Routine {
    pub name: String,
    pub enabled: bool,
    /// Which agent runs this routine (game-system §7). Scopes the insertable vocabulary + which
    /// agent's interpreter ticks it.
    pub agent: Agent,
    pub trigger: Trigger,
    pub body: Vec<Step>,
    /// `When`-edge state: was the condition satisfied last tick? (Runtime only; not persisted.)
    pub armed: bool,
}

impl Routine {
    fn new(name: impl Into<String>, agent: Agent, trigger: Trigger, body: Vec<Step>) -> Self {
        Routine {
            name: name.into(),
            enabled: true,
            agent,
            trigger,
            body,
            armed: false,
        }
    }

    /// The persistent identity of a routine, ignoring the transient `armed` edge-state.
    fn key(&self) -> (&str, bool, Agent, Trigger, &[Step]) {
        (
            &self.name,
            self.enabled,
            self.agent,
            self.trigger,
            &self.body,
        )
    }

    /// Does this routine steer the autopilot? (A continuous routine whose body does a nav block.)
    pub fn is_nav(&self) -> bool {
        matches!(self.trigger, Trigger::Continuous)
            && self
                .body
                .iter()
                .any(|s| matches!(s, Step::Do(b) if b.is_nav()))
    }
}

/// One resolved action the app dispatches: a block plus the match-filter context it runs under.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Act {
    pub block: Block,
    pub filter: Option<MatchField>,
}

/// This tick's intents, produced by the interpreter from the enabled routines. The app applies
/// them through the existing G1–G6 effect paths (replacing the old named-accessor hacks).
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Tick {
    /// Autopilot steering from a continuous nav routine (`None` ⇒ no drift routine ⇒ autopilot off).
    pub nav: Option<Block>,
    /// A continuous routine wants to pulse the scan (the app throttles it to `scan::INTERVAL`).
    pub scan: bool,
    /// Other one-shot acts to dispatch this tick (continuous non-nav/non-scan + `when`-edge fires).
    pub acts: Vec<Act>,
}

/// What the editor cursor is on while editing a routine.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EditFocus {
    Trigger,
    Step(usize),
    AddStep,
}

/// What the home cursor is on.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sel {
    Routine(usize),
    NewRoutine,
    Block(Block),
}

/// Which screen the console is showing.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum View {
    /// Routine list + "new routine" + the manual-run block palette.
    Home,
    /// Editing routine `i`'s trigger + body (the free-form editor).
    Edit(usize),
}

/// The console: the routines (data), the manual/insert block palette, cursor + view state.
pub struct Console {
    pub open: bool,
    pub view: View,
    pub cursor: usize,
    pub routines: Vec<Routine>,
    pub palette: Vec<Block>,
    /// Comprehended strata (synced from `progress` each frame) — gates the block vocabulary (G6).
    pub unlocked: HashSet<Stratum>,
}

impl Default for Console {
    fn default() -> Self {
        Console {
            open: false,
            view: View::Home,
            cursor: 0,
            // The onboarding artifact: the hands-off loop shown as three plain data routines
            // (drift; survey scans; collect harvests on scan) — all ordinary instances (§8).
            routines: vec![
                Routine::new(
                    "drift",
                    Agent::Ship,
                    Trigger::Continuous,
                    vec![Step::Do(Block::Drift)],
                ),
                Routine::new(
                    "survey",
                    Agent::Ship,
                    Trigger::Continuous,
                    vec![Step::Do(Block::Scan(ScanItem::Shards))],
                ),
                Routine::new(
                    "collect",
                    Agent::Ship,
                    Trigger::OnScan,
                    vec![Step::Do(Block::Collect)],
                ),
            ],
            palette: vec![
                Block::Scan(ScanItem::Shards),
                Block::Collect,
                Block::FireBeam,
                Block::Decode,
                Block::Goto,
                Block::Drift,
            ],
            unlocked: HashSet::new(),
        }
    }
}

impl Console {
    // ---- vocabulary gating ------------------------------------------------------------------

    /// Has this block been recovered? (Starters always; gated blocks need their stratum decoded.)
    pub fn is_unlocked(&self, b: Block) -> bool {
        b.required().is_none_or(|s| self.unlocked.contains(&s))
    }

    fn step_unlocked(&self, s: Step) -> bool {
        s.required().is_none_or(|st| self.unlocked.contains(&st))
    }

    /// The steps `agent` may insert / cycle to, in cycle order — filtered to what's recovered (G6)
    /// **and** scoped to the agent's context + shared blocks (game-system §7). (Blocks, then the
    /// `match` modifier, then `repeat`.)
    pub fn vocabulary(&self, agent: Agent) -> Vec<Step> {
        let all = [
            Step::Do(Block::Scan(ScanItem::Shards)),
            Step::Do(Block::Collect),
            Step::Do(Block::FireBeam),
            Step::Do(Block::Decode),
            Step::Do(Block::Drift),
            Step::Do(Block::Seek),
            Step::Do(Block::Circle),
            Step::Do(Block::Walk),
            Step::Do(Block::Goto),
            Step::Do(Block::Spend),
            Step::Do(Block::Hail),
            Step::Match(MatchField::Rare),
            Step::Repeat(2),
        ];
        all.into_iter()
            .filter(|s| self.step_unlocked(*s) && step_for_agent(*s, agent))
            .collect()
    }

    // ---- the interpreter --------------------------------------------------------------------

    /// Expand a body into resolved acts: `match` sets the filter for following Collects;
    /// `repeat(n)` multiplies the next `Do`.
    fn expand(body: &[Step]) -> Vec<Act> {
        let mut out = Vec::new();
        let mut filter = None;
        let mut times: u8 = 1;
        for step in body {
            match step {
                Step::Match(f) => filter = Some(*f),
                Step::Repeat(n) => times = (*n).max(1),
                Step::Do(b) => {
                    for _ in 0..times {
                        out.push(Act { block: *b, filter });
                    }
                    times = 1;
                }
            }
        }
        out
    }

    /// Run one interpreter tick for `agent`'s **continuous** / **when** / **on-arrive** routines,
    /// given the current `data` (total banked strata) for `when` conditions and whether the agent
    /// has just `arrived` at the site it's heading to. Returns this tick's [`Tick`] intents.
    /// (`on-scan` routines fire separately, on a scan hit — see [`Console::on_scan_acts`].)
    pub fn tick(&mut self, agent: Agent, data: u32, arrived: bool) -> Tick {
        let mut t = Tick::default();
        for r in &mut self.routines {
            if r.agent != agent {
                continue;
            }
            if !r.enabled {
                r.armed = false;
                continue;
            }
            match r.trigger {
                Trigger::Continuous => {
                    for act in Self::expand(&r.body) {
                        if act.block.is_nav() {
                            t.nav = Some(act.block);
                        } else if matches!(act.block, Block::Scan(_)) {
                            t.scan = true;
                        } else {
                            t.acts.push(act);
                        }
                    }
                }
                Trigger::OnScan => {} // fired on a scan hit, not per tick
                Trigger::When(c) => {
                    let sat = c.holds(data);
                    if sat && !r.armed {
                        t.acts.extend(Self::expand(&r.body)); // rising edge → fire once
                    }
                    r.armed = sat;
                }
                Trigger::OnArrive => {
                    if arrived && !r.armed {
                        t.acts.extend(Self::expand(&r.body)); // reached the site → fire once
                    }
                    r.armed = arrived;
                }
            }
        }
        t
    }

    /// The acts `agent`'s enabled **on-scan** routines want, when a scan finds something (typically
    /// a filtered collect). Walked through the same interpreter as everything else.
    pub fn on_scan_acts(&self, agent: Agent) -> Vec<Act> {
        self.routines
            .iter()
            .filter(|r| r.agent == agent && r.enabled && matches!(r.trigger, Trigger::OnScan))
            .flat_map(|r| Self::expand(&r.body))
            .collect()
    }

    // ---- home navigation --------------------------------------------------------------------

    /// Home rows: routines (toggle/edit), a "new routine" row, then the manual block palette.
    pub fn home_rows(&self) -> usize {
        self.routines.len() + 1 + self.palette.len()
    }

    /// Editor rows for routine `i`: the trigger row, each body step, then an "add step" row.
    fn edit_rows(&self, i: usize) -> usize {
        1 + self.routines[i].body.len() + 1
    }

    fn rows(&self) -> usize {
        match self.view {
            View::Home => self.home_rows(),
            View::Edit(i) => self.edit_rows(i),
        }
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let n = self.rows().max(1) as i32;
        self.cursor = (((self.cursor as i32 + delta) % n + n) % n) as usize;
    }

    /// Resolve the home cursor.
    pub fn selected(&self) -> Sel {
        let nr = self.routines.len();
        if self.cursor < nr {
            Sel::Routine(self.cursor)
        } else if self.cursor == nr {
            Sel::NewRoutine
        } else {
            Sel::Block(self.palette[self.cursor - nr - 1])
        }
    }

    /// Resolve the editor cursor for routine `i`.
    pub fn edit_focus(&self, i: usize) -> EditFocus {
        let nb = self.routines[i].body.len();
        if self.cursor == 0 {
            EditFocus::Trigger
        } else if self.cursor <= nb {
            EditFocus::Step(self.cursor - 1)
        } else {
            EditFocus::AddStep
        }
    }

    // ---- routine-level edits ----------------------------------------------------------------

    /// Toggle the routine under index `i`. Returns its new enabled state.
    pub fn toggle_routine(&mut self, i: usize) -> bool {
        let r = &mut self.routines[i];
        r.enabled = !r.enabled;
        r.enabled
    }

    /// Create a fresh empty `agent` routine and open its editor. Returns the new index.
    pub fn create_routine(&mut self, agent: Agent) -> usize {
        let name = format!("routine-{}", self.routines.len() + 1);
        self.routines
            .push(Routine::new(name, agent, Trigger::Continuous, Vec::new()));
        let i = self.routines.len() - 1;
        self.view = View::Edit(i);
        self.cursor = 0;
        i
    }

    /// Flip the edited routine's agent (ship ↔ foot) — re-scopes its insertable vocabulary.
    pub fn cycle_agent(&mut self) {
        if let View::Edit(i) = self.view {
            self.routines[i].agent = self.routines[i].agent.other();
        }
    }

    /// Delete routine `i` and return to the home screen.
    pub fn delete_routine(&mut self, i: usize) {
        if i < self.routines.len() {
            self.routines.remove(i);
        }
        self.view = View::Home;
        self.cursor = self.cursor.min(self.home_rows().saturating_sub(1));
    }

    pub fn open_editor(&mut self, i: usize) {
        self.view = View::Edit(i);
        self.cursor = 0;
    }

    pub fn close_editor(&mut self) {
        if let View::Edit(i) = self.view {
            // Going home — land the cursor on the routine we were editing (if it still exists).
            self.cursor = i.min(self.routines.len().saturating_sub(1));
        }
        self.view = View::Home;
    }

    // ---- body / step edits (the free-form editor) -------------------------------------------

    /// Insert a step after the editor cursor (or append, on the "add step" row). The inserted
    /// step is the first unlocked vocabulary entry; ←/→ then cycles it. No-op if nothing unlocked.
    pub fn insert_step(&mut self, i: usize) {
        let Some(first) = self.vocabulary(self.routines[i].agent).into_iter().next() else {
            return;
        };
        let at = match self.edit_focus(i) {
            EditFocus::Trigger => 0,
            EditFocus::Step(s) => s + 1,
            EditFocus::AddStep => self.routines[i].body.len(),
        };
        self.routines[i].body.insert(at, first);
        self.cursor = at + 1; // land on the new step (trigger row is 0)
    }

    /// Remove the step under the editor cursor.
    pub fn remove_step(&mut self, i: usize) {
        if let EditFocus::Step(s) = self.edit_focus(i) {
            self.routines[i].body.remove(s);
            self.cursor = self.cursor.min(self.edit_rows(i).saturating_sub(1));
        }
    }

    /// Move the step under the cursor up/down within the body (reorder).
    pub fn move_step(&mut self, i: usize, dir: i32) {
        if let EditFocus::Step(s) = self.edit_focus(i) {
            let body = &mut self.routines[i].body;
            let j = s as i32 + dir;
            if (0..body.len() as i32).contains(&j) {
                body.swap(s, j as usize);
                self.cursor = (j as usize) + 1;
            }
        }
    }

    /// ←/→ on the focused row: cycle the trigger kind, or cycle the focused step through the
    /// unlocked vocabulary. (`When` threshold + `Repeat` count are nudged by [`Console::adjust`].)
    pub fn cycle(&mut self, i: i32) {
        let View::Edit(r) = self.view else {
            return;
        };
        match self.edit_focus(r) {
            EditFocus::Trigger => self.cycle_trigger(r, i),
            EditFocus::Step(s) => self.cycle_step(r, s, i),
            EditFocus::AddStep => {}
        }
    }

    fn cycle_trigger(&mut self, r: usize, i: i32) {
        let kinds = [
            Trigger::Continuous,
            Trigger::OnScan,
            Trigger::When(Cond {
                state: State::Data,
                min: 10,
            }),
            Trigger::OnArrive,
        ];
        // Compare by discriminant so the current `When(min)` matches the `When` slot.
        let cur = kinds
            .iter()
            .position(|k| {
                std::mem::discriminant(k) == std::mem::discriminant(&self.routines[r].trigger)
            })
            .unwrap_or(0) as i32;
        let n = kinds.len() as i32;
        self.routines[r].trigger = kinds[(((cur + i) % n + n) % n) as usize];
        self.routines[r].armed = false;
    }

    fn cycle_step(&mut self, r: usize, s: usize, i: i32) {
        let vocab = self.vocabulary(self.routines[r].agent);
        if vocab.is_empty() {
            return;
        }
        let cur = self.routines[r].body[s];
        // Find the current step in the vocabulary by *kind* (so a Repeat(5) matches the Repeat slot).
        let pos = vocab
            .iter()
            .position(|v| std::mem::discriminant(v) == std::mem::discriminant(&cur))
            .unwrap_or(0) as i32;
        let n = vocab.len() as i32;
        self.routines[r].body[s] = vocab[(((pos + i) % n + n) % n) as usize];
    }

    /// −/+ on the focused row: nudge a numeric parameter — the `When` threshold (±5) or a
    /// `Repeat` count (±1, 1..=9).
    pub fn adjust(&mut self, delta: i32) {
        let View::Edit(r) = self.view else {
            return;
        };
        match self.edit_focus(r) {
            EditFocus::Trigger => {
                if let Trigger::When(c) = &mut self.routines[r].trigger {
                    let step = 5 * delta;
                    c.min = (c.min as i32 + step).clamp(0, 9_999) as u32;
                }
            }
            EditFocus::Step(s) => {
                if let Step::Repeat(n) = &mut self.routines[r].body[s] {
                    *n = (*n as i32 + delta).clamp(1, 9) as u8;
                }
            }
            EditFocus::AddStep => {}
        }
    }

    // ---- persistence (`co=` segment) --------------------------------------------------------

    fn step_code(s: Step) -> String {
        match s {
            Step::Do(Block::Scan(ScanItem::Shards)) => "S".into(),
            Step::Do(Block::Collect) => "C".into(),
            Step::Do(Block::FireBeam) => "B".into(),
            Step::Do(Block::Decode) => "D".into(),
            Step::Do(Block::Spend) => "p".into(),
            Step::Do(Block::Goto) => "g".into(),
            Step::Do(Block::Drift) => "d".into(),
            Step::Do(Block::Seek) => "k".into(),
            Step::Do(Block::Circle) => "o".into(),
            Step::Do(Block::Walk) => "W".into(),
            Step::Do(Block::Hail) => "H".into(),
            Step::Match(MatchField::Rare) => "m".into(),
            Step::Repeat(n) => format!("r{n}"),
        }
    }

    fn parse_steps(s: &str) -> Vec<Step> {
        let mut out = Vec::new();
        let mut it = s.chars().peekable();
        while let Some(c) = it.next() {
            let step = match c {
                'S' => Step::Do(Block::Scan(ScanItem::Shards)),
                'C' => Step::Do(Block::Collect),
                'B' => Step::Do(Block::FireBeam),
                'D' => Step::Do(Block::Decode),
                'p' => Step::Do(Block::Spend),
                'g' => Step::Do(Block::Goto),
                'd' => Step::Do(Block::Drift),
                'k' => Step::Do(Block::Seek),
                'o' => Step::Do(Block::Circle),
                'W' => Step::Do(Block::Walk),
                'H' => Step::Do(Block::Hail),
                'm' => Step::Match(MatchField::Rare),
                'r' => {
                    let mut num = String::new();
                    while let Some(d) = it.peek().filter(|d| d.is_ascii_digit()) {
                        num.push(*d);
                        it.next();
                    }
                    Step::Repeat(num.parse::<u8>().unwrap_or(2).clamp(1, 9))
                }
                _ => continue,
            };
            out.push(step);
        }
        out
    }

    fn trigger_code(t: Trigger) -> String {
        match t {
            Trigger::Continuous => "c".into(),
            Trigger::OnScan => "s".into(),
            Trigger::When(c) => format!("w:{}", c.min),
            Trigger::OnArrive => "a".into(),
        }
    }

    fn parse_trigger(s: &str) -> Trigger {
        match s.chars().next() {
            Some('s') => Trigger::OnScan,
            Some('a') => Trigger::OnArrive,
            Some('w') => Trigger::When(Cond {
                state: State::Data,
                min: s
                    .split(':')
                    .nth(1)
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(10),
            }),
            _ => Trigger::Continuous,
        }
    }

    /// Encode the authored routines as a `co=` share segment (URL-fragment safe — names are
    /// `[a-z0-9-]`, fields `|`, routines `;`, steps `,`).
    pub fn encode(&self) -> String {
        let body = self
            .routines
            .iter()
            .map(|r| {
                let steps = r
                    .body
                    .iter()
                    .map(|s| Self::step_code(*s))
                    .collect::<Vec<_>>()
                    .join(",");
                let agent = match r.agent {
                    Agent::Ship => 'S',
                    Agent::Foot => 'F',
                };
                format!(
                    "{}|{}|{}|{}|{}",
                    r.name,
                    u8::from(r.enabled),
                    Self::trigger_code(r.trigger),
                    steps,
                    agent
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        format!("co={body}")
    }

    /// Restore authored routines from a `co=` segment (lenient; absent/malformed → the givens).
    pub fn restore(&mut self, s: &str) {
        let s = s.strip_prefix('#').unwrap_or(s);
        let Some(v) = s.split('&').find_map(|p| p.strip_prefix("co=")) else {
            return;
        };
        let mut routines = Vec::new();
        for chunk in v.split(';').filter(|c| !c.is_empty()) {
            let mut f = chunk.split('|');
            let name = f.next().unwrap_or("routine").to_string();
            let enabled = f.next() != Some("0");
            let trigger = Self::parse_trigger(f.next().unwrap_or("c"));
            let body = Self::parse_steps(f.next().unwrap_or(""));
            let agent = if f.next() == Some("F") {
                Agent::Foot
            } else {
                Agent::Ship
            };
            routines.push(Routine {
                name,
                enabled,
                agent,
                trigger,
                body,
                armed: false,
            });
        }
        if !routines.is_empty() {
            self.routines = routines;
        }
    }

    // ---- render -----------------------------------------------------------------------------

    /// The terminal-styled console text for the HUD/text overlay.
    pub fn render(&self) -> String {
        match self.view {
            View::Home => self.render_home(),
            View::Edit(i) => self.render_edit(i),
        }
    }

    fn render_home(&self) -> String {
        let mut s = String::from(
            "OPERATIONS CONSOLE   [O close]\nroutines (Enter toggle · E edit · X delete):\n",
        );
        let mut row = 0usize;
        for r in &self.routines {
            let cur = if row == self.cursor { ">" } else { " " };
            let on = if r.enabled { "on " } else { "off" };
            let steps: Vec<String> = r.body.iter().map(|b| b.label()).collect();
            let pipe = if steps.is_empty() {
                "—".to_string()
            } else {
                steps.join(" → ")
            };
            s.push_str(&format!(
                "{cur} [{on}] {:<4} {:<9} {:<11}: {}\n",
                r.agent.label(),
                r.name,
                r.trigger.label(),
                pipe
            ));
            row += 1;
        }
        let cur = if row == self.cursor { ">" } else { " " };
        s.push_str(&format!("{cur} + new routine\n"));
        row += 1;
        s.push_str("blocks (Enter runs):\n");
        for b in &self.palette {
            let cur = if row == self.cursor { ">" } else { " " };
            let tag = match b.required() {
                Some(st) if !self.unlocked.contains(&st) => {
                    format!("  (locked: decode {})", st.label())
                }
                _ if !b.wired() => "  (—)".to_string(),
                _ => String::new(),
            };
            s.push_str(&format!("{cur} {}{}\n", b.label(), tag));
            row += 1;
        }
        s.push_str("[↑↓ select · Enter run/toggle · E edit · X delete]");
        s
    }

    fn render_edit(&self, i: usize) -> String {
        let r = &self.routines[i];
        let mut s = format!(
            "EDIT ROUTINE  {}  [{}]  agent:{} [Tab]   [O back]\n",
            r.name,
            if r.enabled { "on" } else { "off" },
            r.agent.label(),
        );
        // Trigger row.
        let cur = if self.cursor == 0 { ">" } else { " " };
        s.push_str(&format!("{cur} trigger: {}\n", r.trigger.label()));
        // Body steps.
        for (si, step) in r.body.iter().enumerate() {
            let cur = if self.cursor == si + 1 { ">" } else { " " };
            s.push_str(&format!("{cur}   {}. {}\n", si + 1, step.label()));
        }
        // Add-step row.
        let cur = if self.cursor == r.body.len() + 1 {
            ">"
        } else {
            " "
        };
        s.push_str(&format!("{cur}   + add step\n"));
        s.push_str(
            "[↑↓ move · ←→ change · -/+ value · Enter insert · X remove · [ ] reorder · Tab agent]",
        );
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_routines_are_plain_data() {
        let c = Console::default();
        // Three givens, all ordinary instances (no per-name behaviour anywhere).
        assert_eq!(c.routines.len(), 3);
        let drift = &c.routines[0];
        assert_eq!(drift.name, "drift");
        assert!(drift.is_nav());
        assert_eq!(drift.trigger, Trigger::Continuous);
        let survey = &c.routines[1];
        assert_eq!(survey.trigger, Trigger::Continuous);
        assert_eq!(survey.body, vec![Step::Do(Block::Scan(ScanItem::Shards))]);
        let collect = &c.routines[2];
        assert_eq!(collect.trigger, Trigger::OnScan);
        assert_eq!(collect.body, vec![Step::Do(Block::Collect)]);
    }

    #[test]
    fn interpreter_runs_the_givens() {
        let mut c = Console::default();
        let t = c.tick(Agent::Ship, 0, false);
        // drift → nav steering; survey → scan request; collect is on-scan (not in the tick).
        assert_eq!(t.nav, Some(Block::Drift));
        assert!(t.scan);
        assert!(t.acts.is_empty());
        // on-scan collect, no filter.
        assert_eq!(
            c.on_scan_acts(Agent::Ship),
            vec![Act {
                block: Block::Collect,
                filter: None
            }]
        );
    }

    #[test]
    fn disabling_drift_drops_the_nav_intent() {
        let mut c = Console::default();
        c.toggle_routine(0); // drift off
        let t = c.tick(Agent::Ship, 0, false);
        assert_eq!(t.nav, None); // no continuous nav routine ⇒ autopilot off
        assert!(t.scan); // survey still scans
    }

    #[test]
    fn authored_match_gated_collect_filters() {
        let mut c = Console::default();
        c.unlocked.insert(Stratum::Rites); // recover match()
                                           // Author the collect routine to "match(rare) → collect".
        c.routines[2].body = vec![Step::Match(MatchField::Rare), Step::Do(Block::Collect)];
        assert_eq!(
            c.on_scan_acts(Agent::Ship),
            vec![Act {
                block: Block::Collect,
                filter: Some(MatchField::Rare)
            }]
        );
    }

    #[test]
    fn repeat_multiplies_the_next_do() {
        let body = vec![
            Step::Repeat(3),
            Step::Do(Block::Decode),
            Step::Do(Block::Collect),
        ];
        let acts = Console::expand(&body);
        // decode ×3 then collect ×1 (repeat only multiplies the *next* Do).
        assert_eq!(acts.len(), 4);
        assert!(acts[..3].iter().all(|a| a.block == Block::Decode));
        assert_eq!(acts[3].block, Block::Collect);
    }

    #[test]
    fn when_fires_once_on_the_rising_edge() {
        let mut c = Console::default();
        // A when(data ≥ 10) → decode routine.
        c.routines.push(Routine::new(
            "auto-decode",
            Agent::Ship,
            Trigger::When(Cond {
                state: State::Data,
                min: 10,
            }),
            vec![Step::Do(Block::Decode)],
        ));
        assert!(c.tick(Agent::Ship, 5, false).acts.is_empty()); // below threshold
        let fired = c.tick(Agent::Ship, 12, false); // crosses ⇒ fires once
        assert_eq!(
            fired.acts,
            vec![Act {
                block: Block::Decode,
                filter: None
            }]
        );
        assert!(c.tick(Agent::Ship, 12, false).acts.is_empty()); // still high ⇒ no re-fire (edge, not level)
        c.tick(Agent::Ship, 0, false); // drop below ⇒ re-arm
        assert!(!c.tick(Agent::Ship, 20, false).acts.is_empty()); // crosses again ⇒ fires
    }

    #[test]
    fn on_arrive_fires_once_when_the_ship_reaches_a_site() {
        // G8c: an `on-arrive → decode` ship routine fires once on the arrival edge.
        let mut c = Console::default();
        c.routines.push(Routine::new(
            "land",
            Agent::Ship,
            Trigger::OnArrive,
            vec![Step::Do(Block::Decode)],
        ));
        assert!(c.tick(Agent::Ship, 0, false).acts.is_empty()); // not arrived
        let fired = c.tick(Agent::Ship, 0, true); // reaches a site → fires once
        assert!(fired.acts.iter().any(|a| a.block == Block::Decode));
        assert!(c.tick(Agent::Ship, 0, true).acts.is_empty()); // still there ⇒ no re-fire
        c.tick(Agent::Ship, 0, false); // leaves ⇒ re-arm
        assert!(!c.tick(Agent::Ship, 0, true).acts.is_empty()); // arrives again ⇒ fires
                                                                // The trigger round-trips through `co=`.
        let mut back = Console::default();
        back.restore(&c.encode());
        assert_eq!(back.routines, c.routines);
    }

    #[test]
    fn create_insert_remove_reorder() {
        let mut c = Console::default();
        let i = c.create_routine(Agent::Ship);
        assert_eq!(c.routines.len(), 4);
        assert_eq!(c.view, View::Edit(i));
        assert!(c.routines[i].body.is_empty());
        // Insert a step (cursor on the trigger row → inserts at front).
        c.cursor = 0;
        c.insert_step(i);
        assert_eq!(c.routines[i].body.len(), 1);
        // Insert another after it, then reorder.
        c.insert_step(i);
        assert_eq!(c.routines[i].body.len(), 2);
        c.routines[i].body = vec![
            Step::Do(Block::Scan(ScanItem::Shards)),
            Step::Do(Block::Collect),
        ];
        c.cursor = 1; // first step
        c.move_step(i, 1); // swap down
        assert_eq!(c.routines[i].body[0], Step::Do(Block::Collect));
        // Remove the step under the cursor.
        c.cursor = 1;
        c.remove_step(i);
        assert_eq!(c.routines[i].body.len(), 1);
    }

    #[test]
    fn cycle_step_skips_locked_vocabulary() {
        let mut c = Console::default();
        let i = c.create_routine(Agent::Ship);
        c.cursor = 0;
        c.insert_step(i); // a step exists now (cursor on it, row 1)
        c.cursor = 1;
        // Cycle through the whole vocabulary; with nothing decoded it must never become a
        // locked block (seek/circle/goto/match).
        for _ in 0..20 {
            c.cycle(1);
            let s = c.routines[i].body[0];
            assert!(c.step_unlocked(s), "cycled into a locked step: {s:?}");
        }
        // Once Schematics is comprehended, seek becomes reachable.
        c.unlocked.insert(Stratum::Schematics);
        let reachable = c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Seek));
        assert!(reachable);
    }

    #[test]
    fn cycle_trigger_and_adjust_value() {
        let mut c = Console::default();
        let i = c.create_routine(Agent::Ship);
        c.cursor = 0; // trigger row
        assert_eq!(c.routines[i].trigger, Trigger::Continuous);
        c.cycle(1);
        assert_eq!(c.routines[i].trigger, Trigger::OnScan);
        c.cycle(1);
        assert!(matches!(c.routines[i].trigger, Trigger::When(_)));
        // -/+ nudges the threshold by 5.
        c.adjust(1);
        if let Trigger::When(cond) = c.routines[i].trigger {
            assert_eq!(cond.min, 15);
        } else {
            panic!("expected When");
        }
    }

    #[test]
    fn adjust_repeat_count() {
        let mut c = Console::default();
        let i = c.create_routine(Agent::Ship);
        c.routines[i].body = vec![Step::Repeat(2)];
        c.cursor = 1; // the step
        c.adjust(3);
        assert_eq!(c.routines[i].body[0], Step::Repeat(5));
        c.adjust(-10); // clamps to 1
        assert_eq!(c.routines[i].body[0], Step::Repeat(1));
    }

    #[test]
    fn delete_routine_returns_home() {
        let mut c = Console::default();
        c.open_editor(1);
        c.delete_routine(1);
        assert_eq!(c.view, View::Home);
        assert_eq!(c.routines.len(), 2);
    }

    #[test]
    fn routines_round_trip_through_co_segment() {
        let mut c = Console::default();
        c.unlocked.insert(Stratum::Schematics);
        c.unlocked.insert(Stratum::Rites);
        // Author: drift → seek; collect → match(rare) → collect; add a when-decode routine.
        c.routines[0].body = vec![Step::Do(Block::Seek)];
        c.routines[2].body = vec![Step::Match(MatchField::Rare), Step::Do(Block::Collect)];
        c.toggle_routine(1); // survey off
        c.routines.push(Routine::new(
            "routine-4",
            Agent::Ship,
            Trigger::When(Cond {
                state: State::Data,
                min: 25,
            }),
            vec![Step::Repeat(3), Step::Do(Block::Decode)],
        ));
        let s = format!("s=1&{}&x=2", c.encode());
        let mut back = Console::default();
        back.restore(&s);
        assert_eq!(back.routines, c.routines);
        // Lenient: no co= leaves the givens.
        let mut d = Console::default();
        d.restore("s=1&x=2");
        assert_eq!(d.routines.len(), 3);
        assert_eq!(d.routines[0].body, vec![Step::Do(Block::Drift)]);
    }

    #[test]
    fn vocabulary_is_gated_by_comprehension() {
        let mut c = Console::default();
        // Starters available; nav (Schematics) + match (Rites) locked by default.
        assert!(c.is_unlocked(Block::Scan(ScanItem::Shards)) && c.is_unlocked(Block::Decode));
        assert!(!c.is_unlocked(Block::Seek));
        let vocab = c.vocabulary(Agent::Ship);
        assert!(!vocab.contains(&Step::Do(Block::Seek)));
        assert!(!vocab.contains(&Step::Match(MatchField::Rare)));
        c.unlocked.insert(Stratum::Schematics);
        c.unlocked.insert(Stratum::Rites);
        let vocab = c.vocabulary(Agent::Ship);
        assert!(vocab.contains(&Step::Do(Block::Seek)));
        assert!(vocab.contains(&Step::Match(MatchField::Rare)));
    }

    #[test]
    fn vocabulary_is_scoped_by_agent() {
        // G8b: ship vocab has scan/nav but not the survey-beam; foot vocab the reverse; collect &
        // hail are shared (in both). Decode everything so comprehension-gating isn't the cause.
        let mut c = Console::default();
        for s in Stratum::ALL {
            c.unlocked.insert(s);
        }
        let ship = c.vocabulary(Agent::Ship);
        let foot = c.vocabulary(Agent::Foot);
        assert!(ship.contains(&Step::Do(Block::Scan(ScanItem::Shards))));
        assert!(!ship.contains(&Step::Do(Block::FireBeam))); // beam is a foot block
        assert!(foot.contains(&Step::Do(Block::FireBeam)));
        assert!(foot.contains(&Step::Do(Block::Walk))); // walk is the foot nav block (G8c)
        assert!(Block::Walk.is_nav() && Block::Walk.agent() == Some(Agent::Foot));
        assert!(!ship.contains(&Step::Do(Block::Walk))); // ship doesn't walk
        assert!(!foot.contains(&Step::Do(Block::Seek))); // seek is a ship nav block
        for shared in [
            Step::Do(Block::Collect),
            Step::Do(Block::Hail),
            Step::Match(MatchField::Rare),
            Step::Repeat(2),
        ] {
            assert!(
                ship.contains(&shared) && foot.contains(&shared),
                "shared: {shared:?}"
            );
        }
    }

    #[test]
    fn agent_scopes_which_routines_tick() {
        // A foot routine doesn't run on the ship tick, and vice-versa; agent persists.
        let mut c = Console::default();
        let i = c.create_routine(Agent::Foot);
        c.routines[i].trigger = Trigger::OnScan;
        c.routines[i].body = vec![Step::Do(Block::Collect)];
        // Ship on-scan = the given `collect` (1); foot on-scan = the new routine (1).
        assert_eq!(c.on_scan_acts(Agent::Ship).len(), 1);
        assert_eq!(c.on_scan_acts(Agent::Foot).len(), 1);
        c.cycle_agent(); // editor is on routine i (just created) → flip foot→ship
        assert_eq!(c.on_scan_acts(Agent::Foot).len(), 0);
        assert_eq!(c.on_scan_acts(Agent::Ship).len(), 2);
        // Agent survives a co= round-trip.
        let mut back = Console::default();
        back.restore(&c.encode());
        assert_eq!(back.routines, c.routines);
    }

    #[test]
    fn hail_block_is_a_starter_in_vocab_and_round_trips() {
        // G8a: `hail` is a shared starter block (no comprehension gate), insertable, and persists.
        let c = Console::default();
        assert!(c.is_unlocked(Block::Hail));
        assert!(c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Hail)));
        let mut c = Console::default();
        c.routines[2].body = vec![Step::Do(Block::Hail)];
        let mut back = Console::default();
        back.restore(&c.encode());
        assert_eq!(back.routines[2].body, vec![Step::Do(Block::Hail)]);
    }

    #[test]
    fn home_cursor_resolves_routine_new_and_block() {
        let c = Console::default();
        assert_eq!(c.selected(), Sel::Routine(0));
        let mut c = Console::default();
        c.cursor = c.routines.len(); // the "new routine" row
        assert_eq!(c.selected(), Sel::NewRoutine);
        c.cursor = c.routines.len() + 1; // first palette block
        assert!(matches!(c.selected(), Sel::Block(_)));
    }
}
