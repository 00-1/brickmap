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
> *interesting* bit, not infrastructure; numbered **E1–E18 in build order**) ·
> **D = dev tooling & process** (cross-cutting; see the D-series below). More
> exploration candidates (researched and fit-graded) live in
> [`exploration-backlog.md`](exploration-backlog.md).

> **Aesthetic identity (resolved 2026-06).** The look the project *found* — and committed to
> — is **dark, grimy, low-fi, "exposing the tech."** Its spine is the **configurable palette**
> (E10): the finished frame is mapped onto a small restrained ramp (20 curated 1–2-hue
> palettes) with **ordered (Bayer) dithering**, which at low internal resolution reads as a
> **halftone dot-screen**. It pairs with **deep shadows + a dim ambient floor**, an optional
> **sun-off, point-lit mood** (the world lit only by coloured emissive crystals), and a
> per-seed **doom-drone** (E16, "Sleep — *Dopesmoker*"). The palette + dither stack is what
> made the look click (it "saved the project aesthetically").
>
> **The lush point-cloud vegetation stays central** — ground foliage, undergrowth, and
> point-cloud trees (E6/E7) are very much part of the look, *within* the dark/murky brief.
> What changed is the **mood, not the content**: we still want a foliage-heavy, point-cloud
> world — just **dim and murky**, recoloured by the palette and lit into the dark, rather than
> the **bright, sunny forest** the earlier note targeted (capslpop / Superbien supplied the
> tech, not the brightness). Meshed cubes stay the near-field base; the shared **splat render
> path** (E6 foliage, E7 forest, M7 mesh→points dissolve;
> [`research-points-splatting.md`](research-points-splatting.md)) carries the foliage + points
> on top, where points win on weak hardware.
> **Treat the splat path + the palette/dither/lighting/doom-audio stack as the identity; new
> content (text, colossal bodies) plugs into it.**

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
- **Refined later (2026-06), folded into the E10 aesthetic spine:** deepened shadows + a
  dim ambient floor (wider light→dark range), a `sun` toggle (a dark **point-lit-only**
  mood), and **varied-colour emissive crystals** at higher density — so a sun-off world
  reads as scattered pools of coloured light.
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

### ✨ E6 — Splats & ground foliage *(exploration)* ✅ &nbsp;→ [`milestones/E6-foliage-splats.md`](milestones/E6-foliage-splats.md)
First rung of the **point-cloud / foliage aesthetic pivot** (see the note below). Stand
up the **splat render path** (instanced camera-facing billboard points) and use it for
**wind-swept ground foliage** scattered on the terrain — the world stops being bare
cubes. Grounded in [`research-points-splatting.md`](research-points-splatting.md).
- **Outcome:** a lush, alive grassy field of points that sways; toggleable.
- **De-risks:** the splat pipeline (look + cost on weak hardware) and bounding the
  on-screen splat count — both reused by everything later in the pivot.
- **Landed (look polish):** blade sizes vary far more widely (squared roll → mostly small
  wisps with the odd tall tuft), and every foliage splat carries a per-splat **`alpha`**
  rendered as **dithered transparency** (a screen-locked Bayer stipple in the fragment shader,
  so leaves read lacy/see-through with no blending or sorting). The Bayer threshold is shared
  with the distance-melt path.

### ✨ E7 — The forest & atmosphere *(exploration)* ✅ &nbsp;→ [`milestones/E7-forest-atmosphere.md`](milestones/E7-forest-atmosphere.md)
The *destination* look: **point-cloud trees**, layered vegetation, light shafts, glow,
and a lusher palette — the Superbien point-cloud-forest mood. Reuses E6's splat pipeline.
- **Outcome:** flying the world feels like the reference — drifts of glowing points
  forming a forest, atmospheric and alive.
- **De-risks:** density/perf at forest scale, and the aesthetic itself (heavy look-journal).

### ✨ E8 — A richer world *(exploration)* 🛠
Make exploring rewarding (and finally exercise M5's cave-culling). From the 2026-06
research (backlog §G): **ridged noise + domain warping** ✅ (landed — terrain flows in
valleys/spurs with ridge-line peaks instead of obvious axis-aligned fbm), **sea-level
water** ✅ (lakes/seas up to `SEA_LEVEL`, stylised in E9), **biomes** ✅ (temp×humidity →
materials + foliage density), **jittered-grid tree/foliage placement** ✅ (E7), **rivers** ✅
(ridged-crest channels carved toward the waterline → water-filled ponds/channels), and
**3D caves** ✅ (a thin band of a 3D noise hollows connected tunnels within the 32-tall
layer, keeping a solid floor + surface skin — finally giving M5's dormant cave-culling real
structure). Remaining: **vertical chunk stacks** (single→multi-layer streaming, for worlds
taller than one section + caves/overhangs spanning layers) — the bigger architectural step,
its own brief.
- **Outcome:** varied biomed terrain with cliffs, rivers, woods, and caves to fly through.
- **De-risks:** 3D-noise cost (gate to surface bands); unlocks the dormant cave-culling.
- ✅ **Biome auto-mode (2026-06, `src/biome.rs`)** — the **new default mode**: **every palette is a
  biome**, each a full preset (palette, spawn densities, lighting/wobble, ground amplitude, drone
  mix). A low-frequency field maps world position → biome and `biome::at` **blends the two nearest**
  so palette ramp + all scalars transition smoothly as you fly. Drives the palette (crossfaded
  ramp), wobble, a *float* sun amount, and the **drone vol/murk/heavy**; **spawn densities**
  (grass/trees/colossi/wisps/inscriptions) scale per-biome (always, worldgen-level); **structure-
  approach wobble** pulls the wobble to each colossus's own extreme as you near it; and a rare
  **ethereal variant** of any biome fades the ink grid on + turns the drone less deep. Toggle with
  key `G` / a web checkbox (which disables the manual controls it overrides). Pure parts tested;
  the look is live-verified. **Deferred:** biome-driven *ground amplitude* — it changes terrain for
  every seed (breaks share-link reproducibility + the E12 golden hash) and is capped within one
  section, so it's best paired with E8 vertical stacks + the human's eye (field + `amplitude` are
  already in the model, just not applied to `worldgen::height`).
- *All within-layer wins landed (warp/ridged/water/biomes/rivers/caves); only multi-layer
  vertical chunk stacks remain — the bigger architectural step (single→multi-layer
  streaming). Brief written: [`milestones/E8-vertical-stacks.md`](milestones/E8-vertical-stacks.md)
  (`ChunkCoord` is already 3D, so it's mostly removing the `cy == 0` / `y = 0` hardcodes +
  filling worldgen by absolute world-y + wiring the ±Y mesher neighbours).*

### ✨ E9 — Weather, water & sound *(exploration)* ⏳
From the research (backlog §H): a single **global weather state** (wind + gust + precip +
sun) driving everything; **screen-space god-rays**; **rain/snow as camera-space
particles** with **snow/wetness accumulation as a per-column shader blend**; **stylized
water** (sky-Fresnel + depth, no extra pass) + caustics; flat scrolling clouds + analytic
sky; and **procedural ambient audio** (wind/birds/water, web + native).
- **Outcome:** a living, weathered, audible world — the mood, coherent across systems.
- **De-risks:** coupling many effects to one cheap state; keeping water off the re-mesh path.

### ✨ E10 — Palette & aesthetic spine *(exploration)* 🛠
The aesthetic pass that **became the identity** (backlog §I; see the aesthetic note above).
- ✅ **Configurable palette post-process** — the finished frame is mapped onto a small
  restrained ramp by **luminance** (a tonal gradient-map, sRGB-correct), with **Bayer
  ordered dithering** to fake extra shades (even 2–3 colours read as a smooth/halftone
  gradient). **20 curated 1–2-hue palettes** (dark-leaning; two-hue ones pop the point
  lights in a complementary accent). In-engine + web controls (palette / colour-count /
  dither). *(This is a post-process map, not the originally-sketched render-to-index; the
  indexed-**storage** bandwidth win remains an optional perf refinement under M8.)*
- ✅ **Deep shadows + sun-off point-lighting + doom audio** — see E3 refinements and E16;
  together with the palette they *are* the look.
- ✅ **Deliberate low-res internal buffer (the "pixel-scale" dial)** — the scene + post chain
  render into a low-res internal buffer; the palette pass then *presents* it to the surface
  with **nearest** sampling, upscaling by the scale, so the dithered **halftone is intentional
  and cross-platform** (no longer only when a device upscales the canvas). The HUD stays
  full-res/crisp. Web `pixel` slider (1–6) + native key `K`; also the **biggest single perf
  dial** (fewer fragments). Verified headless (`screenshot … scale=N`).
- ✅ **"Ink" blueprint-grid outlines** (2026-06): a cheap **in-shader** voxel-edge overlay — the
  chunk fragment darkens thin lines along voxel boundaries (from the per-voxel world `uv` + face
  normal), so the cube lattice reads as drawn-on ink. No G-buffer/depth pass needed; fades with
  the fog so distant edges don't fizz. Opt-in `ink` toggle (default off) — native key `I`, web
  checkbox, share bit. Verified headless (close-up A/B).
- ⏳ **G-buffer-as-art** mode (normals/depth presented as the look) — deferred: the forward
  renderer keeps no normal buffer, so this needs a normal/world-pos target (a bigger change) and
  is lower value than the ink grid; parked as the remaining E10 idea.
- ✅ **Explored biome map (2026-06, `src/map.rs` + `map.wgsl`)** — a progressively-built world
  map: each streamed-in chunk is recorded with its biome colour, so the explored area fills in as
  you fly. A fullscreen overlay (gamepad **X** / key **N**) draws the chunk-image panned/zoomed in
  chunk space (biome colours with the field's natural transitions) + a **blinking you-are-here
  dot**; pan with the left stick / arrows while the world flies on underneath. GPU image rebuilt
  only when the explored set grows. `map.wgsl` validated via the headless render.
- **De-risks:** done — funnelling the frame through a tiny palette + dither *is* the look.

### ✨ E11 — More dynamic voxels *(exploration)* ⏳
Extend E5 onto a proper substrate (backlog §J): **block/Margolus CA** (free
mass-conservation + safe rayon parallelism) + **active-set/dirty-AABB** (Noita-style),
then **pressure water** (compressible-mass; rendered as a separate vertex-displaced pass,
*not* re-meshed), **fire/smoke/steam under one heat field**, the **destruction loop**
(explode→carve→eject→rest→write→slump), and **growth** (moss/vines/crystals).
- **Outcome:** water that finds its level, fire that spreads, things that grow and crumble.
- **De-risks:** re-mesh churn (active-set + off-thread mesher + don't-mesh-water absorb it).

### ✨ Features (E12–E18) — from the research passes (backlog §L–P) + 2026-06 ideas
User-facing features, all rated *great fit* because they reuse machinery we already have.
*(Build order is fluid. E12 ✅, E14/E16 🛠; E17–E18 are the new 2026-06 directions.)*
- **✨ E12 — Shareable seeds & permalinks** ✅ — runtime seed + a `share` codec (URL fragment
  `#s=…&…`, web + native); seed input / 🎲 random / 📅 seed-of-the-day / copy-link / copy-seed;
  restore on load; seed on the HUD. Golden voxel-hash test guards same-target determinism;
  cross-target (wasm-in-CI) check + the worldgen-versioning policy noted in the brief.
- **✨ E13 — Photo / cinematic mode** — pause + free-cam + FOV/roll/exposure, vignette/
  letterbox/hide-HUD, in-app screenshot (reuse the headless RTT path), Catmull-Rom camera
  paths (→ deterministic headless flythroughs/clips); photo-mode-only DoF.
- **✨ E14 — Creative / voxel editing** 🛠 — **command/event seam landed** (`edit::Edit` +
  `apply`, the multiplayer-untangle prep): DDA voxel pick ✅, place/break over the overlay
  via the sand dirty→re-mesh path ✅, undo log (inverse edits) ✅, native V/B/U keys ✅.
  *Deferred:* wireframe hover highlight, multi-voxel brushes, web mouse-picking, and
  **seed + sparse-delta** build sharing (ties to E12; the `Edit` log is the payload).
- **✨ E15 — Point-cloud creatures** ✅ — decorative life/motion. Abstract **drifting wisps**
  (`src/creatures.rs`): a seeded `Swarm` of wisps, each a wandering centre + an **organic** cluster
  of member points (scattered in a ball, not a ring) that tumbles + wobbles, leashed loosely to a
  focus (the camera), emitted as splats through the existing billboard pipeline. Pure + tested
  (determinism, stays near focus); verified headless. **Live-wired:** a small swarm tethered to
  the camera, advanced + re-uploaded every frame (`State::set_creature_points`), so motes drift
  and shimmer through the fly-through. The form can evolve to flocking boids / point-cloud critters.
- **✨ E16 — Reactive audio** 🛠 — seeded generative music + a weather/biome-reactive mix;
  `fundsp`+`kira` (native) / Web Audio (web); equalpower pan + one FDN reverb.
  **Direction: dark/heavy doom-drone (Sleep — *Dopesmoker*)** — slow, downtuned, crushing
  sustained drones, minor/Phrygian, sparse + hypnotic; per-seed dirge.
  - ✅ **Synth core** (`src/audio.rs`): dependency-free per-seed doom drone — stacked
    detuned oscillators (sub + body + power-chord fifth + faint Phrygian ♭2) → hard
    waveshaping → a slowly-swept resonant low-pass → slow amplitude swell. Verified by
    rendering to WAV (`cargo run --bin drone`) + spectral check (dark, sub-heavy, breathing).
  - ✅ **Playback I/O**: live in-game — Web Audio (ScriptProcessor pulling wasm synth blocks,
    autoplay-unlocked on first tap) + cpal (native) feeding `Drone::next_frame`. In-game
    controls: volume / heaviness / murk sliders (web) + mute key M (native); the dirge
    re-seeds with the world. (Android audio still a follow-up.)
  - 🛠 **Reactive layer**: ✅ **flight-reactive intensity** — the app feeds the camera's flight
    state (speed + altitude, blended, clamped `0..1`) into `Drone::set_intensity`, which opens the
    filter cutoff and lifts the swell a touch (smoothed per-sample, no zipper). Plumbed on native
    (a lock-free atomic, read each audio block) and web (set each frame). Pure mapping + bound/
    finite tested. *(Built blind — the modulation is conservative; the exact feel wants the human's
    ear to tune.)* ⏳ Remaining: biome/weather terms, a voice cap, and one FDN reverb.

### ✨ E17 — In-world text *(exploration)* ✅
Render text **inside the 3D world**, cheaply, by reusing the bitmap-font HUD rasteriser + the
splat billboard path: a string → a small texture → world-space quads (camera-facing).
**Real writing systems**, abstract content (decided 2026-06): inscriptions, markers, signage.
Lo-fi by construction; **emissive glyphs glow** + the palette + dither recolour and crumble them
like everything else. Cost: one tiny texture + a quad per label — negligible.
- ✅ **Capability** (`src/text.rs` + `text.wgsl`): a **script-aware** rasteriser → `WorldText`,
  camera-facing billboards drawn **in the scene pass** (depth-tested, emissive, palettised +
  fogged like the world). **Five writing systems:** Latin, Greek, Hiragana, **Standard Galactic**
  (mapped off its Private-Use codepoints so the Latin set can't shadow it), and a hand-authored
  **Runic** (Elder-Futhark-style) set — explicit `Script` selection so each renders its own glyphs.
- ✅ **Live placement (2026-06):** `structures::inscriptions_near` seed-scatters abstract
  inscriptions on a coarse grid (denser than colossi), composed deterministically — a script + a
  few glyphs + a glowing tint, **small and tethered just above a ground voxel** ("a few words on a
  voxel"). Streamed in/out around the camera and cached per cell (textures rebuilt only when the
  in-range set changes), pushed via `State::set_text_labels`. Verified headless (all five scripts).
- ✅ **Inscribed colossi (E17×E18):** each nearby colossus also gets a **monument label** at its
  base (`structures::colossus_label`, composed from the giant's own seed), so the fallen giants
  read as ancient *labelled* monuments. Merged into the same streamed label set (tagged so the two
  placement grids' cell keys can't conflate in change-detection).
- **Outcome:** glowing abstract inscriptions scattered in the dark world, on-aesthetic.
- **Substrate for an eventual in-engine UI** — moving the toggles/sliders off the DOM onto the
  same text path so the controls are identical on every platform (the long-promised "no DOM
  UI" step; today web uses HTML controls).

### ✨ E18 — Colossal structures *(exploration)* 🛠
Enormous seed-placed giants in the world (a "structure" layer, independent of chunk terrain),
in **ethereal point-forms** you drift through and **solid voxel** forms you explore. **Two
content kinds (pivot 2026-06):**
1. **Tube-tech relics** — *procedural*, the original "skeleton" generator reworked into wild
   non-human **tangles of tubes** ("ancient mechanical giant tech"; the limbs-everywhere look
   we found more interesting than a literal body). The main procedural structure.
2. **Human figures** — from real **CC0 models** (sourced: the MakeHuman base mesh, CC0 — see
   `assets/base-human.obj`), via a sampling/`voxelize` pipeline. A separate, later track.
- ✅ **Tube-tech relics, ethereal + placed** (`src/relic.rs`): the procedural generator,
  reworked from the humanoid "skeleton" into a wild **tangle of tubes** (hub girders + near-axis
  pipe runs with elbows + long spars, varied radii) — ancient mechanical giants, non-human, the
  "limbs everywhere" look. Surface cells → points reusing the splat pipeline (glow, palettise,
  fog, dissolve; drift-through). Seed-driven scatter + the live `structures::colossi_near`
  placement. Deterministic; tested. Verified headless: relics strewn across the world, plain +
  palettised.
- 🛠 **Human figures (CC0 model → points):** `src/model.rs` loads the CC0 base mesh
  (`assets/base-human.obj`, MakeHuman, 19k verts → ~37k tris), area-samples its surface to a
  point cloud, and topples + scales it to a giant lying on the terrain — rendered through the
  splat pipeline like the relics. Verified headless: a recognisable human point-figure. Tested
  (OBJ parse, sample count/bounds, rests-on-ground). ⏳ Next: solid voxelisation of the mesh +
  live placement; baking a compact asset (so it needn't ship the raw OBJ to the web).
- 🛠 **Solid / explorable kind, now live (built blind):** `relic::relic_voxels` →
  `relic_chunk_instances` greedy-meshes a relic into chunk instances; `gfx` draws them via a
  separate `structure_draws` list (terrain pipeline, out of the stream map). ~1 in 3 placed
  relics is solid (the rest ethereal). Verify the solid giants in-app.
- 🛠 **Live placement, cached + budgeted (built blind):** `structures::colossi_near` seed-places
  giants (ethereal + solid) on a coarse cell grid; the app **caches each giant's geometry per
  cell** and generates at most one new one per frame, so crossing cells no longer regenerates
  everything at once — the fix for the framerate hitch. Points are smaller/sparser now (coarser
  sampling), and **per-point jittered** off the grid so they read organic/natural rather than
  lattice-aligned, which also cuts the generation cost and overdraw. Verify feel/density/perf in-app.
- **Outcome:** drift through ghostly fallen giants strewn across the seeded world; later, land
  on a solid one and explore.
- **Perf:** a ~300-voxel-tall body is millions of surface voxels — only near chunks mesh, far
  chunks are cheap point sets / voxel mips (bandwidth-bound; chunk-LOD essential).
- **De-risk first** with a *procedural humanoid blockout* (capsule torso/limbs/head, no assets)
  to prove placement + the mesh→points dissolve before sourcing real anatomy.
- **Open decisions:** model source/licence (recommend MakeHuman CC0); content framing
  (environmental art / monument, not gore).

### ✨ E19 — Movement: walk + cruiser *(exploration)* ✅
A movement-mode system so the world is explored on foot *and* by ship, not just a fly-cam.
- ✅ **Walking** (`src/player.rs`): on-foot movement with **gravity**, falling, and an **animated
  auto-step** up ~1-block ledges (taller = a wall), collided against the **actual voxels** via
  `worldgen::solid_at` (shares the cave logic with `generate_section`; golden hash unchanged) — so
  you can walk *down into cave-mouths* and along cave floors, not just over the surface. Per-axis
  wall-slide; the camera controller drives the look, the `Walker` constrains the position.
  Unit-tested. *(Colossi aren't in `solid_at` yet → not collidable on foot.)*
- ✅ **Space cruiser** (`src/ship.rs` + `ship.wgsl`): a **polygonal** ship (hull/wings/fin/cockpit/
  engines) with glowing **nav-lights** (white-blue nose, amber tail, port-red/starboard-green
  wingtips), drawn in its **own pass after the palette** so its true colours + lights survive (not
  palettised), with its own depth buffer (self-occludes), shown parked while on foot. **Mode
  machine:** start piloting on **autopilot** (cinematic — now **wanders** to new terrain in
  S-curves, no longer a circle); toggle autopilot↔manual (F / pad A); land (within ~9 blocks of
  the ground) and press **E / pad B** to step out and **walk**; walk back to the ship to re-enter.
  Gamepad B/circle = enter/exit (native/web/android). HUD shows the mode; the map shows the parked
  cruiser (orange marker).
- **Outcome:** land the ship, get out, walk the terrain (down into cave-mouths), fly on.
- **Next ideas:** colossi collision on foot (walk inside the structures), a jump button,
  third-person cruiser cam, cruiser banking/physics.

### M7 — Distance dissolve (LOD that's also the look) 🛠
Reframed from "octree-mip LOD": instead of coarser distant *meshes* (crack-prone, low
value on a surface world), dissolve distant **terrain into points** via E6's splat
pipeline + a **dithered (Bayer) mesh→points crossfade** — bounding far cost *and*
sidestepping LOD seams, while extending the point-cloud look to the whole world.
- **Outcome:** long view distances with bounded cost; terrain melts into a pixel haze at
  the horizon (with the fog) instead of popping.
- **De-risks:** distant-geometry cost; the crossfade staying stable (no twinkle/crawl —
  use screen-locked Bayer, not blue noise).
- **Acceptance:** far cost bounded as view distance grows (HUD); crossfade is stable;
  dissolve logic tested. *(Octree-mip meshes remain a fallback if points lose the perf
  A/B on the reference iGPU.)*
- **Landed (look):** an opt-in `melt` toggle — terrain + foliage stipple into a pixel haze
  toward the horizon via the shared screen-locked Bayer threshold (default off; on-thesis).
- **Landed (look — ethereal recession):** point-rendered things (foliage, the misty point-colossi,
  wisps) **back away from the camera as you close in** — the splat VS pushes each splat outward in
  the horizontal plane (~6–16 blocks onset, up to ~3–7 blocks of drift), so you can never quite
  touch the dots; the misty giants part around you as you drift through. Driven by a **lagged
  camera** (an eased-behind position uploaded each frame) so points move out of the way **with
  inertia / at their own pace** rather than tracking you rigidly — the faster you fly, the lazier
  they drift out in your wake. A per-splat position hash **staggers** the onset radius + drift so
  the field reacts unevenly, not in lockstep. Computed from the fixed instance offset (no feedback);
  cheap. Tunable via `splat.wgsl` consts + the lag time-constant in `gfx.rs`.
- **Landed (LOD, for structures):** **solid relics dissolve mesh→dots by distance** (E18), done
  **in the shader**: a solid relic's mesh (origin.w flag) stipples out over `RELIC_LOD..+BAND`
  using the screen-stable Bayer threshold, so it crumbles gradually into sparse dots as it
  recedes (mesh near → point-cloud-ish far). Geometry is uploaded once per cell (no per-distance
  CPU switch) → the transition is smooth *and* causes no rebuild hitch. Tunable via the shader
  consts. A complementary points-fade-in (true cross-fade to the ethereal cloud) is the polish;
  terrain dissolve to points is still the deferred general case below.
- **Deferred (perf) → a general point-cloud render mode:** decimate distant chunks (and any
  voxel volume) into actual **point sets** so *primitives* drop with distance — today
  fragments are discarded but geometry isn't. Points are **sized camera-facing billboards**
  (any size; true 1px `PointList` points are size-locked in wgpu, so we use billboards),
  shrinking with distance — cheap when small/far, the killer being overdraw when big/near.
  This same mode is the **far-LOD substrate for E18 colossal bodies** (mesh-near, points-far)
  and the home for "clouds of small pixels".

### M8 — Hit the budget (perf systems + profiling) 🛠
Two halves. **(a) Engine perf systems** (doable now, no special hardware; from research
backlog §K): **vertex pooling + one shared static quad index buffer** (kills upload
churn, enables multi-draw), **further vertex quantization + quad-expansion**,
**render-pass load/store discipline** (`loadOp:clear`/depth `storeOp:discard` —
free tiler bandwidth), **dynamic resolution + FSR1/EASU** spatial upscale (biggest
mobile pixel win), **front-to-back opaque sort**, and **upload prioritization +
coalescing**; optional WebGPU `drawIndirect`/render-bundles + `WEBGL_multi_draw`.
**(b) Profiling** on the **reference iGPU and phone** (design §8): tighten to budgets,
wire the lighting data path, record real numbers. *(Skip: depth pre-pass, Hi-Z — see §K.)*
- **Outcome:** meets the stated frame budgets on real weak hardware.
- **De-risks:** turns "should be fast" into measured-fast.
- **Acceptance:** perf systems landed + measured on the HUD; recorded frame times on both
  reference devices within budget; budgets in `design.md` §8 updated with real numbers.
- **Landed (a, output-neutral):** front-to-back opaque sort (early-Z) + depth `storeOp:
  Discard` (free tiler bandwidth) — verified byte-identical headless. *(Shared static quad
  index buffer was evaluated and **rejected**: the greedy mesher flips each quad's diagonal
  by AO, so a fixed index pattern would regress the AO look.)*
- **Landed (a, streaming hitch):** the periodic chunk-crossing stall (web inline-meshed a whole
  ring at once — measured ~11 ms/chunk × 4 = ~43 ms spikes) is fixed by **(i)** a generated-
  section cache so each section is built once, not 5× as a neighbour, and **(ii)** a per-frame
  **time budget** on inline meshing so a ring spreads over frames instead of one freeze. (Native
  meshes off-thread already.)
- **Blocked (b):** profiling needs the reference iGPU + phone to measure — logged in
  `docs/unattended-questions.md` for when the hardware's available.

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

### D4 — Native Android app + downloadable APK ✅ &nbsp;*(sideload, no store; device-verified)*
A real native Android app (not a web wrapper), packaged as an **APK you download from
GitHub and sideload**. **Verified on a real phone (2026-06): installs, launches, and the
release build is fast** — the first real-hardware confirmation of the weak-hardware perf goal.
- **App:** ✅ winit `android-activity` entry point (`android_main` in `lib.rs`, behind
  `cfg(target_os = "android")`) reusing the existing wgpu/winit code path via a shared
  `run_event_loop`; logs to logcat via `android_logger`. *(Lifecycle robustness — surface
  recreate on suspend/resume — is a follow-up.)*
- **Build + publish:** ✅ `.github/workflows/android.yml` — NDK + cargo-apk,
  `cargo apk build --release --lib` (optimised), then **zipalign + apksigner re-sign
  (v1+v2+v3)** with a committed stable sideload keystore so it installs and updates in
  place. Published to a rolling **`dev` GitHub Release** → a direct, phone-friendly download
  (workflow artifacts can't be fetched on mobile). `v*` tags also publish versioned assets.
- **Input:** D7 digital controller (D-pad/buttons; analog sticks are a winit limitation).
- **Retired:** the initial debug-APK path (was only a pipeline-de-risking stepping stone).
- **Pairs with D7 (gamepad):** native + a USB-C controller is the intended "fly it on the
  phone" experience — better perf than the web build, and the controller works natively.
- **Honest caveat:** I **cannot test the APK here** (no Android device/emulator in the
  container), so it's built blind — the human installs + verifies on their phone, and we
  iterate from their reports.
- **Out of scope (still):** true background **auto-update** (re-download from GitHub, or
  a later optional in-app version check). Store-channel auto-update (Play internal track /
  Firebase) remains deferred and needs the human's accounts/keys.

**Status:** planned + actionable now (the auto-update requirement that blocked it is
dropped; downloadable-APK is achievable autonomously, modulo the blind-build caveat).

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

### D7 — Gamepad / controller controls 🛠
Fly the world with a **game controller** — the natural way to explore, especially on a
phone with a USB-C/Bluetooth pad.
- **Web:** ✅ poll the **Gamepad API** (`navigator.getGamepads()`) each frame (`gamepad::web`).
- **Native desktop:** ✅ **`gilrs`** (`gamepad::native`), behind a desktop-only Cargo cfg
  (excluded on Android to keep the APK build NDK-free; needs `libudev-dev` on Linux, now in
  CI). Both feed a normalised `PadInput` → the new analog `CameraController::add_move` +
  `add_look`.
- **Mapping:** left stick → move, right stick → look, bumpers → up/down, **A** → toggle
  auto-fly, **Y** → toggle the `melt` distance-dissolve (so the effect is easy to A/B with a pad
  in hand); deadzoned, analog (partial-stick = slower). Pure mapping + deadzone unit-tested.
- **Android-native pad:** **full analog** — since winit drains the Android input queue and
  drops the joystick axes, `android_main` drives the loop with `pump_app_events` and drains
  the input queue *itself first*, reading sticks + triggers from `MotionEvent` axes and
  buttons from `KeyEvent`s into the same `Pad`/`PadInput` path. Left stick moves, right
  stick looks, shoulders add turn, triggers go up/down, A toggles auto-fly. Touch is
  range-gated out so a screen tap can't fling the camera. *(Built blind — on-device tuning
  expected.)*
- **Outcome:** pick up a pad and fly; auto-fly yields to stick input like WASD does.
- **Status:** all three targets compile (native clippy+tests, wasm, android); **runtime is
  built blind** (no pad/browser/device here) — verified by the human with a controller.

### D8 — Downloadable desktop builds (Windows + Linux) 🛠
A native desktop binary you download and run — best perf + native controller (D7),
sibling to the D4 Android APK. The native binary already builds; this is the
**distribution**.
- **Build + publish:** ✅ `.github/workflows/desktop.yml` — a matrix builds `brickmap.exe`
  on `windows-latest` (wgpu → DX12/Vulkan) and a Linux binary on `ubuntu-latest`, publishes
  them as **SHA-named assets** (`brickmap-<sha>-windows-x86_64.exe` / `-linux-x86_64`) on the
  rolling **`dev` GitHub Release** — the *same scheme as the Android APK*, so builds are
  distinguishable (a per-platform prune keeps one current each) — plus downloadable workflow
  artifacts. Runs on main pushes + `workflow_dispatch` + `v*` tags. (macOS later if wanted.)
- **Outcome:** download a Windows `.exe` (the human is on Windows) and fly the world
  natively with a controller — no toolchain needed on their side.
- **Status:** Linux release binary build verified locally (11 MB); the Windows path is the
  same `cargo build --release` on `windows-latest` — **built blind, the human verifies the
  `.exe` runs**. Workflow lands; first artifacts produced via a manual dispatch.
- **Fit:** straightforward CI packaging (much simpler than D4 — no NDK/entry shim).

## Big future directions (beyond the ladder)

Major scope shifts we want to move *toward* — planned at the skeleton level, de-risked by
research before committing.

### N1 — Multiple viewers (lightweight multiplayer) ⏳ &nbsp;*(big; researched — stack chosen below)*
Shared exploration: several people flying the **same world**, seeing each other (and,
later, each other's edits and dynamic sim). A genuine scope shift — and the first thing
that needs a **server** (GitHub Pages is static).
- **Why it's *less* of a rewrite than it looks (the determinism dividend):** the world is a
  pure function of the seed, so it **never has to be synced** — every client regenerates it
  locally. Only **presence** (camera positions/avatars), **edits** (E14's seed+sparse
  deltas), and **dynamic-sim divergence** (sand/CA) need to travel. So **E12 shareable
  seeds and E14 edit-deltas are literally multiplayer's groundwork**, not a detour: a shared
  link already puts everyone in the same world; multiplayer adds a thin layer that
  broadcasts who's where + what changed.
- **The server question ("separation"):** we'd need a small **relay / signaling** server
  (a WebSocket relay, or WebRTC peer-to-peer + a signaling server) hosted *somewhere other
  than Pages* (tiny Rust service / a managed realtime service). This is an **added thin
  networking layer, not a client/server rewrite** — *provided* we keep the existing
  `world`/`worldgen`/`sim` ↔ `gfx` seam clean and keep edits/sim **serializable** (which
  E12/E14 already push us toward). That's the low-regret prep to do now; a full
  authoritative-server rewrite is **not** warranted yet (premature, fights weak-hardware
  solo-first).
- **Research conclusions (2026-06, 5-agent pass):**
  - **Authority model → host-authoritative hybrid, *not* lockstep.** Cross-platform float
    determinism (FMA/libm/transcendentals, wasm vs native) makes deterministic lockstep
    fragile — exactly E12's caveat — so don't sync the sim by stepping it in lockstep.
    Instead: terrain from the seed (never synced) + a **host-sequenced last-writer-wins edit
    log** + **unreliable presence** + **host-authoritative broadcast of active dirty-region
    sim** (sand/CA). Lockstep stays a documented future option *only* if the sim goes
    all-integer/fixed-point.
  - **Transport → `ewebsock` (WebSocket) + a tiny Rust WS relay, behind a `Transport`
    trait.** It's the one transport that spans wasm + native simply; `matchbox` (WebRTC P2P)
    is a later latency/bandwidth upgrade behind the same trait; **skip `ggrs`** (rollback-
    lockstep — wrong model for casual shared viewing). `wss://` works fine from static Pages
    (no CORS issue for sockets).
  - **Hosting → lowest-ops first.** Cloudflare Durable Objects / PartyKit (free tier, near-
    zero ops) or a small `axum` + tungstenite relay on Fly.io (~$2–5/mo). The relay only
    fans out messages + holds a room's edit-diff set; it is **not** an authoritative game
    server.
  - **Presence is the cheap, great-fit MVP.** 15 Hz **delta** presence + **entity
    interpolation** (render peers ~100 ms in the past) → smooth flight from infrequent
    packets. Avatars/cursors **reuse the splat pipeline** for ~free: splat-cluster avatar,
    colour-by-peer-id, a heading arrow, billboard name tags, and a **ghost-block cursor** at
    each peer's targeted cell. Render-side distance-culling of avatars reuses existing
    culling; **network-side interest management is unnecessary** at our scale (design for
    ≤8–16 in a room, full broadcast).
  - **Shared editing → defer, then keep minimal.** When we do it: **Figma-style
    last-writer-wins per cell** (object = cell, property = block id; relay defines order) —
    **no CRDTs, no locking**. Sync only **diffs from the seed baseline**; a late joiner
    replays the seed then receives the room's sparse `cell→block` set. Edits inject into the
    existing dirty-section → re-mesh path, so the renderer never learns multiplayer exists.
  - **Rooms/join → `?seed=…&room=…` link, no accounts, ephemeral.** A direct extension of
    E12 shareable seeds: the seed *is* the world; the room id is just the relay channel.
- **The low-regret prep to do now — a serializable command/event seam.** The one thing that
  gets *painful to untangle* later is leaving world mutations as scattered ad-hoc
  `Section::set` calls (today: sand seeding writes the overlay directly). Fix while the
  surface is tiny: route **every** mutation through one `enum Event` (`SetVoxel`, `Brush`,
  `SeedSand`, `Ignite`, `SimStep`) + a single `World::apply(&Event) -> dirty chunks`, all
  `serde`-derived; make the sim tick **logical (not wall-clock)** and iterate active chunks
  in **sorted order** (kills a latent desync/replay source); keep `seed + Vec<Event>` as the
  artifact. That one seam *is* the substrate for **undo/redo, replay, share-with-edits
  (E14), and broadcast (N1)** — they all become transport+ordering problems, not rewrites.
  Versioning gotcha to decide early: a share-link is `seed + deltas`, so freeze worldgen per
  a seed/worldgen version (or old links drift), and treat the `Event` enum as append-only
  with upcasters.
- **Stance:** research done — multiplayer is a **thin presence relay + sparse edit-diff
  log**, not a client/server rewrite, *provided* we land the event seam alongside **E11/E14**
  and keep edits/sim serializable. No netcode is written until presence is actually built;
  fold this guidance into the E11, E14, and (eventual) N1 milestone briefs.

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
