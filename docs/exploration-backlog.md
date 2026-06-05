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
