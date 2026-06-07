# Scraped Again — The operations substrate (blocks)

> **Status: core-architecture design.** This is the spine of *Scraped Again*'s
> progression, management, and automation — and it **supersedes the "tech tree" framing**
> in [`game-mechanics.md`](game-mechanics.md) §8. It's a single block-based substrate
> underlying *everything you do through the console*, surfaced from one-click triggers up
> to full composition. Melancholy-archival, **expose-the-tech**, no-typing,
> controller/phone-first. Pairs with [`game-mechanics.md`](game-mechanics.md) (the loop,
> strata, modes) — read that first.

## 1. The one idea: everything is blocks, fully transparent

There is **no separate "menu" layer over a hidden system.** The game's actions, upgrades,
and automation are all **blocks** — and the blocks themselves *are* the interface:

- A block is **visible** (you see the actual operation, terminal-style — on-brand
  "expose the tech").
- A block is **clickable to trigger** — clicking `collect` *is* the manual collect; the
  block is its own button. (So there are no buttons hiding blocks; the block is the button.)
- A block is **wireable** — feed it a trigger/condition and it runs itself.

**Full transparency:** everything decomposes to **atomic leaf-blocks**; nothing is an
opaque button you can't open. The "simple" experience and the "deep" experience are the
*same surface* at different amounts of wiring.

## 2. The manual → automatable ladder (one substrate, chosen depth)

Depth is opt-in, with **no cliff** between casual and expert — you just wire more:

- **L0 · Click** — trigger visible blocks by hand (`collect`, `scan`, `spend-shard`,
  `fire-beam`). A player can stay here forever; it reads as clicking a living terminal.
- **L1 · Trigger** — attach a block to an event: `on-scan → collect`.
- **L2 · Condition / filter** — gate it: `on-scan → if matches(…) → collect`.
- **L3 · Compose** — chain into named **routines** / pipelines the agent runs.
- **L4 · Budget / policy** — allocate and prioritise (`70% of Records → faculties`; routine
  priority; spend caps).
- **L5 · Meta** — blocks that act on the system: enable/disable routines, switch routine-
  sets by context, auto-queue upgrades.
- **L6 · Optimise** — tune the self-running machine: bottlenecks, elegant compositions,
  policies. The skill ceiling that never bottoms out.

This ladder **is** the relationship between the three modes (game-mechanics §1): autopilot
players climb it (the management game), hands-on players live near the bottom. Same system.

## 3. Block kinds

- **Action-blocks** — the automatable twin of every manual thing: `collect`, `scan`,
  `decode`, `goto`, `fire-beam`, `descend`, `spend-shard`, … Clickable to do once; wireable
  to automate.
- **Control / meta-blocks** — `on-event` triggers, `if`/`filter` conditions, `loop`,
  `budget`, `priority`, and system-meta (enable/switch routines). **This is where the
  deepest complexity lives** — each new control block multiplies what every action-block can
  do. The expansive, intricate "tree" is mostly *these*.
- **Upgrades** — passive boosts (sharper scan, bigger buffer, longer beam) bought with a
  spendable resource. They're not outside the substrate: **acquiring one is just the
  `spend-shard(target)` action** — so upgrades, too, can be triggered by hand or automated
  (`when shards ≥ N → spend-shard(sensing)`). Normal upgrade *menus* are simply the
  collection of spend-targets, shown as blocks.

## 4. Progression = the growing block vocabulary (the "tech tree", reframed)

The "tech tree" is the **library of blocks you've recovered**. Unlocking a block = having
**comprehended a function** of the dead machine. So:

- **Data unlocks vocabulary.** Collected/decoded **strata** (game-mechanics §5) unlock new
  action and control blocks — the recovered operations of the dead civilization's console.
- **Comprehension feeds automation.** What your **scan can detect** gates what your
  **filters can match on** — you can only `filter(Runic)` once scan identifies scripts,
  only `filter(rare-stratum)` once you detect strata. The knowledge-web and the
  automation-web are wired together, not parallel.
- **Lore is implied, never read.** Comprehension is *operational* — understanding how the
  machine works, not reading its diary. The grief is the *vibe* of operating a dead
  people's console alone; block names and tone carry a general feeling, not text dumps.

## 5. Discovery, not programming puzzles

The interest is **learning the machine's idiom**, not solving for a right answer:

- New blocks (especially control/meta) arrive with **terse, slightly cryptic** descriptions
  — a recovered man-page — and you **experiment** to learn what they reference and combine
  with. *This* is the "figure out how it works."
- It stays a **sandbox, not a puzzle**: no syntax to get wrong, no single solution, results
  immediately visible, no fail-state. You compose what *you* want; mastery is fluency.

## 6. No typing — and why that's also the right call

Blocks are **inserted and clicked**; values (thresholds, budgets, targets) are **pickers
and steppers**, never typed expressions. This keeps it from becoming literal programming —
*and* it's the **practical** necessity: the game targets controller + phone (D4/D7), where
block selection works and typing doesn't. The terminal vibe and the platform agree.

## 7. Two agents, run independently — and the *hail*

Routines are **per-agent**, drawing on a **shared block library**; the *available* blocks
are **context-scoped**:

- **Ship blocks** — scan, seek/goto, route, cruiser-beam…
- **Foot blocks** — walk/path, survey-beam, grapple-ride, descend, collect-up-close…
- **Shared** — collect, filter, decode, spend, all the control/meta blocks.

The two agents **run independently and simultaneously**: the **ship can fly its own routine
while you're off walking** — both working at once. Because the ship may no longer be parked
where you left it, you can **hail** it — recall the autonomous ship to your position (the
counterpart to the survey-beam's *board*, game-mechanics §6, for when the ship is away).

**The ceiling** this opens: automate a whole **expedition** — *ship flies to a scanned site
→ lands → the walker disembarks and runs an on-foot collection routine → returns → ship
flies on.* Two programmable agents in one dead world, running a survey you authored. The
L6 "self-running machine" made vivid, spanning modes.

## 8. First instance: scan → collect (and how a session opens)

The earliest concrete arc *is* the substrate working:

- Manual: click `scan` (or it auto-scans ahead) → the map fills; click `collect` /
  fire the beam at finds yourself.
- `on-scan → collect` → naïve auto-collection; `on-scan → if matches(…) → collect` →
  selective; `… → prioritise/budget` → tuned. The old "Auto-Collect upgrade" *is* this
  composition.

**Onboarding** by full transparency: the player is **handed a small pre-wired routine,
shown as its blocks** (a working `on-scan → collect`). They can click to run it, toggle it,
or **open and rewire** it — learning the whole system from a working artifact, not a blank
console. No tutorial wall; the example *is* the teacher.

## 9. Honest hard parts

- **It's the meatiest build in the game** — a block runtime *plus* a composition/edit UI
  *plus* a multi-agent simulation. Prove a tiny version first (a few triggers + actions +
  one filter + one running routine), then grow the vocabulary.
- **Legibility & discovery without text walls** — blocks must be learnable *by doing*; a
  careful design+UI problem, and it leans on the E17 text path rendering a terminal cleanly.
- **Controller/phone UX** for composition — block selection, wiring, and value-picking must
  feel good on a pad and a touchscreen, not just a mouse.
- **Balance** — emergent automation should be *satisfying*, not a degenerate exploit; idle
  economies need iterative tuning (design the shape now, tune feel on play).
- **Multi-agent simulation** — two independently-running agents (one possibly off-screen) +
  the hail + the handoff is real systems work; keep the off-screen agent cheap/abstracted.
- **Avoid drifting into actual programming** — chunky forgiving blocks, no syntax, immediate
  feedback; resist creep.

## 10. Build implications

- **The block runtime is foundational, introduced early and minimally**, then grown — *not*
  a late "tree" feature. Even early manual collection is "clicking the `collect` block."
- This **reframes existing docs**: game-mechanics §6 (auto-collect → a composed routine),
  §8 (the tree → the block vocabulary), and the **G-series** build order (a minimal block
  runtime comes in with the first gameplay slices; scan/collect/beam are *blocks*). Those
  will be reconciled to point here; the detailed re-slice of the G-series is a follow-up.
