# G7 — Routine runtime & free-form editor (+ control vocabulary)

> **Status: ready to build — TOP PRIORITY.** This is the fix for the babysitter's escalation
> ([`../agent-review.md`](../agent-review.md)): the **free-form routine runtime + editor** — the
> load-bearing core of "compose your own automation" ([`../game-system.md`](../game-system.md)
> §2) — was deferred at **every** milestone G4–G6 (G4 gates → G5 steppers → G6 punted the
> control vocabulary). Build it now, in full, **before any further content**. Do **not** defer or
> scope it down; if it's genuinely too large for one milestone, split **runtime → editor**, but
> build *both* before moving on. In `scraped-again`.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Replace the **named-routine accessor hacks** (`drift_enabled`/`survey_enabled`/
  `survey_autocollects`/`nav_block`/`filter`) with a **real `Routine` model executed by an
  interpreter**, and let the player **author routines** from the console (insert/remove/reorder
  unlocked blocks, no typing). Then the first **control blocks** (`when`/`repeat`) honoured *by
  the interpreter*.
- **Demonstrable outcome.** In the console: **create a new routine**, insert
  `scan(shards) → on-scan → match(rare) → collect` from the palette, enable it, and the ship
  runs it; add a `when(shards ≥ N) → decode` rule that fires at the threshold. The two given
  routines (`drift`, `survey`) are **ordinary instances of the same model** — no per-name
  branches anywhere.
- **De-risks.** The entire "interesting purely through menus" pillar: that a general,
  interpreted routine model + a no-typing editor is viable and feels good — *before* more
  vocabulary, two-agents, or content pile on top of a substrate you can't author on.

## Scope

**In:**
- **A general `Routine` model** — not the G4 hardcoded `continuous`/`on_scan` two-bucket. A
  routine has a **trigger** and an executable **body** of steps. Cover, minimally but
  *generally*: triggers `continuous` (every tick) · `on-scan` · `when(state)` (threshold);
  steps `Action(Block)` · `Match(field)` gate · `Repeat`. Data-driven, interpretable.
- **An interpreter (runtime tick)** that executes each enabled routine whose trigger fires,
  dispatching block effects through the **existing G1–G6 effect paths** (parity). The app reads
  behaviour **from the interpreter** (nav intent, scan requests, collect requests) — **delete
  the accessor hacks**; the two given routines become **default instances** of the general model
  with **no special-casing by name**.
- **The free-form editor** in the console, **no typing**: create / delete a routine; **insert** a
  block from the (unlocked) palette at the cursor; **remove** / **reorder**; cycle a block's
  parameter (reuse G5's stepper); enable/disable. Locked blocks (G6 gating) can't be inserted.
- **Control blocks on the interpreter:** `when(state)` (pickable state + value) and `repeat`;
  `budget`/`priority` as far as is clean (a first pass, not the full set).
- **Persist authored routines** in the `pg=` payload (next version, append-only; the given
  routines are the default if absent).
- **Cleanup folded in:** parameterise `Scan` as `scan(item)` (only `shards` for now), so the
  model carries block params uniformly.

**Out:** two independent agents / cross-agent meta (**G8**); new world content; the *full*
`budget`/`priority`/meta vocabulary beyond a first pass; a node-graph UI (keep it a linear
body with optional `if`/`repeat`, not a visual graph).

## Design sketch

- `enum Trigger { Continuous, OnScan, When(StateCond) }`; `enum Step { Do(Block), If(MatchField,
  Vec<Step>), Repeat(u8, Vec<Step>) }`; `struct Routine { name, enabled, trigger, body: Vec<Step> }`.
  (Pick the simplest shape that expresses the two givens **and** `when`/`repeat`/match-gated
  collect; keep it general, not bespoke.)
- **Interpreter:** each tick, for each enabled routine whose `trigger` is satisfied (continuous
  always; on-scan on a scan hit; when when the threshold holds), walk `body` dispatching `Do`
  via the existing effect fns, honouring `If`/`Repeat`. The interpreter emits intents
  (nav-mode, want-scan, want-collect-where) the app applies — replacing the boolean accessors.
- **Editor:** a console "edit routine" mode — cursor over steps; insert-from-palette / delete /
  move / param-cycle; create/delete routine. Reuse the existing cursor + confirm + stepper.
- **Givens as data:** `drift` = `{trigger: Continuous, body: [Do(Drift)]}`; `survey` =
  `{trigger: OnScan, body:[Do(Collect)]}` + a continuous `Do(Scan)` (or a paired routine) — i.e.
  the *same* model, no `if name=="drift"` anywhere.

## Decisions to resolve (with recommended defaults)

1. **Model shape.** *Default:* **trigger + linear body with optional `If`/`Repeat`** — enough for
   the givens + control blocks; **not** a node graph. Keep it interpretable + data-driven.
2. **Editor interaction.** *Default:* cursor + **insert-from-palette / delete / reorder / param-
   cycle** (no typing); reuse G5's steppers.
3. **Persistence.** *Default:* `pg=` next version, append-only; authored routines serialise;
   absent → the given defaults.
4. **Scope split if needed.** *Default:* if one milestone is too big, land **runtime + parity +
   given-routines-as-data** first, then **editor + when/repeat** — but **both before G8**.

## Tests

- **Interpreter parity:** the two given routines run via the interpreter with **behaviour
  identical to G6** — golden voxel-hash + headless render **unchanged**.
- **Authored routine:** a constructed `scan → match(rare) → collect` collects only rare; a
  `when(shards≥N) → …` fires at the threshold (pure/logic tests).
- **Editor ops:** create/delete/insert/remove/reorder/param on the model are correct; locked
  blocks can't be inserted; authored routines **round-trip** through `pg=`.
- clippy `-D` clean; tests + wasm green; `bm-*` → game boundary intact.

## Risks & mitigations

- **Scope creep into a visual node editor.** *Mitigation:* Decision 1 — linear body + `If`/
  `Repeat`, cursor-based editing; no graph.
- **Regressing G1–G6 behaviour.** *Mitigation:* dispatch through the existing effect fns; the
  parity test (golden hash + headless) gates it.
- **Deferring *again*.** *Mitigation:* this brief exists *because* it was deferred 3×. The
  accessor hacks **must be deleted** and given routines **must be data** — that's the
  non-negotiable acceptance bar.

## Acceptance checklist

- [ ] A **general `Routine` model + interpreter**; the accessor hacks
      (`drift_enabled`/`survey_enabled`/`survey_autocollects`/`nav_block`/`filter`) are **gone**;
      the two given routines are **ordinary instances** (no per-name branches).
- [ ] The player can **create/delete routines** and **insert/remove/reorder/param** unlocked
      blocks from the console **with no typing**; authored routines **persist** (`pg=`).
- [ ] `when(state)` + `repeat` are **honoured by the interpreter**; a match-gated collect works
      in an **authored** pipeline.
- [ ] `Scan` is parameterised as `scan(item)`.
- [ ] **Behaviour-parity** for the givens — golden voxel-hash + headless render unchanged;
      interpreter + editor **tested**; clippy `-D` / tests / wasm green.
- [ ] **Roadmap re-scoped on `main`:** G7 = this; the old "G7+ two agents/expedition/arc/co-op"
      → **G8+**. Stale `console.rs` module/`Routine` docs refreshed.
