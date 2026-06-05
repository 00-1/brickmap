# ✨ E5 — Dynamic / cellular-automata voxels

> Status: **in progress** 🛠. Exploration rung in [`../roadmap.md`](../roadmap.md);
> backlog rationale [`../exploration-backlog.md`](../exploration-backlog.md) §A. The
> headline "interesting" — voxels that *behave*. Builds on M6 (off-thread re-meshing).

## Goal · Outcome · De-risk

- **Goal:** matter that moves — the most distinctive direction, genuinely beyond static
  cube terrain.
- **Outcome:** falling sand (v1) that piles on the terrain; re-meshed only where it
  changes.
- **De-risks:** the simulate → dirty → re-mesh loop, and doing it without hitching
  (M6's off-thread mesher absorbs the re-mesh churn).

## Scope

**In:**
- **Sand cellular automaton** (`sim`): pure logic over a `Section` — sand falls straight
  down, else slides diagonally, settling into piles. Deterministic + tested. **Done.**
- **Live integration:** an overlay of sim-modified sections on top of the procedural
  base; an active set near the camera stepped on a fixed tick; dirty sections re-meshed
  through the M6 loader (so no main-thread hitch). Sand seeded as a cheap "hourglass"
  ahead of the flight. A `sand` toggle (D6).

**Out (later):**
- Water / fire / smoke automata (same engine, more rules) — sand proves the loop.
- Cross-section flow (sand crossing chunk borders) — v1 keeps sand within a section
  (it falls onto the terrain inside the same 32-tall chunk).
- Debris-settles-into-grid bridge to E2 particles.

## Tests

- **Sand rules:** a grain falls to the floor; one step = one cell; sand is conserved and
  reaches rest; a dropped column spreads into a wider pile; grounded sand is inert. (All
  passing.)
- Live behaviour (the falling animation) is eyeballed on the deployed build; the meshed
  result of a sim state is checked headlessly.

## Acceptance checklist

- [x] Sand CA in `sim`; deterministic; behaviour tested.
- [ ] Live: sand seeded, stepped on a tick, dirty sections re-meshed off-thread.
- [ ] `sand` toggle; HUD shows it; no hitch (re-mesh goes through the M6 loader).
- [ ] Runs native + web; CI green; docs synced; snapshot/render.

> Status: **in progress** 🛠 — simulation core done; live integration next.
