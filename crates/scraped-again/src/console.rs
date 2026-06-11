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

/// The item a `scan` block senses (G10: scan is honest now — `shards` senses the typed shard
/// items, `sites` senses inscription sites, each its own scannable).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ScanItem {
    Shards,
    Sites,
}

impl ScanItem {
    pub fn label(self) -> &'static str {
        match self {
            ScanItem::Shards => "shards",
            ScanItem::Sites => "sites",
        }
    }
}

/// A generic filter field for a `match(field)` step (game-system §11 Tier 1). v1: by *rarity*
/// (the collectible set is already uncollected, so the useful filter is "save the buffer for the
/// good stuff"). Recovered by comprehending **Rites**.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MatchField {
    /// Only rare strata (Relics + Signals) — and, for shards (G10), only rare rarity.
    Rare,
    /// G10: only one shard **domain** (the five strata; also filters inscriptions by stratum).
    Domain(Stratum),
}

impl MatchField {
    pub fn label(self) -> &'static str {
        match self {
            MatchField::Rare => "rare",
            MatchField::Domain(d) => match d {
                Stratum::Records => "records",
                Stratum::Schematics => "schematics",
                Stratum::Rites => "rites",
                Stratum::Relics => "relics",
                Stratum::Signals => "signals",
            },
        }
    }
    /// The stratum whose comprehension recovers the `match` step.
    pub fn required(self) -> Stratum {
        Stratum::Rites
    }

    /// G12: the filter argument as glyphs — a world-vocabulary item: the rarity tier (in the
    /// `match` step's own Rites script), or a stratum **domain** rendered in *that stratum's own*
    /// script so it reads as that family's glyphs. Mapped to overlay codepoints for the flat HUD.
    pub fn glyphs(self) -> String {
        use crate::progress::script_for;
        let (word, script) = match self {
            MatchField::Rare => ("rare", script_for(self.required())),
            MatchField::Domain(d) => (self.label(), script_for(d)),
        };
        crate::text::to_overlay(&crate::structures::transliterate(word, script), script)
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
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Block {
    Scan(ScanItem),                  // sense sites of `item` in the forward cone (G3)
    Collect,                         // collect the aimed/nearest in-reach site (G1)
    FireBeam,                        // cast the survey-beam (G2)
    Decode,                          // decode/comprehend the richest affordable stratum (G6)
    Spend(crate::progress::Faculty), // spend banked shards on a faculty level (G10)
    Goto,    // direct travel to a picked map target (vocabulary stub — no map picker yet)
    Drift,   // aimless cinematic wander — the default autopilot
    Seek,    // head to the nearest known-uncollected site (ship nav)
    Circle,  // loiter / orbit the current area (ship nav)
    Walk,    // on foot, walk to the nearest known site (foot nav — G8c)
    Hail,    // recall the autonomous/parked ship to the walker (G8a — foot/shared)
    RunFoot, // cross-agent: deploy the walker to collect on foot (G8c — the expedition)
}

impl Block {
    /// The full block vocabulary (G9: every one of these has a **name** findable in the world).
    pub const ALL: [Block; 15] = [
        Block::Scan(ScanItem::Shards),
        Block::Scan(ScanItem::Sites),
        Block::Collect,
        Block::FireBeam,
        Block::Decode,
        Block::Spend(crate::progress::Faculty::Sensing),
        Block::Spend(crate::progress::Faculty::Reach),
        Block::Spend(crate::progress::Faculty::Drive),
        Block::Goto,
        Block::Drift,
        Block::Seek,
        Block::Circle,
        Block::Walk,
        Block::Hail,
        Block::RunFoot,
    ];

    /// The block's bare **name** (no parameter sugar) — what an in-world name-inscription spells
    /// (G9) and what the HUD announces on discovery. Distinct across [`Block::ALL`] (tested).
    pub fn name(self) -> &'static str {
        match self {
            Block::Scan(_) => "scan",
            Block::Collect => "collect",
            Block::FireBeam => "beam",
            Block::Decode => "decode",
            Block::Spend(_) => "spend",
            Block::Goto => "goto",
            Block::Drift => "drift",
            Block::Seek => "seek",
            Block::Circle => "circle",
            Block::Walk => "walk",
            Block::Hail => "hail",
            Block::RunFoot => "runfoot",
        }
    }

    /// A stable byte for the `pg=` discovery payload (G9). Append-only — never renumber.
    pub fn code(self) -> u8 {
        match self {
            Block::Scan(_) => 0,
            Block::Collect => 1,
            Block::FireBeam => 2,
            Block::Decode => 3,
            Block::Spend(_) => 4, // the spend family shares one code (starter: never persisted)
            Block::Goto => 5,
            Block::Drift => 6,
            Block::Seek => 7,
            Block::Circle => 8,
            Block::Walk => 9,
            Block::Hail => 10,
            Block::RunFoot => 11,
        }
    }

    /// Inverse of [`Block::code`] (lenient: unknown bytes → `None`, so old/new payloads coexist).
    pub fn from_code(b: u8) -> Option<Block> {
        Block::ALL.iter().copied().find(|x| x.code() == b)
    }

    pub fn label(self) -> &'static str {
        match self {
            Block::Scan(i) => match i {
                ScanItem::Shards => "scan(shards)",
                ScanItem::Sites => "scan(sites)",
            },
            Block::Collect => "collect",
            Block::FireBeam => "fire-beam",
            Block::Decode => "decode",
            Block::Spend(f) => match f {
                crate::progress::Faculty::Sensing => "spend(sensing)",
                crate::progress::Faculty::Reach => "spend(reach)",
                crate::progress::Faculty::Drive => "spend(drive)",
            },
            Block::Goto => "goto(area)",
            Block::Drift => "drift",
            Block::Seek => "seek(uncollected)",
            Block::Circle => "circle",
            Block::Walk => "walk(uncollected)",
            Block::Hail => "hail",
            Block::RunFoot => "run(foot)",
        }
    }

    /// G12: the block's **glyph-name** — its bare [`name`](Self::name) transliterated into its
    /// stratum's script and mapped to the self-identifying overlay codepoints the flat HUD renders
    /// (via [`crate::text::to_overlay`]). Visually the *exact* glyph cluster the player finds carved
    /// in the world for this block (same `transliterate` + script — the world↔console recognition
    /// loop), recognisable *as a symbol* though unreadable *as a word*. This is the player-facing
    /// identity (palette, routine rows, codex, discovery toast); [`name`](Self::name)/
    /// [`label`](Self::label) stay for internal codes/tests/the `co=` codec.
    pub fn glyphs(self) -> String {
        let script = crate::structures::block_script(self);
        crate::text::to_overlay(
            &crate::structures::transliterate(self.name(), script),
            script,
        )
    }

    /// G12: the full player-facing form — the glyph-name plus, for a parameterised block, its
    /// world-vocabulary argument rendered glyph too (the scan target, the spent faculty), in the
    /// block's own stratum script. Structural punctuation (the parens) stays; pure quantities
    /// never appear here.
    pub fn glyph_label(self) -> String {
        let arg = match self {
            Block::Scan(i) => Some(i.label()),
            Block::Spend(f) => Some(f.label()),
            _ => None,
        };
        match arg {
            Some(a) => {
                let script = crate::structures::block_script(self);
                let arg =
                    crate::text::to_overlay(&crate::structures::transliterate(a, script), script);
                format!("{}({})", self.glyphs(), arg)
            }
            None => self.glyphs(),
        }
    }

    /// The stratum whose **comprehension** (G6 `decode`) recovers this block — `None` = a starter
    /// (Tier 0). This is the "tree": decoding grows the vocabulary (game-system §4).
    pub fn required(self) -> Option<Stratum> {
        match self {
            Block::Seek | Block::Circle | Block::Goto => Some(Stratum::Schematics),
            // The cross-agent expedition is deep meta — gated on a rare stratum (game-system §11
            // Tier 3).
            Block::RunFoot => Some(Stratum::Relics),
            _ => None, // scan/collect/fire-beam/decode/drift/spend/walk/hail are starters
        }
    }

    /// Wired to a real effect? (`goto` is the remaining vocabulary stub.)
    pub fn wired(self) -> bool {
        !matches!(self, Block::Goto)
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
            Block::Scan(_)
            | Block::Drift
            | Block::Seek
            | Block::Circle
            | Block::Goto
            | Block::RunFoot => Some(Agent::Ship),
            Block::FireBeam | Block::Walk => Some(Agent::Foot),
            Block::Collect | Block::Decode | Block::Spend(_) | Block::Hail => None, // shared
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
    /// G12: player-facing rendering — block + world-item vocabulary as glyphs; the structural
    /// modifiers (`match`/`repeat`) stay minimal-English instrumentation, only their vocabulary
    /// argument goes glyph.
    pub fn glyph_label(self) -> String {
        match self {
            Step::Do(b) => b.glyph_label(),
            Step::Match(f) => format!("match({})", f.glyphs()),
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

/// A state the `when` trigger can test: the total banked **data** (all strata), or the shard
/// **bank** (G10) — so `when(shards ≥ N) → spend(…)` is wireable.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum State {
    Data,
    Shards,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Data => "data",
            State::Shards => "shards",
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
    fn holds(self, data: u32, shards: u32) -> bool {
        match self.state {
            State::Data => data >= self.min,
            State::Shards => shards >= self.min,
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

/// G11: why a routine isn't producing right now — only reasons the interpreter/app actually
/// evaluated (the honesty rule: never invent a diagnosis).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BlockReason {
    /// The body acted but nothing was in reach to take.
    NothingInReach,
    /// A `match` filter excluded everything in reach.
    NoMatch,
    /// The body contains a step that isn't unlocked (discovered + decoded) yet.
    LockedStep,
}

impl BlockReason {
    pub fn label(self) -> &'static str {
        match self {
            BlockReason::NothingInReach => "nothing in reach",
            BlockReason::NoMatch => "no match",
            BlockReason::LockedStep => "locked step",
        }
    }
}

/// G11: a routine's live execution state, derived each tick (pure — tested as a matrix).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum RState {
    Disabled,
    /// The trigger fired/held this tick and the body ran.
    Running,
    /// Enabled, waiting on its trigger (the label says which).
    #[default]
    Waiting,
    /// The trigger fires but the body can't produce — with the honest reason.
    Blocked(BlockReason),
}

/// G11: per-routine **telemetry** — session-local working memory (never persisted; excluded
/// from equality + `co=`). Counters are increments; no allocation on the tick path.
#[derive(Clone, Debug, Default)]
pub struct RoutineStats {
    /// Total trigger fires (continuous: ticks run; when/on-arrive: edge fires; on-scan: hits).
    pub fires: u32,
    /// Items + data credited to this routine's collects.
    pub items: u32,
    pub yields: u64,
    /// `console.now` when the routine last fired.
    pub last_fired: Option<f32>,
    /// The body step index that executed most recently this tick (the live highlight).
    pub executing_step: Option<usize>,
    /// Current derived state.
    pub state: RState,
    /// Yield-rate window: (window start time, yields at window start).
    pub window: Option<(f32, u64)>,
}

impl RoutineStats {
    /// Yield/hour over the live window — `None` (render `—`) until ~10 s of data exist.
    pub fn rate_per_hour(&self, now: f32) -> Option<f32> {
        let (t0, y0) = self.window?;
        let dt = now - t0;
        if dt < 10.0 {
            return None;
        }
        Some((self.yields.saturating_sub(y0)) as f32 / dt * 3600.0)
    }
    /// Terse row suffix: `run 412 · y 1.2k · <state>` (the console list's at-a-glance line).
    pub fn suffix(&self, now: f32) -> String {
        let state = match self.state {
            RState::Disabled => "off".to_string(),
            RState::Running => "run".to_string(),
            RState::Waiting => "wait".to_string(),
            RState::Blocked(r) => format!("blk:{}", r.label()),
        };
        match self.rate_per_hour(now) {
            Some(r) => format!("×{} · y{} · {r:.0}/h · {state}", self.fires, self.yields),
            None => format!("×{} · y{} · {state}", self.fires, self.yields),
        }
    }
}

/// A routine: **data** the interpreter runs. No per-name behaviour — the givens are just the
/// default instances. (`armed` + `stats` are transient runtime state, excluded from equality.)
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
    /// G11: live telemetry (runtime only; not persisted).
    pub stats: RoutineStats,
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
            stats: RoutineStats::default(),
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

/// One resolved action the app dispatches: a block plus the match-filter context it runs under,
/// and (G11) the routine it came from — so collect outcomes credit back to their author.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Act {
    pub block: Block,
    pub filter: Option<MatchField>,
    /// Index of the originating routine (G11 telemetry attribution).
    pub routine: usize,
}

/// This tick's intents, produced by the interpreter from the enabled routines. The app applies
/// them through the existing G1–G6 effect paths (replacing the old named-accessor hacks).
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Tick {
    /// Autopilot steering from a continuous nav routine (`None` ⇒ no drift routine ⇒ autopilot off).
    pub nav: Option<Block>,
    /// A continuous routine wants to pulse the **site** scan (throttled to `scan::INTERVAL`).
    pub scan: bool,
    /// G10: a continuous routine wants to pulse the **shard** scan (same throttle).
    pub scan_shards: bool,
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
    /// Discovered gated blocks (G9; synced from `progress`) — a block's name must be found in the
    /// world before it's even *listed*. Starters are implicitly discovered (see
    /// [`Console::is_discovered`]), so this only carries the gated vocabulary.
    pub discovered: HashSet<Block>,
    /// G11: the app's clock (set each frame before `tick`) — drives last-fired age + yield rates.
    pub now: f32,
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
                    // G10 opening parity: the given survey scans inscription *sites* exactly as
                    // it always effectively did (G3 autoscan + map pins unchanged)…
                    vec![Step::Do(Block::Scan(ScanItem::Sites))],
                ),
                Routine::new(
                    "prospect",
                    Agent::Ship,
                    Trigger::Continuous,
                    // …and a fourth given scans the new typed shards, so the opening hands-off
                    // loop collects both (brief Decision 4: two givens is fine — they're data).
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
                Block::Scan(ScanItem::Sites),
                Block::Collect,
                Block::FireBeam,
                Block::Decode,
                Block::Spend(crate::progress::Faculty::Sensing),
                Block::Spend(crate::progress::Faculty::Reach),
                Block::Spend(crate::progress::Faculty::Drive),
                Block::Goto,
                Block::Drift,
            ],
            unlocked: HashSet::new(),
            discovered: HashSet::new(),
            now: 0.0,
        }
    }
}

impl Console {
    // ---- vocabulary gating ------------------------------------------------------------------

    /// G11: set the telemetry clock (the app's `time`, each frame before `tick`).
    pub fn set_now(&mut self, now: f32) {
        self.now = now;
    }

    /// Is this block **discovered** (G9) — its name found in the world? Starters always;
    /// gated blocks only once a name-bearing inscription was collected. Undiscovered blocks are
    /// *absent* from every listing (you can't covet what you've never read).
    pub fn is_discovered(&self, b: Block) -> bool {
        b.required().is_none() || self.discovered.contains(&b)
    }

    /// Usable? **Two-stage** (G9): the name must be *discovered* AND its stratum *decoded*.
    /// (Starters pass both implicitly, so the opening + given routines are untouched.)
    pub fn is_unlocked(&self, b: Block) -> bool {
        self.is_discovered(b) && b.required().is_none_or(|s| self.unlocked.contains(&s))
    }

    fn step_unlocked(&self, s: Step) -> bool {
        let discovered = match s {
            Step::Do(b) => self.is_discovered(b),
            _ => true, // the match/repeat modifiers aren't name-gated
        };
        discovered && s.required().is_none_or(|st| self.unlocked.contains(&st))
    }

    /// The steps `agent` may insert / cycle to, in cycle order — filtered to what's **discovered**
    /// (G9: an unfound name is absent) **and** recovered (G6 decode) **and** scoped to the agent's
    /// context + shared blocks (game-system §7). (Blocks, then the `match` modifier, then `repeat`.)
    pub fn vocabulary(&self, agent: Agent) -> Vec<Step> {
        let all = [
            Step::Do(Block::Scan(ScanItem::Shards)),
            Step::Do(Block::Scan(ScanItem::Sites)),
            Step::Do(Block::Collect),
            Step::Do(Block::FireBeam),
            Step::Do(Block::Decode),
            Step::Do(Block::Drift),
            Step::Do(Block::Seek),
            Step::Do(Block::Circle),
            Step::Do(Block::Walk),
            Step::Do(Block::Goto),
            Step::Do(Block::Spend(crate::progress::Faculty::Sensing)),
            Step::Do(Block::Spend(crate::progress::Faculty::Reach)),
            Step::Do(Block::Spend(crate::progress::Faculty::Drive)),
            Step::Do(Block::Hail),
            Step::Do(Block::RunFoot),
            Step::Match(MatchField::Rare),
            Step::Match(MatchField::Domain(Stratum::Records)),
            Step::Match(MatchField::Domain(Stratum::Schematics)),
            Step::Match(MatchField::Domain(Stratum::Rites)),
            Step::Match(MatchField::Domain(Stratum::Relics)),
            Step::Match(MatchField::Domain(Stratum::Signals)),
            Step::Repeat(2),
        ];
        all.into_iter()
            .filter(|s| self.step_unlocked(*s) && step_for_agent(*s, agent))
            .collect()
    }

    // ---- the interpreter --------------------------------------------------------------------

    /// Expand a body into resolved acts: `match` sets the filter for following Collects;
    /// `repeat(n)` multiplies the next `Do`. `routine` tags each act for telemetry attribution.
    fn expand(body: &[Step], routine: usize) -> Vec<Act> {
        let mut out = Vec::new();
        let mut filter = None;
        let mut times: u8 = 1;
        for step in body {
            match step {
                Step::Match(f) => filter = Some(*f),
                Step::Repeat(n) => times = (*n).max(1),
                Step::Do(b) => {
                    for _ in 0..times {
                        out.push(Act {
                            block: *b,
                            filter,
                            routine,
                        });
                    }
                    times = 1;
                }
            }
        }
        out
    }

    /// Run one interpreter tick for `agent`'s **continuous** / **when** / **on-arrive** routines,
    /// given the current `data` (total banked strata) + `shards` (the G10 bank) for `when`
    /// conditions and whether the agent has just `arrived` at the site it's heading to. Returns
    /// this tick's [`Tick`] intents. (`on-scan` routines fire on a scan hit — [`Console::on_scan_acts`].)
    pub fn tick(&mut self, agent: Agent, data: u32, shards: u32, arrived: bool) -> Tick {
        let mut t = Tick::default();
        let now = self.now;
        for (idx, r) in self.routines.iter_mut().enumerate() {
            if r.agent != agent {
                continue;
            }
            if !r.enabled {
                r.armed = false;
                r.stats.state = RState::Disabled;
                r.stats.executing_step = None;
                continue;
            }
            // G11: telemetry — a routine with an unlocked-gate problem is honestly `blocked`.
            let has_locked = r.body.iter().any(|st| {
                matches!(st, Step::Do(b) if {
                    let disc = b.required().is_none() || self.discovered.contains(b);
                    !(disc && b.required().is_none_or(|s| self.unlocked.contains(&s)))
                })
            });
            let mut fired = false;
            match r.trigger {
                Trigger::Continuous => {
                    fired = true;
                    for act in Self::expand(&r.body, idx) {
                        if act.block.is_nav() {
                            t.nav = Some(act.block);
                        } else if matches!(act.block, Block::Scan(ScanItem::Sites)) {
                            t.scan = true;
                        } else if matches!(act.block, Block::Scan(ScanItem::Shards)) {
                            t.scan_shards = true;
                        } else {
                            t.acts.push(act);
                        }
                    }
                }
                Trigger::OnScan => {} // fired on a scan hit, not per tick
                Trigger::When(c) => {
                    let sat = c.holds(data, shards);
                    if sat && !r.armed {
                        fired = true;
                        t.acts.extend(Self::expand(&r.body, idx)); // rising edge → fire once
                    }
                    r.armed = sat;
                }
                Trigger::OnArrive => {
                    if arrived && !r.armed {
                        fired = true;
                        t.acts.extend(Self::expand(&r.body, idx)); // reached the site → fire
                    }
                    r.armed = arrived;
                }
            }
            // G11: derive the state + bump counters (pure increments — no allocation).
            if fired {
                r.stats.fires = r.stats.fires.saturating_add(1);
                r.stats.last_fired = Some(now);
                r.stats.window.get_or_insert((now, r.stats.yields));
                // The live step highlight: the last Do that executed this tick.
                r.stats.executing_step = r.body.iter().rposition(|st| matches!(st, Step::Do(_)));
            } else {
                r.stats.executing_step = None;
            }
            r.stats.state = if has_locked {
                RState::Blocked(BlockReason::LockedStep)
            } else if fired {
                RState::Running
            } else {
                RState::Waiting
            };
        }
        t
    }

    /// G11: credit a resolved collect back to its routine (`items` taken, `yields` banked).
    /// Zero items downgrades the routine's state to the honest blocked-reason.
    pub fn credit(&mut self, routine: usize, items: u32, yields: u64, filtered: bool) {
        let Some(r) = self.routines.get_mut(routine) else {
            return;
        };
        if items > 0 {
            r.stats.items += items;
            r.stats.yields += yields;
        } else if matches!(r.stats.state, RState::Running) {
            r.stats.state = RState::Blocked(if filtered {
                BlockReason::NoMatch
            } else {
                BlockReason::NothingInReach
            });
        }
    }

    /// G11: an on-scan routine fired (a scan hit ran its acts) — count it.
    pub fn note_scan_fire(&mut self, routine: usize) {
        let now = self.now;
        if let Some(r) = self.routines.get_mut(routine) {
            r.stats.fires = r.stats.fires.saturating_add(1);
            r.stats.last_fired = Some(now);
            r.stats.window.get_or_insert((now, r.stats.yields));
            r.stats.state = RState::Running;
            r.stats.executing_step = r.body.iter().rposition(|st| matches!(st, Step::Do(_)));
        }
    }

    /// The acts `agent`'s enabled **on-scan** routines want, when a scan finds something (typically
    /// a filtered collect). Walked through the same interpreter as everything else.
    pub fn on_scan_acts(&self, agent: Agent) -> Vec<Act> {
        self.routines
            .iter()
            .enumerate()
            .filter(|(_, r)| r.agent == agent && r.enabled && matches!(r.trigger, Trigger::OnScan))
            .flat_map(|(i, r)| Self::expand(&r.body, i))
            .collect()
    }

    // ---- home navigation --------------------------------------------------------------------

    /// The home screen's **visible** block listing (G9): the palette's discovered entries, plus
    /// any *other* discovered gated blocks (a found name appears here, dimmed until decoded).
    /// Undiscovered blocks are absent entirely.
    pub fn visible_palette(&self) -> Vec<Block> {
        let mut out: Vec<Block> = self
            .palette
            .iter()
            .copied()
            .filter(|b| self.is_discovered(*b))
            .collect();
        for b in Block::ALL {
            if b.required().is_some() && self.is_discovered(b) && !out.contains(&b) {
                out.push(b);
            }
        }
        out
    }

    /// Home rows: routines (toggle/edit), a "new routine" row, then the visible block listing.
    pub fn home_rows(&self) -> usize {
        self.routines.len() + 1 + self.visible_palette().len()
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
            Sel::Block(self.visible_palette()[self.cursor - nr - 1])
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
            Trigger::When(Cond {
                state: State::Shards,
                min: 25,
            }),
            Trigger::OnArrive,
        ];
        // Match the current trigger by `when`-STATE (min is user-set, so compare the state, not
        // the value), else by discriminant — so both `when` kinds are reachable in the cycle.
        let cur_t = self.routines[r].trigger;
        let cur = kinds
            .iter()
            .position(|k| match (k, &cur_t) {
                (Trigger::When(a), Trigger::When(b)) => a.state == b.state,
                _ => std::mem::discriminant(k) == std::mem::discriminant(&cur_t),
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
        // Find the current step in the vocabulary by full value first (so parameterised families
        // — scan items, spend faculties, match domains — cycle through their variants), falling
        // back to *kind* (so a Repeat(5) still matches the Repeat slot).
        let pos = vocab
            .iter()
            .position(|v| *v == cur)
            .or_else(|| {
                vocab
                    .iter()
                    .position(|v| std::mem::discriminant(v) == std::mem::discriminant(&cur))
            })
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
            Step::Do(Block::Scan(ScanItem::Sites)) => "T".into(),
            Step::Do(Block::Collect) => "C".into(),
            Step::Do(Block::FireBeam) => "B".into(),
            Step::Do(Block::Decode) => "D".into(),
            Step::Do(Block::Spend(crate::progress::Faculty::Sensing)) => "p".into(),
            Step::Do(Block::Spend(crate::progress::Faculty::Reach)) => "P".into(),
            Step::Do(Block::Spend(crate::progress::Faculty::Drive)) => "q".into(),
            Step::Do(Block::Goto) => "g".into(),
            Step::Do(Block::Drift) => "d".into(),
            Step::Do(Block::Seek) => "k".into(),
            Step::Do(Block::Circle) => "o".into(),
            Step::Do(Block::Walk) => "W".into(),
            Step::Do(Block::Hail) => "H".into(),
            Step::Do(Block::RunFoot) => "R".into(),
            Step::Match(MatchField::Rare) => "m".into(),
            Step::Match(MatchField::Domain(d)) => format!(
                "M{}",
                match d {
                    Stratum::Records => '0',
                    Stratum::Schematics => '1',
                    Stratum::Rites => '2',
                    Stratum::Relics => '3',
                    Stratum::Signals => '4',
                }
            ),
            Step::Repeat(n) => format!("r{n}"),
        }
    }

    fn parse_steps(s: &str) -> Vec<Step> {
        let mut out = Vec::new();
        let mut it = s.chars().peekable();
        while let Some(c) = it.next() {
            let step = match c {
                'S' => Step::Do(Block::Scan(ScanItem::Shards)),
                'T' => Step::Do(Block::Scan(ScanItem::Sites)),
                'C' => Step::Do(Block::Collect),
                'B' => Step::Do(Block::FireBeam),
                'D' => Step::Do(Block::Decode),
                'p' => Step::Do(Block::Spend(crate::progress::Faculty::Sensing)),
                'P' => Step::Do(Block::Spend(crate::progress::Faculty::Reach)),
                'q' => Step::Do(Block::Spend(crate::progress::Faculty::Drive)),
                'g' => Step::Do(Block::Goto),
                'd' => Step::Do(Block::Drift),
                'k' => Step::Do(Block::Seek),
                'o' => Step::Do(Block::Circle),
                'W' => Step::Do(Block::Walk),
                'H' => Step::Do(Block::Hail),
                'R' => Step::Do(Block::RunFoot),
                'm' => Step::Match(MatchField::Rare),
                'M' => {
                    let d = match it.next() {
                        Some('1') => Stratum::Schematics,
                        Some('2') => Stratum::Rites,
                        Some('3') => Stratum::Relics,
                        Some('4') => Stratum::Signals,
                        _ => Stratum::Records, // '0' / missing
                    };
                    Step::Match(MatchField::Domain(d))
                }
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
            // `w:{min}` = when(data) (the pre-G10 form, kept); `wS:{min}` = when(shards).
            Trigger::When(c) => match c.state {
                State::Data => format!("w:{}", c.min),
                State::Shards => format!("wS:{}", c.min),
            },
            Trigger::OnArrive => "a".into(),
        }
    }

    fn parse_trigger(s: &str) -> Trigger {
        match s.chars().next() {
            Some('s') => Trigger::OnScan,
            Some('a') => Trigger::OnArrive,
            Some('w') => Trigger::When(Cond {
                state: if s.starts_with("wS") {
                    State::Shards
                } else {
                    State::Data // the pre-G10 `w:` form
                },
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
                stats: RoutineStats::default(),
            });
        }
        if !routines.is_empty() {
            self.routines = routines;
        }
    }

    /// G11: the HUD's **one lit goal** — the single nearest-to-done thing (≥ ~75%), priority by
    /// completion: a `when` threshold close to firing · an affordable-unbought faculty · a
    /// stratum nearly affordable to decode (named after a discovered-locked block wanting it,
    /// when one exists). Exactly one line; nothing qualifying → `None` (never a quest log).
    pub fn lit_goal(&self, p: &crate::progress::Progress) -> Option<String> {
        let mut best: Option<(f32, String)> = None;
        let mut consider = |pct: f32, label: String| {
            let pct = pct.min(1.0);
            if pct >= 0.75 && best.as_ref().is_none_or(|(b, _)| pct > *b) {
                best = Some((pct, label));
            }
        };
        // 1. `when` thresholds approaching their trigger (skip already-satisfied ones).
        let data = p.strata.total() as f32;
        let shards = p.shard_bank() as f32;
        for r in self.routines.iter().filter(|r| r.enabled) {
            if let Trigger::When(c) = r.trigger {
                if c.min == 0 || r.armed {
                    continue;
                }
                let cur = match c.state {
                    State::Data => data,
                    State::Shards => shards,
                };
                let pct = cur / c.min as f32;
                if pct < 1.0 {
                    consider(pct, format!("{} {:.0}%", r.name, pct * 100.0));
                }
            }
        }
        // 2. An affordable, unbought faculty (completion = 100%).
        for f in crate::progress::Faculty::ALL {
            let lvl = p.faculty_levels()[f.idx()];
            if lvl < crate::progress::MAX_FACULTY_LEVEL
                && p.shard_bank() >= crate::progress::FACULTY_COSTS[lvl as usize]
            {
                consider(1.0, format!("{} affordable", f.label()));
            }
        }
        // 3. A stratum nearing its decode cost — named after a discovered-locked block that
        //    wants it, when there is one (the "I've seen this name" pull).
        for st in Stratum::ALL {
            if p.is_comprehended(st) {
                continue;
            }
            let pct = p.strata.get(st) as f32 / crate::progress::DECODE_COST as f32;
            if pct >= 1.0 {
                // Fully affordable decode beats a near-threshold (completion = 1). G12: where the
                // goal names a block (the "I've seen this name" pull), that part goes glyph; the
                // `decode SCH ready` instrumentation stays minimal-English.
                let who = self
                    .discovered
                    .iter()
                    .find(|b| b.required() == Some(st))
                    .map(|b| format!("{}: ", b.glyphs()))
                    .unwrap_or_default();
                consider(1.0, format!("{who}decode {} ready", st.label()));
            } else {
                consider(pct, format!("decode {} {:.0}%", st.label(), pct * 100.0));
            }
        }
        best.map(|(_, label)| label)
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
            let steps: Vec<String> = r.body.iter().map(|b| b.glyph_label()).collect();
            let pipe = if steps.is_empty() {
                "—".to_string()
            } else {
                steps.join(" → ")
            };
            s.push_str(&format!(
                "{cur} [{on}] {:<4} {:<9} {:<11}: {}   {}\n",
                r.agent.label(),
                r.name,
                r.trigger.label(),
                pipe,
                r.stats.suffix(self.now), // G11: ×fires · yields · rate · state
            ));
            row += 1;
        }
        // G11: detail line for the selected routine (Decision 2 — details on selected only).
        if let Sel::Routine(i) = self.selected() {
            let st = &self.routines[i].stats;
            let age = st
                .last_fired
                .map(|t| format!("{:.0}s ago", (self.now - t).max(0.0)))
                .unwrap_or_else(|| "never".into());
            let rate = st
                .rate_per_hour(self.now)
                .map(|r| format!("{r:.0}/h"))
                .unwrap_or_else(|| "—".into());
            s.push_str(&format!(
                "    └ items {} · yield {} · rate {} · last fired {}\n",
                st.items, st.yields, rate, age
            ));
        }
        let cur = if row == self.cursor { ">" } else { " " };
        s.push_str(&format!("{cur} + new routine\n"));
        row += 1;
        s.push_str("blocks (Enter runs):\n");
        // G9: only *discovered* blocks are listed (a name you've read in the world); a discovered-
        // but-undecoded one renders **dimmed** with its stratum tag — the "I've seen this name"
        // tease. Undiscovered blocks are absent entirely.
        for b in self.visible_palette() {
            let cur = if row == self.cursor { ">" } else { " " };
            let tag = match b.required() {
                Some(st) if !self.unlocked.contains(&st) => {
                    format!("  (locked: decode {})", st.label())
                }
                _ if !b.wired() => "  (—)".to_string(),
                _ => String::new(),
            };
            // Dim a found-but-locked name (lowercase-dotted) so it reads as known-of, not usable.
            // G12: the block shows by its glyph-name; the `(locked: decode SCH)` tag stays
            // minimal-English instrumentation (a stratum gauge, not the block's vocabulary).
            if !self.is_unlocked(b) && b.required().is_some() {
                s.push_str(&format!("{cur} · {}{}\n", b.glyph_label(), tag));
            } else {
                s.push_str(&format!("{cur} {}{}\n", b.glyph_label(), tag));
            }
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
        // Body steps (G11: `▶` lights the step the interpreter executed this tick).
        for (si, step) in r.body.iter().enumerate() {
            let cur = if self.cursor == si + 1 { ">" } else { " " };
            let live = if r.stats.executing_step == Some(si) {
                "▶"
            } else {
                " "
            };
            s.push_str(&format!(
                "{cur} {live} {}. {}\n",
                si + 1,
                step.glyph_label()
            ));
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

    /// G12: a block's console glyph-name is the overlay form of the **exact** cluster a world
    /// name-inscription spells for it (same `transliterate` + stratum script) — the world↔console
    /// recognition loop. Stable, non-empty, every char HUD-renderable (never a fallback dot), and
    /// (across the unique names) distinct.
    #[test]
    fn block_glyphs_match_world_inscriptions() {
        use crate::structures;
        for b in Block::ALL {
            let script = structures::block_script(b);
            let world = structures::transliterate(b.name(), script);
            assert_eq!(
                b.glyphs(),
                crate::text::to_overlay(&world, script),
                "console glyphs must be the overlay of the world inscription for {b:?}"
            );
            assert_eq!(b.glyphs(), b.glyphs(), "deterministic");
            assert!(!b.glyphs().is_empty());
            // Every glyph the console emits is renderable by the HUD (no fallback dots).
            for c in b.glyphs().chars() {
                assert!(
                    c.is_ascii_graphic() || crate::text::overlay_glyph(c).is_some(),
                    "char {c:?} of {b:?} glyphs is not HUD-renderable"
                );
            }
        }
        // Parameterised families share a name → share a glyph cluster (by design).
        use std::collections::{HashMap, HashSet};
        let mut by_name: HashMap<&str, String> = HashMap::new();
        for b in Block::ALL {
            if let Some(prev) = by_name.insert(b.name(), b.glyphs()) {
                assert_eq!(prev, b.glyphs(), "same name ⇒ identical glyphs");
            }
        }
        // Distinct *names* produce distinct clusters (collision guard, G9 spirit).
        let clusters: HashSet<String> = Block::ALL.iter().map(|b| b.glyphs()).collect();
        let names: HashSet<&str> = Block::ALL.iter().map(|b| b.name()).collect();
        assert_eq!(
            clusters.len(),
            names.len(),
            "distinct block names ⇒ distinct glyph clusters"
        );
    }

    /// G12: a parameterised block's `glyph_label` is its glyphs plus its argument rendered glyph
    /// (so the two `scan` variants read distinctly); a plain block is just its glyphs.
    #[test]
    fn glyph_label_renders_parameter_as_glyphs() {
        let shards = Block::Scan(ScanItem::Shards);
        let lbl = shards.glyph_label();
        assert!(lbl.starts_with(&shards.glyphs()) && lbl.ends_with(')') && lbl.contains('('));
        assert_ne!(
            Block::Scan(ScanItem::Shards).glyph_label(),
            Block::Scan(ScanItem::Sites).glyph_label(),
            "the scan variants must read distinctly"
        );
        assert_eq!(Block::Drift.glyph_label(), Block::Drift.glyphs());
        // A `match` step glyphs its field but keeps the structural keyword (instrumentation).
        let m = Step::Match(MatchField::Rare).glyph_label();
        assert!(m.starts_with("match(") && m.ends_with(')'));
    }

    #[test]
    fn given_routines_are_plain_data() {
        let c = Console::default();
        // Four givens (G10 added `prospect`), all ordinary instances (no per-name behaviour).
        assert_eq!(c.routines.len(), 4);
        let drift = &c.routines[0];
        assert_eq!(drift.name, "drift");
        assert!(drift.is_nav());
        assert_eq!(drift.trigger, Trigger::Continuous);
        let survey = &c.routines[1];
        assert_eq!(survey.trigger, Trigger::Continuous);
        assert_eq!(survey.body, vec![Step::Do(Block::Scan(ScanItem::Sites))]); // opening parity
        let prospect = &c.routines[2];
        assert_eq!(prospect.trigger, Trigger::Continuous);
        assert_eq!(prospect.body, vec![Step::Do(Block::Scan(ScanItem::Shards))]);
        let collect = &c.routines[3];
        assert_eq!(collect.trigger, Trigger::OnScan);
        assert_eq!(collect.body, vec![Step::Do(Block::Collect)]);
    }

    #[test]
    fn interpreter_runs_the_givens() {
        let mut c = Console::default();
        let t = c.tick(Agent::Ship, 0, 0, false);
        // drift → nav steering; survey → scan request; collect is on-scan (not in the tick).
        assert_eq!(t.nav, Some(Block::Drift));
        assert!(t.scan);
        assert!(t.acts.is_empty());
        // on-scan collect, no filter.
        assert_eq!(
            c.on_scan_acts(Agent::Ship),
            vec![Act {
                block: Block::Collect,
                filter: None,
                routine: 3, // the given `collect`
            }]
        );
    }

    #[test]
    fn disabling_drift_drops_the_nav_intent() {
        let mut c = Console::default();
        c.toggle_routine(0); // drift off
        let t = c.tick(Agent::Ship, 0, 0, false);
        assert_eq!(t.nav, None); // no continuous nav routine ⇒ autopilot off
        assert!(t.scan); // survey still scans
    }

    #[test]
    fn authored_match_gated_collect_filters() {
        let mut c = Console::default();
        c.unlocked.insert(Stratum::Rites); // recover match()
                                           // Author the collect routine to "match(rare) → collect".
        c.routines[3].body = vec![Step::Match(MatchField::Rare), Step::Do(Block::Collect)];
        assert_eq!(
            c.on_scan_acts(Agent::Ship),
            vec![Act {
                block: Block::Collect,
                filter: Some(MatchField::Rare),
                routine: 3,
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
        let acts = Console::expand(&body, 0);
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
        assert!(c.tick(Agent::Ship, 5, 0, false).acts.is_empty()); // below threshold
        let fired = c.tick(Agent::Ship, 12, 0, false); // crosses ⇒ fires once
        assert_eq!(
            fired.acts,
            vec![Act {
                block: Block::Decode,
                filter: None,
                routine: 4, // the appended when-routine
            }]
        );
        assert!(c.tick(Agent::Ship, 12, 0, false).acts.is_empty()); // still high ⇒ no re-fire (edge, not level)
        c.tick(Agent::Ship, 0, 0, false); // drop below ⇒ re-arm
        assert!(!c.tick(Agent::Ship, 20, 0, false).acts.is_empty()); // crosses again ⇒ fires
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
        assert!(c.tick(Agent::Ship, 0, 0, false).acts.is_empty()); // not arrived
        let fired = c.tick(Agent::Ship, 0, 0, true); // reaches a site → fires once
        assert!(fired.acts.iter().any(|a| a.block == Block::Decode));
        assert!(c.tick(Agent::Ship, 0, 0, true).acts.is_empty()); // still there ⇒ no re-fire
        c.tick(Agent::Ship, 0, 0, false); // leaves ⇒ re-arm
        assert!(!c.tick(Agent::Ship, 0, 0, true).acts.is_empty()); // arrives again ⇒ fires
                                                                   // The trigger round-trips through `co=`.
        let mut back = Console::default();
        back.restore(&c.encode());
        assert_eq!(back.routines, c.routines);
    }

    #[test]
    fn create_insert_remove_reorder() {
        let mut c = Console::default();
        let i = c.create_routine(Agent::Ship);
        assert_eq!(c.routines.len(), 5);
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
        // Two-stage (G9): decoding Schematics alone is no longer enough — seek's *name* must
        // also have been found in the world.
        c.unlocked.insert(Stratum::Schematics);
        assert!(!c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Seek)));
        c.discovered.insert(Block::Seek);
        assert!(c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Seek)));
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
        assert_eq!(c.routines.len(), 3);
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
        // G10: a shards-gated spend routine round-trips too (new step codes).
        let mut c2 = Console::default();
        c2.routines.push(Routine::new(
            "auto-spend",
            Agent::Ship,
            Trigger::When(Cond {
                state: State::Shards,
                min: 25,
            }),
            vec![
                Step::Do(Block::Spend(crate::progress::Faculty::Reach)),
                Step::Do(Block::Scan(ScanItem::Sites)),
                Step::Match(MatchField::Domain(Stratum::Relics)),
            ],
        ));
        let mut back2 = Console::default();
        back2.restore(&c2.encode());
        assert_eq!(back2.routines, c2.routines);
        // Lenient: no co= leaves the givens (4 since G10's `prospect`).
        let mut d = Console::default();
        d.restore("s=1&x=2");
        assert_eq!(d.routines.len(), 4);
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
        // G9 two-stage: the decode alone unlocks `match` (a modifier — not name-gated) but a
        // *block* additionally needs its name discovered.
        let vocab = c.vocabulary(Agent::Ship);
        assert!(!vocab.contains(&Step::Do(Block::Seek)));
        assert!(vocab.contains(&Step::Match(MatchField::Rare)));
        c.discovered.insert(Block::Seek);
        assert!(c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Seek)));
    }

    #[test]
    fn discovery_states_absent_dimmed_insertable() {
        // G9: the three console visibility states for a gated block (`seek`).
        let mut c = Console::default();
        // 1. Undiscovered → absent from the home listing AND the editor vocabulary.
        assert!(!c.visible_palette().contains(&Block::Seek));
        assert!(!c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Seek)));
        // 2. Discovered (name found) but stratum undecoded → listed, locked (not insertable).
        c.discovered.insert(Block::Seek);
        assert!(c.visible_palette().contains(&Block::Seek));
        assert!(!c.is_unlocked(Block::Seek));
        assert!(!c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Seek)));
        let home = c.render_home();
        // G12: the discovered name is listed by its **glyph** cluster (not the English "seek"),
        // still tagged with its stratum (instrumentation stays English).
        assert!(
            home.contains(&Block::Seek.glyphs()),
            "discovered name should be listed as its glyphs"
        );
        assert!(
            !home.contains("seek"),
            "the English block name must not appear in the console"
        );
        assert!(
            home.contains("locked: decode SCH"),
            "and tagged with its stratum"
        );
        // 3. Discovered + decoded → insertable.
        c.unlocked.insert(Stratum::Schematics);
        assert!(c.is_unlocked(Block::Seek));
        assert!(c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Seek)));
        // Starters were never gated: present in the listing + vocabulary from the start.
        let fresh = Console::default();
        assert!(fresh.visible_palette().contains(&Block::Collect));
        assert!(fresh
            .vocabulary(Agent::Ship)
            .contains(&Step::Do(Block::Scan(ScanItem::Shards))));
        // The undiscovered gated palette entry (goto) is absent on a fresh console.
        assert!(!fresh.visible_palette().contains(&Block::Goto));
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
    fn run_foot_is_a_rare_gated_ship_block() {
        // G8c: the cross-agent `run(foot)` is a ship block, gated on the rare Relics stratum, and
        // round-trips through `co=`.
        let mut c = Console::default();
        assert_eq!(Block::RunFoot.agent(), Some(Agent::Ship));
        assert_eq!(Block::RunFoot.required(), Some(Stratum::Relics));
        assert!(!c.is_unlocked(Block::RunFoot)); // locked until Relics decoded
        assert!(!c
            .vocabulary(Agent::Ship)
            .contains(&Step::Do(Block::RunFoot)));
        c.unlocked.insert(Stratum::Relics);
        c.discovered.insert(Block::RunFoot); // G9: its name must also be found
        assert!(c
            .vocabulary(Agent::Ship)
            .contains(&Step::Do(Block::RunFoot)));
        c.routines[2].body = vec![Step::Do(Block::RunFoot)];
        let mut back = Console::default();
        back.restore(&c.encode());
        assert_eq!(back.routines[2].body, vec![Step::Do(Block::RunFoot)]);
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
    fn telemetry_state_matrix() {
        // G11: the honest state machine — disabled / running / waiting / blocked(reason).
        let mut c = Console::default();
        // Disabled.
        c.toggle_routine(0); // drift off
        c.tick(Agent::Ship, 0, 0, false);
        assert_eq!(c.routines[0].stats.state, RState::Disabled);
        // Continuous + enabled → running, fires count, step highlight on its Do.
        assert_eq!(c.routines[1].stats.state, RState::Running); // survey
        assert!(c.routines[1].stats.fires >= 1);
        assert_eq!(c.routines[1].stats.executing_step, Some(0));
        // `when` below threshold → waiting; crossing → running once.
        c.routines.push(Routine::new(
            "w",
            Agent::Ship,
            Trigger::When(Cond {
                state: State::Data,
                min: 10,
            }),
            vec![Step::Do(Block::Decode)],
        ));
        let wi = c.routines.len() - 1;
        c.tick(Agent::Ship, 5, 0, false);
        assert_eq!(c.routines[wi].stats.state, RState::Waiting);
        c.tick(Agent::Ship, 12, 0, false);
        assert_eq!(c.routines[wi].stats.state, RState::Running);
        // A locked step → blocked(locked step), honestly, regardless of trigger.
        c.routines[wi].body = vec![Step::Do(Block::Seek)]; // undiscovered + undecoded
        c.tick(Agent::Ship, 0, 0, false);
        assert_eq!(
            c.routines[wi].stats.state,
            RState::Blocked(BlockReason::LockedStep)
        );
    }

    #[test]
    fn telemetry_credit_and_blocked_reasons() {
        let mut c = Console::default();
        c.tick(Agent::Ship, 0, 0, false); // survey runs
                                          // Credit accrues items + yields to the routine.
        c.credit(1, 3, 12, false);
        assert_eq!(c.routines[1].stats.items, 3);
        assert_eq!(c.routines[1].stats.yields, 12);
        // A zero outcome downgrades a running routine to the honest reason.
        c.tick(Agent::Ship, 0, 0, false);
        c.credit(1, 0, 0, false);
        assert_eq!(
            c.routines[1].stats.state,
            RState::Blocked(BlockReason::NothingInReach)
        );
        c.tick(Agent::Ship, 0, 0, false);
        c.credit(1, 0, 0, true);
        assert_eq!(
            c.routines[1].stats.state,
            RState::Blocked(BlockReason::NoMatch)
        );
    }

    #[test]
    fn telemetry_rate_windows_honestly() {
        // `—` (None) until ~10 s of window data; a real rate after.
        let mut c = Console::default();
        c.set_now(100.0);
        c.tick(Agent::Ship, 0, 0, false); // opens the window at 100.0
        c.credit(1, 1, 10, false);
        assert_eq!(c.routines[1].stats.rate_per_hour(c.now), None); // no time elapsed
        c.set_now(130.0); // 30 s later
        c.tick(Agent::Ship, 0, 0, false);
        c.credit(1, 2, 20, false);
        let r = c.routines[1].stats.rate_per_hour(c.now).unwrap();
        assert!(
            (r - 3600.0).abs() < 1.0,
            "30 yield over 30 s = 3600/h, got {r}"
        );
        // Idle routine: no window → None, no div-by-zero.
        assert_eq!(c.routines[3].stats.rate_per_hour(c.now), None);
    }

    #[test]
    fn lit_goal_picks_one_nearest_to_done() {
        use crate::progress::{Event, Progress, Stratum as PStratum};
        let mut c = Console::default();
        let p = Progress::default();
        // Nothing qualifying → no line (never a quest log).
        assert_eq!(c.lit_goal(&p), None);
        // A when(data ≥ 10) at 80% qualifies…
        c.routines.push(Routine::new(
            "almost",
            Agent::Ship,
            Trigger::When(Cond {
                state: State::Data,
                min: 10,
            }),
            vec![Step::Do(Block::Decode)],
        ));
        let mut p = Progress::default();
        p.strata.records = 8; // data total 8/10 = 80%
        let goal = c.lit_goal(&p).expect("80% when-threshold qualifies");
        assert!(goal.contains("almost"), "goal names the routine: {goal}");
        // …but a fully affordable faculty (100%) beats it.
        for _ in 0..30 {
            p.apply(&Event::CollectShard {
                domain: PStratum::Records,
                rarity: crate::shards::Rarity::Common,
            });
        }
        let goal = c.lit_goal(&p).expect("affordable faculty qualifies");
        assert!(goal.contains("affordable"), "100% beats 80%: {goal}");
        // A decode-ready stratum names a discovered-locked block wanting it.
        let mut p2 = Progress::default();
        p2.strata.schematics = crate::progress::DECODE_COST + 1;
        c.discovered.insert(Block::Seek);
        let goal = c.lit_goal(&p2).expect("decode-ready qualifies");
        // G12: the goal names the waiting block by its **glyphs** (the `decode SCH` part stays
        // minimal-English instrumentation).
        assert!(
            goal.contains(&Block::Seek.glyphs()) && goal.contains("SCH"),
            "names the waiting block by its glyphs: {goal}"
        );
        assert!(
            !goal.contains("seek"),
            "no English block name in the goal: {goal}"
        );
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
