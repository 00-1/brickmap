# E8 (final piece) — Vertical chunk stacks (multi-layer streaming)

> Brief for the one remaining E8 item: turn the world from a **single 32-tall layer** into a
> **vertical stack of section layers**, so terrain can exceed one section and caves/overhangs
> span layers. The within-layer wins (domain-warp, ridged, water, biomes, rivers, 3D caves) all
> landed; this is the bigger architectural step the roadmap deferred to its own brief.

## Goal · outcome · de-risks
- **Goal:** stream and mesh a **column** of sections per `(cx, cz)` column instead of one section
  at `y = 0`, so the world has real vertical extent (tall mountains, deep caves, overhangs).
- **Demonstrable outcome:** fly **up** over peaks that rise past 32 blocks and **down** into caves
  that drop below the old floor, with chunks streaming in/out vertically as well as horizontally —
  no holes at layer seams, no hitch.
- **De-risks:** the last structural assumption baked into the hot path (single layer). Unlocks
  genuinely 3D terrain for M5 cave-culling to bite on, and is a prerequisite for any tall built
  structures sitting *in* the terrain rather than floating on the surface skin.

## Scope
**In:**
- A configurable vertical band of layers `cy ∈ [Y_MIN, Y_MAX]` (e.g. `-1..=3` → world y `-32..=127`).
- `worldgen::generate_section(cx, cy, cz, seed)` filling each section by **absolute world-y**.
- Vertical neighbours wired into the mesher (the ±Y faces stop being hardcoded `None`).
- Streaming a **column** per `(cx, cz)`: request/evict/within all gain a `cy` range.
- The camera free to climb/descend through layers (auto-fly already hugs the height field; keep it,
  but let manual/pad flight leave the surface band).

**Explicitly out (this milestone):**
- Per-layer LOD / point-decimation of far/low layers (that's M7's deferred general mode).
- Infinite vertical (keep a fixed, generous `[Y_MIN, Y_MAX]`; bedrock floor + sky ceiling).
- Re-tuning the terrain *look* — only the **vertical fill** changes, biome/material rules stay.

## Design sketch
**Coords.** `ChunkCoord` is already `(i32, i32, i32)` — the `cy` slot exists and is threaded
through `World`, `loaded`, the overlay, `ChunkInstance`, and the shader's per-chunk origin (which
already uses `origin.y`). So most of the plumbing is removing the `cy == 0` / `y = 0` hardcodes,
not adding a dimension.

**Worldgen (the real work).** Today `generate_section(cx, cz, seed)` builds one section from a 2D
height field clamped to `Section::SIZE`. Change to `generate_section(cx, cy, cz, seed)`:
- Compute the column's continuous surface height `h` in **absolute world-y** (un-clamped) from the
  existing `height_f` / ridged-warp field.
- For each local `y` in the section, its world-y is `wy = cy * 32 + y`. Fill solid/stone/dirt/grass
  by comparing `wy` to `h` (grass only at the surface cell `wy == floor(h)`), water for
  `wy < SEA_LEVEL` above the surface, air above. Caves carve as today but sampled at absolute `wy`.
- Sections fully below the surface band are all-stone (cheap early-out: if `cy*32+31 < h_min` for
  the column's min height → solid fill, skip per-voxel noise); fully above → all-air (skip).
  These early-outs keep the column cheap (only the 1–2 sections straddling the surface do real work).
- `height(wx, wz, seed)` stays (used by camera/structures/foliage/text placement) but returns the
  **un-clamped** absolute surface y.

**Meshing.** `mesh_chunk` currently regenerates 4 horizontal neighbours and passes `None` for ±Y.
Add the up/down neighbours: `generate_section(cx, cy±1, cz, seed)` and put them in
`Neighbors.faces[2]` (−Y) and `[3]` (+Y). Then seam-culling hides faces between vertically-stacked
solid sections (no interior walls at layer boundaries). The cached web builder
(`SectionCache` + `build_chunk_instance_cached`) caches by `(cx, cy, cz)` and reuses neighbour
sections it already generated (the existing time-budget still applies).

**Streaming.** Generalise `stream()`:
- `within(coord)`: `(cx-ccx).abs() ≤ keep && (cy-ccy).abs() ≤ V_KEEP && (cz-ccz).abs() ≤ keep`,
  where `ccy = floor(cam.y / 32)` and `V_KEEP` is a small vertical radius (e.g. 2 — the band is
  shallow, so we don't need the full horizontal radius vertically).
- Request loop: wrap the existing ring loop in `for cy in (ccy-V_KEEP)..=(ccy+V_KEEP)`, clamped to
  `[Y_MIN, Y_MAX]`, and request `(ccx+dx, cy, ccz+dz)`. Keep nearest-first by extending the ring
  metric to include `|dy|` (or just iterate layers inside each ring — vertical band is tiny).
- Budgets (`STREAM_REQUESTS`, `STREAM_UPLOADS`, web mesh time-budget) are unchanged but now spread
  over more chunks; verify the per-frame cost stays bounded (the all-air/all-stone early-outs make
  most column sections nearly free to generate and trivial to mesh — empty mesh → no draw).

**Culling.** Frustum cull already uses each chunk's real AABB (origin includes `cy`), so it works
unchanged. Cave-cull's visibility graph is per-section; vertical neighbours now connect, which is
exactly the 3D structure M5 was waiting for.

## Decisions to resolve (with a recommended default)
- **Band extent `[Y_MIN, Y_MAX]`** → **default `-1..=3`** (world y `-32..127`, 5 layers): enough for
  the current terrain amplitude + caves with headroom, cheap to stream. Revisit if terrain grows.
- **Vertical keep radius `V_KEEP`** → **2** (load 2 layers above/below the camera's layer).
- **Un-clamp terrain amplitude?** → **modestly**: let peaks reach ~world-y 80–100 (was 31). Keep it
  tasteful so the band stays shallow; this is a fill change, not a new terrain look.
- **Bedrock / ceiling** → solid floor at `Y_MIN*32`, air above `Y_MAX*32` (no fall-through, no
  infinite climb).

## Tests (pure logic — no GPU)
- `generate_section` determinism in `(cx, cy, cz, seed)`; same column across layers stitches (the
  top voxel of layer `cy` and bottom of `cy+1` agree on solid/air at the shared world-y).
- A column straddling the surface has air in its top section and solid in a section well below.
- All-air / all-stone early-outs produce the same sections as the per-voxel path (equivalence test
  on a few columns).
- `height()` returns the un-clamped absolute surface; structures/foliage still sit on it.
- Streaming set math: `within` + the request ring select the expected column set for a given camera
  (unit-test the index math, no renderer).

## Risks & mitigations
- **Per-frame cost balloons** (N× more chunks). → all-air/all-stone early-outs make off-surface
  sections nearly free; empty meshes upload nothing; keep `V_KEEP` small; watch the HUD ms.
- **Layer-seam holes** (mis-wired ±Y neighbours → culled faces that should show, or interior walls
  that shouldn't). → the neighbour-stitch test + a headless render straddling a layer boundary.
- **Web meshing time-budget** now spread over a column — confirm it still drains without a hitch
  (the section cache already dedupes neighbour regeneration).
- **Regression to the current single-layer look** — gate the change so the default band still
  frames the same surface; verify the hero shot is ~unchanged headless before/after.

## Acceptance checklist
- [ ] `generate_section` takes `cy` and fills by absolute world-y; early-outs verified equivalent.
- [ ] Mesher receives real ±Y neighbours; no interior walls at layer seams (test + render).
- [ ] `stream()` loads/evicts a column; `within`/request math unit-tested.
- [ ] Fly up over a >32-tall peak and down past the old floor with no holes/hitch (headless stills
      at a layer boundary + a tall peak).
- [ ] HUD frame time stays within budget with the deeper band (no per-frame cost blow-up).
- [ ] Docs in lockstep: flip E8's "vertical stacks" from pending to done in `roadmap.md`, update the
      architecture "World model" row (multi-layer streaming → landed), README status.
