# ✨ E6 — Splats & ground foliage

> Status: **planned** ⏳. Exploration rung in [`../roadmap.md`](../roadmap.md). First rung
> of the **point-cloud / foliage** aesthetic pivot. Grounded in
> [`../research-points-splatting.md`](../research-points-splatting.md); references:
> capslpop "voxel splatting for animated foliage" + Superbien point-cloud forest.
> Builds on M6 (off-thread per-chunk generation) and E3 (bloom/fog/light).

## Goal · Outcome · De-risk

- **Goal:** stand up the **splat render path** and use it for a first lush layer —
  **wind-swept ground foliage** (grass/plants) as camera-facing point-splats over the
  meshed terrain.
- **Outcome:** the ground reads as a grassy, alive field of points that sways; the world
  stops being bare meshed terrain. Toggleable.
- **De-risks:** (1) the splat pipeline itself — does it look good and run cheap on weak
  hardware? (2) keeping the on-screen splat count bounded (the research's #1 perf lever).
  Everything later in the pivot (trees, atmosphere, terrain-dissolve) reuses this path.

## Scope

**In:**
- **Splat pipeline** (shared windowed + headless): one unit quad, **instanced**, VS
  billboards it in view space and scales by per-instance size; FS discards outside a
  disc (alpha-*test*, no blend) and flat-shades. Shares the globals (group 0). Drawn
  after terrain; bright foliage glows through the existing bloom.
- **`SplatInstance { offset:[f32;3], size, color:[f32;3], sway }`** (Pod), per chunk.
- **Ground foliage generation** (`foliage` module, pure): `scatter(section, cx, cz,
  seed) -> Vec<SplatInstance>` — find each column's grass surface, emit a few
  **world-positioned** splats just above it with hashed jitter / size / green-variation
  / sway phase. Deterministic. Generated in the M6 worker alongside the mesh.
- **Wind sway** in the VS: offset the splat by a cheap `sin(time + sway)` so the field
  moves. (Adds a `time` value to the globals.)
- **Bounded count:** foliage only within a **foliage radius ≤ stream radius**, frustum-
  culled per chunk, with a per-chunk cap and density dial. HUD shows the splat count.
- **`foliage` toggle** (D6).

**Out (→ E7 / later):**
- Trees and tall point-cloud structures, denser layered vegetation, the lush palette and
  light-shaft atmosphere — that's **E7**.
- Terrain itself dissolving into points at distance — that's the reframed **M7**.
- Distance LOD of the foliage (fewer/bigger splats far) — add only if the count needs it.

## Design sketch
- Per chunk, the worker returns `(mesh, graph, foliage: Vec<SplatInstance>)`; the
  renderer uploads a per-chunk instance buffer and draws it in a foliage pass, frustum-
  culled by the chunk AABB (reusing the M5 cull).
- Splats are **world-positioned** (gen adds the chunk origin), so the pipeline needs no
  per-chunk uniform — just globals + the instance buffer.
- `time` goes in a spare globals slot (e.g. `camera_pos.w`) for the wind.
- Perf discipline (from the research): keep splats ≥ a few px, cull sub-pixel, flat FS,
  no per-splat texture fetch; bound on-screen count hard.

## Tests
- `scatter`: deterministic; only emits over grass; respects the density dial; splats sit
  just above the surface; count is bounded per chunk.
- The look (grass density, colour, sway, glow) is eyeballed via headless renders + a
  look-journal entry.

## Acceptance checklist
- [ ] Splat pipeline (instanced billboards, round mask, distance fade); shared with headless.
- [ ] Ground foliage scattered on grass; lush + visible in a headless render.
- [ ] Wind sway in the VS.
- [ ] `foliage` toggle; on-screen splat count bounded + shown on the HUD.
- [ ] Runs native + web; CI green; docs synced; look-journal entry.

> Status: **planned** ⏳ — the splat-pipeline foundation + ground foliage.
