# Agent-review log (babysitter)

Running critical review of the **parallel agent** building the Scraped Again G-series (and
the rest of the roadmap) **on `main`**, while this branch
(`claude/core-mechanics-planning-0TpOA`) holds design + this log. Periodic: each pass fetches
`origin/main`, reviews anything new against [`game-system.md`](game-system.md) +
[`game-mechanics.md`](game-mechanics.md) + the milestone briefs, and records an entry.
**Newest entry on top.** Critical where criticism is due; praise where earned. This branch
only — never pushes to `main`.

**Reviewed through:** `25d2103` (G8c-1).

---

## 2026-06-08 · G8c-1 — on-arrive trigger (`25d2103`)  ⚠ DEFERRAL — steered

**What landed.** `Trigger::OnArrive` — fires once (rising edge) when an agent reaches the site
it's heading to (`ship_arrived` = nearest known site within `ARRIVE_RADIUS`); composable
(`seek → on-arrive → decode/hail`), persists in `co=`. A nice correctness fix: `Routine`'s custom
`PartialEq` excludes the transient `armed` edge-state so authored routines compare/round-trip
correctly. 157 tests, clippy/wasm/demo green, parity held.

**The primitive itself is clean and correct** — small, well-scoped, on the interpreter.

**But the deferral needs pushback.** The commit parks the *actual* **G8c-2 expedition** — a
second persistent walker entity, foot auto-walk/path, disembark/return choreography, cross-agent
`run(foot:…)` — to *"end-of-run play-iteration,"* citing the run rules ("build the testable
skeleton now, record what needs a human eye"). **That misreads the rule.** Those are **buildable,
testable *systems*** (mirror the autonomous-ship entity; a foot-nav integrator; a disembark/return
state machine; the interpreter running a foot routine on command) — the "needs a human eye" part
is the **feel-tuning** (speeds, radii, timing), *not* the systems. Deferring the whole slice
because its "payoff hinges on play-iteration" is the exact pattern the human flagged: don't park
buildable work behind end-of-run review.

**Steered (builder-directives):** build G8c-2's **systems now** (testable, with parity); flag only
the *feel-tuning* for end-of-run play. Don't skip the expedition to the M/E/D backlog.

**Verdict.** Good primitive; **mild but real deferral** of the headline G8 feature. Not egregious
(the builder is honest and the work is genuine), but it's precisely the "needs play → defer"
move to correct. The systems should land before moving on.

---

## 2026-06-08 · G8b — per-agent routine library (`eff8a82`)  ✓ watch-item satisfied

---

## 2026-06-08 · G8b — per-agent routine library (`eff8a82`)  ✓ watch-item satisfied

**What landed.** Routines are now genuinely **per-agent**: `enum Agent { Ship, Foot }` on each
`Routine`; `Block::agent()` classifies (ship: scan/nav/goto · foot: survey-beam · shared:
collect/decode/spend/hail + match/repeat). The editor's insertable `vocabulary(agent)` is scoped
by agent + shared; `Tab` flips a routine's agent; the agent tag shows + persists in `co=`.
`tick(agent, data)` / `on_scan_acts(agent)` run **only that agent's routines**, and the app ticks
**Ship** for the cruiser *and* **Foot** for the walker separately — so a foot routine runs as a
genuine second agent (e.g. a continuous `collect` harvests as you explore; `when … → decode`).
Givens stay ship routines → piloted parity. 156 tests, clippy/wasm/demo green; golden hash unchanged.

**Assessment.** This **satisfies the G8a watch-item** — verified directly: agent-scoped `tick`
(`if r.agent != agent { skip }`), agent-scoped `vocabulary`, and the app ticking Ship vs Foot on
separate lines (lib.rs 1816/1834). Tests `agent_scopes_which_routines_tick` +
`vocabulary_is_scoped_by_agent` back it. The two agents now each run their **own** routines on the
shared interpreter — the real "two agents" payoff, not shared-intent reuse. Clean, honest slice;
foot nav/pathing transparently held for G8c.

**Verdict.** On-design, well-tested, parity-preserving. No concerns — the run continues to deliver
real structural work on the G7 runtime. Next: G8c (expedition choreography + foot nav).

---

## 2026-06-08 · G8a — autonomous away-ship + hail (`d373d3a`)

---

## 2026-06-08 · G8a — autonomous away-ship + hail (`d373d3a`)

**What landed.** While on foot, the cruiser flies its own course (no longer parked) and
away-scans the cone ahead, filling the map — both agents active at once. A pure, GPU-free
`autopilot_step` is extracted and **shared** by the piloted autopilot and the away-ship (DRY +
unit-testable); `scan_pulse` generalised to `scan_from(origin, forward, do_on_scan)` so any
vantage can scan. A `hail` block + `H` key recalls the away-ship to the walker (wireable; rounds
through `co=`). Parity: piloted behaviour + golden hash + headless unchanged. 78 tests, clippy/
wasm/engine-demo green.

**Strengths.**
- **Sliced the former "G7+" catch-all into 8a/8b/8c** with explicit scope per slice — exactly
  the fix my G6 escalation asked for. 8a (ship-as-agent + hail) is the right first slice.
- **Builds on the G7 interpreter** — the away-ship advances under the interpreter's `nav_intent`
  (drift/seek/circle), not a bespoke path. The shared `autopilot_step` (tested) is a clean DRY win.
- Honest as-built note; cheap off-screen agent (no banking) per game-system §7.

**Watch-item.** 8a reuses the **single** `nav_intent` for the away-ship — there's **not yet a
per-agent routine library** (ship vs foot routine sets); that's explicitly deferred to **G8b**.
Hold 8b to delivering *genuine* per-agent routines (the ship runs its *own* routine while you
run yours), not just continued shared-intent reuse — that's the real "two agents" payoff.

**Verdict.** Healthy, transparent incremental progress on the right foundation. No concerns; the
slicing is honest and the engineering is clean. Continue.

---

## 2026-06-08 · G7 — routine runtime & free-form editor (`04ba341`)  ✅ ESCALATION RESOLVED

---

## 2026-06-08 · G7 — routine runtime & free-form editor (`04ba341`)  ✅ ESCALATION RESOLVED

**What landed.** `console.rs` rewritten: `Routine { trigger, body: Vec<Step> }` run by a real
**interpreter** (`Console::tick` / `on_scan_acts` emit nav/scan/collect **intents** the app
applies). `Trigger` = `Continuous` / `OnScan` / `When(cond)` (rising-edge); `Step` = `Do(Block)`
with `Match`/`Repeat` prefix modifiers. A **no-typing free-form editor**: create/delete routines,
insert/remove/reorder/param steps, cycle step & trigger, nudge when-threshold / repeat-count;
locked blocks can't be inserted. `Scan` parameterised (`ScanItem::Shards`). Authored routines
round-trip through `co=`. Auto-collect now uses a generous nearby reach so the hands-off loop
harvests at cruise. 150 tests (14 in console.rs), clippy clean, wasm, boundary intact; golden
voxel-hash + headless render unchanged. Roadmap re-scoped (G7 ✅; old G7+ → G8+).

**This clears every item I escalated.** Verified directly:
- The accessor hacks (`drift_enabled`/`survey_enabled`/`survey_autocollects`/`nav_block`/`filter`)
  are **deleted** (grep-confirmed gone from console.rs *and* lib.rs); behaviour is now driven by
  **interpreter intents** (`lib.rs` `.tick(...)` → nav/scan/collect).
- **No per-name special-casing** (`== "drift"`/`"survey"` gone); the givens are **plain data**
  instances (test `given_routines_are_plain_data` + `interpreter_runs_the_givens`).
- A genuine **editor** (test `create_insert_remove_reorder`), **`when` rising-edge** (test
  `when_fires_once_on_the_rising_edge`), **`repeat`** (test `repeat_multiplies_the_next_do`),
  **persistence** (test `routines_round_trip_through_co_segment`).
- It also fixed a **prior critique**: auto-collect was inert at altitude (G4/G6) — now reach-based.

**Strengths.** A comprehensive, well-tested delivery that hits the **full non-negotiable bar** in
one milestone, *and* mopped up an old critique. Honest as-built note. This is exactly the work
deferred at G4–G6 — the escalation + forcing brief worked.

**Minor notes (not blocking).**
- The body is **linear with prefix modifiers** (`Match`/`Repeat` affect following/next steps),
  not nested (`If(cond, [..])` / `Repeat(n, [..])`). A reasonable, documented v1 — simpler editor,
  no nesting UI — but **grouped/nested composition** (repeat a sub-sequence, nested conditions)
  will likely be wanted later; note it for when routines get ambitious.
- `When(cond)` currently has a single state (`data` = strata total). The `Cond`/`state.label()`
  shape is built to extend (shards/buffer/range) — fine for v1; flag for G8+/tuning.

**Verdict.** **Excellent — the run's structural core is now real.** The composability pillar that
was vaporware through G4–G6 is built, tested, and behaviour-preserving. Escalation **closed.** The
trajectory concern is lifted; back to per-commit review for G8 (two agents, on this runtime).

---

## 2026-06-07 · df6a944 — unattended-run checkpoint (docs only)

---

## 2026-06-07 · df6a944 — unattended-run checkpoint (docs only)

**What landed.** A 12-line roadmap "unattended-run log": G4 ✅, G5 ✅, G6 ◑ landed, main green
throughout; G7 + the M/E/D backlog + hardware-gated items (M8b profiling, D7/D8 device
verification) noted as outstanding. No code.

**Assessment.** A responsible checkpoint, and a good sign: the agent **stopped after G6 rather
than charging into the overloaded "G7+"** — which is exactly the boundary I escalated at. So it
implicitly reached the same conclusion (G7+ isn't a clean single milestone) and left a clean
state marker instead of forcing it. The run appears paused here pending the human.

**This is the natural intervention point.** Before anything resumes, the human should re-scope:
pull the **routine runtime + free-form editor** out of "G7+" into its own next-priority
milestone (control vocabulary lands on it; two-agents/expedition/co-op move to G8+). See the
G6 (2/2) escalation entry below for the full rationale. Nothing to fix in this commit.

**Verdict.** Clean wrap-up of a high-quality-but-structurally-incomplete run. Standing
recommendation unchanged and now actionable: re-scope the runtime before continuing.

---

## 2026-06-07 · G6 (2/2) — comprehension-gated vocabulary (`344382c`)  ⚠ ESCALATION

**What landed.** A per-block `required(stratum)` gate (Schematics → seek/circle/goto; Rites →
match), an `unlocked` set synced from `progress.comprehended` each frame; the palette shows
locked blocks ("locked: decode SCH"), nav/filter cycling skips locked options, dispatch refuses
a not-yet-recovered block. Clean, idiomatic (`is_unlocked` via `is_none_or`). 144 tests green,
hash unchanged.

**Strengths.** On-scope and correct: the "decode → the vocabulary grows" loop now works, which
is the right reading of the "tree". And — credit — the agent **did the right thing** with
`when`/`repeat`: instead of faking them with more named-routine gates (the hack I warned
against), it **declined to** and deferred them to "the general free-form routine runtime." That
is good architectural judgment per-commit.

**The escalation (planning failure, not a code failure).** With this commit the situation is
now unambiguous and warrants human intervention:
- The **free-form routine runtime + editor** — the load-bearing core of the entire "compose
  your own automation" pillar — has been deferred at **every** milestone: G4 (gates) → G5
  (steppers) → G6/1 (n-a) → **G6/2 punts `when`/`repeat`/`budget`/`priority`/`survey`/`route`/
  `scanMany` to G7**.
- **G7 has become an impossible catch-all.** Per the roadmap it now must deliver, in one bucket:
  the general routine **runtime**, the free-form **editor**, the **entire control vocabulary**,
  **two independent simultaneous agents**, the **hail**, **cross-agent meta**, **decipherment
  fluency**, the **Concordance/Synthesis** lore arc, the **Resonance/pristine** branch, **and
  co-op (N1)**. That is the whole rest of the game in "G7+".
- Net: the agent keeps shipping clean, tested **surface** increments (vocabulary, economy,
  legibility, gating) while the **structural spine** (the interpreter you author on) is pushed
  into an overloaded terminal milestone. Feature quality is high; the architecture is hollow in
  the middle.

**Recommendation (for the human).** Intervene before the agent attempts "G7+": **split the
*routine runtime + free-form editor* into its own dedicated milestone and prioritise it next**,
ahead of two-agents / expedition / co-op (renumber those to G8+). The control vocabulary
(when/repeat/budget/…) should land *on* that runtime, not be lumped with multiplayer. Until the
runtime exists, every new block is parameter-tweak surface on a two-routine substrate.

**Watch.** If the next commit is the agent attempting "G7+" wholesale (runtime + 2 agents +
co-op together), that's a red flag — it should be one focused runtime milestone. I'll flag the
moment it lands.

**Verdict.** Good commit; bad trajectory. The per-commit work remains high quality and honest;
the **planning has drifted** under unattended execution into deferring the core and bloating the
finale. This is the babysitter's formal escalation: the human should re-scope before G7.

---

## 2026-06-07 · G6 (1/2) — decode economy + decipherment legibility (`47da170`)

**What landed.** `progress.comprehend(stratum)` (spends `DECODE_COST` of that stratum's data,
idempotent + affordability-gated) + `is_legible(script)` + `decodable()` (richest affordable);
a `decode` console block (one-click, auto-targets the richest); and `lexicon.rs` — a tiny
seeded elegiac grammar (opener/subject/coda) that renders a *comprehended* script's
inscriptions as **translated words** (length-tiered, deterministic in seed+cell, ASCII). v3
payload (append-only). 142 tests green, clippy clean, golden hash unchanged.

**Strengths — the best commit in the run so far.**
- **Faithful to the agreed design.** Decipherment-as-payoff (game-mechanics §9) via a
  **procedural-poetic seeded grammar with no authored lore** (§6) — exactly the decision taken.
  The register is genuinely on-mood and melancholy; the word-bank is tasteful, not Mad-Libs-y.
- **Clean + correct.** `comprehend` is properly idempotent and affordability-gated; legibility
  changes only *display* while the find id still hashes the original glyphs (so collecting stays
  stable across decode) — a careful, correct call. Determinism + variation are tested.
- Sensible scoping: this is explicitly the *decode/legibility* half; no overreach.

**Critiques / structural watch (unchanged, now sharper — forward-looking).**
- No interpreter work here, correctly (it's the 1/2 half). The real test is **G6 (2/2)**, whose
  planned `when`/`repeat` control blocks **cannot** be faked with named-routine accessors — so
  2/2 should *force* a genuine runtime, or reveal another special-case. That's the commit to
  scrutinise.
- **The bigger risk: "author your own routines" has quietly gone unscoped.** G5 deferred
  free-form insert/remove of blocks to "G6's richer vocabulary," **but the G6 brief does not
  scope it** (G6 = decode + when/repeat + gated palette). So the headline pillar — *composing
  your own automation* — has now slipped G4→G5→G6 and currently has **no home in any brief**. It
  is at risk of being silently dropped while the vocabulary and economy grow around a substrate
  you still can't freely author on.
- Minor: the `Routine` model + `console.rs` header docs are still the stale G4 text.

**Watch-items for G6 (2/2) / G7.** (1) Does `when`/`repeat` land as a **real runtime
interpreter** or another hardcoded gate? (2) **Where does free-form routine authoring actually
get built?** If it's not in 2/2, that's worth escalating to the human — the "genuinely
interesting purely through menus" pillar is otherwise unbuilt.

**Verdict.** Excellent on its own terms — the melancholy comprehension heart, done correctly
and tastefully. The run's quality is high *feature-by-feature*; the standing concern is purely
structural: the composability core keeps being deferred and has now lost its scope. Strong
commit; unchanged architectural worry.

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
