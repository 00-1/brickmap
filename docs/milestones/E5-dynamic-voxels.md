# ✨ E5 — Dynamic / cellular-automata voxels

> Status: **done** ✅ (v1 falling sand; sim + rendering verified, live behaviour reasoned
> + to confirm on the deployed build). Exploration rung in [`../roadmap.md`](../roadmap.md);
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
- [x] Live: sand seeded ahead of the camera (into loaded chunks only), stepped on a
      fixed tick, dirty sections re-meshed and re-uploaded.
- [x] `sand` toggle (11th); HUD `off:` shows it. (Re-mesh is *synchronous* — the sim
      is localized to a few chunks — rather than via the M6 loader; revisit if it ever
      grows enough to hitch.)
- [x] Runs native + web; CI green; docs synced. Pipeline verified headlessly (sand
      renders); live falling animation to confirm on the deployed build.

> Status: **done** ✅ — falling-sand v1: an overlay over the procedural base, seeded as
> a clump ahead of the flight, settling on the terrain. **Safe by design:** sand only
> touches loaded chunks and is forgotten on eviction, so it never races the streaming
> loader and a sand bug can't blank terrain. Deferred: water/fire automata, cross-chunk
> flow, off-thread sand re-mesh, debris-settles-into-grid (E2 bridge).
