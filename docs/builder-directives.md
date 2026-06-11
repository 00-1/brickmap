# Builder directives — babysitter → builder channel

**Live steering** from the babysitter (this branch, `claude/core-mechanics-planning-0TpOA`) to
the **builder** agent (working on `main`). `main` is canonical for code / roadmap / built
milestone briefs; **this branch is the steering channel.** Pair with
[`agent-review.md`](agent-review.md) (the per-commit critique).

## How to use this channel (builder)

- **While working — check periodically, not just between milestones.** Re-read this channel
  (`git fetch origin claude/core-mechanics-planning-0TpOA` then
  `git show origin/claude/core-mechanics-planning-0TpOA:docs/builder-directives.md` +
  `…:docs/agent-review.md`): **before every commit**, **at the start of each milestone**, and
  **at least every ~15 minutes during a long build** (or at each natural sub-step). The point is
  the babysitter can steer you **mid-milestone** — if the directive or an `agent-review.md`
  escalation has changed, adjust *now*, don't wait until the milestone is done. Follow the
  **CURRENT DIRECTIVE**; address escalations.
- **Do not merge this branch into `main`.** Just *read* it. Apply any roadmap re-scope to
  `main` yourself. You may copy a milestone brief from here onto `main` to build it.

### KEEP WORKING — do not falsely pause or defer (read this twice)

Your default is to **keep building the next milestone**. Pausing and deferring are the main
risks; resist both.

- **Deferred human review is NOT a blocker.** The human reviews everything **at the end** of the
  run. "I can't see/verify the visual result", "this needs the human to look", "I'd like sign-off
  before continuing" — **none of these stop you.** Build it (using the headless render + tests to
  sanity-check what you can), **record the assumption / what needs a human eye**, and **move on to
  the next milestone.** Visual polish gets reviewed in a batch later, not by halting the run.
- **Finishing a milestone is not a reason to pause.** Chain straight into the next one.
- **Uncertainty is not a blocker.** Make the best-judgment call, record it, continue.
- **A big milestone is not a blocker.** Split it (e.g. runtime → editor) and keep building.
- **The ONLY real blockers:** a missing secret/credential you cannot obtain; reference hardware/a
  physical device you cannot access (e.g. M8b profiling, an APK install); a hard external
  dependency; or something destructive/irreversible. For these: **skip and NOTE them, then keep
  going** with everything that *isn't* blocked. Do not stop the run for a blocker that only blocks
  *one* item.
- **You only truly stop when every non-blocked milestone is done**, or this channel says "no
  further work."
- **When you do pause** (only for the above), **push a checkpoint commit to `main` stating exactly
  why** — so the babysitter sees it and can **override** if the "blocker" isn't real (it will post
  a directive telling you to resume; your steering-branch watcher picks it up). Then arm a
  **Monitor** on `origin/claude/core-mechanics-planning-0TpOA` (poll `git ls-remote`, emit on tip
  change), and **resume** when this file changes.

## 🐛 BUG FOUND (fix + regression-assert in D11) — share-link extreme-coords crash — 2026-06-11

The parallel adversarial bug-hunt finished. **One genuinely serious, game-reachable crash**
(babysitter-verified independently) + one cosmetic. Fix the crash (interleave with D11) and
**encode each as a D11 regression assert**.

### BUG 1 — CRASH (severity: high; reachable via the E12 share-link headline feature)
A crafted/garbage share link with an **extreme-but-finite** camera coordinate (e.g.
`#v=1&s=1&z=300000000000`) overflows world streaming on the **first frame** → debug panic
`attempt to add with overflow` (release: wraps to garbage cell math). **Verified** (code +
arithmetic): `share::set_f32` (`share.rs:84`) only guards `is_finite()` — no magnitude clamp —
so z=3e11 reaches `Camera::new`; then `(cam.z / CELL).floor() as i32` **saturates to
`i32::MAX`** and `ccz + reach` overflows in the cell loops:
- `shards.rs` `shards_near` (~`:83`, `(ccz - reach)..=(ccz + reach)`) — overflows at cam.z ≳ 1e11
- `structures.rs` `inscriptions_near` (~`:252`, same pattern) — and `colossi_near` shares it
**Fix (do both — boundary + defense-in-depth):**
1. **Clamp share-link pos** to a sane world bound in `share::set_f32`/decode (extreme coords
   are never legitimate input) — the primary fix at the trust boundary.
2. **Saturating/checked cell math** in `shards_near` / `inscriptions_near` / `colossi_near`
   (clamp `ccx`/`ccz`, or `saturating_add(reach)`), so *any* extreme cam coord — not only the
   share link — can't overflow streaming.
3. **D11 regression assert:** a share-link with extreme coords decodes + first-frame-streams
   without panic; the `*_near` fns are panic-free at huge `cam` coords.

### BUG 2 — COSMETIC (severity: low; not gameplay)
`screenshot out.png 0 0` panics (wgpu zero-size-texture validation). CLI misuse only. Cheap
guard: reject 0 width/height with a clean error in the `screenshot` bin. Low priority — fold
into D11 or a cleanup commit; don't let it block.

### What HELD UP (the hunt's coverage — reassuring)
Full progression (6 seeds), research-economy edge cases (off-domain/maxed/mid-fill-switch),
routines + cycle-guard + scoped-group register, two-agent expedition, **persistence
round-trip + 5000-input codec fuzz (all graceful)**, determinism, a 40k-tick soak (no
panic/NaN/overflow), render robustness. The core loop is robust; this is a defensive-input
gap at the share-link boundary, not a systemic problem.

---

## CURRENT DIRECTIVE — 2026-06-11: D11 end-to-end harness (Archive still fork-blocked)

The human asked for **autonomous end-to-end testing** — drive everything end to end and find
problems. This is the right work *now*: G9–G16 are unit-tested + golden-hash + render-checked,
but the **per-frame orchestration in `App` that wires the systems together over time has never
been driven end to end**. (Archive feature milestones remain fork-blocked on the human; this is
productive non-fork work meanwhile.)

- **D11 — End-to-end play harness (headless integration + soak)** — build per
  [`milestones/D11-e2e-harness.md`](milestones/D11-e2e-harness.md) (copy to `main`). A
  **durable, CI-wired** harness that drives the **real** game loop headlessly and asserts
  *integration* outcomes: a scripted seeded playthrough (discover a block → allocate →
  domain shards fill → comprehend → author + run a routine → expedition cycle → state
  round-trip), **persistence fidelity** (round-trip equality, v1..v5 migration, malformed →
  graceful-not-panic), **determinism** (same seed+inputs → identical state), and a **bounded
  seeded soak/fuzz** (no panic / NaN / overflow / unbounded growth; research progresses-or-
  honestly-blocked). **Central design problem first:** `App` owns the GPU, so either extract
  a **sim-core** (preferred — behaviour-preserving; golden hash unchanged) or add a
  **headless-App** drive path; document the choice; the harness must drive the *same* tick
  the live frame uses (no parallel re-implementation). Keep CI bounded/seeded (heavy soak
  env-gated). Pinned defaults in the brief.
- **Parallel:** a separate adversarial bug-hunt is running against `main` right now. Its
  **confirmed** bugs will arrive here as fix directives — fix them, and **encode each as a
  regression assert in D11**. So: build the harness; expect bug-fix directives to interleave.
- After D11 + the bug-fixes: stand by for the Archive fork decision from the human.

Toolchain prevention in force; CI is the authority; keep main green.

---

## CURRENT DIRECTIVE — 2026-06-11 (G12 done → G13 record-to-program)

**G12 (glyph console) is complete + reviewed** (both halves; `baa4a73`; the console now
renders block identity as stratum-script glyphs, English only as instrumentation). New work:

- **G13 — Record-to-program** — build per
  [`milestones/G13-record-to-program.md`](milestones/G13-record-to-program.md) (copy to
  `main`). Turn the player's last ~10 **manual** actions into a **draft routine** with one
  console action — the on-ramp from playing-by-hand to authoring. **Non-negotiable contract
  (the research's load-bearing finding): record LITERALLY, generalize MANUALLY** — the draft
  is the *exact* concrete blocks, with the only transformation being mechanical run-length
  folding of identical adjacent actions into `repeat(n)`; **no inference, no "noise"
  dropping, no auto-generalization** (that's what killed every real programming-by-
  demonstration system). The player generalizes afterward via the existing steppers. Draft =
  ordinary G7 routine data (persists/edits/runs through existing paths). Include Eager-style
  **pre-highlight of the agent's next target** during playback if it's small (else split to
  G13b — note it). Show the trace **filling as you act** (a ticker); glyph-rendered per G12.
  Record only *manual* actions (autopilot/auto-collect must not pollute the trace). Pinned
  defaults in the brief — veto only on a genuine fork.
- **Then stand by** — do not wind down. Next after G13: G14 (subroutines/templates/nested
  steps), then the comprehension-as-research economy (G15) and the Archive groundwork
  (the fork-free **lexicon v2** statistical-honesty pass, G16a, is buildable any time).

### Two eye-pass notes from the G12 review (no action needed now)
Routine names and faculty names render English (not glyph) — defensible scope calls (author
labels / inherent faculties, not world-discovered vocabulary), flagged for the human's
eye-pass; faculties may want glyph once G15 makes them research targets. Carry forward; don't
change now.

---

*(superseded — D10 landed + verified)*

### ~~CURRENT DIRECTIVE — 2026-06-08 (D10 touch overlay visuals)~~

**Playtest:** D9 touch *mapping* works well, but the on-screen overlay is just a HUD text line —
**the player can't see where to touch.** New work:

- **D10 — touch control overlay** — build per [`milestones/D10-touch-overlay.md`](milestones/D10-touch-overlay.md).
  Render the D9 controls (two edge sliders + buttons 1/2/A/B) as **visible, dimmed on-screen
  controls** with press/value feedback + context labels, drawn **from the existing `touch::Layout`**
  (so visual == hit-zones, single source of truth). Game-side HUD in `scraped-again`; any new HUD
  rect primitive in `bm-render` must be **generic** (no game concept). Headless-verifiable (opt-in
  screenshot flag like `SCRAPED_BEAM`, golden stays clean); on-device size/opacity/placement *feel*
  is the human follow-up — build the visible overlay now, don't block on a phone. Pinned defaults in
  the brief; styling is the human's later call (don't pause on it).
- Then (only if you want breadth): **E9 god-rays** is build-blind-on-request only — leave it for the
  human's eye-pass unless directed. Everything else buildable is done.

---

*(superseded — D9 + the autopilot wander fix landed)*

### ✅ QUICK FIX DONE (`a5f316f`) — autopilot `drift` wanders, not circles

*(Resolved: fbm-of-three-sines heading in the shared `autopilot_step`; meanders + covers ground;
tested turns-both-ways; piloted + away-ship. No further action.)*

~~**Human playtest feedback:** the default autopilot currently flies a **tight circle**~~ — it needs
to **wander, as if purposely surveying the planet**. The `drift` heading integrator is turning at
a near-constant rate → a loop. Make it **meander and cover ground**: drive the heading from a
**slow smooth noise** (value-noise / fbm / low-freq sine over the ship clock) so the turn rate varies
and the path drifts *outward across the world*, not around one spot. Apply to the shared
`autopilot_step` so the **piloted drift, the autonomous away-ship, and the away-walker** all
wander (keep it cheap + deterministic; it's live-loop, doesn't touch the golden hash). It should
read as an unhurried survey sweep, not an orbit. Small change — interleave with D9.

**The M/E/D backlog pass is complete + green (wind-down confirmed). New work:**

- **D9 — touch controls (phone)** — build per [`milestones/D9-touch-controls.md`](milestones/D9-touch-controls.md).
  A native touchscreen UI (2 sliders + 4 buttons + tap-the-view), mapping the core controls. Engine
  side: generic winit **touch events** in `bm-platform` (no game concepts). Game side
  (`scraped-again`): the on-screen overlay + a **unit-tested pure touch→action mapping** that reuses
  the existing camera/mode/console paths; **tap-the-view = cast the survey-beam** at the hit point
  (the universal interaction verb). Pinned defaults are in the brief — follow them; veto only on a
  genuine fork. **Build the logic + overlay now (testable/headless); the on-device *feel*-tuning
  (slider sensitivity, button size, tap targeting) is the device-gated human follow-up** — don't
  block on a real phone, and don't pause for it.
- Then: the remaining **Checklist-2 "deferred but buildable"** items if you want breadth (E18
  solid-human placement+bake, E9 v2 fog/god-rays/water, web weather→audio bridge). Skip the
  hardware/secret-gated ones (M8b/D7/D8/D5/N1).

---

*(superseded — kept for the record)*

- ✅ **G7 (routine runtime & free-form editor) — PASSED review.** Escalation resolved: real
  interpreter, accessor hacks deleted, givens-as-data, editor, when/repeat, parameterised scan,
  auto-collect reach fixed, parity held. (See `agent-review.md`.) Good work — **keep this
  momentum; do not pause for review.**

### ⚠ STEER (2026-06-08, after backlog checkpoint) — DON'T PAUSE; build the backlog systems

You paused the run, saying the rest (M7, M8a-rest, E9, E16, E18) is "feel/visual/perf/audio whose
quality bar is human iteration," and asked which to build. **That's a false pause — resume now.**
"Feel-heavy / needs human iteration" is **NOT** a blocker: these are **buildable, testable
systems**; only the *feel-tuning* waits for end-of-run play. Build them. Don't stop to ask which —
**build them all**, in this order, deferring only feel:

1. **E11-2 — wire the water CA into the live world** (finish E11): active-set seeding + re-mesh
   budget; handle the golden-hash determinism (gate live flow so the static golden world/voxel-hash
   stay valid, or version it). Water should actually flow in-game, not just in a unit test.
2. **M8a (rest) — perf systems** (dynamic resolution + FSR1/EASU upscale; further vertex quant +
   quad-expansion; upload prioritisation/coalescing). *Measurable on the HUD — barely feel-gated.*
3. **M7 — point-decimation LOD** (the deferred perf half: decimate distant chunks to real point
   sets). System now; tune distances by eye later.
4. **E9 — weather/water/sound**: the global weather state + precip particles + snow/wetness blend +
   stylised water + god-rays + procedural ambient audio. Build the *systems*; tune feel later.
5. **E16 — reactive-audio layer**: biome/weather→param mapping, a voice cap, one FDN reverb. DSP
   systems — testable (finite/bounded), tune the *sound* later.
6. **E18 — remainder** (solid/explorable colossi follow-ups).

**Note (M7) — WITHDRAWN.** I'd pushed to wire the M7 far-LOD now; on reflection that's
over-prescriptive. M7's integration value is *purely the perf win*, which is **M8b/hardware-gated**
— so **bundle the M7 far-LOD wiring with M8b** (when the reference hardware is available to tune +
measure it). `decimate_surface` is tested + appropriately shelved. *(Distinction worth keeping:
"wire it" still applies to slices that deliver a **here-verifiable feature** — e.g. E11's water;
M7's only payoff is unmeasurable-here perf, so it's the exception, not the rule.)*

**Only D5 + M8b/D7/D8/N1 are real skips** (browser/hardware/secrets — noted, fine). **Do NOT pause
again** because something is "feel-heavy" — build the system, record the tuning note, keep going.
Only stop when all of the above are built or you hit a *real* external blocker.

### ✅ G8 systems-complete (2026-06-08)

G8a/G8b/G8c-1/G8c-2a/**G8c-2b** all ✅ — the automated expedition (cross-agent `run(foot)`,
deploy→harvest→return) is **built and tested** on the G7 interpreter; only feel-tuning + an
in-world walker avatar are noted for end-of-run. The earlier deferral steer is **resolved** —
good work taking it. (See `agent-review.md`.) Keep this discipline: build systems now, tune feel
later, don't park buildable work behind "needs play".

**Now, continue unattended — the M/E/D backlog:**

1. **The M/E/D backlog:** M7, M8a, E9, E11, E13, E16, E18, D5 … — pick a sensible order
   (finish in-progress, respect deps). Write each brief just before building.
3. **Skip + NOTE (don't stop the run for):** anything needing hardware/devices/secrets — M8b
   profiling, D7/D8 device verification, co-op/N1 server. These block *those items only*.
4. **Optional later polish (don't block on it):** nested/grouped routine steps (repeat a
   sub-sequence; nested conditions); more `when` states (shards/buffer/range). Note, defer.

**Reminder — keep working:** finishing a milestone, "needs human/visual review", and uncertainty
are **not** stopping points. Human review is end-of-run. Chain straight into the next milestone.

## Directive log (newest on top)
- **2026-06-11** — 🐛 bug-hunt done: **BUG 1 (high, verified)** share-link extreme-but-finite camera coord overflows i32 cell math in shards_near/inscriptions_near/colossi_near → first-frame crash; fix = clamp share pos (share.rs:84) + saturating cell math + D11 regression assert. BUG 2 (cosmetic) screenshot 0x0 panic. Core loop otherwise robust (progression/economy/persistence/codec-fuzz/determinism/40k soak all held).
- **2026-06-11** — STANDBY lifted: human asked for **autonomous end-to-end testing**. **New directive: D11 — E2E play harness** (headless integration: scripted playthrough + persistence fidelity + determinism + bounded soak/fuzz; extract a sim-core or headless-App; CI-wired). A parallel adversarial bug-hunt is running; its confirmed bugs land as fix directives + D11 regression asserts.
- **2026-06-11** — G16 ✅ complete (lexicon v2, four-way green). **STANDBY**: remaining Archive milestones are fork-gated; babysitter fork-checking the human before the next dispatch.
- **2026-06-11** — G15 ✅ complete (economy unified, 61235b6, CI-green). **New directive: G16 — lexicon v2** (statistical honesty: Zipf/Heaps/entropy/morphology checklist as unit tests + corpus-shape; fork-free Archive groundwork; output stays nonsense-words, no lore). Deeper Archive milestones briefed next with fork-checks. (Handshakes → G17.)
- **2026-06-11** — G14 ✅ complete (subroutines a0aaa89, CI-green). **New directive: G15 — comprehension-as-research economy** (the human's unified model: discover→allocate shards→fill→comprehend; domain+rarity-gated; retrofits G9 decode-unlock + G10 spend; faculties fold in; LARGE → split G15a runtime/retrofit, G15b levels). Next: G16a lexicon v2.
- **2026-06-11** — ✅ RED cleared: CI/Android/Desktop green on `30daefb`. **New directive: G14 — subroutines** (same-agent run(routine) as a no-seams glyph block, cycle guard, duplicate, one-level nested groups, one-register rule). Next: G15 research economy, G16a lexicon v2.
- **2026-06-11** — RED resolved over 3 commits (wrap+pin `70524d2`, android target `833f57c`, path-filter trigger `30daefb` — builder caught the trigger gap itself). All four workflows running on `30daefb`; G14 dispatches on confirmed green.
- **2026-06-11** — round 2: fmt RED fixed (wrap + toolchain pin 1.94.1, good), but the pin's `targets` list (wasm only) **broke the Android APK** (`aarch64-linux-android` missing). Escalated: add the android target(s) to rust-toolchain.toml. Still blocks G14 until all four workflows green.
- **2026-06-11** — 🔴 **main RED on G13**: `cargo fmt --all --check` only (console.rs:2272 chain-wrap). Logic fine (212 tests/clippy pass locally). Escalated: apply the wrap + push; root cause = rustfmt version skew vs CI — pin the toolchain / prefer wrapped chains. Blocks G14 until green.
- **2026-06-11** — G12 ✅ both halves (glyph console complete, console rendered, recognition loop pixel-proven). **New directive: G13 — record-to-program** (manual actions → draft routine; LITERAL record + manual generalize, the non-negotiable PBD contract; Eager pre-highlight if small; trace ticker; manual-only). Eye-pass notes: routine/faculty names left English.
- **2026-06-11** — G12 (1/2) ✅ (glyph identity + world-text de-Anglicization). Brief's "no engine change" line **withdrawn** — the console needs a generic mixed-script HUD overlay (builder's correct catch); sanctioned for G12 (2/2), boundary intact (no new scripts).
- **2026-06-11** — queue (G9→M10→G10→G11→M11) **fully drained, all green/reviewed**. **New directive: G12 — glyph console (de-Anglicization)** — block names render as stratum-script glyphs everywhere player-facing; learn-by-clicking; structural UI stays minimal-English; docs (game-system §1/§6) in lockstep. Human decision: names unreadable, no English layer.
- **2026-06-11** — **NEW RUN (incremental dispatch).** D10 ✅ (verified rendered). **New
  directive: G9 — block-name discovery** (in-world inscriptions carry block names; collect →
  discover → decode unlocks; two-stage gate; starter pre-discovered; brief on this branch).
  G10 (typed shards) being planned — stand by between directives, don't wind down.
- **2026-06-08** — D9 ✅ (touch mapping works in playtest). **New directive: D10 — touch control overlay** (render the sliders/buttons visibly from `touch::Layout` so you can see where to touch; game-side HUD; generic engine rect primitive; headless-verifiable; on-device feel = human follow-up).
- **2026-06-08** — ⚡ quick fix (human playtest): autopilot `drift` does a tight circle → make it wander/meander (slow-noise heading, covers ground) as a purposeful survey sweep; applies to the shared autopilot_step (piloted + away-ship + away-walker). Squeeze in around D9.
- **2026-06-08** — wind-down confirmed (176 tests green, independent check). **New directive: D9 touch controls** (phone touch UI; engine touch-events + game overlay + tap=beam; on-device feel-tuning is the device-gated follow-up).

- **2026-06-08** — initial directive: G7 runtime/editor mandated (see above); re-scope G7+ → G8+;
  cleanups; then G8 + the M/E/D backlog. Issued after the babysitter's G6 (2/2) escalation.
- **2026-06-08** — G7 ✅ passed review (escalation resolved). Directive updated: proceed to G8
  (two agents on the new interpreter) → M/E/D backlog; keep moving, don't pause for review.
- **2026-06-08** — G8a/G8b/G8c-1 ✅. **Steer:** don't defer G8c-2 (the expedition systems) to
  "end-of-run play-iteration" — build them now (testable), tune feel later. Build before the
  M/E/D backlog.
- **2026-06-08** — G8 systems-complete ✅ (expedition + away-walker landed; steer resolved).
- **2026-06-08** — E13 ✅ (photo v1), E11-1 ✅ (water CA). Then the builder **paused** ("rest is
  feel-heavy"). **Steer: false pause — resume; build E11-2 → M8a-rest → M7 → E9 → E16 → E18,
  defer only feel; don't pause for "needs human iteration" again.**
