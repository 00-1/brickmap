# M2 — Greedy meshing + a grid of chunks

> Status: **in progress** 🛠. Part of the ladder in [`../roadmap.md`](../roadmap.md).
> Builds directly on M1.

## Goal · Outcome · De-risk

- **Goal:** land the project's #1 performance pillar — **binary greedy meshing** —
  and the **compressed vertex**, across a **grid of chunks** with **frustum
  culling** and correct **inter-chunk** face culling.
- **Demonstrable outcome:** fly through a multi-chunk world drawn as **merged
  greedy quads** (far fewer triangles than M1's naïve mesh), with off-screen
  chunks skipped and no doubled walls at chunk seams.
- **De-risks:** meshing throughput/quality (design §7.1), the ≤8-byte vertex
  budget (§7.2, §9–10), and the multi-chunk render path.

## Scope

**In:**
- **Binary greedy mesher**: merge coplanar, same-material faces into maximal
  quads. Verified against M1's naïve mesher as a correctness oracle; benchmarked.
- **Packed vertex**: finalize and implement the compact face vertex (target **4
  bytes**, see D1) with encode/decode round-trip tests. Shader unpacks it.
- **Neighbour-aware meshing**: cull faces against adjacent chunks so seams have no
  hidden double faces.
- **Chunk grid**: a `World`/chunk map holding many `Section`s at chunk coords;
  a simple multi-chunk test scene (hand-built or trivial procedural — *not* real
  terrain yet).
- **Frustum culling**: cull chunks whose world AABB is outside the view frustum.
- **Render**: draw many per-chunk meshes, each offset to its world position; a
  small material→colour palette in the shader (textures are still M4).
- **On-screen stats** (lightweight): triangles + draw calls, to see the win.

**Explicitly out (later):**
- Palette-compressed storage, real procedural generation, streaming → **M3**.
- Texture-array materials, ambient occlusion → **M4** (reserve AO bits now).
- Visibility-graph / cave culling → **M5** (M2 is frustum-only).
- Async/threaded meshing → **M6** (M2 may mesh on the main thread).
- LOD → **M7**.

## Design sketch

### Binary greedy meshing (`mesh`)
For each of the 3 axes and 2 facing directions (6 face sets): sweep the section
slice by slice; for each slice build a 2D mask of cells that have an **exposed**
face (solid voxel whose neighbour in that direction is air — *including across
chunk borders*, see neighbours below). Greedily merge mask cells into maximal
rectangles, emitting one quad per rectangle. The "binary" form represents each
row as a bitmask and finds exposed faces with bit ops (`col & !neighbour_col`),
which is the fast path; correctness-first then optimise, validated against M1.

- **Merge key (D2):** two faces merge only if same **material id** and same face
  direction. (AO would also gate merging, but AO is M4 — reserve it.)
- Quads carry their extent implicitly: the 4 corner positions sit on the
  `0..=SIZE` grid, so no separate width/height field is needed in the vertex.
- The naïve `mesh_section` stays (renamed/kept) as the **test oracle**: greedy
  output must cover exactly the same faces/area as naïve for any fixture.

### Packed vertex (D1)
Target **one `u32` (4 bytes)** per corner:

| field | bits | notes |
|---|---:|---|
| position x/y/z | 6 × 3 = 18 | `0..=32` per axis (greedy quads land on grid lines) |
| face dir | 3 | one of 6 |
| material id | 9 | 512 materials; maps to a colour now, texture layer in M4 |
| AO | 2 | **reserved**, written 0 until M4 |
| — | total **32** | |

The vertex shader unpacks position/dir/material and looks colour up from a small
palette uniform; the chunk's world origin is added (see D4). Round-trip
encode/decode is unit-tested per field.

### Neighbours (D3)
`mesh_section` needs to see one voxel past each face to cull seam faces. Options
in D3; default is to pass the 6 neighbour `Section`s (or `Option<&Section>`,
treating absent neighbours as air).

### Chunk grid + render
- A `World` maps chunk coordinates → `Section`. M2 builds a small NxN(xM) grid.
- Each chunk meshes to a `ChunkMesh` with a **world-space AABB** (local AABB +
  chunk origin).
- Render keeps a per-chunk vertex/index buffer; each draw needs the chunk's world
  origin (D4). A palette uniform supplies material colours.
- `scene` extracts 6 frustum planes from the view-projection and tests each
  chunk's AABB; only visible chunks are drawn.

## Decisions to resolve (with recommended defaults)

- **D1 — Vertex layout.** *Recommend* the 4-byte `u32` layout above (18/3/9/2).
  Revisit material-id width once M4's texture-array size is known.
- **D2 — Merge key.** *Recommend* material id + face direction for M2 (add AO in
  M4).
- **D3 — Neighbour delivery.** *Recommend* pass `[Option<&Section>; 6]` to the
  mesher (absent = air). *(Alt: a padded 34³ copy — simpler indexing, more copy.)*
- **D4 — Per-chunk transform.** *Recommend* a **dynamic-offset uniform** (or a
  per-chunk uniform) carrying the chunk world origin — **not push constants**
  (unsupported on WebGL, and web must keep working).
- **D5 — Test scene.** *Recommend* a small hand-built/trivially-generated grid
  (e.g. a few chunks of rolling height) — enough to show merging + seams + frustum
  culling, without M3's real generation.

## Tests

- **Greedy == naïve (oracle):** for a set of fixtures (single voxel, slab, 2×2×2,
  full section, a checker pattern, a chunk-border case), greedy and naïve cover
  the **same set of faces** (same total area; every naïve face lies within exactly
  one greedy quad).
- **Greedy actually merges:** a flat NxN slab meshes to **1 quad**, not N².
- **Seam culling:** two adjacent full chunks produce **no faces** on their shared
  boundary plane.
- **Packed vertex round-trips:** every field survives encode→decode across its
  range; out-of-range is debug-asserted.
- **Frustum culling:** AABBs clearly inside/outside a known frustum are
  included/excluded; straddling AABBs are included.
- **Bench (criterion):** record greedy meshing throughput (chunks/sec or ms/chunk)
  on a representative section, tracked against design §8.

## Risks & mitigations

- **Greedy correctness is fiddly** → the naïve oracle + fixture tests catch
  regressions; build correctness-first, optimise to bitwise after green.
- **Seam handling** → explicit two-adjacent-chunks test; absent neighbour = air.
- **Web per-chunk transform** → D4 avoids push constants.
- **Vertex bit-packing off-by-one** → round-trip tests over full field ranges.

## Acceptance checklist

- [x] Greedy mesher; greedy==naïve oracle tests + "actually merges" tests.
      *(Correctness-first rectangle merging; the bitwise "binary" speed-up is a
      follow-up optimisation within M2.)*
- [ ] 4-byte packed vertex with round-trip tests; shader unpacks + palette colour.
- [x] Neighbour-aware meshing; adjacent-chunk seam test passes.
- [x] `World` chunk grid; many chunks drawn at their world origins.
      *(World-space baked vertices + per-chunk buffers; per-chunk transform via the
      packed vertex comes later.)*
- [x] Frustum culling with tests; off-screen chunks skipped.
- [x] Triangle/draw-call stats (throttled console log for now; on-screen HUD is M5).
- [ ] A meshing throughput number recorded (criterion) against the budget.
- [ ] Runs native + web; preview shows the multi-chunk world; snapshot to gallery.
- [ ] CI green; roadmap M2 flipped to done; docs synced in-commit.
