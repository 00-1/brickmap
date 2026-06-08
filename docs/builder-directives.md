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

## CURRENT DIRECTIVE — 2026-06-08 (updated after G7 review)

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

**Note (M7, after `9cdf089`):** `decimate_surface` is built + tested — good. But **wire the M7
far-LOD** as a **default-off, headless-verifiable** system (per-chunk point buffer + mesh
suppressed past a distance), the same way E11-2 wired the water (toggle-gated, golden-safe). The
*system* (distant chunks drawn as points) is **not** hardware-gated — only the **perf measurement**
(M8b) and the **crossfade feel** are. Don't defer the whole integration; defer only those two. Keep
"wire it, don't leave a dead tested function" as the rule for algorithm slices (E11, M7, …).

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
