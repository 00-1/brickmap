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

- [x] Per-routine (`RoutineStats` on `Routine` — session-local, excluded from equality/`co=`):
      fires (per trigger semantics), items + yields credited, last-fired age, windowed yield/hr
      (`None`→`—` until ≥10 s of window), state from the honest enum (`Disabled`/`Running`/
      `Waiting`/`Blocked{nothing in reach, no match, locked step}`); terse row suffix
      (`×fires · y · rate · state`) + a detail line under the **selected** routine only.
- [x] Live executing-step highlight: `▶` lights the body step the interpreter executed this
      tick (the D10 press-highlight idiom; cleared when not firing).
- [x] HUD "one lit goal": `◆ <goal>` — priority-picked across when-thresholds ≥75% /
      affordable faculties / decode-ready (naming a discovered-locked block wanting the
      stratum); exactly one, nearest-to-done wins, nothing → no line.
- [x] Pure fns unit-tested: the state matrix (disabled/running/waiting/blocked × triggers ×
      outcomes), credit attribution + blocked-reason downgrade, the honest rate window
      (no fake rates, no div-by-zero), the goal picker's priority cases. Attribution is
      act-level: `Act.routine` tags every emission; the app credits resolved outcomes back
      (`credit`) across both agents' loops + the on-scan/shard paths.
- [x] Golden voxel-hash + headless render unchanged (console-only when open; the HUD line is
      text); CI green (fmt / clippy -D / 203 tests / wasm); boundary intact (no engine
      change); `pg=` untouched (Decision 1 — session-local working memory).
- [x] Roadmap G11 entry + this checklist ticked on `main`.

## As-built (2026-06-11) — assumptions recorded

1. **Attribution is exact for routine-driven collects** (the act carries its routine; the app
   measures the progress delta around each resolution) — not the emission-count approximation
   the brief allowed. Manual collects (T/beam/console clicks) are unattributed by design.
2. **`fires` semantics per trigger:** continuous = ticks run (large numbers are honest);
   when/on-arrive = edge fires; on-scan = hits (`note_scan_fire`).
3. **Blocked-reason precedence:** a locked step is reported even while the trigger fires
   (it's the more actionable truth); reach/match reasons arrive from the collect outcome.
4. **The step highlight** marks the last `Do` executed in the tick (bodies execute atomically
   per tick — there is no mid-body suspension to animate; honest, no animation machinery).
