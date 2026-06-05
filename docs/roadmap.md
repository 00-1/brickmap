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

This is a **single sequence, in execution order** — top to bottom is the order we
build. Most rungs are the standard voxel "performance spine" (necessary, shared
with every Minecraft-like). The **✨ exploration** rungs are what make brickmap its
own thing — the content/visual work from [`design.md`](design.md) §12 — interleaved
*in place* so they don't get deferred to the end (or lost).

> Legend: ✅ done · 🛠 in progress · ⏳ planned · **✨ = exploration** (the
> *interesting* bit, not infrastructure; numbered **E1–E5 in build order**) ·
> **D = dev tooling & process** (cross-cutting; see the D-series below). More
> exploration candidates (researched and fit-graded) live in
> [`exploration-backlog.md`](exploration-backlog.md).

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

### M2 — Greedy meshing + a grid of chunks ✅ &nbsp;→ [`milestones/M2-greedy-grid.md`](milestones/M2-greedy-grid.md)
The real **binary greedy mesher** (correctness tests + a `criterion` bench), a
multi-chunk manager, **frustum culling**, and the finalized **4–8 byte packed
vertex** (encode/decode round-trip tests).
- **Outcome:** fly through a grid of chunks rendered as merged greedy quads, with
  off-screen chunks frustum-culled.
- **De-risks:** the #1 performance pillar (meshing) and vertex compression.
- **Acceptance:** greedy output is verified correct vs the naïve mesher on shared
  fixtures; packed vertex round-trips for every field; a meshing throughput number
  is recorded against the design §8 budget.

### ✨ E1 — Aesthetic pass: expose the tech *(exploration)* ✅ &nbsp;→ [`milestones/E1-aesthetic-pass.md`](milestones/E1-aesthetic-pass.md)
**Vertex-quantization wobble** (render the compressed-vertex quantization *as* the
look) + **dithered shading/colour** (render the palette compression). The §11
thesis made literal — roughly a day's work for an immediate, distinctive face.
- **Outcome:** the world stops looking like a generic engine; the artifacts become
  the style.
- **Why here:** nearly free, and rides along with M2's packed vertex.

### ✨ E2 — Particles + destruction *(exploration)* ✅ &nbsp;→ [`milestones/E2-particles.md`](milestones/E2-particles.md)
Instanced point-sprite / cube **particles** in the forward pass, and **shattering**
a chunk into flying, fading, emissive cubes — the cheapest path to "alive on
screen" and a direct test of the §11 look (design §12).
- **Outcome:** disturb a chunk and it bursts into glowing debris that scatters and fades.
- **Why here:** needs M2's meshed chunks to shatter; placed *before* more
  infrastructure on purpose, so something distinctive lands early.

### M3 — A world to fly through ✅ &nbsp;→ [`milestones/M3-world.md`](milestones/M3-world.md)
**Palette-compressed** section storage, **procedural** (noise) terrain, and chunk
**streaming** around a travelling camera (runtime load/unload, bounded memory) — all done.
- **Outcome:** endless-feeling procedural terrain you fly across, streamed in around a
  cinematic travel camera.
- **De-risks:** storage RAM/bandwidth and the streaming model.
- **Acceptance:** palette pack/unpack round-trip tests; memory-per-section
  measured; chunks load/unload without leaks as the camera moves.

### M4 — It starts to look like brickmap ✅ &nbsp;→ [`milestones/M4-materials.md`](milestones/M4-materials.md)
**Texture-array materials** (per-face texture id, nearest sampling, mips), baked
**ambient occlusion** in the mesher, and the first real **aesthetic exploration** —
the [look journal](look-journal.md) (design §11): deliberately surfacing and curating
the artifacts the tech produces.
- **Outcome:** a textured, AO-shaded world with an emerging, opinionated look.
- **De-risks:** the material path and the first concrete aesthetic decisions.
- **Acceptance:** AO values tested in the mesher; texture-array path works on web
  (WebGL2) and native; a short look-journal entry captures what we kept and why.

### ✨ E3 — Light & atmosphere: cheap, no GI *(exploration)* ✅ &nbsp;→ [`milestones/E3-light-atmosphere.md`](milestones/E3-light-atmosphere.md)
Flood-fill block + **coloured emissive light** (a cheap *fake GI* — crystal light
bleeds around the terrain folds), hemispheric ambient, a sky/horizon gradient, bloom,
and distance fog. The highest beauty-per-cycle on the backlog
([`exploration-backlog.md`](exploration-backlog.md) §C).
- **Distance fog landed early** (pulled forward to hide the M3 streaming load edge);
  the rest — ambient, sky, bloom, emissive crystals, flood-fill light — landed as
  slices 1–4. Deferred: cross-chunk light, day–night shift, view-ray sky.
- **Outcome:** mood and glow — what makes "pretty voxel" demos pretty, faked without
  ray tracing.
- **Why here:** builds on M4's materials, and implements the "lighting data path"
  that M8 only hand-waved.

### ✨ E4 — Sub-voxel surface displacement *(exploration)* ✅ &nbsp;→ [`milestones/E4-displacement.md`](milestones/E4-displacement.md)
Per-face relief (brick / grain / cobble) so blocks aren't flat — richness with no
extra geometry (design §12). Cheap **bump** relief (gradient of the detail texture
perturbs the lit normal), not per-fragment parallax marching (the §E cost risk).
- **Outcome:** surfaces gain depth and texture without shrinking the voxels.
- **Why here:** pairs naturally with M4's material/texture path.

### M5 — Light to draw (occlusion culling) ✅ &nbsp;→ [`milestones/M5-culling-hud.md`](milestones/M5-culling-hud.md)
**Visibility-graph / "cave" culling** (flood-fill chunk connectivity) combined
with frustum culling, plus an on-screen **frame-time / draw-call / triangle HUD**.
(Cave-cull is a no-op on today's surface world seen from above — proven in tests;
it bites with 3D terrain. The HUD makes frustum culling's reduction visible now.)
- **Outcome:** large worlds stay cheap; hidden regions aren't drawn.
- **De-risks:** the pillar that actually makes the world feel light.
- **Acceptance:** visibility-graph logic tested; HUD shows a measurable draw-call
  reduction on a cave-heavy scene.

### M6 — Off the main thread (async meshing) ✅ &nbsp;→ [`milestones/M6-async-meshing.md`](milestones/M6-async-meshing.md)
A **rayon** job system that meshes off the critical path; the main thread only
dispatches jobs + uploads finished meshes. Web falls back to time-sliced main-thread
meshing (no `wasm-bindgen-rayon` — Pages can't set the COOP/COEP headers it needs).
- **Outcome:** no frame hitches while streaming/meshing.
- **De-risks:** threading, and the known web threading constraint.
- **Acceptance:** meshing never blocks a frame on native; web still functions
  (slower meshing accepted).

### ✨ E5 — Dynamic / cellular-automata voxels *(exploration)* ✅ &nbsp;→ [`milestones/E5-dynamic-voxels.md`](milestones/E5-dynamic-voxels.md)
Falling sand / fluid / fire on the world grid, re-meshing only the dirty regions.
The headline "interesting" — voxels that *behave* (design §12). **v1: falling sand** —
seeded ahead of the flight, settling on the terrain; `sand` toggle. Water/fire are the
same engine with more rules (deferred).
- **Outcome:** matter that flows, falls, and burns in real time.
- **Why here:** needs a real world to disturb (M3) and async meshing (M6) to stay
  smooth. (A cheap prototype could be spiked sooner if we're itching for it.)

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

## Dev tooling & process (D-series)

Cross-cutting tooling that supports the work — done *as needed*, not in the linear
build order.

### D1 — Headless render-to-PNG ✅ &nbsp;→ [`milestones/D1-headless-render.md`](milestones/D1-headless-render.md)
Render the scene offscreen to a PNG using software Vulkan (**llvmpipe** — confirmed
working in-container, no GPU/display needed), so renders can be verified without a
display.
- **Outcome:** a `screenshot` tool that captures the live scene to an image.
- **Unlocks:** golden-image regression tests, and *supervised* autonomous runs
  (Claude can sanity-check its own renders instead of working blind).

### D2 — Adjustable params (live dials) ✅
Aesthetic params (wobble, dither) adjustable at runtime — web sliders driving a
params uniform via `#[wasm_bindgen]` setters, instead of recompiling. Reusable
mechanism; particle/gravity/spawn dials can join later.

### D3 — Auto-fly + mobile-friendly viewing ✅
A default **auto-fly orbit** so the build is watchable with no keyboard/mouse (mobile,
or just hands-off). Manual input (click/WASD) takes over; `F` resumes the orbit.

### D4 — Native Android app (+ auto-update) ⏳ &nbsp;*(partly blocked on the human)*
A real native Android app (not a web wrapper): winit `android-activity` entry point +
NDK cross-build + a CI APK job. **Two halves:**
- **Build** (I can do, autonomously): the Android entry point + a CI job producing a
  signed APK. Caveat: I can't test it here (no Android device/emulator), so it'd be
  built blind.
- **Auto-update on push** (needs the human's accounts/keys): a bare sideloaded APK
  *cannot* auto-update. The realistic channels are **Google Play internal-testing
  track** (true background auto-update; needs a $25 Play account + signing key + CI
  publish) or **Firebase App Distribution** (tap-to-update; needs a Firebase project).
  A self-hosted-APK + in-app updater is possible but Android still prompts per install.

**Status/decision:** deferred. The auto-update half depends on the human picking and
setting up a distribution channel; until then the **auto-updating web build is the
mobile preview**. Revisit when native *performance* is the goal and the human is ready
to stand up a Play/Firebase channel.

### D5 — Web-render verification (headless browser) ⏳
Verify the *actual* deployed web build (WebGL2 fallback + JS/wasm integration), not
just the WebGPU path, by driving a headless **Chromium against a locally-served build**
(localhost avoids the sandbox's network block; Chromium can use llvmpipe/swiftshader).
Lower priority — the native llvmpipe render already covers WebGPU faithfully.

### D6 — Feature toggles (live A/B) ✅
Live on/off switches for renderer features so we can A/B their cost (with the M5 HUD)
and their look: frustum cull, cave cull, sky, sparks, bloom, fog, AO, block light,
emissive glow. Number keys `1`–`9` on native; checkboxes on the web; the HUD shows
what's off. **Norm going forward:** new visual/perf features land with a toggle where
it's a cheap runtime branch. *(Not toggled: structural choices baked into data layout —
packed vertices, palette storage — which get dedicated benchmarks instead. A
greedy↔naïve mesh switch needs a re-mesh; deferred.)*

## What we're deliberately *not* doing

Discarded because they fight weak-hardware-first / §6 / §11 (see design §12):
global illumination & path tracing, ray-marched/ray-traced voxels, photoreal
looks. We chase the *mood* of those references with cheap fakes, not their
techniques.

## Notes on ordering

- **Greedy meshing (M2) precedes procedural generation (M3)** on purpose: we
  perfect and test the mesher on cheap, hand-built, *known* chunks before a noisy
  world makes bugs hard to read.
- The ladder doubles as the **crate-split schedule** from `architecture.md`: M1
  carves out `world` / `mesh` / `render` / `scene` as modules; later milestones
  promote them to workspace crates once the boundaries have proven stable.
- Milestones are sequential by dependency, but small independent improvements
  (input polish, HUD tweaks) can slip in between without a brief.
