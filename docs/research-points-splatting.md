# Research — voxels as points / splats (rasterized, weak hardware)

> A deep-research pass (5 parallel agents, ~25 sources) on rendering voxels as
> points/splats instead of meshed cubes, filtered against brickmap's constraints:
> **rasterized only, weak bandwidth-bound hardware (Iris Xe / mid phones), WebGPU +
> WebGL2, low-fi "expose the tech" aesthetic.** Date: 2026-06. Sources at the bottom.
> Caveat: the sandbox blocked `WebFetch`, so findings come from search extracts of the
> listed sources — specific perf magnitudes are directional, the mechanisms are solid.

## TL;DR

**Points-as-the-default renderer is the wrong call on our hardware** — but points as a
**far-LOD "dissolve" layer** and as **foliage/detail splats** are a great, distinctive
fit. The honest shape of the answer:

- **WebGPU points are 1px, period** (no `gl_PointSize`; no geometry shaders). WebGL2
  has `gl_PointSize` but only guarantees max = 1.0. So "points" in practice means
  **you emit your own quads** (vertex-shader expansion or instancing). You never escape
  the triangle pipeline.
- **Tiny primitives are taxed twice** on weak hardware: the **2×2 fragment-quad rule**
  wastes ~75% of shader lanes on a 1px primitive, and **tiled mobile GPUs** choke on
  the primitive count (binning + low primitive-setup rate; small-triangle workloads run
  ~10–20× slower than large ones). One quad per surface voxel maximizes exactly the
  thing weak GPUs are worst at.
- **Greedy meshing wins for foreground** precisely because it collapses big flat faces
  to 2 triangles — the opposite of a primitive flood. So keep the cube mesher as the
  base.
- **Points win in two regimes:** (1) **distant / sub-pixel** voxels, where meshing
  can't reduce noisy far detail and points' vertex count *drops* with distance; (2)
  **high-frequency detail greedy can't merge** — foliage, grass, scatter (this is
  literally what capslpop's "voxel splatting for animated foliage" targets).
- **The fast points path (compute `atomicMin` software rasterizer) is off-limits:** it
  needs compute + 64-bit atomics — absent in WebGL2, weak on iGPUs. The *rasterized*
  points path we can use is the slower one, which reinforces "layer, not default."

So: **don't replace cubes with points. Add points as a layer** — a distance-dissolve
LOD and/or a foliage/detail splat system — which is more distinctive *and* on-budget.

## What's a "great fit" (build on these)

- **Vertex-shader quad expansion** — expand one packed surface-voxel (~8 bytes) into 2
  triangles in the VS via `vertex_index`, no per-corner vertex data, one draw. Lowest
  bandwidth of all options. *(Equivalent: instanced billboard quads — same GPU cost;
  pick whichever fits the data layout.)*
- **Distance-scaled splat size tied to voxel spacing** — size each splat so neighbours
  just kiss → distant surfaces stay solid for free, one pass, no blending. (Or
  deliberately under-size to keep gaps for the look.)
- **Static per-chunk LOD** picking coarser splat sets at distance (constant *screen*
  density). We already have chunked streaming to hang this on.
- **Dithered (Bayer, screen-locked) alpha-test crossfade** for the mesh→points
  transition band: render both, `discard` a distance-dependent fraction of each via a
  4×4/8×8 Bayer threshold. Order-independent (alpha-*test*, writes depth), nearly free,
  and the regular dither reads as deliberate quantization — on-brand. **Use Bayer, not
  blue noise** (blue noise crawls/shimmers in motion without TAA, which we can't afford).
- **Flat-shaded splats** with a cheap round mask from the quad UV; no per-splat texture
  fetch. Cull sub-pixel splats; keep them ≥ a few px to dodge the quad tax.

## What to avoid (too heavy / wrong aesthetic / off-constraint)

- **EWA / surface splatting** — multi-pass, float-accumulation + per-pixel
  normalization; exists to make *smooth watertight* surfaces (the opposite of our look).
- **Pull-push / Jump-Flood hole-filling** — O(log res) full-screen passes; manufactures
  the smooth gap-free surface we don't want.
- **Depth-peel / deferred / sorted-OIT surfels** — extra passes / fat G-buffers /
  sorting; all attack the bandwidth bottleneck.
- **Compute `atomicMin` point rasterizer** — fast, but compute + 64-bit atomics →
  unavailable on WebGL2 / weak iGPU.
- **Per-fragment ray-box "splats"** (jojendersie-style) — that's ray-marching dressed
  as rasterization; off-constraint.
- **Per-particle simulated dissolve** — per-particle velocity/lifetime state + edge
  bloom across a whole scene is too much; take the *look* (threshold dissolve) not the sim.

## Recommended directions (ranked)

1. **Distance-dissolve LOD-as-look** — meshed cubes near → Bayer dithered crossfade band
   → splat/point cloud far (fading into the fog). Turns LOD into our signature *and*
   sidesteps the M7 crack problem (far chunks become points, no seams). Every piece is
   "great fit." **This is the headline.**
2. **Animated foliage / scatter splats** — grass, motes, foliage as cheap instanced/VS
   splats with vertex-shader wind sway. The capslpop direction; greedy meshing can't do
   this anyway, so it's points playing to their strength. Pairs with E5/atmosphere.
3. **Pure "points" render toggle** — the whole world as a 1px/few-px splat field, as a
   deliberate low-fi *art mode* (accept it's a stylistic toggle, slower than meshed, not
   the perf default). Cheap to add once #1's splat pipeline exists. (D6 toggle.)

The splat pipeline (VS-expanded, packed surface voxels, distance-sized, Bayer-dither
crossfade, flat shade) is the shared foundation for all three — build it once for #1 and
#2 and #3 fall out.

## Falsifiable things to test in a spike
- Does the far cloud **twinkle** when moving? → stabilise the per-point hash; screen-lock
  the dither; don't dither in world space.
- Does the crossfade band **double overdraw** enough to hurt? → keep the band narrow.
- Do **holes** open in the far cloud? → splat size must match the decimation spacing.
- Is the splat count actually a **win vs meshing** at distance on the iGPU? → measure on
  the HUD; meshing may still win until voxels go sub-pixel.

## Sources
Point/splat techniques & WebGPU points: webgpufundamentals (points), gpuweb issues
#332/#1190/#263, okaydev WebGPU pt4, webgl2fundamentals cross-platform. Splatting &
hole-filling: Harvard/CMU/ETH EWA pages, Zwicker SIG01, Twinklebear webgl-ewa-splatter,
Magnopus "extremely large point clouds", Potree/QSplat. Engines/devs: Vercidium (voxel +
particle optimisation), Schütz compute_rasterizer + CLOD (ieeevr_2019), Euclideon
(codersnotes), Teardown (acko/Montoya — ray-marched, OUT), John Lin (path-traced, OUT),
Daniel Schroeder voxel-displacement, Distant Horizons (MC LOD), capslpop "voxel splatting
for animated foliage". Dissolve/LOD: Unity LOD cross-fade, Unreal dithered LOD, Wyman &
McGuire hashed alpha (i3d17), Bayer-vs-blue-noise, Codrops dissolve. Perf: 2×2 quad rule
(selfshadow "Counting Quads", Unigine quad-overdraw, Stanford fragmerging), tiled-GPU
binning (Meta/Oculus, Wikipedia tiled rendering), g-truc small-triangle, nickmcd voxel
engine.
