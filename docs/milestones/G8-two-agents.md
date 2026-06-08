# G8 — Two agents, run independently (+ the hail), then the expedition

> **Status: building (2026-06-08), in slices.** This is the old overloaded "G7+" catch-all,
> re-scoped after G7 landed the routine runtime. Per [`../game-system.md`](../game-system.md) §7
> and the babysitter's standing note that G8 must **not** be swallowed whole, it is built in
> tractable, independently-green slices. The runtime ([`G7-routine-runtime.md`](G7-routine-runtime.md))
> is the substrate; G8 makes it span **two simultaneous agents**.

## The pillar (game-system §7)

Routines are **per-agent**, drawing on a **shared block library**, scoped by context: **ship**
blocks (scan/seek/route/cruiser-beam…), **foot** blocks (walk/survey-beam/descend/collect-up-
close…), **shared** (collect/filter/decode/spend + control/meta). The two agents **run
independently and simultaneously** — the ship can fly its own routine while you're off walking —
and because the ship may wander off, you can **hail** it: recall the autonomous ship to your
position (the counterpart to the survey-beam's *board*). The ceiling: automate a whole
**expedition** (*ship flies to a site → lands → walker disembarks + collects → returns → ship
flies on*).

## Slices

### G8a — the ship as an autonomous agent + the hail  ✅ (landed 2026-06-08)
- **Goal.** While you're on foot (`Mode::Walk`), the cruiser **keeps running its ship routines**
  instead of sitting parked: it flies its nav (`drift`/`seek`/`circle`) under the **G7 interpreter**
  and **auto-scans** the world it passes (feeding the map opportunity surface) — both agents
  working at once. A new **`hail`** action recalls the autonomous ship to the walker.
- **Scope (in).** A lightweight autonomous-ship tick (advance `cruiser_pos` by the same nav math
  the piloted autopilot uses, kept at cruise height; scan around the ship when `survey` is enabled).
  `scan_pulse` generalised to scan from a given **vantage** (camera *or* ship). A `hail` block +
  key that flies/recalls the ship to the walker. Map/HUD already show the moving ship dot.
- **Scope (out).** A full **per-agent routine library** (ship/foot/shared scoping in the console) —
  G8b. Foot-collection routines + the disembark/return **expedition** choreography — G8c.
  Off-screen *collection* by the ship (keep the away-agent cheap: it scans, it doesn't bank).
- **De-risks.** That "two independently-running agents + the hail" is viable on the G7 runtime and
  feels good, before the per-agent library + expedition pile on.

### G8b — per-agent routine library (ship / foot / shared)  ✅ (landed 2026-06-08)
- **Goal.** Routines are **per-agent** (`Agent { Ship, Foot }`); the console scopes the insertable
  vocabulary by agent context + shared blocks; the interpreter ticks each agent's routines for
  that agent. On foot, the walker is a second agent whose shared routines (collect/decode/hail)
  run simultaneously.
- **As-built.** `Routine.agent`; `Block::agent()` classifies ship (scan/nav/goto) vs foot (the
  survey-beam) vs shared (collect/decode/spend/hail); `vocabulary(agent)` filters the editor's
  insert/cycle list; `tick(agent, data)` / `on_scan_acts(agent)` run only that agent's routines.
  The editor flips a routine's agent with **Tab**; the home/edit screens show the agent tag; the
  agent persists in `co=`. The app ticks **ship** for the cruiser (piloted or autonomous) and —
  when walking — **foot** for the walker's shared acts (a continuous `collect` harvests as you
  explore; `when … → decode`/`hail` fire). The three givens are ship routines (parity).
- **Deferred to G8c:** foot **nav** (auto-walk / pathing) — the walker doesn't yet steer itself;
  foot routines currently contribute their *shared* acts + on-scan collect while you move.

### G8c — the expedition + cross-agent meta  ◑ (in progress)
The choreography (`goto(site) → land → run(foot: collect) → return → fly on`), `on-arrive`
triggers, and cross-agent meta (a ship routine that runs the walker's routine) — the rare-stratum-
gated deep end (game-system §7 ceiling, §11 Tier 3).
- **G8c-1 ✅ — the `on-arrive` trigger** (the expedition skeleton). `Trigger::OnArrive` fires once
  (rising edge) when the agent reaches the site it's heading to (the ship's nearest known site is
  within an arrival radius). Composable like the other triggers (cycle to it in the editor;
  persists in `co=`); so you can author `seek → on-arrive → decode/hail/…`. The interpreter takes
  an `arrived` flag; the app computes it for the ship (`ship_arrived`).
- **G8c-2a ✅ — foot `walk` nav + on-foot auto-walk.** A foot nav block `walk(uncollected)`
  (foot analog of the ship's `seek`); while on foot, a `walk` routine **auto-walks** the walker
  toward the nearest known site (only when you're not steering — manual input always wins), fed
  through the existing voxel-collision walk (gravity / auto-step). With `on-arrive → collect` (or
  a continuous foot `collect`), this is the **on-foot auto-collection loop** you compose — the
  walker gets the ground-level finds the high-cruising ship misses. Pure `walk_toward` step is
  unit-tested; `walk` is foot-scoped + persisted.
- **G8c-2b ⏳ — the off-screen expedition + cross-agent meta.** A **second *persistent* walker
  entity** that auto-walks + banks **while you pilot** (the away-walker, mirroring G8a's
  away-ship), the disembark/return choreography, and cross-agent `run(foot: …)`. This needs a
  change to the board/exit movement flow (the walker persists where you left it and tours on its
  own), so its payoff hinges on play-feel — flagged for end-of-run human iteration (the testable
  skeleton — on-arrive, foot nav, per-agent ticks — is in place).

## G8a — design sketch

- **Autonomous ship state.** Reuse the autopilot heading integrator, but on a ship-local clock
  (`ship_t`, `ship_angle`) so the ship's wander is independent of the (walking) camera. Each frame
  in `Mode::Walk`, if a continuous **nav** routine is enabled, advance `cruiser_pos` by the nav
  intent (drift wander / seek nearest known site / circle), tracking cruise height over terrain.
- **Scan from a vantage.** Generalise `scan_pulse` → `scan_from(origin, forward)`; the piloted path
  passes the camera, the autonomous ship passes `cruiser_pos` + its heading. The away-ship scan
  marks sites **known** (map fills) but does **not** auto-collect (cheap away-agent).
- **Hail.** A `hail` block (shared vocabulary) + a key on foot: the parked/auto ship flies back to
  the walker over a short recall, or snaps if already close — re-using the board hop. It's wired so
  a foot routine can later `hail` automatically (G8b/c).

## Tests
- Autonomous-ship nav advances `cruiser_pos` while walking; parity: piloted behaviour unchanged
  (golden voxel-hash + headless render unchanged).
- `scan_from` marks known sites from an arbitrary vantage (pure-ish logic test on the cone).
- `hail` brings the ship within board range of the walker.
- clippy `-D` / tests / wasm green; `bm-*` → game boundary intact.

## Acceptance (G8a)
- [x] The ship **flies its routine while you walk** (autonomous nav + away-scan), visible on the
      map; piloted behaviour is unchanged (parity test green — golden voxel-hash stable).
- [x] **`hail`** recalls the autonomous ship to the walker (`H` key + `hail` block), wireable into
      a routine (round-trips through `co=`).
- [x] Tested (`autopilot_step` drift/seek/circle + hail-step persistence; 78 game-crate tests);
      clippy `-D` / tests / wasm green; roadmap G8a ticked.

## As-built (G8a, 2026-06-08)
- The piloted autopilot + the autonomous away-ship now share one pure **`autopilot_step`** (a nav
  integration step), so the away-ship's drift/seek/circle is identical math on its own clock —
  unit-testable without a GPU.
- `scan_pulse` generalised to **`scan_from(origin, forward, do_on_scan)`**; the away-ship passes
  `do_on_scan=false` (it fills the map but doesn't bank — a cheap off-screen agent per §7).
- `hail` is a **shared starter block** (no comprehension gate) so it's usable + wireable from the
  start; `H` triggers it on foot. The recall re-homes the ship a step from the walker (board range).
