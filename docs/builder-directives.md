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

## CURRENT DIRECTIVE — 2026-06-08

1. **PRIORITY — build G7: routine runtime & free-form editor**, per
   [`milestones/G7-routine-runtime.md`](milestones/G7-routine-runtime.md) (read it from this
   branch). This is the escalation fix: the runtime/editor was deferred at G4–G6. Build it now,
   **in full, before any new content**. **Do not defer or scope it down**; if too large, split
   runtime → editor but build **both** first. Non-negotiable bar: the named-routine accessor
   hacks are **deleted** and the given routines become **data** on a real interpreter.
2. **Re-scope `main`'s roadmap:** insert **G7** (this); shift the old "G7+ — two agents /
   expedition / arc / co-op" to **G8+**.
3. **Cleanups** (fold into G7 where natural): parameterise `Scan` as `scan(item)`; refresh the
   stale `console.rs` module / `Routine` docs; make **auto-collect meaningful at cruise altitude**
   (reach/tuning so the hands-off loop actually collects in normal flight).
4. **Then continue unattended:** **G8** (two agents + expedition + hail + cross-agent meta — now
   on the real runtime), then the **M/E/D backlog** (M7, M8a, E9, E11, E13, E16, E18, D5 …).
   **Skip + note** anything blocked on hardware/devices/secrets (M8b profiling, D7/D8 device
   verification, co-op/N1 server).

## Directive log (newest on top)

- **2026-06-08** — initial directive: G7 runtime/editor mandated (see above); re-scope G7+ → G8+;
  cleanups; then G8 + the M/E/D backlog. Issued after the babysitter's G6 (2/2) escalation.
