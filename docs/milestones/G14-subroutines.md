# G14 — Subroutines, templates & nested steps

> **Status: ready to build — next directive** (cleared after the G13 RED resolved + CI
> confirmed green on `30daefb`). The P10 tedium-killers from the automation research
> ([`../research-block-language.md`](research-block-language.md)) + the deferred
> "nested/grouped steps" polish, folded together — the depth layer that lets routines
> *compose* instead of bloat. Game-side (`scraped-again` console + the G7 interpreter);
> no engine change expected.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** A player's own routine becomes a **reusable named block** they can call from
  other routines (same agent), duplicate as a starting point, and nest a little — so late
  play reads as short compositions of named pieces (Forth/Moore "vocabulary" depth), not
  20-step monoliths.
- **Demonstrable outcome.** Author a routine "sweep" (`scan → on-scan → match(rare) →
  collect`); in another routine, insert **`run(sweep)`** as a step from the palette — it
  appears there as an ordinary (glyph-named) block; the interpreter runs the callee in
  place. Duplicate "sweep" to start a variant. Put a `match`/`repeat` body with **two
  steps** inside it and edit them in the cursor UI.
- **De-risks.** The "interesting purely through menus" ceiling: that depth comes from
  *composition* (named reuse) rather than length — the documented anti-viscosity fix, and
  the thing that keeps the no-typing editor usable as routines grow.

## Scope

**In:**
- **Same-agent `run(routine)`** as a `Step`. (`run` exists cross-agent since G8c — extend
  to same-agent calls.) The interpreter executes the callee's body in place when the step
  runs; results/intents flow through the existing act paths (and credit attribution per
  G11 — a called routine's yields credit sensibly; pin who gets the credit in Decisions).
- **Cycle guard:** a routine cannot `run` itself transitively — a **static check at insert
  time** rejects the insertion (and a runtime depth cap as a backstop). No infinite
  recursion; surfaced as a normal "can't insert" (failsoft, no crash).
- **No seams (Steele):** a player routine offered as `run(routine)` renders in the palette
  and rows as an **ordinary glyph-named block** (G12), indistinguishable in form from a
  primitive — user-defined words look like primitives.
- **Duplicate routine** in the editor (one action → a copy to mutate). "Template" (a
  named reusable starting point) only if duplication proves insufficient — defer.
- **Nested step groups:** `Repeat`/`If(Match)` bodies may contain a **group of steps**
  (the G7 model already has `Vec<Step>` bodies — surface *editing* them in the cursor UI).
  **One nesting level** (a group may not itself contain a group) — pinned, to stay a
  linear-with-grouping editor, **not** a tree IDE.
- **One implicit register (the concatenative rule):** exactly one "current thing" flows
  between steps within a body (the last scanned/collected/matched); a called routine sees
  it on entry and may leave it changed on return — **no second implicit referent** (if a
  design needs two, make one an explicit parameter). Document + test this invariant.
- **Persist:** `run`-steps and nested groups serialise in `pg=` (next version, append-only;
  old payloads load — a missing callee degrades failsoft to a no-op step, noted).

**Out:** cross-agent meta beyond the existing `run(foot)` (that's G8's territory, done);
visual node-graph editing (the explicit anti-goal — linear bodies + one nesting level);
templates-as-library (defer unless duplication is insufficient); new runtime semantics
beyond call + nesting.

## Design sketch

- `Step::Run(routine_ref)` — reference a routine by a stable id (per-agent routine index or
  a small id; survive reorder/rename — pin the ref form in Decisions). Interpreter: on
  `Run`, execute the callee body with a **depth counter** (cap, e.g. 8) and the cycle set.
- Insert-time cycle check: building the call graph among a (small) routine set and
  rejecting an edge that closes a cycle — cheap, pure, unit-testable.
- Editor: `run` is a palette entry listing the agent's *other* routines (glyph-named);
  duplicate is an action on the selected routine; nested-group editing reuses the existing
  cursor/insert/remove/reorder on the inner `Vec<Step>` (one level — the UI simply doesn't
  offer "insert group" inside a group).
- Credit (G11): a called routine's collect outcomes credit **the callee** (it did the work)
  — or the caller (it orchestrated)? *Pin in Decisions* (lean callee, so per-routine
  yield/hr stays meaningful for the unit that acts).

## Decisions to resolve (pinned defaults — veto via the channel)

1. **One nesting level** (group inside `repeat`/`if`; a group can't contain a group) — keeps
   it a linear-with-grouping editor, not a tree. *Pinned.*
2. **Cycle guard at insert time** (+ runtime depth cap backstop); rejection is failsoft.
   *Pinned.*
3. **Routine ref form:** a stable per-agent id that survives reorder/rename (not a raw
   index). *Pinned* (indices break on reorder).
4. **Call credit → the callee** (the routine that acts), so G11 yields stay legible. Veto
   → caller, if you'd rather orchestrators show the throughput.
5. **One implicit register** (concatenative rule) — non-negotiable; a second implicit
   referent is forbidden. *Pinned.*

## Tests

- `run(routine)` executes the callee's body in place; behaviour matches inlining the body
  (interpreter-level parity, like G7's tests).
- Cycle guard: a self/mutual call is rejected at insert; a deep legal chain runs within the
  depth cap; a cycle that somehow exists (loaded payload) is caught by the runtime cap
  failsoft.
- Nested group: edit (insert/remove/reorder) inside a `repeat`/`if` body; the one-level
  cap holds (no group-in-group offered).
- One-register invariant: the "current thing" semantics across a call (callee sees it,
  may change it) — a pure/interpreter test.
- Duplicate produces an independent editable copy. `pg=` round-trips `run`+groups; old
  payloads load; missing-callee degrades to no-op.
- Golden voxel-hash + headless render unchanged; CI green (fmt / clippy -D / tests / wasm);
  boundary intact (game-side); roadmap G14 entry.

## Risks & mitigations

- **Recursion/blowup** → insert-time cycle guard + runtime depth cap (Decision 2).
- **Editor → tree IDE creep** → one nesting level, linear bodies (Decision 1; the anti-goal).
- **Two-referent temptation** → the one-register rule is a hard invariant (Decision 5); if a
  feature seems to need two, it's a parameter, not a second register.
- **Credit ambiguity in G11** → Decision 4 pins it; document on the routine detail line.

## As built (2026-06-11) — G14a; nested groups split to G14b

The `run(routine)` composition core shipped as **G14a**; the **nested step groups** half is split
to **G14b** (it needs the deeper `Step` model change — turning the `Repeat`/`Match` *prefix
modifiers* into bodied containers + the editor for one-level inner bodies — riskier, with a codec
migration, so it's its own milestone per the "split big milestones" rule). Game-side only; no
engine change. **Persistence note:** routines live in the console's **`co=`** segment, not `pg=`
(the brief said `pg=` — corrected here; `co=` is where authored routines round-trip).

- **`Step::Run(RoutineId)`** — same-agent call; the interpreter (`expand`) expands the callee's
  body in place via a per-tick routine snapshot, crediting the **callee** (Decision 4). A stable
  per-console `id` (minted monotonically, persisted as a trailing `co=` field, survives
  reorder/rename — Decision 3) is the ref. `run` is offered in the editor cycle for every other
  same-agent routine (no seams), resolved to `run(<name>)` at render.
- **Cycle guard** — `would_cycle` (pure DFS) rejects self/cyclic calls at insert time
  (`editor_vocabulary` omits them); `RUN_DEPTH_CAP` + a visited set are the runtime backstop, so a
  hostile/old payload's cycle is failsoft (bounded, no blowup).
- **One implicit register (Decision 5)** — the `match` filter is the single "current thing"; it
  flows **into** a called routine (shared `&mut`) and persists on return. No second referent.
  Tested across a call.
- **Duplicate** — `duplicate_routine` (key `C` on a selected routine) → an independent,
  freshly-id'd editable copy.
- **Persist** — `co=` gains the `u{id}` step code + the trailing id field (append-only; old
  5-field payloads load with ids assigned by index; a dangling `run` loads + degrades to a no-op).

## Acceptance checklist

- [x] `run(routine)` same-agent step; callee runs in place; offered + rendered like an ordinary
      block (`run(<name>)`; routine names are author labels, English per G12).
- [x] Insert-time cycle guard (`would_cycle` + `editor_vocabulary` omission) + runtime depth cap
      (`RUN_DEPTH_CAP` + visited, failsoft); duplicate-routine action (key `C`).
- [~] Nested step groups in `repeat`/`if` bodies — **split to G14b** (model change; noted above).
- [x] One-implicit-register invariant documented + tested (the `match` register flows across a call).
- [x] `run` persists (`co=`, append-only; old payloads load; missing callee → no-op); G11 credit
      pinned to the **callee**.
- [x] Golden voxel-hash + headless render unchanged; CI green; boundary intact; roadmap G14 entry.
