# brickmap — Design

> Status: **living document**, seeded at project kickoff. Decisions here are the
> agreed defaults from the planning conversation. Where this doc resolves a
> previously-open question it says so and gives the rationale so it can be
> revisited with full context.

## 1. What this is

**brickmap** is a high-performance, cross-platform **voxel rendering engine**.
The name nods to the *brickmap* sparse-voxel brick-storage technique.

It is a **rendering** engine, not a game. The interesting problems here are
graphical: how to store, mesh, cull, and draw very large voxel worlds fast. This
is a personal-interest project, so we favour **interesting and correct**
approaches over shipping speed.

## 2. Goals

- Render large voxel worlds at a smooth, stable frame rate on **weak hardware**.
- One codebase, many platforms: **PC (Win/macOS/Linux) + mobile + web**.
- A world representation clean enough that the *renderer* could be swapped later
  (e.g. for a ray-marched experiment) without rewriting the world model — but we
  do **not** build that abstraction now.
- Code that teaches: clear module boundaries, documented tradeoffs.
- A **distinct, emergent visual identity** that is the honest signature of the
  techniques we use — not a borrowed look. (See §12; this is a first-class goal,
  not decoration.)

## 3. Non-goals (for now)

- Gameplay, physics, networking, entities, AI, persistence-as-a-feature.
- **Photorealism / AAA mimicry.** We borrow Minecraft's *performance spine*, not
  its look, and we are explicitly not chasing a UE5-style render (§12).
- **Retro pastiche.** We are equally not building an SNES/PS1 nostalgia filter.
  "Low-fi" here means *exposing the substrate*, not impersonating old hardware (§12).
- **The stock-engine / tech-demo look.** The appearance must not read as "the
  default output of a tool we built" (untextured cubes, debug grids) (§12).
- A ray-traced / path-traced voxel renderer. (See §6 — explicitly rejected as
  the primary paradigm because it punishes our target hardware.)
- Editor tooling, asset pipelines beyond what a spike needs.
- Squeezing the absolute last frame out of high-end discrete GPUs.

## 4. North-star priority

> **Performance on weak hardware first.** It must feel great on an integrated GPU
> (no dedicated card) and a mid-range phone. Every tradeoff is decided by this.

The dominant constraint on this hardware is **memory bandwidth**, not raw ALU
throughput. Integrated and mobile GPUs share bandwidth with the CPU and have
small caches and (on mobile) tile memory. So most of our performance pillars are
about *moving less data*, not *computing less*. Keep this framing in mind when
reading §7.

### Web is explicitly lower priority

Cross-platform includes the browser, but **web never dictates the design**. If
web ever forces a compromise that hurts native performance, web gets dropped. It
would be a shame to lose, so we keep it *cheap to retain* (this is most of why we
chose wgpu — see §5) but we never let it steer.

## 5. Locked technical decisions

These came out of planning and are treated as settled defaults.

| Decision | Choice | Rationale |
|---|---|---|
| **Language** | **Rust** | Voxel/graphics work is flat arrays + tight loops — Rust's sweet spot. Memory safety without a GC matters for a bandwidth-bound, allocation-sensitive engine. (Author is new to Rust but coming from JS; this domain is friendly to learn in.) |
| **Graphics API** | **wgpu** | One renderer targets Vulkan / Metal / DX12 / GLES natively **and** WebGPU/WebGL in the browser. This is what makes web *nearly free* while keeping native uncompromised. |
| **Math** | **glam** | Fast, SIMD-friendly, the de-facto Rust gamedev math crate. |
| **Windowing** | **winit** | Cross-platform window + input + event loop, including web canvas + mobile. |
| **Rendering paradigm** | **Rasterized meshing** (not ray-marched voxels) | Ray-marching is fragment-/bandwidth-hungry and murders integrated/mobile GPUs. Rasterizing greedily-meshed chunks is the proven, hardware-friendly path. |
| **Lighting/pipeline** | **Forward (→ forward+)**, not deferred | Mid-range mobile GPUs are tile-based deferred (TBDR). A fat deferred G-buffer is bandwidth-hostile and fights tile memory. Forward keeps render-target switches minimal. Baked in early because reversing it later is expensive. |

### Escape hatches (deliberately not used yet)

- **Native max-perf:** if we ever need to beat wgpu's abstraction overhead on one
  native backend, drop to `ash` (raw Vulkan) for that backend only. Not now.
- **Renderer swap:** keep the world-data ↔ renderer boundary clean (§ architecture)
  so an alternative renderer is *possible*. Don't build the abstraction speculatively.

### Decisions re-confirmed after scaffolding

The kickoff spike (single cube, desktop + WASM, see `docs/spikes.md`) compiles and
runs through wgpu 29 on both targets with one code path. Nothing about the
Rust / wgpu / rasterized-meshing / forward-rendering set fought the perf goal
during scaffolding, so all of §5 stands. Flags, if any, will be raised here.

## 6. Why not ray-marched voxels?

Ray/voxel marching (DDA through a brickmap/SVO in a fragment or compute shader)
produces gorgeous results and is conceptually clean, but its cost scales with
*screen pixels × steps-per-ray × memory latency per step*. On bandwidth-starved
integrated/mobile GPUs that is exactly the wrong cost model. Rasterized meshing
moves the heavy lifting to **infrequent** CPU-side meshing and lets the GPU do
what its fixed-function hardware is fastest at. We borrow Minecraft's spine
because it is *empirically* light on weak hardware.

We keep the door open (clean world model) but do not invest in it.

## 7. Performance pillars

All in service of §4. Most are bandwidth plays.

1. **Binary greedy meshing** — merge coplanar voxel faces into large quads,
   slashing vertex and triangle counts (the biggest single win).
2. **Compressed vertices** — pack a face vertex into ~4–8 bytes (chunk-local
   position, face/normal, ambient occlusion, texture id). Directly cuts the
   per-frame vertex bandwidth that dominates on memory-bound GPUs. (See §9.)
3. **Palette-compressed chunk storage** — Minecraft's per-section trick: store a
   small palette of distinct block types + tightly bit-packed indices. Cuts RAM
   *and* the bandwidth the mesher must read.
4. **Visibility-graph / "cave" occlusion culling** — flood-fill chunk-to-chunk
   connectivity and combine with frustum culling so hidden chunks are never
   drawn. This is the thing that actually makes Minecraft feel light.
5. **Chunk LOD** (octree mips) — bound distant triangle counts.
6. **Async meshing** off the main thread (rayon on native). This is the one place
   web genuinely suffers (constrained browser threading); acceptable given web's
   low priority.

## 8. Target hardware & frame-time budgets

> Resolves open question: *which reference integrated GPU + which reference phone?*

We pin two concrete reference devices so "fast on weak hardware" is measurable.
These are commodity, widely-owned, and squarely "weak" by 2024+ standards.

### Reference: integrated desktop/laptop GPU
- **Intel Iris Xe Graphics** (96 EU, Tiger/Alder Lake class, ~2020+).
- Secondary sanity check: **AMD Radeon 660M / Vega 8** iGPU.
- Reference resolution: **1920×1080** windowed.

### Reference: mid-range phone
- **Pixel 6a class** — ARM **Mali-G78** (Google Tensor), or equivalently a
  **Snapdragon 7-series (Adreno 6xx/7xx)** device.
- Reference resolution: render at **native panel ÷ ~1.3** (dynamic-resolution
  friendly), roughly 1080p-equivalent of work.

### Frame-time budgets

| Target | Frame budget | Goal | Hard floor |
|---|---|---|---|
| Reference iGPU @ 1080p | **16.6 ms** (60 FPS) | 60 FPS sustained, typical scene | never below 30 FPS (33 ms) in worst case |
| Reference phone | **16.6 ms** (60 FPS) | 60 FPS sustained at the dynamic-res target | 30 FPS floor; thermal-throttle aware |
| Desktop integrated, stretch | **8.3 ms** (120 FPS) | nice-to-have, not gating | — |

Within the 16.6 ms frame, an indicative GPU sub-budget to design against:
~10–11 ms draw, ≤2 ms post/UI, the rest as safety margin. CPU meshing is
**off the critical path** (async) and budgeted separately as "chunks meshed per
second under load" rather than per-frame ms.

These numbers are deliberately conservative starting targets; we tighten them
once the first real chunk renderer exists and we can profile on the reference
devices.

## 9. Texture / material approach

> Resolves open question: *texture array vs atlas.*

**Decision: 2D texture *array*** (one array layer per block texture), **not** a
packed atlas.

Rationale:
- **No bleeding.** Atlas tiles bleed into neighbours under mipmapping and
  anisotropic filtering; you fight it forever with padding and clamped UVs. Array
  layers have clean edges by construction.
- **Real mipmaps + aniso** work naturally per-layer — important for distant
  chunks and the LOD pillar (§7.5), and a bandwidth win (smaller mips sampled far
  away).
- **Cheap vertex packing.** A face just carries a **texture id = array layer
  index** (an integer), no per-vertex UV rectangle math. This is what lets the
  compressed-vertex budget (§7.2) stay tiny.
- **Portability is fine.** WebGL2 and all native backends support 2D texture
  arrays with ≥2048 layers — far more block types than we need near-term.

Constraints we accept: all tiles share one size (base **16×16**, configurable),
and total layers are capped by the backend (≥256 guaranteed everywhere, ≥2048
typical). If we ever exceed that, we shard into multiple arrays bound per draw
batch. Animated/variant textures become extra layers.

## 10. World extent & coordinate ranges

> Resolves open question: *world extent / coordinate ranges (affects chunk-local
> position bit budget).*

- **Section (chunk) size: 32×32×32 voxels.** A power of two that meshes well and
  matches the palette-section idea. (Minecraft uses 16³; 32³ amortises per-chunk
  overhead better and we are bandwidth- not occupancy-bound. Revisit if 32³
  meshing latency hurts the async budget.)
- **Vertical world extent (initial): 0..512** voxels = **16 stacked sections**.
  Generous for terrain; expandable later.
- **Horizontal extent: effectively unbounded** — chunk coordinates are `i32`
  along X/Z (±2.1 billion chunks), so the playable area is bounded by streaming
  and memory, not the coordinate type. World-space voxel positions use `i64` /
  `f64` for origin math; rendering uses a **floating origin** (camera-relative
  chunk offsets) to keep GPU-side coordinates small and precise.

### Resulting vertex position bit budget (§7.2)

Greedy-meshed **face** vertices sit on chunk-local *grid lines*, so each axis
spans `0..=32` — that's **33 distinct values → 6 bits/axis = 18 bits** for
position. Combined with face direction (**3 bits**, 6 faces), ambient occlusion
(**2 bits**, 4 levels), and a texture id (**~8–12 bits**, the array layer from
§9), a face vertex fits comfortably in **8 bytes (two `u32`s)**, with a path to
**4 bytes** once the texture-id range is known and AO/normal share the spare bits.
Exact packing is owned by the meshing/render contract and will be specced when
the mesher lands.

## 11. Visual identity — an *emergent* aesthetic

> This is a first-class design pillar, weighted alongside performance. It is also
> the part most likely to evolve; treat the *principles* here as committed and the
> specific examples as illustrative. The running record of concrete look decisions
> lives in the **[look journal](look-journal.md)** (started M4).

### The intent

brickmap should be **aesthetically opinionated**. The bar: if it ends up looking
like Minecraft or like a UE5 scene, we have failed — and equally, if it ends up
looking like an SNES/PS1 nostalgia parody, we have *also* failed. Both are
**borrowed, pre-planned looks** lifted from somewhere else. We don't want that.
We also don't want the third failure mode: the **stock-engine look** — the
untextured-cube, debug-grid, "this is obviously the default output of a renderer
someone just wrote" appearance. The look must not be tied to the default
appearance of the tools we built.

What we *do* want: a **low-fidelity** aesthetic in a specific sense —
**low-fi as in exposing the underlying technology**, not low-fi as in retro.
Rendering aberrations, banding, stray pixels, quantization, seams, sampling
crunch — the honest fingerprints of how the image is actually computed — are
surfaced and embraced rather than smoothed away. Think *truth to materials*,
where the "material" is the **algorithm and the hardware**.

Crucially, the look should **emerge from the technology**, not be decided up
front. We do not write a style guide and then bend the renderer to hit it. We
build the performance pillars (§7), observe the artifacts they *naturally*
produce, and then make aesthetic decisions by **choosing which of those artifacts
to keep, amplify, or tame** — composition over decoration.

### Why this fits the rest of the design (it's not in tension — it's a free win)

The weak-hardware-first techniques in §7 are *artifact generators*. Normally an
engine spends effort hiding their fingerprints; we spend that effort curating
them instead. So the cheap path and the beautiful path are the **same path** —
which is exactly the kind of correct-and-interesting coincidence this project
exists to chase. Notably, this means **we do not bolt on expensive
"beautifying" post-processing** to cover the tech up (that would fight both the
aesthetic *and* the bandwidth budget).

### Where the look is expected to emerge from (illustrative, not prescriptive)

| Performance tech (§7) | Artifact it naturally produces | How we might lean in |
|---|---|---|
| Compressed/quantized vertices (§7.2, 6-bit positions) | Positional snapping, sub-pixel wobble, affine-ish interpolation crunch | Let geometry visibly *quantize* instead of forcing smoothness |
| Palette-compressed storage (§7.3) | Hard, limited colour sets; banding | Treat the palette as an actual, visible palette — flat colour fields, deliberate banding |
| Binary greedy meshing (§7.1) | Large flat quads; visible merge seams; T-junction cracks | Keep the seams legible; the merged-quad structure becomes a texture in itself |
| Texture *arrays*, nearest sampling (§9) | Crisp, unfiltered texels; no smoothing | Point-sample by default; let texels be texels |
| Baked AO + low-bit normals (§7.2) | Stepped, quantized shading; 3-bit normal facets | Stepped/dithered lighting rather than smooth gradients |
| Chunk LOD / octree mips (§7.5) | Popping, geometric simplification at distance | Don't hide the pop; let distance dissolve detail honestly |
| Depth precision / z-fighting on weak GPUs | Shimmer, stray pixels at coplanar faces | Tolerate (and frame) the shimmer as part of the substrate |
| Forward shading, no AA by default | Aliasing, crawling edges | Embrace crisp aliased edges over a smeared anti-aliased mush |

These are **candidates**, surfaced as the relevant pillar lands (see the spike
backlog in `docs/spikes.md`). The kickoff spike deliberately makes **no**
aesthetic commitment — it is a smooth-shaded cube precisely *because* there is no
voxel technology yet for a look to emerge from. The identity is discovered, not
declared.

### Working rule

When a performance technique introduces a visible artifact, the default is **not**
"add code to hide it." The default is: **look at it, decide if it has character,
and if so make it intentional.** Suppression is a deliberate choice we justify,
not a reflex.

## 12. World content & visual destination (what we're building toward)

> The gap our earlier docs left: they were strong on *performance* and on an
> abstract *aesthetic* (§11), but silent on what the world is *made of* or *does* —
> i.e. what makes it worth looking at. Decided from a reference mood board.

Cubes are a placeholder, not the point. The destination is a voxel world that
feels **alive and atmospheric**, reached entirely through **cheap, rasterized,
artifact-embracing** means. Most "pretty voxel" demos online are pretty because of
*lighting* (global illumination, ray tracing) — which is exactly the cost we
refuse. We chase the **mood** (motion, glow, drama, depth), not the GI.

### Directions we're pursuing (fit weak-hardware-first + §11)
- **Dynamic / destructible voxels** — voxels that move, break, fall, flow, burn.
  Destruction shattering geometry into flying cubes; falling sand / fluids / fire
  as cellular automata on the world grid. This is the headline "interesting," and
  the reason the clean world↔mesh seam (architecture §4) and async meshing (M6)
  exist.
- **Particles** — instanced point-sprites/cubes for debris, embers, dust, sparks,
  motes. Cheap in a forward rasterizer; stray glowing pixels *are* the §11 look.
  The cheapest path to life on screen.
- **Sub-voxel surface displacement** — per-face relief (brick/grain/cobble) so
  blocks aren't flat, without shrinking voxels or adding triangles.
- **Faked atmosphere** — emissive palettes, stepped/dithered "fake light," fog and
  depth cueing. Drama without bounce lighting.

### Directions we're explicitly NOT taking
- **Global illumination / path tracing** (John-Lin-style organic GI) — gorgeous,
  but bought with the GPU cost we refuse (§4–6). We fake the mood instead.
- **Ray-marched / ray-traced voxels** (sparse-tree DDA) — rejected as the paradigm
  in §6; fragment/bandwidth-hungry, murders integrated/mobile GPUs.
- **Photoreal / soft-natural** looks — a non-goal (§3, §11). Low-fi by intent.

> If we ever decide a GI look is the *actual* dream, that reopens
> weak-hardware-first (§4) — a real pivot, not a tweak. For now it stays. These
> directions are queued as an **exploration track** in `docs/roadmap.md`,
> interleaved with the infrastructure ladder rather than deferred to the end.

## 13. Open items still to resolve (later)

- Lighting model details (flood-fill block light + sky light? baked AO only at
  first?). Forward(+) is locked; the light *data path* is not yet.
- LOD transition scheme (stitching vs skirts vs popping tolerance).
- Threading/job-system shape on native (rayon pool vs custom) and the web
  fallback (single-thread mesher vs Web Workers via `wasm-bindgen-rayon`).
- Concrete vertex bit-packing layout (pending mesher + final texture-id range).
- **Aesthetic curation method:** how we capture/compare artifacts as pillars land
  (a "look journal" of screenshots per spike?) so the emergent identity is steered
  on evidence, not vibes.
