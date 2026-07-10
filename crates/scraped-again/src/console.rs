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

use std::collections::{HashSet, VecDeque};

use crate::progress::Stratum;

/// G13: how many of the player's most-recent **manual** actions the per-agent trace remembers
/// (session-local working memory — the source for "trace → routine").
pub const TRACE_CAP: usize = 10;

/// G14: a stable per-console routine identity (survives reorder/rename), so a `run(routine)`
/// step keeps pointing at the same routine — unlike a raw list index (brief Decision 3).
pub type RoutineId = u32;

/// G14: runtime backstop against `run` recursion blowup — the interpreter expands at most this
/// deep (the insert-time cycle guard prevents legal cycles; this catches a hostile/old payload).
pub const RUN_DEPTH_CAP: u8 = 8;

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
    /// G20: the word itself is the seeded lexicon vocabulary word (never English).
    pub fn glyphs(self, seed: u32) -> String {
        use crate::progress::script_for;
        // G22: each script writes its own stratum's surface form (the daughter, not the proto).
        let (key, stratum) = match self {
            MatchField::Rare => ("rare", self.required()),
            MatchField::Domain(d) => (self.label(), d),
        };
        let script = script_for(stratum);
        let word = crate::lexicon::vocab_word(seed, key, stratum);
        crate::text::to_overlay(&crate::structures::transliterate(&word, script), script)
    }
}

/// Which agent a routine drives (game-system §7). Routines are **per-agent**, drawing on a shared
/// block library; the *insertable* vocabulary is context-scoped to the agent (+ shared blocks).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
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
    Deposit, // G17: foot — empty the walker's carry into the site cache (the expedition handshake)
}

impl Block {
    /// The full block vocabulary (G9: every one of these has a **name** findable in the world).
    pub const ALL: [Block; 16] = [
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
        Block::Deposit,
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
            Block::Deposit => "deposit",
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
            Block::Deposit => 12,
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
            Block::Deposit => "deposit",
        }
    }

    /// G12: the block's **glyph-name** — its seeded true name (G20: a lexicon word per world,
    /// never the internal English [`name`](Self::name)) transliterated into its stratum's script
    /// and mapped to the self-identifying overlay codepoints the flat HUD renders (via
    /// [`crate::text::to_overlay`]). Visually the *exact* glyph cluster the player finds carved
    /// in the world for this block (same [`crate::structures::name_text`] — the world↔console
    /// recognition loop), recognisable *as a symbol* though unreadable *as a word*. This is the
    /// player-facing identity (palette, routine rows, codex, discovery toast);
    /// [`name`](Self::name)/[`label`](Self::label) stay for internal codes/tests/the `co=` codec.
    /// G20: the cluster is **cartouched** — the same enclosure marks the world inscription
    /// carries, so the console teaches "that frame = a name" by pure recurrence.
    pub fn glyphs(self, seed: u32) -> String {
        let script = crate::structures::block_script(self);
        crate::text::to_overlay(
            &crate::structures::cartouche(&crate::structures::name_text(seed, self)),
            script,
        )
    }

    /// G12: the full player-facing form — the glyph-name plus, for a parameterised block, its
    /// world-vocabulary argument rendered glyph too (the scan target, the spent faculty), in the
    /// block's own stratum script. Structural punctuation (the parens) stays; pure quantities
    /// never appear here. G20: the argument word is a seeded lexicon word too.
    pub fn glyph_label(self, seed: u32) -> String {
        let arg = match self {
            Block::Scan(i) => Some(i.label()),
            Block::Spend(f) => Some(f.label()),
            _ => None,
        };
        match arg {
            Some(a) => {
                let script = crate::structures::block_script(self);
                // G22: the argument word in the block's own stratum's surface form.
                let stratum = self.required().unwrap_or(Stratum::Records);
                let word = crate::lexicon::vocab_word(seed, a, stratum);
                let arg = crate::text::to_overlay(
                    &crate::structures::transliterate(&word, script),
                    script,
                );
                format!("{}({})", self.glyphs(seed), arg)
            }
            None => self.glyphs(seed),
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
            Block::FireBeam | Block::Walk | Block::Deposit => Some(Agent::Foot),
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
/// `Match`/`Repeat` are *prefix modifiers* (see the module decision note); `Group` (G14b) is the
/// one bodied container — a small **nested block** of steps. (Not `Copy`: `Group` owns a `Vec`.)
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Step {
    /// Do a block.
    Do(Block),
    /// Filter the Collect(s) that follow it in this body to only matching sites.
    Match(MatchField),
    /// Repeat the **next** `Do` this many times (1..=9).
    Repeat(u8),
    /// G14a: call another **same-agent** routine by its stable [`RoutineId`] — its body runs in
    /// place (subroutine). No seams: it reads in the palette/rows as an ordinary block.
    Run(RoutineId),
    /// G14b: a **nested step group** — run `body` `times` (1..=9), under an optional `match`
    /// `filter`, as a unit. Covers both "repeat a group" (`times>1`) and "if-match a group"
    /// (`filter=Some`). **One nesting level**: a group's `body` may not itself contain a `Group`
    /// (the editor never offers it inside a group — Decision 1, linear-with-grouping, not a tree).
    Group {
        times: u8,
        filter: Option<MatchField>,
        body: Vec<Step>,
    },
}

impl Step {
    /// Internal label (codes/tests). `Run`/`Group` show structural placeholders; the player-facing
    /// console resolves callee names + inner steps via [`Console::step_render`].
    pub fn label(&self) -> String {
        match self {
            Step::Do(b) => b.label().to_string(),
            Step::Match(f) => format!("match({})", f.label()),
            Step::Repeat(n) => format!("repeat ×{n}"),
            Step::Run(id) => format!("run(#{id})"),
            Step::Group {
                times,
                filter,
                body,
            } => {
                let inner = body.iter().map(|s| s.label()).collect::<Vec<_>>().join(" ");
                let f = filter
                    .map(|f| format!(" match({})", f.label()))
                    .unwrap_or_default();
                format!("group ×{times}{f} {{{inner}}}")
            }
        }
    }
    /// G12: player-facing rendering — block + world-item vocabulary as glyphs; the structural
    /// modifiers (`match`/`repeat`/`group`) stay minimal-English instrumentation, only their
    /// vocabulary argument goes glyph. (`Run`/`Group` inner content resolves in
    /// [`Console::step_render`], which knows the routine set.)
    pub fn glyph_label(&self, seed: u32) -> String {
        match self {
            Step::Do(b) => b.glyph_label(seed),
            Step::Match(f) => format!("match({})", f.glyphs(seed)),
            Step::Repeat(n) => format!("repeat ×{n}"),
            Step::Run(id) => format!("run(#{id})"),
            Step::Group { times, filter, .. } => {
                let f = filter
                    .map(|f| format!(" match({})", f.glyphs(seed)))
                    .unwrap_or_default();
                format!("group ×{times}{f}")
            }
        }
    }
    /// The stratum gating this step (so a locked step can't be inserted/cycled into). `Run`/`Group`
    /// are composition, not gated — their inner steps gate themselves when expanded.
    fn required(&self) -> Option<Stratum> {
        match self {
            Step::Do(b) => b.required(),
            Step::Match(f) => Some(f.required()),
            Step::Repeat(_) | Step::Run(_) | Step::Group { .. } => None,
        }
    }
}

/// Is `step` available to `agent`? `Do(block)` follows the block's agent; the `Match`/`Repeat`/
/// `Group` structural steps are shared; a `Run` is offered per-agent by
/// [`Console::editor_vocabulary`] (only same-agent callees), so it's agent-agnostic here.
fn step_for_agent(step: &Step, agent: Agent) -> bool {
    match step {
        Step::Do(b) => b.for_agent(agent),
        Step::Match(_) | Step::Repeat(_) | Step::Run(_) | Step::Group { .. } => true,
    }
}

/// A state the `when` trigger can test: the total banked **data** (all strata), or the shard
/// **bank** (G10) — so `when(shards ≥ N) → spend(…)` is wireable.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum State {
    Data,
    Shards,
    /// G17: the walker's carry as a **percentage** of its cap (`when(carry ≥ %)`) — the foot side
    /// of the handshake (`when(carry full) → deposit`).
    Carry,
    /// G17: the site cache **count** (`when(cache ≥ N)`) — the ship side (`when(cache ≥ N) → goto`).
    Cache,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Data => "data",
            State::Shards => "shards",
            State::Carry => "carry",
            State::Cache => "cache",
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
    /// Does the condition hold? `carry` is the walker carry **percentage** (0..=100), `cache` the
    /// site cache **count** — the G17 handshake referents alongside data + the shard bank.
    fn holds(self, data: u32, shards: u32, carry: u32, cache: u32) -> bool {
        let cur = match self.state {
            State::Data => data,
            State::Shards => shards,
            State::Carry => carry,
            State::Cache => cache,
        };
        cur >= self.min
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
            // G18 (the pinned §I.2 rider): the *state* trigger displays as `while` — it holds
            // while a condition holds, unlike the `on-…` event triggers. Display only: the
            // `co=` codec (`w:`/`wS`/`wY`/`wK`) and the rising-edge semantics are untouched.
            Trigger::When(c) => format!("while {} ≥ {}", c.state.label(), c.min),
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
    /// G17: the walker's carry is full — `collect` can't take more until it `deposit`s (the honest
    /// expedition-handshake state, not an error).
    CarryFull,
    /// G17: the walker is waiting at a non-empty cache for the ship to drain it (the broken-handoff
    /// vignette as honest state).
    CacheFull,
}

impl BlockReason {
    pub fn label(self) -> &'static str {
        match self {
            BlockReason::NothingInReach => "nothing in reach",
            BlockReason::NoMatch => "no match",
            BlockReason::LockedStep => "locked step",
            BlockReason::CarryFull => "carry full",
            BlockReason::CacheFull => "cache full",
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
/// default instances. (`id` is a stable ref target; `armed` + `stats` are transient runtime
/// state — all three are excluded from equality, which is name/enabled/agent/trigger/body.)
#[derive(Clone, Debug)]
pub struct Routine {
    /// G14: stable identity for `run(routine)` refs — assigned by the owning [`Console`]
    /// (`Routine::new` leaves it 0; the console mints the real id). Excluded from equality.
    pub id: RoutineId,
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
            id: 0, // the owning Console assigns a real id (see `mint_id`)
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

/// G13 — the **literal** trace→program transform: map recorded blocks to body steps, the *only*
/// change being mechanical run-length folding of identical **adjacent** actions into a
/// `Repeat(n)` + one `Do` (chunked at `Repeat`'s 1..=9 ceiling). Nothing else: non-adjacent
/// repeats stay separate (`scan, collect, scan, collect` does NOT become a loop), no "noise" is
/// dropped, nothing is generalized — that's the player's job afterward via the steppers. Pure.
fn trace_to_steps(blocks: &[Block]) -> Vec<Step> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < blocks.len() {
        let b = blocks[i];
        let mut run = 1;
        while i + run < blocks.len() && blocks[i + run] == b {
            run += 1;
        }
        i += run;
        // Emit the run, chunked at the `Repeat` ceiling (9); a chunk of 1 is just a `Do`.
        while run > 0 {
            let chunk = run.min(9);
            if chunk >= 2 {
                out.push(Step::Repeat(chunk as u8));
            }
            out.push(Step::Do(b));
            run -= chunk;
        }
    }
    out
}

/// What the home cursor is on.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sel {
    Routine(usize),
    NewRoutine,
    /// G13: build a draft routine from the active agent's recorded manual trace.
    TraceToRoutine,
    Block(Block),
    /// G21: a discovered sensing instrument — Enter allocates its research (the remedy).
    Sense(crate::progress::Sense),
}

/// Which screen the console is showing.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum View {
    /// Routine list + "new routine" + the manual-run block palette.
    Home,
    /// Editing routine `i`'s trigger + body (the free-form editor).
    Edit(usize),
    /// G14b: editing the **inner body** of routine `i`'s step `s` (a `Group`) — the same editor,
    /// one level down. Row 0 is the group's header (`×times` + optional `match`); `O` backs out.
    EditGroup(usize, usize),
}

/// The console: the routines (data), the manual/insert block palette, cursor + view state.
pub struct Console {
    pub open: bool,
    pub view: View,
    pub cursor: usize,
    pub routines: Vec<Routine>,
    pub palette: Vec<Block>,
    /// G15: comprehended **blocks** (synced from `progress` each frame) — a discovered block is
    /// usable once its research filled. Replaces G9's per-stratum decode gate. Starters are
    /// implicitly comprehended (see [`Console::is_unlocked`]); this carries the researched gated
    /// blocks. The vestigial per-stratum legibility set lives in `progress` (folded from research).
    pub comprehended: HashSet<Block>,
    /// Discovered gated blocks (G9; synced from `progress`) — a block's name must be found in the
    /// world before it's even *listed*. Starters are implicitly discovered (see
    /// [`Console::is_discovered`]), so this only carries the gated vocabulary.
    pub discovered: HashSet<Block>,
    /// G18: **confirmed** readings (synced from `progress`) — a discovered gated block absent
    /// here is *provisional* and renders with the underdot sub-mark (Leiden display grammar).
    /// Starters are implicitly confirmed. Display-only (nothing gates on it — Decision 1).
    pub confirmed: HashSet<Block>,
    /// G21: **discovered** sensing instruments (synced from `progress`) — a frustration event
    /// fired, so the console lists the instrument as a research target (the remedy on offer).
    pub senses_discovered: HashSet<crate::progress::Sense>,
    /// G21: **comprehended** sensing instruments (synced) — held ladder rungs (render clean).
    pub senses_comprehended: HashSet<crate::progress::Sense>,
    /// G11: the app's clock (set each frame before `tick`) — drives last-fired age + yield rates.
    pub now: f32,
    /// G13: per-agent rolling memory of the player's last manual block actions (session-local,
    /// never persisted) — the literal source for "trace → routine". Newest at the back.
    pub traces: std::collections::HashMap<Agent, VecDeque<Block>>,
    /// G13: the agent the player is currently controlling (set each frame by the app, like `now`)
    /// — selects which trace the ticker shows and "trace → routine" captures.
    pub active_agent: Agent,
    /// G14: the next stable [`RoutineId`] to mint (monotonic; survives reorder/rename). Persisted
    /// implicitly — `restore` advances it past the largest loaded id.
    pub next_id: RoutineId,
    /// G20: the **world seed** (set once by the app) — the true-name source. Every glyph render
    /// (palette, rows, ticker, goals) draws the per-world lexicon names through it.
    pub seed: u32,
}

impl Default for Console {
    fn default() -> Self {
        let mut c = Console {
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
                // G15: `decode` removed — comprehension is now research (allocate shards into a
                // discovered-but-locked block). The variant stays for `co=` back-compat only.
                Block::Spend(crate::progress::Faculty::Sensing),
                Block::Spend(crate::progress::Faculty::Reach),
                Block::Spend(crate::progress::Faculty::Drive),
                Block::Goto,
                Block::Drift,
            ],
            comprehended: HashSet::new(),
            discovered: HashSet::new(),
            confirmed: HashSet::new(),
            senses_discovered: HashSet::new(),
            senses_comprehended: HashSet::new(),
            now: 0.0,
            traces: std::collections::HashMap::new(),
            active_agent: Agent::Ship,
            next_id: 0,
            seed: 0,
        };
        // G14: the givens get stable ids 0..n; the next mint continues past them.
        for (i, r) in c.routines.iter_mut().enumerate() {
            r.id = i as RoutineId;
        }
        c.next_id = c.routines.len() as RoutineId;
        c
    }
}

impl Console {
    /// G14: mint the next stable routine id (monotonic).
    fn mint_id(&mut self) -> RoutineId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// G14: resolve a routine by stable id → its list index (callee lookup for `run`).
    fn index_of_id(&self, id: RoutineId) -> Option<usize> {
        self.routines.iter().position(|r| r.id == id)
    }
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

    /// Usable? **Two-stage** (G9 + G15): the name must be *discovered* AND the block
    /// *researched* (comprehended). Starters (no required stratum) pass both implicitly, so the
    /// opening + given routines are untouched.
    pub fn is_unlocked(&self, b: Block) -> bool {
        self.is_discovered(b) && (b.required().is_none() || self.comprehended.contains(&b))
    }

    /// G18: is this block's reading **confirmed**? Starters always (never hypotheses); a gated
    /// block once a second sighting or first use attested it. Purely display (Decision 1).
    pub fn is_confirmed(&self, b: Block) -> bool {
        b.required().is_none() || self.confirmed.contains(&b)
    }

    /// G15: has the player cracked *any* of stratum `s`'s vocabulary — i.e. comprehended a block
    /// gated on it? Gates the stratum-scoped non-block modifiers (`match`) now that there's no
    /// decode-a-stratum action. (`match` is gated on Rites, which has no blocks today, so it's
    /// reachable once *any* gated block is comprehended — a progression beat, not free at start.)
    fn stratum_cracked(&self, s: Stratum) -> bool {
        self.comprehended.iter().any(|b| b.required() == Some(s))
            || (s == Stratum::Rites && !self.comprehended.is_empty())
    }

    fn step_unlocked(&self, s: &Step) -> bool {
        match s {
            Step::Do(b) => self.is_unlocked(*b),
            // The `match` modifier is stratum-gated (Rites); repeat/group/run aren't gated.
            Step::Match(_) => s.required().is_none_or(|st| self.stratum_cracked(st)),
            Step::Repeat(_) | Step::Run(_) | Step::Group { .. } => true,
        }
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
            // G15: `decode` removed from the vocabulary (comprehension is research now).
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
            Step::Do(Block::Deposit), // G17: foot-only (agent-filtered below)
            Step::Match(MatchField::Rare),
            Step::Match(MatchField::Domain(Stratum::Records)),
            Step::Match(MatchField::Domain(Stratum::Schematics)),
            Step::Match(MatchField::Domain(Stratum::Rites)),
            Step::Match(MatchField::Domain(Stratum::Relics)),
            Step::Match(MatchField::Domain(Stratum::Signals)),
            Step::Repeat(2),
            // G14b: an empty nested group (one level — the editor won't offer this inside a group).
            Step::Group {
                times: 2,
                filter: None,
                body: Vec::new(),
            },
        ];
        all.into_iter()
            .filter(|s| self.step_unlocked(s) && step_for_agent(s, agent))
            .collect()
    }

    /// G14: the editor's insertable steps for the routine at `editing` — the agent
    /// [`vocabulary`](Self::vocabulary) plus a `run(routine)` for every *other* same-agent routine
    /// whose call wouldn't cycle (Steele's no-seams: a player routine is offered like any block).
    /// Self / cycle-creating calls are omitted, so an inserted `run` can never recurse.
    pub fn editor_vocabulary(&self, editing: usize) -> Vec<Step> {
        let Some(ed) = self.routines.get(editing) else {
            return Vec::new();
        };
        let mut v = self.vocabulary(ed.agent);
        for (j, r) in self.routines.iter().enumerate() {
            if j != editing && r.agent == ed.agent && !self.would_cycle(ed.id, r.id) {
                v.push(Step::Run(r.id));
            }
        }
        v
    }

    /// G14: would inserting `run(callee)` into routine `caller` create a cycle? A self-call is a
    /// cycle; otherwise DFS the callee's transitive `run` graph and report whether it reaches the
    /// caller. Pure — the insert-time guard (brief Decision 2; the runtime depth cap is a backstop).
    pub fn would_cycle(&self, caller: RoutineId, callee: RoutineId) -> bool {
        if caller == callee {
            return true;
        }
        let mut stack = vec![callee];
        let mut seen = vec![callee];
        while let Some(id) = stack.pop() {
            if id == caller {
                return true;
            }
            if let Some(idx) = self.index_of_id(id) {
                for s in &self.routines[idx].body {
                    if let Step::Run(next) = s {
                        if !seen.contains(next) {
                            seen.push(*next);
                            stack.push(*next);
                        }
                    }
                }
            }
        }
        false
    }

    /// G21 rider (G20 review): a routine's **player-facing name**. The four GIVENS were authored
    /// by the dead machine, so they display as its seeded lexicon words (overlay-glyph rendered,
    /// Records/Latin — machine operations — like the faculty research labels); player-created
    /// routines keep their instrumentation defaults (`trace-1`, `routine-2`, `-copy`) verbatim.
    /// `Routine::name` stays the internal identity (codec/tests), like `Block::name`.
    pub fn routine_display_name(&self, r: &Routine) -> String {
        const GIVENS: [&str; 4] = ["drift", "survey", "prospect", "collect"];
        if GIVENS.contains(&r.name.as_str()) {
            // G22: the Records surface form (the dead machine's operational register).
            let word = crate::lexicon::vocab_word(self.seed, &r.name, Stratum::Records);
            let script = crate::text::Script::Latin;
            crate::text::to_overlay(&crate::structures::transliterate(&word, script), script)
        } else {
            r.name.clone()
        }
    }

    /// G14: render a step for the player — like [`Step::glyph_label`], but a `run(id)` resolves to
    /// the callee's **name** (`run(sweep)`; author labels stay English per the G12 review, but a
    /// GIVEN callee shows its lexicon word — G21 rider), so only vocabulary is glyph-rendered.
    fn step_render(&self, s: &Step) -> String {
        match s {
            Step::Run(id) => {
                let name = self
                    .index_of_id(*id)
                    .map(|j| self.routine_display_name(&self.routines[j]))
                    .unwrap_or_else(|| "?".into());
                format!("run({name})")
            }
            // G14b: a group renders its header + its inner steps (each resolved — a `run` inside a
            // group still shows its callee name), kept compact for a row.
            Step::Group {
                times,
                filter,
                body,
            } => {
                let f = filter
                    .map(|f| format!(" match({})", f.glyphs(self.seed)))
                    .unwrap_or_default();
                let inner = body
                    .iter()
                    .map(|s| self.step_render(s))
                    .collect::<Vec<_>>()
                    .join(" → ");
                format!("group ×{times}{f} {{{inner}}}")
            }
            other => other.glyph_label(self.seed),
        }
    }

    // ---- the interpreter --------------------------------------------------------------------

    /// G14: a routine snapshot `(id, agent, body)` for `run` resolution during expansion — taken
    /// once per tick so the recursive [`Self::expand`] can read callee bodies without conflicting
    /// with the live `&mut` loop.
    fn snapshot(&self) -> Vec<(RoutineId, Agent, Vec<Step>)> {
        self.routines
            .iter()
            .map(|r| (r.id, r.agent, r.body.clone()))
            .collect()
    }

    /// Expand a body into resolved acts: `match` sets the filter for following Collects;
    /// `repeat(n)` multiplies the next `Do`; **`run(id)` (G14)** expands the same-agent callee's
    /// body in place, crediting the **callee** (Decision 4) and failsoft on a missing / other-agent
    /// / over-deep / cyclic callee (a no-op). `routine` tags each act for telemetry attribution;
    /// `all` is the snapshot, `depth`/`visited` are the recursion guards (cycle + [`RUN_DEPTH_CAP`]).
    fn expand(
        body: &[Step],
        routine: usize,
        all: &[(RoutineId, Agent, Vec<Step>)],
        depth: u8,
        visited: &mut Vec<RoutineId>,
        filter: &mut Option<MatchField>,
    ) -> Vec<Act> {
        let mut out = Vec::new();
        let mut times: u8 = 1;
        let agent = all.get(routine).map(|e| e.1);
        for step in body {
            match step {
                // G14 Decision 5 — the *one implicit register*: the `match` filter is the single
                // "current thing" that flows between steps AND across a `run` (the callee sees it
                // on entry via the shared `&mut`, and any change persists on return). There is no
                // second implicit referent — anything else must be an explicit parameter.
                Step::Match(f) => *filter = Some(*f),
                Step::Repeat(n) => times = (*n).max(1),
                Step::Do(b) => {
                    for _ in 0..times {
                        out.push(Act {
                            block: *b,
                            filter: *filter,
                            routine,
                        });
                    }
                    times = 1;
                }
                Step::Run(id) => {
                    times = 1; // a `run` is not a `Do`; `repeat` doesn't multiply it (see G14b)
                    if depth >= RUN_DEPTH_CAP || visited.contains(id) {
                        continue; // failsoft: depth / cycle backstop
                    }
                    let Some(ci) = all.iter().position(|e| e.0 == *id && Some(e.1) == agent) else {
                        continue; // failsoft: missing or other-agent callee
                    };
                    visited.push(*id);
                    out.extend(Self::expand(
                        &all[ci].2,
                        ci,
                        all,
                        depth + 1,
                        visited,
                        filter,
                    ));
                    visited.pop();
                }
                Step::Group {
                    times: gtimes,
                    filter: gfilter,
                    body: gbody,
                } => {
                    // G14b: a nested group runs its body `gtimes` as a unit. Its `match` (if any)
                    // is **scoped** to the body — saved/restored around it (a local block), unlike
                    // a `run` whose register flows out. `repeat` doesn't multiply a group.
                    times = 1;
                    let saved = *filter;
                    if let Some(f) = gfilter {
                        *filter = Some(*f);
                    }
                    for _ in 0..(*gtimes).max(1) {
                        out.extend(Self::expand(gbody, routine, all, depth, visited, filter));
                    }
                    *filter = saved;
                }
            }
        }
        out
    }

    /// Run one interpreter tick for `agent`'s **continuous** / **when** / **on-arrive** routines,
    /// given the current `data` (total banked strata) + `shards` (the G10 bank) for `when`
    /// conditions and whether the agent has just `arrived` at the site it's heading to. Returns
    /// this tick's [`Tick`] intents. (`on-scan` routines fire on a scan hit — [`Console::on_scan_acts`].)
    pub fn tick(
        &mut self,
        agent: Agent,
        data: u32,
        shards: u32,
        carry: u32,
        cache: u32,
        arrived: bool,
    ) -> Tick {
        let mut t = Tick::default();
        let now = self.now;
        let all = self.snapshot(); // G14: callee bodies for `run` resolution (read-only)
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
            // G11: telemetry — a routine with a not-yet-researched (locked) block is honestly
            // `blocked` (G15: per-block research gate). Inlined (disjoint field borrows) so it
            // coexists with the `&mut` routines loop — mirrors `is_unlocked`.
            let has_locked = r.body.iter().any(|st| {
                matches!(st, Step::Do(b) if {
                    let disc = b.required().is_none() || self.discovered.contains(b);
                    let comp = b.required().is_none() || self.comprehended.contains(b);
                    !(disc && comp)
                })
            });
            let mut fired = false;
            match r.trigger {
                Trigger::Continuous => {
                    fired = true;
                    for act in Self::expand(&r.body, idx, &all, 0, &mut Vec::new(), &mut None) {
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
                    let sat = c.holds(data, shards, carry, cache);
                    if sat && !r.armed {
                        fired = true;
                        // rising edge → fire once
                        t.acts.extend(Self::expand(
                            &r.body,
                            idx,
                            &all,
                            0,
                            &mut Vec::new(),
                            &mut None,
                        ));
                    }
                    r.armed = sat;
                }
                Trigger::OnArrive => {
                    if arrived && !r.armed {
                        fired = true;
                        // reached the site → fire
                        t.acts.extend(Self::expand(
                            &r.body,
                            idx,
                            &all,
                            0,
                            &mut Vec::new(),
                            &mut None,
                        ));
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

    /// G17: stamp a routine's live state with an honest block `reason` the app evaluated after the
    /// tick (e.g. `collect` hit a full carry, `deposit`/`goto` waited on a full/empty cache) — the
    /// handshake's legible "why nothing's happening" without inventing a diagnosis (G11).
    pub fn note_blocked(&mut self, routine: usize, reason: BlockReason) {
        if let Some(r) = self.routines.get_mut(routine) {
            r.stats.state = RState::Blocked(reason);
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
        let all = self.snapshot(); // G14: `run` resolution
        self.routines
            .iter()
            .enumerate()
            .filter(|(_, r)| r.agent == agent && r.enabled && matches!(r.trigger, Trigger::OnScan))
            .flat_map(|(i, r)| Self::expand(&r.body, i, &all, 0, &mut Vec::new(), &mut None))
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

    /// G21: the home screen's sensing-instrument listing — every *discovered* instrument (the
    /// frustration events fired), research targets first as-needed. Undiscovered are absent
    /// (you can't covet an instrument whose lack you haven't felt).
    pub fn visible_senses(&self) -> Vec<crate::progress::Sense> {
        crate::progress::Sense::ALL
            .into_iter()
            .filter(|s| self.senses_discovered.contains(s))
            .collect()
    }

    /// Home rows: routines (toggle/edit), a "new routine" row, a "trace → routine" row (G13),
    /// then the visible block listing, then (G21) the discovered sensing instruments.
    pub fn home_rows(&self) -> usize {
        self.routines.len() + 2 + self.visible_palette().len() + self.visible_senses().len()
    }

    /// Editor rows for routine `i`: the trigger row, each body step, then an "add step" row.
    /// G14b: the step list the editor is currently acting on — a routine's body (`Edit`) or a
    /// group's inner body (`EditGroup`). Empty slice if the view/step is wrong (failsoft).
    fn target_body(&self) -> &[Step] {
        match self.view {
            View::Edit(i) => self
                .routines
                .get(i)
                .map(|r| r.body.as_slice())
                .unwrap_or(&[]),
            View::EditGroup(i, s) => match self.routines.get(i).and_then(|r| r.body.get(s)) {
                Some(Step::Group { body, .. }) => body,
                _ => &[],
            },
            View::Home => &[],
        }
    }

    /// Mutable counterpart of [`target_body`](Self::target_body).
    fn target_body_mut(&mut self) -> Option<&mut Vec<Step>> {
        match self.view {
            View::Edit(i) => self.routines.get_mut(i).map(|r| &mut r.body),
            View::EditGroup(i, s) => match self.routines.get_mut(i).and_then(|r| r.body.get_mut(s))
            {
                Some(Step::Group { body, .. }) => Some(body),
                _ => None,
            },
            View::Home => None,
        }
    }

    /// The routine being edited (the run/vocabulary context), in either editor view.
    fn editing_routine(&self) -> Option<usize> {
        match self.view {
            View::Edit(i) | View::EditGroup(i, _) => Some(i),
            View::Home => None,
        }
    }

    fn edit_rows(&self) -> usize {
        1 + self.target_body().len() + 1 // params row + steps + add-step
    }

    fn rows(&self) -> usize {
        match self.view {
            View::Home => self.home_rows(),
            View::Edit(_) | View::EditGroup(..) => self.edit_rows(),
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
        } else if self.cursor == nr + 1 {
            Sel::TraceToRoutine
        } else {
            let pal = self.visible_palette();
            let i = self.cursor - nr - 2;
            if i < pal.len() {
                Sel::Block(pal[i])
            } else {
                Sel::Sense(self.visible_senses()[i - pal.len()]) // G21
            }
        }
    }

    /// Resolve the editor cursor against the current target body (routine or group). Row 0 is the
    /// params row (`Trigger` in `Edit`, the group header in `EditGroup`).
    pub fn edit_focus(&self) -> EditFocus {
        let nb = self.target_body().len();
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

    // ---- G13: record-to-program -------------------------------------------------------------

    /// Record one **manual** player action into `agent`'s rolling trace (newest at the back,
    /// capped at [`TRACE_CAP`]). Called only from the app's manual action sites — autopilot /
    /// auto-collect / interpreter acts never reach here, so the trace stays the player's hands.
    pub fn record_manual(&mut self, agent: Agent, block: Block) {
        let ring = self.traces.entry(agent).or_default();
        ring.push_back(block);
        while ring.len() > TRACE_CAP {
            ring.pop_front();
        }
    }

    /// The active agent's recorded manual trace, oldest→newest (for the ticker display).
    pub fn trace(&self, agent: Agent) -> Vec<Block> {
        self.traces
            .get(&agent)
            .map(|r| r.iter().copied().collect::<Vec<Block>>())
            .unwrap_or_default()
    }

    /// Build a **draft routine** from `agent`'s trace (the G13 contract: *record literally,
    /// generalize manually*) and open it in the editor. The only transformation is mechanical
    /// run-length folding of identical adjacent actions into `repeat(n)` — no inference, no
    /// dropping, no generalization. Returns the new routine index, or `None` if the trace is
    /// empty. The draft is ordinary G7 data (persists / edits / runs through existing paths).
    pub fn trace_to_routine(&mut self, agent: Agent) -> Option<usize> {
        let blocks = self.trace(agent);
        if blocks.is_empty() {
            return None;
        }
        let body = trace_to_steps(&blocks);
        let name = format!("trace-{}", self.routines.len() + 1);
        let id = self.mint_id();
        let mut r = Routine::new(name, agent, Trigger::Continuous, body);
        r.id = id;
        self.routines.push(r);
        let i = self.routines.len() - 1;
        self.view = View::Edit(i);
        self.cursor = 0;
        Some(i)
    }

    /// Create a fresh empty `agent` routine and open its editor. Returns the new index.
    pub fn create_routine(&mut self, agent: Agent) -> usize {
        let name = format!("routine-{}", self.routines.len() + 1);
        let id = self.mint_id();
        let mut r = Routine::new(name, agent, Trigger::Continuous, Vec::new());
        r.id = id;
        self.routines.push(r);
        let i = self.routines.len() - 1;
        self.view = View::Edit(i);
        self.cursor = 0;
        i
    }

    /// G14: duplicate routine `i` into an independent editable copy (fresh id + name, same agent /
    /// trigger / body / enabled), opened in the editor. A starting point to mutate — cheaper than
    /// re-authoring (the brief's "duplicate routine"). Returns the new index (or `None` if `i` is
    /// out of range).
    pub fn duplicate_routine(&mut self, i: usize) -> Option<usize> {
        let src = self.routines.get(i)?;
        let id = self.next_id; // mint without a second &mut borrow of self below
        let mut copy = Routine::new(
            format!("{}-copy", src.name),
            src.agent,
            src.trigger,
            src.body.clone(),
        );
        copy.id = id;
        copy.enabled = src.enabled;
        self.next_id += 1;
        self.routines.push(copy);
        let j = self.routines.len() - 1;
        self.view = View::Edit(j);
        self.cursor = 0;
        Some(j)
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

    /// G14b: descend into the focused step if it's a `Group`, to edit its inner body. No-op
    /// otherwise (and never from inside a group — one nesting level).
    pub fn enter_group(&mut self) {
        if let (View::Edit(i), EditFocus::Step(s)) = (self.view, self.edit_focus()) {
            if matches!(
                self.routines.get(i).and_then(|r| r.body.get(s)),
                Some(Step::Group { .. })
            ) {
                self.view = View::EditGroup(i, s);
                self.cursor = 0;
            }
        }
    }

    pub fn close_editor(&mut self) {
        match self.view {
            // G14b: leaving a group → back to its routine editor, cursor on the group step.
            View::EditGroup(i, s) => {
                self.view = View::Edit(i);
                self.cursor = s + 1;
            }
            View::Edit(i) => {
                // Going home — land on the routine we were editing (if it still exists).
                self.cursor = i.min(self.routines.len().saturating_sub(1));
                self.view = View::Home;
            }
            View::Home => {}
        }
    }

    /// The steps insertable in the current editor view: the editing routine's
    /// [`editor_vocabulary`](Self::editor_vocabulary), minus `Group` when already inside a group
    /// (one nesting level — Decision 1).
    fn current_vocabulary(&self) -> Vec<Step> {
        let Some(r) = self.editing_routine() else {
            return Vec::new();
        };
        let mut v = self.editor_vocabulary(r);
        if matches!(self.view, View::EditGroup(..)) {
            v.retain(|s| !matches!(s, Step::Group { .. }));
        }
        v
    }

    // ---- body / step edits (the free-form editor) -------------------------------------------

    /// Insert a step after the editor cursor (or append, on the "add step" row). The inserted
    /// step is the first unlocked vocabulary entry; ←/→ then cycles it. No-op if nothing unlocked.
    pub fn insert_step(&mut self) {
        let Some(first) = self.current_vocabulary().into_iter().next() else {
            return;
        };
        let at = match self.edit_focus() {
            EditFocus::Trigger => 0,
            EditFocus::Step(s) => s + 1,
            EditFocus::AddStep => self.target_body().len(),
        };
        if let Some(body) = self.target_body_mut() {
            body.insert(at, first);
            self.cursor = at + 1; // land on the new step (params row is 0)
        }
    }

    /// Remove the step under the editor cursor.
    pub fn remove_step(&mut self) {
        if let EditFocus::Step(s) = self.edit_focus() {
            if let Some(body) = self.target_body_mut() {
                body.remove(s);
            }
            self.cursor = self.cursor.min(self.edit_rows().saturating_sub(1));
        }
    }

    /// Move the step under the cursor up/down within the body (reorder).
    pub fn move_step(&mut self, dir: i32) {
        let EditFocus::Step(s) = self.edit_focus() else {
            return;
        };
        let j = s as i32 + dir;
        if !(0..self.target_body().len() as i32).contains(&j) {
            return;
        }
        if let Some(body) = self.target_body_mut() {
            body.swap(s, j as usize);
        }
        self.cursor = (j as usize) + 1;
    }

    /// ←/→ on the focused row: cycle the params row (the trigger, or a group's `match` filter), or
    /// cycle the focused step through the unlocked vocabulary. (Numeric counts → [`Console::adjust`].)
    pub fn cycle(&mut self, i: i32) {
        match self.edit_focus() {
            EditFocus::Trigger => match self.view {
                View::Edit(r) => self.cycle_trigger(r, i),
                View::EditGroup(..) => self.cycle_group_filter(i),
                View::Home => {}
            },
            EditFocus::Step(s) => self.cycle_step(s, i),
            EditFocus::AddStep => {}
        }
    }

    /// G14b: cycle the focused group's `match` filter (the header row) through None → Rare →
    /// each domain → None.
    fn cycle_group_filter(&mut self, i: i32) {
        let opts: Vec<Option<MatchField>> = std::iter::once(None)
            .chain(
                [
                    MatchField::Rare,
                    MatchField::Domain(Stratum::Records),
                    MatchField::Domain(Stratum::Schematics),
                    MatchField::Domain(Stratum::Rites),
                    MatchField::Domain(Stratum::Relics),
                    MatchField::Domain(Stratum::Signals),
                ]
                .map(Some),
            )
            .collect();
        if let View::EditGroup(r, s) = self.view {
            if let Some(Step::Group { filter, .. }) =
                self.routines.get_mut(r).and_then(|x| x.body.get_mut(s))
            {
                let cur = opts.iter().position(|o| o == filter).unwrap_or(0) as i32;
                let n = opts.len() as i32;
                *filter = opts[(((cur + i) % n + n) % n) as usize];
            }
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
            // G17: the handshake referents — carry % (foot side) and cache count (ship side).
            Trigger::When(Cond {
                state: State::Carry,
                min: 100,
            }),
            Trigger::When(Cond {
                state: State::Cache,
                min: 8,
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

    fn cycle_step(&mut self, s: usize, i: i32) {
        let vocab = self.current_vocabulary();
        if vocab.is_empty() {
            return;
        }
        let Some(body) = self.target_body_mut() else {
            return;
        };
        let Some(cur) = body.get(s).cloned() else {
            return;
        };
        // Find the current step in the vocabulary by full value first (so parameterised families
        // — scan items, spend faculties, match domains — cycle through their variants), falling
        // back to *kind* (so a Repeat(5) still matches the Repeat slot, a populated Group the
        // Group slot). A group's inner body is preserved across a *value* match; a kind-fallback
        // cycle that leaves a Group replaces it (cycling away from a group is a deliberate edit).
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
        body[s] = vocab[(((pos + i) % n + n) % n) as usize].clone();
    }

    /// −/+ on the focused row: nudge a numeric parameter — the `When` threshold (±5), a `Repeat`
    /// count (±1, 1..=9), or (G14b) a group's `×times` on its header row.
    pub fn adjust(&mut self, delta: i32) {
        match self.edit_focus() {
            EditFocus::Trigger => match self.view {
                View::Edit(r) => {
                    if let Trigger::When(c) = &mut self.routines[r].trigger {
                        c.min = (c.min as i32 + 5 * delta).clamp(0, 9_999) as u32;
                    }
                }
                // G14b: the group header row — nudge the repeat count.
                View::EditGroup(r, s) => {
                    if let Some(Step::Group { times, .. }) =
                        self.routines.get_mut(r).and_then(|x| x.body.get_mut(s))
                    {
                        *times = (*times as i32 + delta).clamp(1, 9) as u8;
                    }
                }
                View::Home => {}
            },
            EditFocus::Step(s) => {
                if let Some(Step::Repeat(n)) = self.target_body_mut().and_then(|b| b.get_mut(s)) {
                    *n = (*n as i32 + delta).clamp(1, 9) as u8;
                }
            }
            EditFocus::AddStep => {}
        }
    }

    // ---- persistence (`co=` segment) --------------------------------------------------------

    fn step_code(s: &Step) -> String {
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
            Step::Do(Block::Deposit) => "e".into(), // G17: dEposit
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
            Step::Run(id) => format!("u{id}"), // G14a: call routine #id
            // G14b: a nested group `(times[filter]:inner)`. Inner step codes are self-delimiting
            // (parse reads char-by-char), so no separators; one level (inner carries no `(`).
            Step::Group {
                times,
                filter,
                body,
            } => {
                let f = match filter {
                    None => String::new(),
                    Some(MatchField::Rare) => "m".into(),
                    Some(MatchField::Domain(d)) => format!(
                        "M{}",
                        match d {
                            Stratum::Records => '0',
                            Stratum::Schematics => '1',
                            Stratum::Rites => '2',
                            Stratum::Relics => '3',
                            Stratum::Signals => '4',
                        }
                    ),
                };
                let inner: String = body.iter().map(Self::step_code).collect();
                format!("({times}{f}:{inner})")
            }
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
                'e' => Step::Do(Block::Deposit), // G17
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
                'u' => {
                    // G14a: run(routine #id). A dangling id (missing callee) loads fine and
                    // degrades to a no-op at expand time (failsoft).
                    let mut num = String::new();
                    while let Some(d) = it.peek().filter(|d| d.is_ascii_digit()) {
                        num.push(*d);
                        it.next();
                    }
                    Step::Run(num.parse::<RoutineId>().unwrap_or(0))
                }
                '(' => {
                    // G14b: a nested group `(times[filter]:inner)`. Read times, an optional filter
                    // token, skip `:`, then collect the inner chars up to `)` and parse them
                    // recursively (one level — a well-formed inner carries no `(`).
                    let mut tnum = String::new();
                    while let Some(d) = it.peek().filter(|d| d.is_ascii_digit()) {
                        tnum.push(*d);
                        it.next();
                    }
                    let times = tnum.parse::<u8>().unwrap_or(2).clamp(1, 9);
                    let filter = match it.peek() {
                        Some('m') => {
                            it.next();
                            Some(MatchField::Rare)
                        }
                        Some('M') => {
                            it.next();
                            let d = match it.next() {
                                Some('1') => Stratum::Schematics,
                                Some('2') => Stratum::Rites,
                                Some('3') => Stratum::Relics,
                                Some('4') => Stratum::Signals,
                                _ => Stratum::Records,
                            };
                            Some(MatchField::Domain(d))
                        }
                        _ => None,
                    };
                    if it.peek() == Some(&':') {
                        it.next();
                    }
                    let mut inner = String::new();
                    for ch in it.by_ref() {
                        if ch == ')' {
                            break;
                        }
                        inner.push(ch);
                    }
                    Step::Group {
                        times,
                        filter,
                        body: Self::parse_steps(&inner),
                    }
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
            // `w:{min}` = when(data) (the pre-G10 form, kept); `wS` = shards; `wY`/`wK` = carry/cache.
            Trigger::When(c) => match c.state {
                State::Data => format!("w:{}", c.min),
                State::Shards => format!("wS:{}", c.min),
                State::Carry => format!("wY:{}", c.min),
                State::Cache => format!("wK:{}", c.min),
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
                } else if s.starts_with("wY") {
                    State::Carry
                } else if s.starts_with("wK") {
                    State::Cache
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
                    .map(Self::step_code)
                    .collect::<Vec<_>>()
                    .join(",");
                let agent = match r.agent {
                    Agent::Ship => 'S',
                    Agent::Foot => 'F',
                };
                // G14: the stable id is a trailing field (append-only — old 5-field payloads
                // still load; their ids are reassigned by index on restore).
                format!(
                    "{}|{}|{}|{}|{}|{}",
                    r.name,
                    u8::from(r.enabled),
                    Self::trigger_code(r.trigger),
                    steps,
                    agent,
                    r.id
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
        for (i, chunk) in v.split(';').filter(|c| !c.is_empty()).enumerate() {
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
            // G14: trailing stable id; old payloads (no field) fall back to the list index.
            let id = f
                .next()
                .and_then(|x| x.parse::<RoutineId>().ok())
                .unwrap_or(i as RoutineId);
            routines.push(Routine {
                id,
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
            // G14: continue minting past the largest loaded id (keep ids unique + stable).
            self.next_id = routines.iter().map(|r| r.id + 1).max().unwrap_or(0);
            self.routines = routines;
        }
    }

    /// G11/G15: the HUD's **one lit goal**. The headline is the **active research** target the
    /// player chose (glyph-named — "what I'm cracking next"); when none is set, fall back to the
    /// single nearest-to-done thing (≥ ~75%): a `when` threshold close to firing · an
    /// affordable-unbought faculty. Exactly one line; nothing qualifying → `None` (never a quest log).
    pub fn lit_goal(&self, p: &crate::progress::Progress) -> Option<String> {
        // G15: a directed research target is the player's explicit focus — it's the lit goal.
        // G19: the bar shows the raw fill and, when the target's stratum demands rare evidence,
        // the rare-pickup gauge (`172/200 · r 1/4`) — structural numbers, not words.
        if let Some(b) = p.active_research() {
            let (filled, cost) = p.research_progress(b);
            let (rh, rn) = p.research_rare_progress(b);
            let rare = if rn > 0 {
                format!(" · r {rh}/{rn}")
            } else {
                String::new()
            };
            return Some(format!(
                "{}: research {}/{cost}{rare}",
                b.glyphs(self.seed),
                filled.min(cost)
            ));
        }
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
        let carry = p.carry_pct() as f32;
        let cache = p.cache_count() as f32;
        for r in self.routines.iter().filter(|r| r.enabled) {
            if let Trigger::When(c) = r.trigger {
                if c.min == 0 || r.armed {
                    continue;
                }
                let cur = match c.state {
                    State::Data => data,
                    State::Shards => shards,
                    State::Carry => carry,
                    State::Cache => cache,
                };
                let pct = cur / c.min as f32;
                if pct < 1.0 {
                    consider(
                        pct,
                        format!("{} {:.0}%", self.routine_display_name(r), pct * 100.0),
                    );
                }
            }
        }
        // (G15b: the old "affordable faculty" prompt is gone — faculties are research targets now;
        // a faculty under research shows via the active-research headline above.)
        if let Some((_, label)) = best {
            return Some(label);
        }
        // G18: the gentle attestation nudge (lowest priority — texture, not a quest log): a
        // comprehended-but-unconfirmed reading suggests its behavioral confirmation ("use once" —
        // minimal-English instrumentation; the name stays its glyph cluster).
        Block::ALL
            .iter()
            .copied()
            .find(|b| {
                b.required().is_some()
                    && p.is_block_comprehended(*b)
                    && p.attestation(*b) == Some(crate::progress::Attestation::Provisional)
            })
            .map(|b| format!("{}: use once", b.glyphs(self.seed)))
    }

    // ---- render -----------------------------------------------------------------------------

    /// The terminal-styled console text for the HUD/text overlay.
    pub fn render(&self) -> String {
        match self.view {
            View::Home => self.render_home(),
            View::Edit(_) | View::EditGroup(..) => self.render_edit(),
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
            let steps: Vec<String> = r.body.iter().map(|b| self.step_render(b)).collect();
            let pipe = if steps.is_empty() {
                "—".to_string()
            } else {
                steps.join(" → ")
            };
            s.push_str(&format!(
                "{cur} [{on}] {:<4} {:<9} {:<11}: {}   {}\n",
                r.agent.label(),
                self.routine_display_name(r), // G21 rider: givens show their lexicon names
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
        // G13: "trace → routine" — turn the recorded manual actions into a draft. The ticker
        // (recent manual actions of the active agent, glyph-rendered per G12) makes the feature
        // announce itself; the count tells you what'll be captured.
        let cur = if row == self.cursor { ">" } else { " " };
        let trace = self.trace(self.active_agent);
        let ticker = if trace.is_empty() {
            "(act by hand to record)".to_string()
        } else {
            trace
                .iter()
                .map(|b| b.glyph_label(self.seed))
                .collect::<Vec<_>>()
                .join(" ")
        };
        s.push_str(&format!(
            "{cur} ↻ trace → routine  [{}]  {}\n",
            trace.len(),
            ticker
        ));
        row += 1;
        s.push_str("blocks (Enter runs):\n");
        // G9: only *discovered* blocks are listed (a name you've read in the world); a discovered-
        // but-undecoded one renders **dimmed** with its stratum tag — the "I've seen this name"
        // tease. Undiscovered blocks are absent entirely.
        for b in self.visible_palette() {
            let cur = if row == self.cursor { ">" } else { " " };
            // G15: a discovered-but-not-yet-researched block reads as a locked **research target**
            // (Enter it to allocate shards). `research` is minimal-English instrumentation.
            let tag = if b.required().is_some() && !self.comprehended.contains(&b) {
                "  (locked: research)".to_string()
            } else if !b.wired() {
                "  (—)".to_string()
            } else {
                String::new()
            };
            // G18: a provisional reading (discovered, unconfirmed) carries the underdot sub-mark
            // directly after its glyph cluster — the Leiden certainty gradient on the HUD path
            // (a mark annotating glyphs, never a word). Confirmed names render clean.
            let mark = if !self.is_confirmed(b) {
                String::from(crate::text::MARK_UNDERDOT)
            } else {
                String::new()
            };
            // Dim a found-but-locked name (lowercase-dotted) so it reads as known-of, not usable.
            // G12: the block shows by its glyph-name; the `(locked: decode SCH)` tag stays
            // minimal-English instrumentation (a stratum gauge, not the block's vocabulary).
            if !self.is_unlocked(b) && b.required().is_some() {
                s.push_str(&format!(
                    "{cur} · {}{mark}{}\n",
                    b.glyph_label(self.seed),
                    tag
                ));
            } else {
                s.push_str(&format!(
                    "{cur} {}{mark}{}\n",
                    b.glyph_label(self.seed),
                    tag
                ));
            }
            row += 1;
        }
        // G21: the discovered sensing instruments — recovered faculties of the dead machine.
        // Listed only once their frustration event fired; a not-yet-researched one is a locked
        // research target (Enter allocates), a comprehended one renders clean (a held rung).
        for sense in self.visible_senses() {
            let cur = if row == self.cursor { ">" } else { " " };
            let glyphs = crate::progress::ResearchTarget::Sense(sense).glyphs(self.seed);
            if self.senses_comprehended.contains(&sense) {
                s.push_str(&format!("{cur} {glyphs}\n"));
            } else {
                s.push_str(&format!("{cur} · {glyphs}  (locked: research)\n"));
            }
            row += 1;
        }
        s.push_str("[↑↓ select · Enter run/toggle · E edit · C dup · X delete]");
        s
    }

    fn render_edit(&self) -> String {
        let Some(i) = self.editing_routine() else {
            return String::new();
        };
        let r = &self.routines[i];
        // Header + the params row (row 0): a routine's trigger, or (G14b) a group's `×times` +
        // optional `match`.
        let (mut s, params, help) = match self.view {
            View::EditGroup(_, gs) => {
                let header = format!(
                    "EDIT GROUP  in {}  (step {})   [O back]\n",
                    self.routine_display_name(r),
                    gs + 1
                );
                let params = match r.body.get(gs) {
                    Some(Step::Group { times, filter, .. }) => {
                        let f = filter
                            .map(|f| format!(" · match({})", f.glyphs(self.seed)))
                            .unwrap_or_else(|| " · match(none)".into());
                        format!("group: repeat ×{times}{f}")
                    }
                    _ => "group: (gone)".into(),
                };
                (header, params, "[↑↓ · ←→ filter · -/+ ×times · Enter insert · X remove · [ ] reorder · O back]")
            }
            _ => {
                let header = format!(
                    "EDIT ROUTINE  {}  [{}]  agent:{} [Tab]   [O back]\n",
                    self.routine_display_name(r),
                    if r.enabled { "on" } else { "off" },
                    r.agent.label(),
                );
                (header, format!("trigger: {}", r.trigger.label()), "[↑↓ · ←→ change · -/+ value · Enter insert · G enter group · X remove · [ ] reorder · Tab agent]")
            }
        };
        let cur = if self.cursor == 0 { ">" } else { " " };
        s.push_str(&format!("{cur} {params}\n"));
        // Body steps (G11: `▶` lights the step the interpreter executed this tick — top level only).
        let body = self.target_body();
        for (si, step) in body.iter().enumerate() {
            let cur = if self.cursor == si + 1 { ">" } else { " " };
            let live = if matches!(self.view, View::Edit(_)) && r.stats.executing_step == Some(si) {
                "▶"
            } else {
                " "
            };
            s.push_str(&format!(
                "{cur} {live} {}. {}\n",
                si + 1,
                self.step_render(step)
            ));
        }
        // Add-step row.
        let cur = if self.cursor == body.len() + 1 {
            ">"
        } else {
            " "
        };
        s.push_str(&format!("{cur}   + add step\n"));
        s.push_str(help);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// G21 rider (G20 review): the four GIVEN routines display their **lexicon names** (authored
    /// by the dead machine — the same words the vocabulary layer uses, per world seed); every
    /// player-created routine keeps its instrumentation default (`trace-N`, `routine-N`,
    /// `-copy`) verbatim, and no given's English name leaks into the home render.
    #[test]
    fn given_routines_display_lexicon_names_player_routines_keep_defaults() {
        let mut c = Console {
            seed: 1337,
            ..Default::default()
        };
        for r in &c.routines {
            let d = c.routine_display_name(r);
            assert_ne!(d, r.name, "a given never displays its English key");
            let word = crate::lexicon::vocab_word(c.seed, &r.name, Stratum::Records);
            let script = crate::text::Script::Latin;
            assert_eq!(
                d,
                crate::text::to_overlay(&crate::structures::transliterate(&word, script), script),
                "a given displays its seeded lexicon word (machine operations — Records/Latin)"
            );
        }
        let home = c.render();
        for english in ["drift", "survey", "prospect", "collect"] {
            assert!(
                !home.contains(english),
                "the home render leaks the English given name {english:?}:\n{home}"
            );
        }
        // Player-created routines keep instrumentation defaults verbatim.
        let i = c.create_routine(Agent::Ship);
        assert!(c.routines[i].name.starts_with("routine-"));
        assert_eq!(c.routine_display_name(&c.routines[i]), c.routines[i].name);
        c.record_manual(Agent::Foot, Block::Collect);
        let j = c.trace_to_routine(Agent::Foot).unwrap();
        assert!(c.routines[j].name.starts_with("trace-"));
        assert_eq!(c.routine_display_name(&c.routines[j]), c.routines[j].name);
        // A `run(given)` step resolves to the callee's lexicon name too.
        let drift_id = c.routines[0].id;
        let rendered = c.step_render(&Step::Run(drift_id));
        assert!(
            !rendered.contains("drift"),
            "run(callee) shows the lexicon name"
        );
        // Per-world: another seed names the givens differently (each world its own tongue).
        let c2 = Console {
            seed: 7,
            ..Default::default()
        };
        let a: Vec<String> = c.routines[..4]
            .iter()
            .map(|r| c.routine_display_name(r))
            .collect();
        let b: Vec<String> = c2.routines[..4]
            .iter()
            .map(|r| c2.routine_display_name(r))
            .collect();
        assert_ne!(a, b);
    }

    /// G12 (re-proven under G20's true names): a block's console glyph-name is the overlay form
    /// of the **exact** cluster a world name-inscription spells for it (same
    /// [`structures::name_text`] — one source). Stable, non-empty, every char HUD-renderable
    /// (never a fallback dot), and (across the unique names) distinct. Seeded: checked across
    /// several world seeds.
    #[test]
    fn block_glyphs_match_world_inscriptions() {
        use crate::structures;
        for seed in [0u32, 42, 1337] {
            for b in Block::ALL {
                let script = structures::block_script(b);
                // The intact world inscription: the cartouched true-name cluster (G20).
                let world = structures::cartouche(&structures::name_text(seed, b));
                assert_eq!(
                    b.glyphs(seed),
                    crate::text::to_overlay(&world, script),
                    "console glyphs must be the overlay of the world inscription for {b:?}"
                );
                assert!(
                    b.glyphs(seed).starts_with(crate::text::MARK_CARTOUCHE_OPEN)
                        && b.glyphs(seed).ends_with(crate::text::MARK_CARTOUCHE_CLOSE),
                    "a console name render is cartouched"
                );
                assert_eq!(b.glyphs(seed), b.glyphs(seed), "deterministic");
                assert!(!b.glyphs(seed).is_empty());
                // Every glyph the console emits is renderable by the HUD (no fallback dots).
                for c in b.glyphs(seed).chars() {
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
                if let Some(prev) = by_name.insert(b.name(), b.glyphs(seed)) {
                    assert_eq!(prev, b.glyphs(seed), "same name ⇒ identical glyphs");
                }
            }
            // Distinct *names* produce distinct clusters (collision guard, G9 spirit).
            let clusters: HashSet<String> = Block::ALL.iter().map(|b| b.glyphs(seed)).collect();
            let names: HashSet<&str> = Block::ALL.iter().map(|b| b.name()).collect();
            assert_eq!(
                clusters.len(),
                names.len(),
                "distinct block names ⇒ distinct glyph clusters (seed {seed})"
            );
        }
    }

    /// G12: a parameterised block's `glyph_label` is its glyphs plus its argument rendered glyph
    /// (so the two `scan` variants read distinctly); a plain block is just its glyphs.
    #[test]
    fn glyph_label_renders_parameter_as_glyphs() {
        let seed = 1337;
        let shards = Block::Scan(ScanItem::Shards);
        let lbl = shards.glyph_label(seed);
        assert!(lbl.starts_with(&shards.glyphs(seed)) && lbl.ends_with(')') && lbl.contains('('));
        assert_ne!(
            Block::Scan(ScanItem::Shards).glyph_label(seed),
            Block::Scan(ScanItem::Sites).glyph_label(seed),
            "the scan variants must read distinctly"
        );
        assert_eq!(Block::Drift.glyph_label(seed), Block::Drift.glyphs(seed));
        // A `match` step glyphs its field but keeps the structural keyword (instrumentation).
        let m = Step::Match(MatchField::Rare).glyph_label(seed);
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
        let t = c.tick(Agent::Ship, 0, 0, 0, 0, false);
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
        let t = c.tick(Agent::Ship, 0, 0, 0, 0, false);
        assert_eq!(t.nav, None); // no continuous nav routine ⇒ autopilot off
        assert!(t.scan); // survey still scans
    }

    #[test]
    fn authored_match_gated_collect_filters() {
        let mut c = Console::default();
        c.comprehended.insert(Block::Seek); // crack a stratum → recovers match()
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
        // No `run` resolution needed here — an empty snapshot suffices.
        let all = [(0u32, Agent::Ship, body.clone())];
        let acts = Console::expand(&body, 0, &all, 0, &mut Vec::new(), &mut None);
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
        assert!(c.tick(Agent::Ship, 5, 0, 0, 0, false).acts.is_empty()); // below threshold
        let fired = c.tick(Agent::Ship, 12, 0, 0, 0, false); // crosses ⇒ fires once
        assert_eq!(
            fired.acts,
            vec![Act {
                block: Block::Decode,
                filter: None,
                routine: 4, // the appended when-routine
            }]
        );
        assert!(c.tick(Agent::Ship, 12, 0, 0, 0, false).acts.is_empty()); // still high ⇒ no re-fire (edge, not level)
        c.tick(Agent::Ship, 0, 0, 0, 0, false); // drop below ⇒ re-arm
        assert!(!c.tick(Agent::Ship, 20, 0, 0, 0, false).acts.is_empty()); // crosses again ⇒ fires
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
        assert!(c.tick(Agent::Ship, 0, 0, 0, 0, false).acts.is_empty()); // not arrived
        let fired = c.tick(Agent::Ship, 0, 0, 0, 0, true); // reaches a site → fires once
        assert!(fired.acts.iter().any(|a| a.block == Block::Decode));
        assert!(c.tick(Agent::Ship, 0, 0, 0, 0, true).acts.is_empty()); // still there ⇒ no re-fire
        c.tick(Agent::Ship, 0, 0, 0, 0, false); // leaves ⇒ re-arm
        assert!(!c.tick(Agent::Ship, 0, 0, 0, 0, true).acts.is_empty()); // arrives again ⇒ fires
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
        c.insert_step();
        assert_eq!(c.routines[i].body.len(), 1);
        // Insert another after it, then reorder.
        c.insert_step();
        assert_eq!(c.routines[i].body.len(), 2);
        c.routines[i].body = vec![
            Step::Do(Block::Scan(ScanItem::Shards)),
            Step::Do(Block::Collect),
        ];
        c.cursor = 1; // first step
        c.move_step(1); // swap down
        assert_eq!(c.routines[i].body[0], Step::Do(Block::Collect));
        // Remove the step under the cursor.
        c.cursor = 1;
        c.remove_step();
        assert_eq!(c.routines[i].body.len(), 1);
    }

    #[test]
    fn cycle_step_skips_locked_vocabulary() {
        let mut c = Console::default();
        let i = c.create_routine(Agent::Ship);
        c.cursor = 0;
        c.insert_step(); // a step exists now (cursor on it, row 1)
        c.cursor = 1;
        // Cycle through the whole vocabulary; with nothing decoded it must never become a
        // locked block (seek/circle/goto/match).
        for _ in 0..20 {
            c.cycle(1);
            let s = c.routines[i].body[0].clone();
            assert!(c.step_unlocked(&s), "cycled into a locked step: {s:?}");
        }
        // Two-stage (G9/G15): researching seek alone is no longer enough — its *name* must
        // also have been found in the world.
        c.comprehended.insert(Block::Seek);
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
        // (bodies are authored directly; the comprehension gate doesn't affect co= round-trip.)
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
        // G15: researching a block comprehends it (and cracks its stratum → `match`); a *block*
        // additionally needs its name discovered, but a modifier doesn't.
        c.comprehended.insert(Block::Seek);
        let vocab = c.vocabulary(Agent::Ship);
        assert!(!vocab.contains(&Step::Do(Block::Seek)));
        assert!(vocab.contains(&Step::Match(MatchField::Rare)));
        c.discovered.insert(Block::Seek);
        assert!(c.vocabulary(Agent::Ship).contains(&Step::Do(Block::Seek)));
    }

    /// G17: `deposit` is a **foot-only, given** block (insertable without research, never offered
    /// to the ship); `when(carry)` / `when(cache)` triggers and a `deposit` step round-trip the
    /// `co=` codec; and `when(carry/cache)` conditions resolve against the threaded referents.
    #[test]
    fn handshake_vocab_states_and_codec() {
        let c = Console::default();
        // deposit is foot vocabulary (given — no discovery/research), and not ship vocabulary.
        assert!(c
            .vocabulary(Agent::Foot)
            .contains(&Step::Do(Block::Deposit)));
        assert!(!c
            .vocabulary(Agent::Ship)
            .contains(&Step::Do(Block::Deposit)));
        assert!(
            Block::Deposit.required().is_none(),
            "deposit is a given (Decision 1)"
        );
        assert_eq!(Block::Deposit.agent(), Some(Agent::Foot));

        // A foot routine `when(carry ≥ 100) → deposit` round-trips through `co=`.
        let mut a = Console::default();
        let i = a.create_routine(Agent::Foot);
        a.routines[i].trigger = Trigger::When(Cond {
            state: State::Carry,
            min: 100,
        });
        a.routines[i].body = vec![Step::Do(Block::Deposit)];
        // …and a ship routine `when(cache ≥ 8) → collect` (the drain side).
        let j = a.create_routine(Agent::Ship);
        a.routines[j].trigger = Trigger::When(Cond {
            state: State::Cache,
            min: 8,
        });
        a.routines[j].body = vec![Step::Do(Block::Collect)];
        let mut back = Console::default();
        back.restore(&format!("s=1&{}", a.encode()));
        assert_eq!(
            back.encode(),
            a.encode(),
            "handshake routines round-trip co="
        );

        // The carry/cache `when` edges fire on their referents (data/shards untouched).
        let mut t = Console::default();
        let k = t.create_routine(Agent::Foot);
        t.routines[k].trigger = Trigger::When(Cond {
            state: State::Carry,
            min: 100,
        });
        t.routines[k].body = vec![Step::Do(Block::Deposit)];
        assert!(
            t.tick(Agent::Foot, 0, 0, 50, 0, false).acts.is_empty(),
            "carry 50% < 100% → no deposit"
        );
        let fired = t.tick(Agent::Foot, 0, 0, 100, 0, false);
        assert!(
            fired.acts.iter().any(|act| act.block == Block::Deposit),
            "carry full → deposit fires"
        );
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
            home.contains(&Block::Seek.glyphs(0)),
            "discovered name should be listed as its glyphs"
        );
        assert!(
            !home.contains("seek"),
            "the English block name must not appear in the console"
        );
        assert!(
            home.contains("locked: research"),
            "and tagged as a research target"
        );
        // 3. Discovered + researched → insertable.
        c.comprehended.insert(Block::Seek);
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
        // Research + discover everything so neither comprehension- nor name-gating is the cause.
        for b in Block::ALL {
            c.comprehended.insert(b);
            c.discovered.insert(b);
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
        c.comprehended.insert(Block::RunFoot);
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
        c.tick(Agent::Ship, 0, 0, 0, 0, false);
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
        c.tick(Agent::Ship, 5, 0, 0, 0, false);
        assert_eq!(c.routines[wi].stats.state, RState::Waiting);
        c.tick(Agent::Ship, 12, 0, 0, 0, false);
        assert_eq!(c.routines[wi].stats.state, RState::Running);
        // A locked step → blocked(locked step), honestly, regardless of trigger.
        c.routines[wi].body = vec![Step::Do(Block::Seek)]; // undiscovered + undecoded
        c.tick(Agent::Ship, 0, 0, 0, 0, false);
        assert_eq!(
            c.routines[wi].stats.state,
            RState::Blocked(BlockReason::LockedStep)
        );
    }

    #[test]
    fn telemetry_credit_and_blocked_reasons() {
        let mut c = Console::default();
        c.tick(Agent::Ship, 0, 0, 0, 0, false); // survey runs
                                                // Credit accrues items + yields to the routine.
        c.credit(1, 3, 12, false);
        assert_eq!(c.routines[1].stats.items, 3);
        assert_eq!(c.routines[1].stats.yields, 12);
        // A zero outcome downgrades a running routine to the honest reason.
        c.tick(Agent::Ship, 0, 0, 0, 0, false);
        c.credit(1, 0, 0, false);
        assert_eq!(
            c.routines[1].stats.state,
            RState::Blocked(BlockReason::NothingInReach)
        );
        c.tick(Agent::Ship, 0, 0, 0, 0, false);
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
        c.tick(Agent::Ship, 0, 0, 0, 0, false); // opens the window at 100.0
        c.credit(1, 1, 10, false);
        assert_eq!(c.routines[1].stats.rate_per_hour(c.now), None); // no time elapsed
        c.set_now(130.0); // 30 s later
        c.tick(Agent::Ship, 0, 0, 0, 0, false);
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
        let _ = PStratum::Records; // (CollectShard import retained for the research case below)
                                   // G15: an **active research** target is the headline lit goal, named by its glyphs —
                                   // it overrides the when/faculty fallbacks (the player's chosen focus).
        let mut p2 = Progress::default();
        p2.apply(&Event::Discover { block: Block::Seek });
        p2.allocate(crate::progress::ResearchTarget::Block(Block::Seek));
        let goal = c.lit_goal(&p2).expect("active research is the lit goal");
        assert!(
            goal.contains(&Block::Seek.glyphs(0)) && goal.contains("research"),
            "names the active research target by its glyphs: {goal}"
        );
        assert!(
            !goal.contains("seek"),
            "no English block name in the goal: {goal}"
        );
        // G19: the bar shows fill/cost; a Schematics target shows no rare gauge…
        assert!(
            goal.contains("0/50") && !goal.contains("· r"),
            "fill/cost, no rare gauge on a shallow target: {goal}"
        );
        // …while a Relics target carries its rare-pickup gauge (`· r 0/4`, structural UI).
        let mut p3 = Progress::default();
        p3.apply(&Event::Discover {
            block: Block::RunFoot,
        });
        p3.allocate(crate::progress::ResearchTarget::Block(Block::RunFoot));
        let goal = c.lit_goal(&p3).expect("active research is the lit goal");
        assert!(
            goal.contains("0/200") && goal.contains("r 0/4"),
            "a deep target shows fill and the rare gauge: {goal}"
        );
    }

    /// G18: the gentle attestation nudge — a comprehended-but-unconfirmed reading lights the
    /// goal ("use once", named by its glyphs), at the **lowest** priority: active research and
    /// near-done thresholds outrank it, and a confirmed reading clears it.
    #[test]
    fn lit_goal_nudges_a_comprehended_unconfirmed_reading() {
        use crate::progress::{Event, Progress, ResearchTarget, Stratum};
        use crate::shards::Rarity;
        let c = Console::default();
        let mut p = Progress::default();
        p.apply(&Event::Discover { block: Block::Seek });
        p.allocate(ResearchTarget::Block(Block::Seek));
        let mut guard = 0;
        while !p.is_block_comprehended(Block::Seek) && guard < 10_000 {
            p.apply(&Event::CollectShard {
                domain: Stratum::Schematics,
                rarity: Rarity::Rare,
            });
            guard += 1;
        }
        // Comprehended, unconfirmed, nothing else pending → the nudge.
        let goal = c.lit_goal(&p).expect("the attestation nudge lights");
        assert!(
            goal.contains(&Block::Seek.glyphs(0)) && goal.contains("use once"),
            "nudges the unconfirmed reading by its glyphs: {goal}"
        );
        assert!(!goal.contains("seek"), "no English name: {goal}");
        // First use confirms → the nudge clears (nothing else qualifies).
        assert!(p.confirm_block_use(Block::Seek));
        assert_eq!(c.lit_goal(&p), None, "a confirmed reading clears the nudge");
    }

    /// G18 rider (§I.2): the state trigger's **display** label is `while …` now (it names a held
    /// condition, unlike the `on-…` event triggers); the `co=` codec is untouched.
    #[test]
    fn state_trigger_displays_as_while_and_codec_is_untouched() {
        let t = Trigger::When(Cond {
            state: State::Carry,
            min: 100,
        });
        assert_eq!(t.label(), "while carry ≥ 100");
        assert!(!t.label().contains("when"));
        // Event triggers keep their on-… labels.
        assert_eq!(Trigger::OnScan.label(), "on scan");
        assert_eq!(Trigger::OnArrive.label(), "on arrive");
        // Display-only: the codec still writes/reads the historic `w…` codes.
        assert_eq!(Console::trigger_code(t), "wY:100");
        assert_eq!(Console::parse_trigger("wY:100"), t);
    }

    /// G18: a provisional (discovered, unconfirmed) block renders the underdot sub-mark after its
    /// glyph cluster on the home palette; confirming it (synced from progress) removes the mark.
    /// The mark is HUD-renderable (never a fallback dot) and annotates glyphs, not words.
    #[test]
    fn provisional_blocks_render_the_underdot_mark() {
        let mark = crate::text::MARK_UNDERDOT;
        assert!(
            crate::text::overlay_glyph(mark).is_some(),
            "the underdot mark renders on the HUD glyph path"
        );
        let mut c = Console::default();
        c.discovered.insert(Block::Seek); // provisional: discovered, not confirmed
        assert!(!c.is_confirmed(Block::Seek));
        let marked = format!("{}{}", Block::Seek.glyph_label(0), mark);
        assert!(
            c.render_home().contains(&marked),
            "a provisional reading is underdotted in the palette"
        );
        // Confirmation (as synced from progress) clears the mark; starters never carry it.
        c.confirmed.insert(Block::Seek);
        assert!(c.is_confirmed(Block::Seek));
        assert!(!c.render_home().contains(mark), "confirmed renders clean");
        assert!(
            c.is_confirmed(Block::Collect),
            "starters implicitly confirmed"
        );
    }

    #[test]
    fn home_cursor_resolves_routine_new_and_block() {
        let c = Console::default();
        assert_eq!(c.selected(), Sel::Routine(0));
        let mut c = Console::default();
        c.cursor = c.routines.len(); // the "new routine" row
        assert_eq!(c.selected(), Sel::NewRoutine);
        c.cursor = c.routines.len() + 1; // the "trace → routine" row (G13)
        assert_eq!(c.selected(), Sel::TraceToRoutine);
        c.cursor = c.routines.len() + 2; // first palette block
        assert!(matches!(c.selected(), Sel::Block(_)));
    }

    /// G13: the trace records manual actions (capped, per-agent), and "trace → routine" builds a
    /// **literal** draft — exact blocks, only identical-adjacent runs folded to `repeat(n)`.
    #[test]
    fn trace_to_routine_is_literal_with_run_length_fold() {
        // Pure fold: non-adjacent repeats stay separate; adjacent identical fold to Repeat+Do.
        let scan = Block::Scan(ScanItem::Sites);
        let steps = trace_to_steps(&[scan, Block::Collect, scan, Block::Collect]);
        assert_eq!(
            steps,
            vec![
                Step::Do(scan),
                Step::Do(Block::Collect),
                Step::Do(scan),
                Step::Do(Block::Collect),
            ],
            "non-adjacent repeats must NOT collapse to a loop"
        );
        let folded = trace_to_steps(&[Block::Collect, Block::Collect, Block::Collect]);
        assert_eq!(
            folded,
            vec![Step::Repeat(3), Step::Do(Block::Collect)],
            "adjacent identical fold to repeat(n)+do"
        );
        // A run past the Repeat ceiling (9) chunks, still literal (no loss).
        let ten = vec![Block::Collect; 10];
        assert_eq!(
            trace_to_steps(&ten),
            vec![
                Step::Repeat(9),
                Step::Do(Block::Collect),
                Step::Do(Block::Collect)
            ]
        );

        // Recording: per-agent, capped, manual-only (only record_manual feeds it).
        let mut c = Console::default();
        for _ in 0..15 {
            c.record_manual(Agent::Ship, scan);
        }
        c.record_manual(Agent::Foot, Block::Collect);
        assert_eq!(c.trace(Agent::Ship).len(), TRACE_CAP, "capped at TRACE_CAP");
        assert_eq!(
            c.trace(Agent::Foot),
            vec![Block::Collect],
            "per-agent isolation"
        );

        // trace → routine: a draft ship routine of the exact (folded) blocks, opened for editing.
        let before = c.routines.len();
        let i = c
            .trace_to_routine(Agent::Ship)
            .expect("non-empty trace builds a draft");
        assert_eq!(c.routines.len(), before + 1);
        assert_eq!(c.routines[i].agent, Agent::Ship);
        assert_eq!(c.routines[i].trigger, Trigger::Continuous);
        assert_eq!(
            c.routines[i].body,
            vec![Step::Repeat(9), Step::Do(scan), Step::Do(scan)],
            "10 identical scans → repeat(9)+do, +1 leftover do (literal, chunked)"
        );
        assert_eq!(
            c.view,
            View::Edit(i),
            "opens in the editor for manual generalization"
        );
        // Empty trace → no draft.
        let mut empty = Console::default();
        assert_eq!(empty.trace_to_routine(Agent::Ship), None);
    }

    /// G13: a recorded draft is **ordinary G7 data** — once built (enabled, continuous) the
    /// interpreter ticks it like any routine, reproducing the manual behaviour automatically.
    #[test]
    fn recorded_draft_runs_through_the_interpreter() {
        let mut c = Console::default();
        // The canonical hand-loop: scan a site, then collect.
        c.record_manual(Agent::Ship, Block::Scan(ScanItem::Sites));
        c.record_manual(Agent::Ship, Block::Collect);
        let i = c.trace_to_routine(Agent::Ship).expect("draft built");
        let t = c.tick(Agent::Ship, 0, 0, 0, 0, false);
        assert!(t.scan, "the recorded scan step requests a site scan");
        assert!(
            t.acts
                .iter()
                .any(|a| a.block == Block::Collect && a.routine == i),
            "the recorded collect step emits a collect act credited to the draft"
        );
    }

    // ---- G14: subroutines ------------------------------------------------------------------

    /// `run(routine)` expands the callee's body in place; the **match register flows into the
    /// callee** (Decision 5) and the outcome credits the **callee** (Decision 4).
    #[test]
    fn run_expands_callee_in_place_with_register_flow() {
        let mut c = Console::default();
        let p = c.create_routine(Agent::Ship);
        c.routines[p].name = "plain".into();
        c.routines[p].body = vec![Step::Do(Block::Collect)];
        c.routines[p].enabled = false; // a callable subroutine (still resolves), not run standalone
        let pid = c.routines[p].id;
        // caller "main": match(rare) THEN run(plain) — the register set in the caller must reach
        // the callee's collect.
        let m = c.create_routine(Agent::Ship);
        c.routines[m].name = "main".into();
        c.routines[m].body = vec![Step::Match(MatchField::Rare), Step::Run(pid)];
        c.view = View::Home;
        let t = c.tick(Agent::Ship, 0, 0, 0, 0, false);
        let collect = t
            .acts
            .iter()
            .find(|a| a.block == Block::Collect && a.routine == p)
            .expect("run drove the callee's collect, credited to the callee");
        assert_eq!(
            collect.filter,
            Some(MatchField::Rare),
            "the one implicit register flowed from caller into callee"
        );
    }

    /// Insert-time cycle guard: self-calls and cycle-closing calls are rejected (not offered in
    /// the editor vocabulary); legal calls are offered.
    #[test]
    fn cycle_guard_blocks_self_and_mutual_calls() {
        let mut c = Console::default();
        let a = c.create_routine(Agent::Ship);
        let aid = c.routines[a].id;
        let b = c.create_routine(Agent::Ship);
        let bid = c.routines[b].id;
        assert!(c.would_cycle(aid, aid), "self-call is a cycle");
        assert!(!c.would_cycle(aid, bid), "a→b is fine while b is empty");
        c.routines[b].body = vec![Step::Run(aid)]; // b already calls a
        assert!(c.would_cycle(aid, bid), "inserting a→b would close a→b→a");
        let vocab_a = c.editor_vocabulary(a);
        assert!(!vocab_a.contains(&Step::Run(aid)), "self not offered");
        assert!(
            !vocab_a.contains(&Step::Run(bid)),
            "cyclic callee not offered"
        );
        assert!(
            c.editor_vocabulary(b)
                .iter()
                .any(|s| matches!(s, Step::Run(_))),
            "non-cyclic runs are offered"
        );
    }

    /// A cycle in a *loaded* payload (no insert guard ran) is caught failsoft at runtime by the
    /// depth cap + visited set — the tick terminates with bounded acts.
    #[test]
    fn loaded_cycle_is_failsoft_at_runtime() {
        let mut c = Console::default();
        let a = c.create_routine(Agent::Ship);
        let aid = c.routines[a].id;
        let b = c.create_routine(Agent::Ship);
        let bid = c.routines[b].id;
        c.routines[a].body = vec![Step::Run(bid), Step::Do(Block::Decode)];
        c.routines[b].body = vec![Step::Run(aid), Step::Do(Block::Decode)];
        let t = c.tick(Agent::Ship, 0, 0, 0, 0, false); // must terminate
        assert!(t.acts.len() < 100, "bounded by the cycle/depth guard");
    }

    /// Duplicate produces an independent, separately-id'd editable copy.
    #[test]
    fn duplicate_is_an_independent_copy() {
        let mut c = Console::default();
        let a = c.create_routine(Agent::Ship);
        c.routines[a].body = vec![Step::Do(Block::Decode)];
        let aid = c.routines[a].id;
        let j = c.duplicate_routine(a).expect("duplicated");
        assert_ne!(c.routines[j].id, aid, "fresh id");
        assert_eq!(c.routines[j].body, c.routines[a].body, "copied body");
        c.routines[j].body.push(Step::Do(Block::Collect));
        assert_ne!(
            c.routines[j].body, c.routines[a].body,
            "edits don't touch the original"
        );
    }

    /// `run` + stable ids round-trip through `co=`; old (no-id) payloads load (ids by index); a
    /// dangling `run` loads and degrades to a no-op at expand.
    #[test]
    fn codec_round_trips_run_and_ids_and_old_payloads() {
        let mut c = Console::default();
        let s = c.create_routine(Agent::Ship);
        c.routines[s].name = "sweep".into();
        c.routines[s].body = vec![
            Step::Do(Block::Scan(ScanItem::Sites)),
            Step::Do(Block::Collect),
        ];
        let sid = c.routines[s].id;
        let m = c.create_routine(Agent::Ship);
        c.routines[m].name = "main".into();
        c.routines[m].body = vec![Step::Run(sid)];
        let mut back = Console::default();
        back.restore(&c.encode());
        assert_eq!(back.routines, c.routines, "run-steps + ids round-trip");
        assert!(
            back.next_id > sid,
            "minting continues past the largest loaded id"
        );
        // Old 5-field payload, dangling run(#99): loads, renders run(?), no-op at tick.
        let mut old = Console::default();
        old.restore("co=main|1|c|u99|S");
        assert_eq!(old.routines.len(), 1);
        assert_eq!(old.routines[0].body, vec![Step::Run(99)]);
        assert_eq!(old.step_render(&Step::Run(99)), "run(?)");
        let t = old.tick(Agent::Ship, 0, 0, 0, 0, false);
        assert!(t.acts.is_empty(), "dangling run is a failsoft no-op");
    }

    // ---- G14b: nested step groups ----------------------------------------------------------

    /// A `Group` runs its body `times`; its `match` is **scoped** to the body (doesn't leak to
    /// later steps). One implicit register, honoured.
    #[test]
    fn group_repeats_body_with_scoped_filter() {
        let mut c = Console::default();
        let r = c.create_routine(Agent::Ship); // continuous
        c.routines[r].body = vec![
            Step::Group {
                times: 3,
                filter: Some(MatchField::Rare),
                body: vec![Step::Do(Block::Collect)],
            },
            Step::Do(Block::Decode),
        ];
        c.view = View::Home;
        let t = c.tick(Agent::Ship, 0, 0, 0, 0, false);
        let collects: Vec<_> = t
            .acts
            .iter()
            .filter(|a| a.block == Block::Collect && a.routine == r)
            .collect();
        assert_eq!(collects.len(), 3, "group repeated its body ×3");
        assert!(
            collects.iter().all(|a| a.filter == Some(MatchField::Rare)),
            "the group's match applies to its body"
        );
        let decode = t
            .acts
            .iter()
            .find(|a| a.block == Block::Decode && a.routine == r)
            .expect("the trailing step ran");
        assert_eq!(
            decode.filter, None,
            "the group's match is scoped — it doesn't leak to later steps"
        );
    }

    /// A `Group` (times + filter + inner body) round-trips through `co=`.
    #[test]
    fn codec_round_trips_a_group() {
        let mut c = Console::default();
        let r = c.create_routine(Agent::Ship);
        c.routines[r].body = vec![Step::Group {
            times: 4,
            filter: Some(MatchField::Domain(Stratum::Relics)),
            body: vec![
                Step::Do(Block::Scan(ScanItem::Sites)),
                Step::Do(Block::Collect),
            ],
        }];
        let mut back = Console::default();
        back.restore(&c.encode());
        assert_eq!(back.routines, c.routines, "group round-trips through co=");
    }

    /// The group sub-editor: enter a group, edit its inner body + header; **one level** (a group's
    /// vocabulary never offers another group); back out returns to the routine.
    #[test]
    fn group_editor_is_one_level_and_edits_inner_body() {
        let mut c = Console::default();
        let r = c.create_routine(Agent::Ship);
        c.routines[r].body = vec![Step::Group {
            times: 2,
            filter: None,
            body: vec![],
        }];
        c.view = View::Edit(r);
        c.cursor = 1; // the group step
        c.enter_group();
        assert_eq!(c.view, View::EditGroup(r, 0));
        assert!(
            !c.current_vocabulary()
                .iter()
                .any(|s| matches!(s, Step::Group { .. })),
            "one level: no group-in-group offered"
        );
        c.cursor = 0; // header row → insert at the front of the inner body
        c.insert_step();
        c.cursor = 0;
        c.adjust(1); // ×times 2 → 3
        c.cycle(1); // filter none → rare
        let Step::Group {
            times,
            filter,
            body,
        } = &c.routines[r].body[0]
        else {
            panic!("still a group");
        };
        assert_eq!(body.len(), 1, "inserted into the group's inner body");
        assert_eq!(*times, 3, "×times nudged on the header");
        assert_eq!(
            *filter,
            Some(MatchField::Rare),
            "filter cycled on the header"
        );
        c.close_editor();
        assert_eq!(c.view, View::Edit(r), "back out to the routine editor");
    }
}
