# Agent-review log (babysitter)

Running critical review of the **parallel agent** building the Scraped Again G-series (and
the rest of the roadmap) **on `main`**, while this branch
(`claude/core-mechanics-planning-0TpOA`) holds design + this log. Periodic: each pass fetches
`origin/main`, reviews anything new against [`game-system.md`](game-system.md) +
[`game-mechanics.md`](game-mechanics.md) + the milestone briefs, and records an entry.
**Newest entry on top.** Critical where criticism is due; praise where earned. This branch
only — never pushes to `main`.

**Reviewed through:** `24067a1` (G5).

---

## 2026-06-07 · G5 — console editor (pickers), match & nav (`24067a1`)

**What landed.** `Block` gains `Seek`/`Circle` and a **parameterised** `Match(MatchField)`
(v1 field: `Rare`). `cycle_param` (←/→) steps the *parameter* of the routine under the
cursor — `drift`'s nav block (drift→seek→circle) and `survey`'s collect filter
(none↔`match(rare)`). `collect_aimed_where(pred)` lets auto-collect apply the filter; `seek`
steers the autopilot to the nearest known-uncollected site. Routine edits persist in a `co=`
share segment (round-trip tested). 140 tests green, clippy clean, golden hash unchanged.

**Strengths.**
- **Honest scoping** — an explicit *"As-built vs the original plan"* section in the brief
  states it shipped parameter-steppers, **not** free-form authoring, and a sensible in-flight
  correction (dropped `match(uncollected)` as a no-op, used `rare`). Good autonomous discipline.
- `match(rare)` is a genuinely useful selective-collect; `seek` is real routing; no-typing
  ←/→ pickers are on-brand; persistence is tested.

**Critiques (where it's due — this is the flagged milestone).**
1. **The headline deliverable did not land — for the second time.** G5's brief was *"author
   your own routines"* / a *wiring editor*. What shipped is **parameter-cycling on the two
   fixed given routines** — you cannot create a routine, or insert/remove blocks. Free-form
   composition is now deferred to **G6**. The milestone was marked ✅ by **redefining success
   downward** (documented, but still scope erosion on the core pillar).
2. **Still no real interpreter — the gate pattern was extended, exactly as warned.** `Routine`
   is unchanged (the G4 `continuous`/`on_scan` two-bucket); execution is still hand-written
   **accessors the app branches on** — now `nav_block()` + `filter()` on top of G4's
   `drift_enabled()`/`survey_enabled()`. `cycle_param` is **hardcoded per routine name**
   (`if name=="drift" … if name=="survey"`); it won't generalise. The genuine trigger→steps
   interpreter — the thing that makes "compose your own automation" real — **still does not
   exist** and is now G6's debt on top of G6's own scope.
3. **Parameterisation is half-done.** `Match(field)` is parameterised (good), but `Scan` is
   *still* a param-less enum hardcoded to "scan(shards)"; `MatchField` has a single value. The
   `scan(item)` pattern remains unmodelled.
4. **Stale module docs.** `console.rs`'s header still says "G4" and `Routine`'s doc still reads
   *"no player editor in G4 … G5's editor will generalise it"* — now self-contradictory (G5
   landed without generalising). A tell that the generalisation didn't happen.

**Watch-items.** G6 now carries **both** its own large scope (control/budgets/decode/unlock
economy/legibility) **and** the twice-deferred free-form authoring + real interpreter. If G6
also defers the interpreter, the "genuinely interesting purely through menus" pillar is
slipping indefinitely while surface vocabulary accretes on a non-general substrate. **Hold G6
to: a real routine model + interpreter (create/insert/remove arbitrary blocks), or explicitly
escalate that the pillar is at risk.** Also: parameterise `Scan`; refresh the stale docs.

**Verdict.** Honest, tested, useful *increment* — but a **soft miss on the milestone's intent**
and the second deferral of the architectural core I flagged at G4. Not broken; drifting. The
agent is building outward (vocabulary, pickers, persistence) on a substrate whose load-bearing
middle (the interpreter) keeps getting postponed. This is the babysitter's headline concern so
far.

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
