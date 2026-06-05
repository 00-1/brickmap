# brickmap — Exploration backlog (idea pool)

A curated pool of *interesting* feature/phenomena ideas for the **✨ exploration**
rungs in [`roadmap.md`](roadmap.md) — the things that make brickmap more than a
fast cube-terrain renderer. Sourced from a research pass over voxel engines, games,
and dev blogs, then filtered hard against our constraints:

- **Rasterized only** — no ray-traced/ray-marched *primary* renderer (design §6).
- **Cheap on weak, bandwidth-bound hardware** (integrated GPUs, mid phones).
- **Low-fi, "expose the tech"** aesthetic (§11) — embrace artifacts; no GI, no
  path tracing, no photoreal, no retro pastiche.

**Fit verdict legend:** 🟢 great fit · 🟡 possible (caveats) · 🔴 probably too
expensive / off-brand. Each idea: *what · why interesting · rough sketch · cost*.

> Honest note up front: two famous references are **ray-marched**, so their
> *renderers* are out of scope even though their ideas inspire us — **Teardown**
> (rasterizes each object's bounding box then ray-marches the voxels in-shader) and
> parts of **Vercidium/Sector's Edge** (DDA ray-march for *particle collision*).
> We borrow their *design* (destruction, instanced cube particles), not their
> renderer.

---

## A. Dynamic & simulated voxels — the headline (maps to ✨E5)

- 🟢 **3D falling-sand / cellular automata** (sand, water, fire, smoke, steam).
  *Why:* voxels that *behave* — the most distinctive direction, genuinely beyond
  Minecraft. *Sketch:* per-cell rules on the grid (à la Sandspiel/3DCellularWorld),
  step on a fixed tick, re-mesh only dirty sections. *Cost:* CA is cheap and
  embarrassingly parallel; the real cost is **re-meshing churn**, which is exactly
  what async meshing (M6) exists to absorb. This is already ✨E5.
- 🟢 **Fire / flammability spread** through flammable block types. *Why:* dramatic,
  emergent, pairs with emissive light + particles. *Sketch:* CA flag per block;
  ignite neighbours probabilistically; consume → ash/air. *Cost:* trivial.
- 🟡 **Growth automata** — vines, moss, fungus, *crystals* spreading over time
  (cf. John Lin's "crystal islands" look, reached cheaply). *Sketch:* slow CA that
  promotes air→solid along surfaces. *Cost:* cheap; mostly a re-mesh-rate question.
- 🟡 **Erosion / fluid that reshapes terrain.** *Why:* living world. *Sketch:*
  hydraulic CA. *Cost:* fine offline / slow-tick; heavier if real-time everywhere.

## B. Destruction & particles (maps to ✨E2)

- 🟢 **Shatter a chunk into instanced cube particles** with gravity, spin, fade,
  and emissive flashes. *Why:* the cheapest "alive on screen," and pure §11. *Sketch:*
  one instanced draw of unit cubes (per-instance pos/scale/color), integrated on
  CPU or in a compute/vertex step; no per-particle meshing. *Cost:* 🟢 instancing
  eats tens of thousands of cubes cheaply (Vercidium territory).
- 🟡 **Debris that settles back into the grid** (broken blocks become sand that
  falls and piles). *Why:* fuses B with A. *Sketch:* particles that, on rest,
  write a voxel back. *Cost:* cheap; couples particle + world systems.
- 🟡 **Particle vs. world collision.** *Sketch:* short DDA step per particle (the
  Vercidium method) — localized ray-march is fine; it's not the primary renderer.
  *Cost:* cheap per particle; skip entirely for pure ballistic debris.

## C. Cheap lighting & atmosphere — best bang-for-buck (maps to ✨E3)

> This cluster is where most "pretty voxel" demos actually get their beauty — and
> all of it is **fakeable without GI/ray tracing**. Strongly recommend promoting
> some of this from a design "open item" to real exploration work.

- 🟢 **Flood-fill block + sky light** (Minecraft's BFS): light spreads cell-to-cell,
  −1 per step; bake per-vertex, merge with greedy meshing. *Why:* glow, depth, mood
  — and **coloured** flood-fill is a cheap *fake GI* (light visibly bleeds around
  corners). *Cost:* 🟢 CPU BFS on dirty regions; near-free on the GPU.
- 🟢 **Smooth lighting / vertex AO** — darken corners from diagonal neighbours;
  greedy merges faces sharing an AO value. *Cost:* 🟢 classic, cheap. (Already on
  the M4 list — keep.)
- 🟢 **Emissive blocks + bloom** for neon/lava/crystal glow. *Cost:* 🟢 one bright
  channel + a cheap blur. Huge atmosphere-per-cycle.
- 🟢 **Day–night palette shift + distance fog.** *Cost:* 🟢 a few shader constants;
  fog also hides LOD pops (M7) and bounds draw distance. **Distance fog shipped early**
  (post-M3) to hide the streaming load edge — see roadmap E3; day–night still open.

## D. The "expose-the-tech" aesthetic — cheapest personality (maps to ✨E1, §11)

> Nobody does these *on purpose*; they're nearly free and they make the look ours.

- 🟢 **Vertex-position quantization wobble** (PS1-style affine jitter): snap clip
  positions to a coarse grid in the vertex shader. *Why:* literally renders the
  compressed-vertex quantization as the aesthetic. *Cost:* 🟢 a couple of shader ops.
- 🟢 **Dithered shading / colour reduction** (ordered/Bayer dither): quantize
  lighting and palette with visible dither instead of smooth gradients. *Why:*
  exposes palette compression; unmistakable signature. *Cost:* 🟢 a texture lookup.
- 🟢 **Keep greedy-quad seams and crisp aliased edges** (don't add AA). *Cost:* free.
- 🟡 **CRT/scanline post** — tempting but risks the *retro pastiche* we ruled out
  (§3). Use sparingly, if at all.

## E. Surface detail (maps to ✨E4)

- 🟡 **Sub-voxel displacement / relief / parallax-occlusion mapping** per face
  (Daniel Schroeder's "voxel displacement" look). *Why:* blocks stop being flat;
  rich without shrinking voxels. *Cost:* ⚠️ this is the one to watch — per-pixel
  relief mapping marches a few steps **in the fragment shader**, i.e. fragment/
  bandwidth cost on exactly our weak targets. Keep it cheap (shallow steps), LOD it
  by distance, make it toggleable. 🟡, not 🟢.
- 🟢 **Per-face damage / crack decals & block damage states.** *Cost:* 🟢 a texture
  swap; cheap and satisfying.

## F. Distinctive geometry & motion

- 🟢 **Vertex-shader life:** wind-sway foliage/grass, rippling water surfaces.
  *Why:* cheap motion makes a static world feel alive. *Cost:* 🟢 vertex displacement.
- 🟢 **Weather as particles** — rain/snow, with snow *accumulating* into voxels
  (ties to A/B). *Cost:* 🟢 instanced particles + occasional voxel writes.
- 🟡 **Hexagonal / rhombic voxels** instead of cubes. *Why:* instantly not-Minecraft.
  *Cost:* 🟡 a real world-model + mesher change; big commitment for a look.
- 🔴 **Marching-cubes / smooth surfaces.** *Why:* organic. *But:* fights the cube
  aesthetic *and* costs more; off-brand. Discard for now.

---

## Shortlist — slotted into the roadmap

1. **"Cheap lighting & atmosphere" (C) → now ✨E3** (after M4). Highest
   beauty-per-cycle on the list: flood-fill coloured light + emissive + bloom + fog.
2. **"Aesthetic pass" (D) → now ✨E1** (right after M2). Vertex wobble + dithering —
   a day's work, the §11 thesis made real; rides along with the packed vertex.
3. **✨E2 (particles+destruction)** and **✨E5 (dynamic voxels)** stay the headline
   content; fold **debris-settles-into-grid** in as the bridge between them.
4. **✨E4 (displacement)** stays 🟡 with a hard cost budget — the one most likely to
   bust the weak-hardware frame.

---

## Second research pass (2026-06) — curated additions

A 5-agent deep pass (after the engine reached E5) for *new* work beyond the above.
Full agent detail is summarized here; the points/splat sibling pass lives in
[`research-points-splatting.md`](research-points-splatting.md). Fit legend as above;
"→ rung" notes where it's slotted on the [roadmap](roadmap.md).

### G. World richness (→ **E8**, and unlocks M5 cave-culling)
- 🟢 **Ridged noise + domain warping** — `1-|noise|` ridgelines + `fbm(p + k·fbm(p))`
  warping; near-free, the biggest "less obviously-noise" look-per-cycle win.
- 🟢 **Biomes from temperature × humidity** (low-freq 2D maps → a param table for
  materials + **foliage density**) — cheap, and the backbone of forest *variety* (woods,
  clearings, treeline). Feeds E6/E7.
- 🟢 **Jittered-grid foliage/tree/POI placement** gated by biome+slope — this *is* E7's
  tree-scatter engine; cheaper + more natural than per-voxel noise.
- 🟢 **Sea level + rivers** (`|river_noise|<width` carves meanders) — lush riverbanks.
- 🟡 **Vertical chunk stacks + 3D density** (`noise3 − y·squash`) → **overhangs + caves**
  (cheese/spaghetti via noise intersection, *stateless*/deterministic). The pricey one
  (3D noise per voxel — gate to surface bands) but **foundational**: it's what finally
  makes the dormant cave-culling do work. 🔴 *Stateful "worm" caves* — breaks per-chunk
  determinism; use the noise-intersection form instead.

### H. Weather, water & ambient audio (→ **E9**; some atmosphere → E7)
- 🟢 **One global "weather state"** (wind vector + gust energy + precip + sun) driving
  god-rays, clouds, precip, water ripple, tree-sway, *and* audio — coherence for nearly
  free. The key unifying idea.
- 🟢 **Screen-space god-rays** (radial blur from the sun, ¼-res) — and through a *splat*
  canopy the gaps become hard flickering shafts: on-brand. (→ E7/E9)
- 🟢 **Snow/wetness accumulation as a per-column shader blend** (not new voxels) —
  height-splat snowline, world-reactive, nearly free; promote-to-voxel only past a depth
  threshold.
- 🟢 **Camera-space precipitation** (rain/snow particles, vertex-animated → WebGL2-safe).
- 🟢 **Stylized water** (sky-Fresnel + depth tint + scrolling normals + depth-foam; no
  2nd pass) + 🟢 **projected animated caustics**. 🟡 planar reflection (a 2nd scene pass —
  half-res, gate hard). 🔴 SSR (ray-march).
- 🟢 **Procedural ambient audio** (filtered-noise wind, pooled spatial bird one-shots,
  proximity water) — Web Audio on web, `kira`/`fundsp` native; cheap, world-reactive.
- 🟢 **Scrolling-noise flat clouds + hash stars + analytic (Hoffman–Preetham) sky** with
  **fog colour pulled from the sky** (aerial perspective ≈ free given our fog). 🔴 LUT
  multiscatter / volumetric ray-march clouds.

### I. Palette & aesthetic spine (→ **E10**)
- 🟢 **Indexed palette + colour cycling** — render to a scalar *index* buffer (R8, a
  bandwidth *win*), look up RGB from a tiny palette strip; quantization becomes
  structurally true; cycling animates water/lava/sky by rotating the palette. The
  deepest "expose the tech" idea; reskin the world by swapping one LUT.
- 🟢 **Depth/normal "ink" outlines** (Roberts cross) — voxel creases light up into a
  blueprint grid; ~6 taps; thresholds keyed off depth so it's a function of the pipeline.
- 🟢 **Deliberate low-res internal buffer + nearest upscale** — aesthetic foundation
  *and* the biggest perf dial; run all post at this res.
- 🟢 **Banded lighting folded into the palette index** (free); 🟢 **G-buffer-as-art** mode
  (6-colour normals / depth shells — nearly free, maximally on-theme).
- 🟡 **Edge-gated chromatic aberration**, 🟡 **halftone/cross-hatch** (alt identity, don't
  stack on the palette), 🟡 **cheap temporal feedback trails** (half-res — it's the one
  that spends scarce bandwidth). 🔴 full-screen radial CA / CRT / VHS pastiche.

### J. Dynamic-voxel follow-ups (→ **E5 continued**)
- 🟢 **Block/Margolus CA** as the sim substrate — free mass-conservation + trivially-safe
  rayon parallelism (disjoint 2×2×2 blocks).
- 🟢 **Active-set / dirty-AABB** (Noita-style: only tick moving regions) — *mandatory*
  infra; reuses our dirty-section path.
- 🟢 **Pressure water** via "compressible mass" (finds its level, no pressure solve) —
  **rendered as a separate vertex-displaced translucent pass, not in the chunk mesh** →
  zero re-mesh churn.
- 🟡 **Heat field** unifying fire/smoke/steam (slow tick, bounded). 🟡 **Destruction loop**
  (explode → carve ragged → eject particles → rest-detector → write voxels → CA slumps).
- 🟢 **Growth automata** (moss/vines/crystals, seconds-per-tick — high charm/cycle);
  🟡 erosion (leash to where water flows). 🔴 GPU-CA (WebGL2 has no compute; fights our
  CPU-owned world — keep CA on CPU/rayon).

### K. Perf systems for weak hardware (→ **M9**, feeds M8)
- 🟢 **Vertex pooling + one shared static quad index buffer** — buckets in a persistent
  buffer, no per-chunk index data; kills upload churn; enables multi-draw. #1 perf item.
- 🟢 **Render-pass load/store discipline** (`loadOp:clear`, depth `storeOp:discard` /
  `invalidateFramebuffer`) — free mobile/tiler bandwidth.
- 🟢 **Dynamic resolution + FSR1/EASU** spatial upscale (fragment-only, both APIs) —
  biggest *pixel*-bandwidth win on mobile; composes with E10's low-res buffer.
- 🟢 **Further vertex quantization + quad-expansion** (1 vertex/quad, expand in VS) —
  cuts vertex bandwidth (the named bottleneck).
- 🟢 **Front-to-back opaque sort** (near-free early-Z win) + 🟢 **upload prioritization by
  screen importance + write-coalescing**.
- 🟡 **`WEBGL_multi_draw`** (WebGL2 CPU savings, feature-detect) / **render bundles +
  `drawIndirect`** (WebGPU-only). 🟡 Basis/KTX2 **texture compression** (if the atlas
  grows). 🔴 depth pre-pass (redundant on tilers; we're not fragment-bound) / 🔴 Hi-Z
  occlusion (compute-heavy, WebGPU-only — cave-culling already covers it).

## Discarded (off-brand for us; see design §12)
Global illumination & path tracing; ray-traced / ray-marched *primary* voxel
renderers (incl. Teardown's renderer and sparse-tree DDA); photoreal/soft-natural
looks; heavy smooth-surface (marching cubes). We chase the *mood* of those with the
cheap fakes in C/D, not their techniques.

## Sources
- Cellular-automata voxels: [Sandspiel](https://sourceforge.net/projects/sandspiel.mirror/),
  [3DCellularWorld](https://github.com/ccrock4t/3DCellularWorld),
  [Liquid Voxels (Unity)](https://forum.unity.com/threads/liquid-voxels.242821/),
  [Real-Time Fluid + Voxel Engine (Springer)](https://link.springer.com/article/10.1007/s40869-016-0020-5).
- Destruction & particles: [Teardown breakdown](https://juandiegomontoya.github.io/teardown_breakdown.html),
  [Teardown design (Game Developer)](https://www.gamedeveloper.com/design/how-beautiful-voxels-laid-the-way-for-i-teardown-s-i-heist-y-framework),
  [Vercidium — particle optimisations](https://vercidium.com/blog/particle-optimisations/).
- Cheap lighting: [0 FPS — Voxel lighting](https://0fps.net/2018/02/21/voxel-lighting/),
  [Minecraft Wiki — Light](https://minecraft.wiki/w/Light).
- Surface detail: [Voxel Displacement Renderer (Daniel Schroeder)](https://blog.danielschroeder.me/blog/voxel-displacement-modernizing-retro-3d/),
  [80.lv coverage](https://80.lv/articles/modernizing-retro-3d-games-visuals-with-voxel-displacement-renderer).
- Meshing/perf/look: [Exile voxel pipeline](https://thenumb.at/Voxel-Meshing-in-Exile/),
  [Vertex pooling (Nick McD)](https://nickmcd.me/2021/04/04/high-performance-voxel-engine/),
  [Aokana — GPU-driven voxels (arXiv)](https://arxiv.org/pdf/2505.02017).
