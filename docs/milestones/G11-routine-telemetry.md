# G11 — Routine telemetry ("the machine answers why")

> **Status: planned — do NOT build until the channel directs it** (queued after G10). The
> research's highest-leverage UI finding for this genre
> ([`../research-automation-depth.md`](../research-automation-depth.md) P17/P20): automation
> stays loved only while it stays *legible* — per-routine telemetry, a live view of execution,
> and a one-tap answer to "why is this routine idle?". Pure game-side (`scraped-again`):
> interpreter instrumentation + console/HUD rendering on existing paths. No engine changes.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Every routine reports what it's doing and why: lifetime counters (fires, yields),
  a state line (`running` / `waiting: <trigger>` / `blocked: <reason>`), a **live step
  highlight** while executing, and a HUD "one lit goal" line (the nearest almost-done thing).
- **Demonstrable outcome.** Open the console while the given routines run: each routine row
  shows its state and counters ticking; the executing step glows as the interpreter walks
  it; a stalled authored routine (e.g. `match(rare)` with nothing rare in range) reads
  `waiting: no match in range` instead of sitting inert; the HUD shows one line like
  `decode Records 87%` or `sensing affordable`.
- **De-risks.** The "interesting purely through menus" pillar's *retention* half: the
  difference between a management layer that feels alive and one that feels opaque. Also
  the substrate every later metrics/bands feature (G16+) reads from.

## Scope

**In:**
- **Interpreter instrumentation.** Per-routine, accumulated in the runtime (not persisted):
  total trigger fires; yields attributed to the routine (what its collects brought in, by
  item); last-fired tick/age; current execution state. State derivation: `disabled` /
  `running` (trigger satisfied this tick) / `waiting: <trigger label>` (enabled, trigger
  unsatisfied) / `blocked: <reason>` (trigger fires but the body can't act — pick from a
  small honest set the interpreter actually knows: `nothing in reach`, `no match`,
  `locked step`, plus capacity reasons when G14 lands).
- **Console rendering.** Routine rows gain a compact state/counter suffix (existing text
  path; respect the terse terminal idiom — `run 412 · y 1.2k · wait:scan` not prose). In
  edit/view mode, the **currently-executing step highlights** on the tick it runs (reuse
  the press-highlight idiom from D10 — brief glow, no animation machinery).
- **Yield rate.** Per-routine yield/hr derived from a windowed sample (show `—` until the
  window has data — never fake a rate).
- **The one lit goal (HUD).** One line, priority-picked from: a `when` threshold ≥ ~75% of
  its target · an affordable-but-unbought faculty (post-G10) · a stratum with banked
  undecoded data · a discovered-but-locked block whose stratum is part-banked. Exactly one,
  nearest-to-done wins; nothing qualifying → no line (never a quest log).
- **Persistence: none.** Counters are session-local (the dead console's working memory);
  `pg=` untouched.

**Out:** historical graphs; metrics scoring/bands (G16+); alerts/notifications; any
engine change; the G14 capacity reasons (the enum is just extensible).

## Design sketch

- A `RoutineStats` table in (or beside) the console runtime keyed by routine index/agent —
  updated inside the existing `tick`/dispatch where fires and act-emissions already happen;
  yields attributed where collect events resolve (thread the originating routine through
  the act → collect path; the G7 intent structs likely carry or can carry it).
- State derivation is a pure fn of (routine, trigger eval, last body outcome) — unit-test
  the matrix. Honesty rule: only report reasons the interpreter actually evaluated; if
  unknowable, say `idle`, don't invent.
- The lit-goal picker: pure fn of (progress state, console state) → `Option<String>`;
  unit-test the priority ordering.
- Step highlight: console render reads "executing step index" recorded per routine per tick.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Session-local counters** (no persistence) — the working-memory framing fits the tone
   and dodges `pg=` bloat. Revisit if play disagrees.
2. **Terse row suffix** in the console list; details (yield breakdown, last-fired age) on
   the *selected* routine only — keeps rows scannable on a phone.
3. **Honest blocked-reasons** from a small fixed enum; no speculative diagnosis.
4. **One goal line max**, threshold ~75%, nearest-to-done wins.

## Tests

- State-derivation matrix (disabled/running/waiting/blocked × trigger kinds × body
  outcomes); blocked-reason correctness for `nothing in reach` / `no match` / `locked step`.
- Yield attribution: an authored `scan → on-scan → collect` accrues its collects to the
  right routine across two agents (interpreter-level, like G7's parity tests).
- Windowed rate: `—` before data; correct after; no div-by-zero on idle.
- Lit-goal picker priority/threshold cases.
- Golden voxel-hash + headless render unchanged (console-only when open; HUD line is text);
  CI green (fmt / clippy -D / tests / wasm); boundary intact.

## Risks & mitigations

- **Console clutter** (phone-width rows) → terse suffix + details-on-selected (Decision 2).
- **Attribution plumbing** (threading routine identity to collect resolution) → if the act
  path can't carry it cleanly, attribute at emission (count *requested* collects) and note
  the approximation — don't contort the G7 model for perfect accounting.
- **Tick-cost creep** → counters are increments; state derivation per routine per tick is
  O(routines); no allocation in the tick path.

## Acceptance checklist

- [ ] Per-routine: fires, yields (by item), last-fired age, windowed yield/hr (`—` until
      sampled), state line from the honest enum; shown terse in rows + detailed on selected.
- [ ] Live executing-step highlight in the console while a routine runs.
- [ ] HUD "one lit goal" line (priority-picked, max one, never a log).
- [ ] Pure fns unit-tested (state matrix, attribution, rate, goal picker); interpreter-level
      attribution test across both agents.
- [ ] Golden voxel-hash + headless render unchanged; CI green; boundary intact; `pg=`
      untouched.
- [ ] Roadmap G11 entry + this checklist ticked on `main`.
