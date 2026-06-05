# ✨ E14 — Creative / voxel editing

> Status: **in progress** 🛠. Exploration/feature rung in [`../roadmap.md`](../roadmap.md).
> Doubles as the **command/event seam** the multiplayer research (N1) flagged as the
> low-regret prep to do early — so editing, undo/replay, seed+delta sharing, and (later)
> network broadcast all funnel through one mutation type.

## Goal · Outcome · De-risk
- **Goal:** place and remove voxels in the live world, with undo, through a single
  serialisable-shaped edit command — not ad-hoc overlay pokes.
- **Outcome:** look at a surface, place/break a block, undo it; edits persist in the
  `overlay` and re-mesh through the existing sand path.
- **De-risks:** the mutation seam (so multiplayer/replay/sharing don't need a rewrite), and
  voxel picking (DDA).

## Landed
- **`edit::Edit`** — a plain-old-data command (`Set { pos, block }`; AIR = remove), kept
  trivially serialisable without taking a serde dep yet (consistent with `share`).
- **`edit::apply`** — the *one* door that mutates the overlay: materialises the section
  from the seed if needed, sets the voxel, returns `(dirty chunk, inverse edit)` for undo
  and as the future broadcast payload. No-ops (same block) return `None`.
- **`edit::raycast`** — Amanatides–Woo DDA voxel pick returning the hit voxel + entry-face
  normal (for placing *against* a surface). Fully unit-tested.
- **App wiring** — overlay-aware solidity (overlaid cell wins, else base terrain `y <
  height`); native dev keys **V** place / **B** break / **U** undo; re-mesh via the shared
  `remesh` path (same as sand). An `undo: Vec<Edit>` log.

## Tests
- `apply` sets the voxel and its returned inverse restores it; no-op detection.
- `world_to_chunk_local` wraps negatives and rejects out-of-layer `y`.
- `raycast` hits along an axis, reports the correct entry normal (place-against-floor),
  and misses empty space.

## Out / deferred
- Wireframe **hover highlight** of the targeted voxel (needs a small render path).
- **Multi-voxel brushes** (one `Edit::Brush` expands to many cells) — the enum is ready
  to grow a variant.
- **Web mouse-picking** (the logic is platform-agnostic; only the input wiring is native
  so far).
- **Seed + sparse-delta sharing** — serialise the `Edit` log into the share link / a blob
  (ties to E12; serialisation hand-rolled or via serde when added).
- Boundary re-mesh: editing a chunk edge re-meshes only that chunk (neighbours regenerate
  from seed, so an edited seam can be momentarily stale) — fine for v1.

## Acceptance checklist
- [x] `Edit` + `apply` seam (single mutation door); inverse-for-undo; unit-tested.
- [x] DDA voxel pick with face normal; unit-tested.
- [x] Place / break / undo wired (native keys), re-meshing via the overlay path.
- [ ] Hover highlight; brushes; web picking; seed+delta share. *(deferred)*
- [x] CI green; docs synced.

> Status: **in progress** 🛠 — the seam + pick + place/break/undo core landed; UX polish
> and delta-sharing deferred.
