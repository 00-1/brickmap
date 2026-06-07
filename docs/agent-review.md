# Agent-review log (babysitter)

Running critical review of the **parallel agent** building the Scraped Again G-series (and
the rest of the roadmap) **on `main`**, while this branch
(`claude/core-mechanics-planning-0TpOA`) holds design + this log. Periodic: each pass fetches
`origin/main`, reviews anything new against [`game-system.md`](game-system.md) +
[`game-mechanics.md`](game-mechanics.md) + the milestone briefs, and records an entry.
**Newest entry on top.** Critical where criticism is due; praise where earned. This branch
only — never pushes to `main`.

**Reviewed through:** `344382c` (G6 2/2).

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
