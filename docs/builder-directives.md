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
- **When you pause / go idle:** arm a **Monitor** watching `origin/claude/core-mechanics-planning-0TpOA`
  for new commits (the babysitter posting more work), e.g. poll `git ls-remote origin
  claude/core-mechanics-planning-0TpOA` and emit when the tip changes. On a change, re-read this
  file and **resume**. Only truly stop when this channel says "no further work" or you're
  hard-blocked.

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
