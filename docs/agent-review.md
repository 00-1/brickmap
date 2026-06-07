# Agent-review log (babysitter)

Running critical review of the **parallel agent** building the Scraped Again G-series (and
the rest of the roadmap) **on `main`**, while this branch
(`claude/core-mechanics-planning-0TpOA`) holds design + this log. Periodic: each pass fetches
`origin/main`, reviews anything new against [`game-system.md`](game-system.md) +
[`game-mechanics.md`](game-mechanics.md) + the milestone briefs, and records an entry.
**Newest entry on top.** Critical where criticism is due; praise where earned. This branch
only — never pushes to `main`.

**Reviewed through:** `f1adfb7` (G4).

---

## 2026-06-07 · G4 — block substrate & operations console (`f1adfb7`)

**What landed.** A `console` module in `scraped-again` (227 lines): a Tier-0 block enum
(`Scan`/`Collect`/`FireBeam`/`Spend`/`Goto`/`Drift`/`OnScan`), two given routines (`drift`;
`survey` = scan → on-scan → collect) shown as their blocks, cursor/toggle model, terminal
render — pure, 4 unit tests. Wiring in `lib.rs`: `O` opens the console, ↑↓ select, Enter
run/toggle; manual block clicks go through `dispatch_block` (the real G1–G3 effect paths);
`scan_pulse` shared by the survey routine + a manual scan click. 137 tests green, clippy
clean, wasm builds, golden hash + headless render unchanged.

**Strengths.**
- **Faithful to the brief's intent**: re-expression, not rewrite — `dispatch_block` reuses
  the existing collect/scan/beam paths, so behaviour-parity is real (and tested).
- **Excellent discipline on the autonomy ask**: a clear *"Assumptions / decisions taken solo"*
  header in `console.rs` documenting the three judgment calls — exactly what unattended work
  should leave behind.
- Clean, well-tested pure model; on-aesthetic terminal render on the E17/HUD path; no-typing
  cursor+confirm (controller/phone-first) as designed.

**Critiques (where it's due).**
1. **The "runtime" is not yet a runtime — the given routines are *boolean gates*, not
   interpreted.** `survey_enabled()` / `survey_autocollects()` / `drift_enabled()` are queried
   by hand-written branches in `lib.rs`; only *manual* clicks are genuinely dispatched. For
   G4's two fixed routines + parity this is a fair shortcut (and the code admits "G5's editor
   will generalise it"), **but there is no general trigger→steps interpreter yet.** This is the
   #1 risk for G5: the editor must introduce a real interpreter, *not* extend the
   flag-gating — otherwise player-authored routines won't have an execution model. Watch that
   G5 doesn't bolt the editor onto booleans.
2. **Blocks aren't parameterised yet.** `Block` is a param-less enum; `Scan` is hardcoded and
   merely *labelled* `scan(shards)`. The design's "parameterised blocks (a single typed arg
   whose options unlock)" — `scan(item)`, `match(field)` — isn't modelled. Fine while only
   `shards` exists, but G5/G6 will need to retrofit a param onto `Block` (a small refactor);
   the brief technically said G4 ships `scan(item)`, so this is a minor drift to keep honest.
3. **The given auto-collect is largely inert at cruise** (their assumption #1): collect reuses
   `collect_aimed`, so sites below the aim ray aren't taken until you fly low (or reach grows
   in G6). Honest and parity-preserving — but it means the headline "auto-collect closes the
   hands-off loop" doesn't visibly *do* much yet. Acceptable for G4; **G6 must actually make
   auto-collect meaningful**, or the "autopilot is a complete way to play" pillar stays
   unproven.
4. **"Clickable blocks" is, for now, cursor+confirm** (mouse hit-testing deferred to G5).
   Reasonable deferral; just noting the design word "clickable" is aspirational at G4.

**Verdict.** Good, honest, well-tested G4 that achieves the stated goal (parity + the console
surfaced) and documents its shortcuts. No correctness concerns. The deferrals are legitimate
*provided* G5 delivers a genuine routine interpreter and block parameterisation rather than
extending the G4 gates — that's the thing to hold the next milestone to.
