# brickmap — Roadmap

The source of truth for *where we're going and in what order*. Read
[`design.md`](design.md) for the why, [`architecture.md`](architecture.md) for the
structure, and [`development.md`](development.md) for how we work.

## How we plan (two layers)

Heavy planning, without the waterfall trap of speccing things we'll only
understand later.

1. **This roadmap — written in full, kept living.** The milestone ladder: each
   milestone's *goal*, its *demonstrable outcome*, what it *de-risks*, and its
   *acceptance criteria*. This is the map. It changes rarely.
2. **Milestone briefs — written just before we build each one.** A focused
   `docs/milestones/Mx-*.md` with scope, data-structure/algorithm sketch, tests,
   the decisions to resolve, risks, and the demo. Written when we actually have
   the context to write it well — because downstream details genuinely depend on
   upstream learnings (real perf numbers, and the *emergent* visual identity we've
   deliberately refused to pre-design).

We plan the skeleton completely now; we flesh each rung just before climbing it.

## Definition of done (every milestone)

A milestone is done when **all** hold:

- The demonstrable outcome works and is visible in the **live preview**.
- Pure logic introduced is **tested** (see `development.md` testing strategy).
- **CI is green** (fmt, clippy `-D warnings`, tests, wasm build).
- Docs touched by the change are updated; the milestone brief's acceptance
  checklist is fully ticked.
- It runs on **native and web** (web may be slower; never broken).

## Milestone brief template

Each `docs/milestones/Mx-*.md` follows this shape:

```
# Mx — <title>
Goal · Demonstrable outcome · What it de-risks
Scope: in / explicitly out
Design sketch: data structures, algorithms, module placement
Decisions to resolve (with a recommended default)
Tests
Risks & mitigations
Acceptance checklist
```

## The ladder

> Status legend: ✅ done · 🛠 in progress · ⏳ planned

### M0 — Foundation & rig ✅
Planning docs, the cross-platform render spike (one cube, desktop + web), CI, and
an auto-deploying GitHub Pages preview.
- **Outcome:** a spinning cube in the browser and on desktop from one code path.
- **De-risked:** the "web is nearly free" premise; the whole toolchain/preview loop.

### M1 — One real chunk on screen ✅ &nbsp;→ [`milestones/M1-one-chunk.md`](milestones/M1-one-chunk.md)
A voxel data model (`Section` 32³), a *naïve* face-culling mesher, the
`ChunkMesh` world↔render contract, and a fly camera + input.
- **Outcome:** fly around a single hand-built 32³ chunk (a stepped pyramid); faces
  between solid voxels are culled, boundary faces are drawn.
- **De-risks:** the core data → mesh → GPU pipeline and the sacred module seam.
- **Delivered:** `world`/`mesh`/`scene` modules with unit tests, a naïve mesher
  feeding the renderer via the `ChunkMesh` contract, and a pointer-lock fly
  camera — on native and web. Snapshots: `/archive/01-first-chunk/` (auto-orbit)
  and `/archive/02-fly-camera/`.

### M2 — Greedy meshing + a grid of chunks 🛠 &nbsp;→ [`milestones/M2-greedy-grid.md`](milestones/M2-greedy-grid.md)
The real **binary greedy mesher** (correctness tests + a `criterion` bench), a
multi-chunk manager, **frustum culling**, and the finalized **4–8 byte packed
vertex** (encode/decode round-trip tests).
- **Outcome:** fly through a grid of chunks rendered as merged greedy quads, with
  off-screen chunks frustum-culled.
- **De-risks:** the #1 performance pillar (meshing) and vertex compression.
- **Acceptance:** greedy output is verified correct vs the naïve mesher on shared
  fixtures; packed vertex round-trips for every field; a meshing throughput number
  is recorded against the design §8 budget.

### M3 — A world to fly through ⏳
**Palette-compressed** section storage, **procedural** (noise) terrain, and chunk
**streaming** around the camera (load/unload; synchronous is fine here).
- **Outcome:** endless-feeling procedural terrain you can fly across.
- **De-risks:** storage RAM/bandwidth and the streaming model.
- **Acceptance:** palette pack/unpack round-trip tests; memory-per-section
  measured; chunks load/unload without leaks as the camera moves.

### M4 — It starts to look like brickmap ⏳
**Texture-array materials** (per-face texture id, nearest sampling, mips), baked
**ambient occlusion** in the mesher, and the first real **aesthetic exploration** —
starting the "look journal" (design §11): deliberately surfacing and curating the
artifacts the tech produces.
- **Outcome:** a textured, AO-shaded world with an emerging, opinionated look.
- **De-risks:** the material path and the first concrete aesthetic decisions.
- **Acceptance:** AO values tested in the mesher; texture-array path works on web
  (WebGL2) and native; a short look-journal entry captures what we kept and why.

### M5 — Light to draw (occlusion culling) ⏳
**Visibility-graph / "cave" culling** (flood-fill chunk connectivity) combined
with frustum culling, plus an on-screen **frame-time / draw-call / triangle HUD**.
- **Outcome:** large worlds stay cheap; hidden regions aren't drawn.
- **De-risks:** the pillar that actually makes the world feel light.
- **Acceptance:** visibility-graph logic tested; HUD shows a measurable draw-call
  reduction on a cave-heavy scene.

### M6 — Off the main thread (async meshing) ⏳
A **rayon** job system that meshes off the critical path, with double-buffered GPU
uploads; plus the web fallback (single-thread or `wasm-bindgen-rayon` workers).
- **Outcome:** no frame hitches while streaming/meshing.
- **De-risks:** threading, and the known web threading constraint.
- **Acceptance:** meshing never blocks a frame on native; web still functions
  (slower meshing accepted).

### M7 — Distance is cheap (LOD) ⏳
**Octree-mip chunk LOD** with distance selection and transition handling.
- **Outcome:** long view distances with bounded triangle counts.
- **De-risks:** distant-geometry cost.
- **Acceptance:** triangle count stays bounded as view distance grows; LOD
  selection tested.

### M8 — Hit the budget (profiling & polish) ⏳
Profile on the **reference iGPU and phone** (design §8), tighten to the frame
budgets, wire the **lighting data path**, and add mobile **dynamic resolution**.
- **Outcome:** meets the stated frame budgets on real weak hardware.
- **De-risks:** turns "should be fast" into measured-fast.
- **Acceptance:** recorded frame times on both reference devices within budget;
  budgets in `design.md` §8 updated with real numbers.

## Exploration track — the *interesting* bit

The ladder above is mostly the standard voxel "performance spine" (necessary, but
shared with every Minecraft-like). These explorations are what make brickmap its
own thing — the **world content & visual destination** from [`design.md`](design.md)
§12. They **interleave** with the ladder (we don't defer all the fun to the end):
the first lands right after a lean M2.

- **E1 — Particles + destruction spike** (right after M2). Instanced point-sprite/
  cube particles in the forward pass; shatter a chunk into flying, fading, emissive
  cubes. Cheapest path to "alive on screen," and a direct test of the §11 look.
- **E2 — Dynamic / cellular-automata voxels.** Falling sand / fluid / fire on the
  world grid, with only dirty regions re-meshed. The headline "interesting";
  leans on async meshing (M6). Likely after M3 (a real world to disturb) and M6.
- **E3 — Sub-voxel surface displacement.** Per-face relief so blocks aren't flat,
  no extra geometry. Pairs naturally with materials (M4).

**Discarded** (don't fit weak-hardware-first / §6 / §11; see design §12): global
illumination & path tracing, ray-marched/ray-traced voxels, photoreal looks. We
chase the *mood* of those references with cheap fakes, not their techniques.

## Notes on ordering

- **Greedy meshing (M2) precedes procedural generation (M3)** on purpose: we
  perfect and test the mesher on cheap, hand-built, *known* chunks before a noisy
  world makes bugs hard to read.
- The ladder doubles as the **crate-split schedule** from `architecture.md`: M1
  carves out `world` / `mesh` / `render` / `scene` as modules; later milestones
  promote them to workspace crates once the boundaries have proven stable.
- Milestones are sequential by dependency, but small independent improvements
  (input polish, HUD tweaks) can slip in between without a brief.
