# E11 — More dynamic voxels

> **Status: ◑ in progress (2026-06-08).** Backlog item (roadmap E11). Built in slices on the
> engine's cellular-automata sim (`bm-world::sim`, where E5 falling-sand lives). Each slice is a
> **pure, deterministic, mass-conserving** rule with unit tests; golden voxel-hash stays unchanged
> (new behaviour is opt-in, not run on the static golden world).

## Goal (roadmap)
Extend E5 onto a proper substrate: block/Margolus CA + active-set, **pressure water**,
fire/smoke/steam under a heat field, the destruction loop, and growth.

## Slices

### E11-1 — flowing water  ◑ (this slice: the CA rule)
- **Landed:** `sim::step_water` — water (the terrain's stylised-water material, `BlockId(7)`)
  falls straight down, slides diagonally down (settling like sand), and otherwise **flows sideways
  into an adjacent air cell that can itself fall**, so it runs **downhill and pools**. The rule is
  **deterministic** (cell-parity tie-break), **mass-conserving** (every move is a water/air swap),
  and **terminating** (a sideways move only happens toward a way down, so water can't ping-pong —
  each grain monotonically descends or feeds a descent). Unit-tested: falls + conserved, flows off
  a ledge to a lower floor + conserved, boxed-in water rests (no spurious dirty).
- **Deferred:** *leveling* a puddle on a flat floor (water seeking a flat surface) needs the
  **pressure / compressible-mass** model (rendered as a vertex-displaced pass, not re-meshed) —
  the roadmap's "pressure water"; a later slice. **Wiring** the live world's water to actually
  flow (active-set seeding around the player, re-mesh budget) is the integration slice — kept out
  of this one so the static golden world / hash are untouched.

### E11-2+ ⏳ (later)
Block/Margolus substrate + active-set/dirty-AABB; pressure water; fire/smoke/steam under one heat
field; the destruction loop (explode→carve→eject→rest→write→slump); growth (moss/vines/crystals).

## Tests / acceptance (E11-1)
- [x] `step_water` is deterministic, mass-conserving, terminating; flows downhill + pools
      (unit-tested in `bm-world::sim`).
- [x] Golden voxel-hash + headless render unchanged (the rule isn't run on the golden world);
      clippy `-D` / tests / wasm green; engine stays generic (`bm-world`, no game dep).
