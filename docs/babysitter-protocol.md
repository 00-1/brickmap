# Babysitter / builder — parallel-agent review protocol

A reusable pattern for running **two agents in parallel**: a **builder** that does the work
unattended, and a **babysitter** that reviews each commit, steers the builder, and escalates
when the work drifts. Born from the *Scraped Again* G-series run (2026-06); written up here so we
can reuse it.

## When to use it

- A long, mostly-mechanical build that can run **unattended**, *but* where you want a second pair
  of eyes catching **architecture drift, scope erosion, or quality issues** the builder won't
  self-correct (builders tend to ship clean per-commit work while quietly deferring the hard core).
- The human is away and wants progress **with a safety rail**, not a blind run.

## Roles

- **Builder** — executes milestones on the canonical branch (`main`), fully unattended:
  trunk-based small commits, green at every step, records assumptions, halts only when blocked or
  done. Owns code + roadmap + built milestone briefs.
- **Babysitter** — does **not** write feature code. It reviews every builder commit critically,
  logs findings, and **steers** via a separate branch. It holds milestones to their acceptance
  bar and **escalates planning problems to the human**. Critical where criticism is due; credits
  honesty and good calls.

## Branch topology

- **`main`** — canonical: the builder's code, roadmap, and built briefs.
- **A steering branch** (here `claude/core-mechanics-planning-0TpOA`) — the babysitter's channel:
  design docs, the review log, the directives channel, and any new milestone briefs. The builder
  **reads** this branch but **never merges** it into `main`; the babysitter **never pushes** to
  `main`.

## Artifacts (on the steering branch)

- **`agent-review.md`** — the per-commit critique log. Newest entry on top; tracks a
  **`Reviewed through: <sha>`** marker. Each entry: *What landed · Strengths · Critiques ·
  Watch-items · Verdict*. Mark serious findings `⚠ ESCALATION`.
- **`builder-directives.md`** — the **live steering channel**: a *Current directive* (the
  prioritised next work) + a how-to-use header + a dated directive log. This is what the builder
  polls for guidance.
- **Milestone briefs** (`milestones/*.md`) — when the babysitter needs to force specific work
  (e.g. a deferred core), it writes a focused brief here with **non-negotiable acceptance
  criteria**; the directive points the builder at it.

## The loop (bidirectional)

```
  BUILDER (on main)                          BABYSITTER (on steering branch)
  ─────────────────                          ───────────────────────────────
  each cycle: read builder-directives.md  ◄─ writes/updates directives + briefs
  + agent-review.md from steering branch
  build per the Current Directive
  push commit to main  ──────────────────►  Monitor on origin/main wakes it
                                             reads the actual changed code
                                             appends a review to agent-review.md
                                             (steers via builder-directives.md)
  on pause/idle: arm a Monitor on the
  steering branch ◄───────────────────────  posts more work → wakes the builder
  resume when directives change
```

- **Babysitter wake signal:** a persistent Monitor that polls `origin/main` and emits when the
  tip advances (event-driven — review exactly when there's something new).
- **Builder wake signal (when paused):** a Monitor that polls the **steering branch** and emits
  when it advances (i.e. the babysitter posted more work) — so a paused builder resumes itself.

## Setup recipe

### Babysitter's `main` watcher (Monitor)

```sh
prev=$(git ls-remote origin main 2>/dev/null | awk '{print $1}') || true
echo "babysitter armed on main; baseline ${prev:0:12}"
while true; do
  sleep 120
  cur=$(git ls-remote origin main 2>/dev/null | awk '{print $1}') || true
  if [ -n "$cur" ] && [ "$cur" != "$prev" ]; then
    echo "main advanced to ${cur:0:12} (was ${prev:0:12}) — review due"; prev=$cur
  fi
done
```

Run as a **persistent Monitor**. On each event: `git fetch origin main`, review
`<reviewed-sha>..origin/main` (read the real changed files via `git show` / `git show
origin/main:<path>`, not just commit messages), prepend an `agent-review.md` entry, bump
**Reviewed through**, commit + push the steering branch, and surface a short summary to the human.

### Builder, on each cycle (in its prompt)

```
git fetch origin <steering-branch>
git show origin/<steering-branch>:docs/builder-directives.md
git show origin/<steering-branch>:docs/agent-review.md
# follow the Current Directive; address escalations; never merge the steering branch.
# on pause: arm a Monitor on origin/<steering-branch>; resume when it advances.
```

**Poll cadence matters.** Have the builder re-read the channel **periodically while working —
before every commit, at each milestone start, and at least every ~15 min during a long build** —
not only between milestones. Otherwise the babysitter can't steer a build *mid-milestone* (a long
milestone would finish on a wrong track before the builder ever re-checks). Frequent trunk-based
commits make "before every commit" a natural, cheap checkpoint.

## Reviewing well (babysitter discipline)

- **Read the code, not the commit message.** Pull the changed files and judge them against the
  design docs + the milestone brief: correctness, **architecture/boundary** adherence, fidelity to
  the intended design, **test coverage**, and shortcuts/drift/bugs.
- **Be specific and critical where due**, but **credit honesty and good calls** (e.g. a builder
  that *declines to fake* a feature and defers it cleanly is making a good call — the failure is
  in the *plan* that keeps deferring, which is the human's to fix).
- **Track the structural through-line**, not just per-commit quality. The classic failure mode:
  high-quality **surface** features accreting while the **load-bearing core** is deferred milestone
  after milestone into an overloaded finale. Name that pattern early and **escalate**.
- **Escalate to the human** when the fix is a re-scope/re-prioritisation (their call), and when
  asked, force it with a dedicated brief + non-negotiable acceptance criteria.

## Mechanics & limitations

- **Event-driven, not timed.** No cron in this environment; the Monitor poll-loop is the wake
  signal — you review on each push, nothing wasted when idle.
- **Monitor windows are bounded (~30 min here).** They time out; **re-arm** on each timeout (or
  when next active). A long idle stretch with no events may lapse the watch — the **durable record
  is the docs** (`agent-review.md`), which survive regardless.
- **The babysitter doesn't intervene in `main`.** It observes, logs, and steers via the channel +
  escalations. Direct re-scoping of the canonical plan is done only on the human's say-so (or by
  the builder applying a directive).

## Reuse checklist

1. Pick the steering branch; ensure it holds the design docs the builder needs.
2. Create `builder-directives.md` (current directive + how-to-use) and an empty `agent-review.md`
   (with a `Reviewed through:` marker).
3. Arm the babysitter's `origin/main` Monitor.
4. Give the builder its prompt: fast-forward `main`; read the channel each cycle; follow
   directives; on pause, watch the steering branch and resume.
5. On each builder push: review the code, log it, steer/escalate. Re-arm the Monitor on timeout.
