# Game depth — the deepening plan (2026-06-11)

> The plan for making *Scraped Again* **deep**, now that the machinery is built. The block
> substrate (G4–G8) is a real interpreter + editor + two agents; what's thin is the *decision
> space* — vocabulary, economy, legibility, and the long arc. This doc turns the design spine
> ([`game-mechanics.md`](game-mechanics.md), [`game-system.md`](game-system.md)) plus the
> research ([`research-automation-depth.md`](research-automation-depth.md), P-numbers below;
> [`research-decipherment.md`](research-decipherment.md)) into a dispatchable milestone
> pipeline. Open forks the human must call live in [`open-questions.md`](open-questions.md).

## 1. The depth thesis

Depth = **decisions that stay interesting**, and for this game they come from five places:

1. **A growing vocabulary that restructures play** — not +% upgrades. Each new block
   (especially control/meta) reopens every old routine (P11, P13). G9 makes acquiring it
   exploration-driven.
2. **An economy with real scarcities** — typed/rare currency (G10), capacities, queues —
   so filters, budgets, and priorities have something to decide *about* (P1, P2).
3. **Legibility** — you can only love a machine you can read. Telemetry, live step
   highlights, "why is this idle?" (P20). The genre's highest-leverage UI.
4. **Two asymmetric agents negotiating through the world** — handshakes the player authors
   both sides of (P18, P19).
5. **An endgame of self-directed optimization** — conflicting metrics, authored bands, no
   fail states (P5, P6) — plus the witnessing arc: operator → author → overseer (P16),
   which is also the melancholy theme made mechanical.

**Standing audits** (apply to every milestone below):
- *Scarcity audit:* every newly automatable loop must surface a new visible bottleneck (P1).
- *Copy-paste audit:* new content/regions must not be served by last region's routine
  unchanged (P7).
- *No-dead-blocks audit:* primitives stay load-bearing inside late routines (P14).
- *Optionality audit:* the wiring depth stays skippable; the core completes naively (P4).
- *Perf charter:* every streamed layer + splat consumer follows
  [`performance.md`](performance.md) §4.

## 2. The dispatch pipeline (this run)

In dispatch order. G9 in flight; M10/G10 staged; G11+ briefed-on-demand from the sketches
below (each gets a full brief just before dispatch, per the two-layer planning norm).

### G9 — Names in the world (block discovery) — *dispatched*
Inscriptions carry block names; collect → discover (listed, locked); decode → unlock.
Exploration drives vocabulary. *(Brief: [`milestones/G9-block-name-discovery.md`](milestones/G9-block-name-discovery.md).)*

### M10 — Perf telemetry & budget gates — *staged (engine guardrail before content)*
Frame-cost counters + CI budget tests, so the content milestones below can't silently eat
the weak-hardware headroom. *(Brief: [`milestones/M10-perf-telemetry.md`](milestones/M10-perf-telemetry.md).)*

### G10 — Typed shards — *staged*
5 domains × 3 rarities, world-scattered, auto-collectible, first spend faculties. Makes
`match`/`priority` matter. *(Brief: [`milestones/G10-typed-shards.md`](milestones/G10-typed-shards.md).)*

### G11 — Routine telemetry ("the machine answers why") — *sketch*
The P20 milestone; pure game-side, mostly console UI + interpreter instrumentation.
- Per-routine counters: fires, yields (by what it collected), last-trigger time; live
  **step highlight** while a routine runs; a routine **state line**: `running` /
  `waiting: <trigger>` / `blocked: <reason>` (reach, nothing-in-range, locked step,
  cargo-full once G14 lands) — the one-tap "why".
- HUD: the **one lit goal** (P17): nearest almost-done thing (a threshold at 87%, an
  affordable faculty, an undecoded stratum with data banked). One line, never a quest log.
- **State triggers render as live gauges** (`when/while(shards ≥ 50)` shows `37/50` on the
  block) while event triggers flash on fire — the measured TAP event/state-confusion fix
  ([`research-block-language.md`](research-block-language.md)) riding telemetry's data.
- Console home shows per-routine yield/hr once enough samples exist (`—` before that).
- *Why now:* every later economy/agent milestone becomes debuggable + lovable through this.

### G12 — Record-to-program — *sketch (research-updated)*
The P9 on-ramp, very on-brand (the dead console remembers your hands). The PBD literature
([`research-block-language.md`](research-block-language.md): Eager, Lieberman post-mortems)
pins the contract: **record literally, generalize manually** — silently-inferred intent is
what killed every programming-by-demonstration system.
- Manual block presses (and the beam-collects/scans they map to) append to a rolling
  per-agent **action memory** (last ~10).
- **"trace → routine"** creates a draft of the *exact* concrete blocks — only mechanical
  folding of identical adjacent repeats into `repeat(n)`; **no dropping "noise", no
  generalization** — opened in the editor where the *player* generalizes via steppers
  (Victor's create-by-abstracting closes the loop PBD couldn't).
- Playback borrows Eager's trick: pre-highlight the agent's next world target so the
  routine's intent is verifiable at a glance.
- The draft is ordinary G7 data; zero new runtime semantics. Discoverability: the console
  shows the trace filling as you act (a faint ticker — expose-the-tech).

### G13 — Subroutines, templates & nested steps — *sketch (research-updated)*
The P10 tedium-killers + the deferred "nested/grouped steps" polish, folded together. The
tiny-language research adds two requirements ([`research-block-language.md`](research-block-language.md)):
**no seams** (Steele — a player's named routine must appear in the palette looking exactly
like a primitive block) and the **one-implicit-register rule** (concatenative stack-shuffle
failure mode — exactly one "current thing" flows between steps; a second implicit referent
is forbidden, make it a parameter).
- `run(routine)` extended to **same-agent** calls (it exists cross-agent since G8c); cycle
  guard (a routine can't call into a loop — static check at insert time).
- **Duplicate routine** in the editor; later "template" if duplication proves common.
- **Nested step groups**: `repeat`/`if` bodies can contain groups (the G7 model already has
  `Vec<Step>` bodies — surface editing them in the cursor UI without becoming a tree IDE;
  one nesting level is probably enough, pin that).

### G14 — Comprehension-as-research (the unified economy) — *human-endorsed model (2026-06-11); pacing TBD*
The reframe the human proposed and endorsed: **discovery + shards + decode are ONE pipe.**
Find a block's glyph-name in the world → it becomes a **research target** (locked); your
auto-collected **shards** are poured into it until research completes → the block is
**comprehended (usable)**; keep feeding for **levels** (parameter unlocks / modest boosts).
"Decoding" and "spending" are the same act. This unifies G9 (discovery), G10 (shards), and
the old decode economy, and resolves several forks at once:
- **Fork C (treadmill):** shards buy **new verbs**, not bigger numbers — progression stays
  vocabulary growth; numbers serve unlocking composition. The not-a-clicker property holds.
- **Fork B.3 (domain-matched):** a block's research consumes (mostly) its **stratum's
  domain** of shards — so deep/rare blocks are gated behind exploring rarer strata, tying
  the management game to manual expeditions (game-system §11's intended gating, mechanized).
- **Fork B.1 (shard↔strata):** shards are the **research substrate**, not a sixth currency
  competing with strata data — strata *are* the shard domains.
- **Fork B.2 (fill vs spend):** leaning **gradual fill** (comprehension takes time) — the
  exact mechanic is the one open sub-point (pacing): allocate-and-fill-over-time
  (idle-resonant, pairs with autopilot) vs bank-then-commit vs hybrid. *Prose decision
  pending; default if unspecified = allocate-and-fill, as the most idle/theme-resonant and
  the closest to the human's "add shards to them" phrasing.*
- **Faculties** (G10's sensing/reach/drive) become **ordinary research targets** in the
  same pipe (game-system §3: "an upgrade is just the spend action") — no separate +% subsystem.
- **G11's "one lit goal"** now naturally points at the nearest-to-complete research.
- **Scarcities still apply** (the original G14 content, now downstream of research):
  carry/buffer caps (asymmetric per agent), a `deposit`/cache verb (first handshake
  primitive), `when(state)` expansion (shards/buffer/range/research-progress), cost pacing
  against the G9 discovery cadence (vocabulary events break walls — Pecorella P13).
- *Still fork-gated:* the pacing sub-point above; whether levels are pure parameter-unlocks
  vs include any capped +% (lean parameter-unlocks; faculties the only modest +%).
- **Note:** G10 already shipped bank-then-spend faculties — so this milestone partly
  *retrofits* G10's `spend` into the research model. Frame as evolution, keep `pg=` compat.

### G15a — Lexicon v2: statistical honesty — *sketch; fork-FREE (buildable any time)*
The one Archive-tranche piece gated on nothing: make the seeded lexicon's output pass the
tests pattern-hunting players will run ([`research-linguistics.md`](research-linguistics.md)
§2 checklist): Zipf rank-frequency, Heaps' law, 3–4 bits/char conditional entropy, Zipf
abbreviation, a small consistent affix inventory, bursty content-words, no adjacent-word
similarity or line-position artifacts — **every property a unit test** (the repo's
test-pure-logic norm). Plus the corpus-shape rules: some long texts, structured
name+logogram+numeral lists, one recurring ritual frame for Rites. Slots anywhere in the
queue; pure logic, golden-neutral (content-keyed like G9).

### G15 — Handshakes & the expedition economy — *sketch*
P18 made real on the G8 expedition: caches the walker fills and the ship collects
(coordination via world state only, no direct agent RPC); failed-handoff vignettes (the
walker waiting at an empty drop — visible, melancholy, legible); per-agent routine-slot
asymmetry (P19) introduced as a *discovered* world property, not a menu number (P12).

### G16+ — the sketched horizon (briefed when reached)
- **Transient world events** (P15): passing signals, aurora windows; `when(event)`;
  routines catch 60%, presence catches 100%.
- **Routine metrics & authored bands** (P5/P6): yield/hr · cost-per-unit · block-count
  against "crude / sound / cunning / uncanny" bands. Needs G11 telemetry + G14 costs.
- **Region conditions** (P7): biome-level operating conditions (interference, tides) that
  invalidate copy-paste routines. Pairs with E8 vertical stacks / new world content.
- **The witnessing arc** (P16): act emphasis shifts operator → author → overseer; mostly
  free (it emerges from the ladder) — needs a design pass in game-mechanics, not a system.
- **The Archive tranche** — ✅ **green-lit by the human (2026-06-11) with a binding
  constraint: no readable lore, ever** — inscriptions stay lexicon nonsense-words in the
  five scripts; the player comprehends *structure* (names, frames, erasures, hands,
  cognates, registers-by-format), never prose — **and block names are unreadable too**
  (human, 2026-06-11): blocks are glyph-named everywhere, learned by clicking (L0 is the
  teacher). A **de-Anglicization retrofit milestone** is queued in the dispatch pipeline.
  (The decipherment deepening, research-shaped — see
  [`research-linguistics.md`](research-linguistics.md),
  [`research-material-text.md`](research-material-text.md),
  [`research-decipherment.md`](research-decipherment.md)): a coherent multi-milestone
  direction, fork-gated on [`open-questions.md`](open-questions.md) §A/§D —
  **Leiden display grammar** (brackets/underdots/⟦erasure⟧ as the survey log's state
  system) · **cartouches** (a name-bracket glyph pair — the real first foothold of every
  historical decipherment) · **formulaic frames** (epitaph/curse/ledger registers; crack a
  frame once, harvest names forever) · **the sensing ladder** (raking → UV → multispectral
  → penetrating; palimpsest undertexts; re-processing old scans as a free late win) ·
  **factoid-graph prosopography** (attributable hands — the player joins the records of
  one dead engineer) · **one proto-language, five daughter scripts** (deterministic
  sound-change cascades → detectable cognates; the endgame comparative puzzle) ·
  **compositional agglutinative names** (Kober-able morphology + bouba/kiki sound
  symbolism — the substrate the knowledge-gate fork needs) · **statistical honesty**
  (the lexicon generator passes Zipf/Heaps/entropy/morphology tests — CI-checkable, per
  this repo's test-pure-logic norm) · **hauntological content slant** (cancelled-future
  texts; ~85% documentary; no expository diaries; the Vindolanda autograph-postscript
  pattern).
- **Audio identity** ([`research-audio.md`](research-audio.md)): lament-bass ostinato,
  incommensurate swell cycles, per-stratum earcon family + a console prosody voice,
  **phone-mode virtual bass** (the weak-hardware audio gap — our sub is inaudible on
  phone speakers), FDN vastness tuning — buildable blind behind toggles, ear-tuned at the
  human pass.
- **Console language polish** ([`research-block-language.md`](research-block-language.md)):
  event-vs-state trigger shapes (`on-…` vs `while-…` — the measured TAP confusion),
  aggregate parameters over `repeat`, the one-implicit-register rule, failsoft semantics
  named honestly — folds into the G12/G13 briefs and the console's evolution.

## 3. What this run does NOT build

- A 2D node canvas (linear step-lists are a design commitment, not a budget cut — P10).
- Prestige/reset mechanics (wrong tone; vocabulary events are our walls — P13).
- Numeric +% upgrade trees beyond the modest faculties (hedonic treadmill — anti-patterns).
- Multiplayer/N1, new engine render features (unless a game milestone needs a generic
  primitive, per the M9 seam discipline).
