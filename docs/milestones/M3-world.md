# M3 — A world to fly through

> Status: **draft** ⏳. Part of the ladder in [`../roadmap.md`](../roadmap.md).
> Builds on M2 (greedy meshing + chunk grid).

## Goal · Outcome · De-risk

- **Goal:** a real, large, procedurally-generated world you can fly across, stored
  cheaply (palette-compressed) and streamed around the camera.
- **Outcome:** endless-feeling terrain — hills, valleys, layered materials — that
  loads as you move and doesn't blow up memory.
- **De-risks:** storage RAM/bandwidth (perf pillar §7.3) and the streaming model.

## Scope

**In:**
- **Palette-compressed `Section`** storage: a per-section palette of distinct
  blocks + bit-packed indices, behind the *same* `get`/`set` API (so `mesh`/`world`
  callers don't change). Memory measured.
- **Procedural generation**: a deterministic noise heightmap (hand-rolled fractal
  value noise — no new dependency) with layered materials (grass/dirt/stone, plus
  room for sand/water).
- **Streaming**: load + mesh chunks within a radius of the camera; unload distant
  ones; the renderer's draw set updates at runtime (add/remove chunk buffers).

**Out (later):**
- Async/threaded meshing (M6) — M3 may mesh on the main thread (accept brief hitches).
- Caves/3D noise, biomes, structures — keep generation a heightmap for now.
- Saving/loading to disk (not a goal, design §3).

## Design sketch

### Palette-compressed `Section` (`world`)
- Replace the dense `Box<[BlockId]>` with `{ palette: Vec<BlockId>, indices: BitBuf }`
  where each voxel stores a palette index of `ceil(log2(palette.len()))` bits
  (min 1). `set` adds to the palette on a new block type and widens the index width
  when it overflows.
- `get`/`set`/`is_empty` keep their signatures; this is a pure storage swap.
- **Decision (D1):** index storage — a `Vec<u32>` bit buffer with width-aware
  read/write. Repacking on palette *shrink* is deferred (palettes only grow for now).
- **Memory:** a section of mostly air + a few materials drops from 64 KiB (dense)
  to a palette (a few entries) + ~1–4 bits/voxel.

### Procedural generation (`world` or a `worldgen` helper)
- Fractal value noise: hash-based lattice noise, a few octaves, seeded. Height =
  base + amplitude · fbm(x, z). Fill columns; surface grass, a few dirt, stone below.
- **Decision (D2):** hand-rolled value noise (deterministic, dependency-free) over
  pulling the `noise` crate — keeps the build lean and the output reproducible for
  golden images.

### Streaming (`app` + `render`)
- Track the camera's chunk coordinate. Maintain a set of *loaded* chunks within a
  radius; each frame (or on chunk-cross) generate+mesh newly-entered chunks and
  drop newly-exited ones.
- The renderer gains **add/remove chunk** operations (today it takes a fixed list
  at init). Meshing newly-loaded chunks needs their neighbours, so generation runs
  one ring wider than meshing.
- **Decision (D3):** synchronous generation+meshing for M3 (simple; brief hitches
  acceptable). M6 moves it off-thread.

## Tests

- **Palette storage:** `get`/`set` round-trip identical to the dense behaviour over
  random fills; palette grows and index width widens correctly; a low-diversity
  section uses materially less memory than dense.
- **Generation:** deterministic (same seed → same section); produces a plausible
  surface (non-empty, grass on top).
- **Streaming:** the loaded set matches the radius as the camera moves; chunks
  load/unload without leaking (counts return to baseline after a round trip).

## Acceptance checklist

- [x] Palette-compressed `Section` behind the unchanged API; round-trip + memory
      tests pass.
- [x] Procedural noise terrain; deterministic; layered materials (snow/grass/sand).
- [ ] Streaming: chunks load/unload around the camera; renderer updates at runtime.
- [x] A bigger world than M2's 3×3 (5×5), snapshotted (`/archive/06-procedural-world/`).
- [x] Runs native + web; snapshot + render in chat (terrain). *(Streaming render TBD.)*
- [ ] CI green; docs synced (per the lockstep rule).

> Status: **parts 1–2 done** 🛠 (noise terrain + palette storage). Part 3 (streaming +
> travelling camera) in progress.
