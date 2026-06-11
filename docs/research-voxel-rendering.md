# Research — voxel & point rendering practice (engine)

> 2026-06-11 research pass (web). The practitioner/literature state of the art for a
> rasterized, bandwidth-bound, weak-hardware voxel engine — meshing, occlusion, streaming/
> upload. Feeds [`performance.md`](performance.md) and future M-series briefs. Sibling of
> [`research-points-splatting.md`](research-points-splatting.md) and
> [`research-gpu-perf.md`](research-gpu-perf.md). Verification key: items marked [S] were
> corroborated via search excerpts (origin pages bot-blocked); the rest verified against
> repos/primary sources.

## 1. Meshing — binary greedy meshing is the current bar

- **cgerikj/binary-greedy-meshing** (C++, MIT): 64³-padded chunks meshed in **50–200 µs**
  (74 µs single-threaded on a Ryzen 3800X) via per-axis u64 column bitmasks, face culling
  as bit-shifts (`col & ~(col >> 1)`), and greedy merging on bit-planes 64 faces at a time.
  Output **8 bytes/quad** consumed by vertex pulling (6b x/y/z/w/h + type).
- **Rust port exists**: the `binary-greedy-meshing` crate (Inspirateur) — 65 µs opaque /
  90 µs with transparency on a Ryzen 5500, **~30× faster than block-mesh-rs**; hard-coded
  62³+padding; used in a shipped Bevy game. TanTan's Bevy/WGSL demo has Criterion benches.
- **Vercidium (Sector's Edge, 32³ chunks like ours)**: run-merging variant = ~20% more
  triangles but ~3.9× faster meshing; **4-byte packed vertex** (18b pos + 5b tex + 4b
  health + 3b normal) on plain vertex buffers.
- **Mesh shaders: confirmed off the table** for our targets through ~2027 (WebGPU issue
  #3015 open/Milestone-4+; no Mali/Adreno retail Android driver ships VK_EXT_mesh_shader;
  Iris Xe lacks it).
- **WebGPU compat-mode gotcha (load-bearing):** `maxStorageBuffersInVertexStage` defaults
  to **0** in compatibility mode (~45% of GLES-class devices support zero) — so
  **vertex-pulling via storage buffers is not weak-hardware-safe**. The portable
  equivalent: **per-instance quad data in a vertex buffer** (instance step mode, 8 B/quad,
  unit-quad expansion in the shader) — same bandwidth, works everywhere.

**For us:** our mesher is already binary-greedy-influenced and tested; the actionable items
are (a) the 8 B/quad *instanced-quad* representation as a candidate vertex-bandwidth halving
(measure against our 4–8 B packed vertices ×4 corners; note our AO-driven diagonal flip
constraint that killed the shared index buffer before), and (b) treat the µs-class meshing
numbers as the budget bar if meshing ever shows up in M10 counters. *(Sources: github
cgerikj/binary-greedy-meshing, Inspirateur/binary-greedy-meshing,
TanTanDev/binary_greedy_mesher_demo, Vercidium/voxel-mesh-generation; gpuweb#3015;
webgpufundamentals compat-mode [S]; Davis Morley Handmade Seattle 2022 talk [S].)*

## 2. Occlusion culling — upgrades to the BFS we already have

- **Tommo's MCPE cave culling** (our M5 ancestor): the residual weakness is
  **over-visibility on open/ravine terrain**, not caves. His step-penalty heuristics
  (+1 below sea level descending, +3 dark chunks) add 5–15% cull.
- **Sodium's `OcclusionCuller`** ships the same BFS plus: **outward-only direction masks**,
  an optional **angle-based occlusion test** (reject neighbors whose far side can't be seen
  from the camera angle), and a fog-distance cut — all near-free additions to an existing
  BFS (verified in source).
- **Vintage Story's raycast variant**: traces to perimeter chunks only (~2/chunk), **1 ms
  for ~1062 traces/frame** at 192-block view distance on an old i5 — the cheapest known fix
  for BFS over-visibility; pure CPU, WASM-identical.
- **Rejects:** Intel masked occlusion culling (x86-intrinsic, no WASM/NEON path, occluder
  pipeline overkill); AVX2-only rasterizers; GPU-driven meshlet culling (needs mesh
  shaders/compute); baked portals/PVS (fights dynamic edits). **Conditional:** WebGPU
  occlusion queries are core API (boolean, no feature flag) but tiler-hostile same-frame;
  previous-frame reuse (CHC-style) only if profiling shows fragment-bound chunks surviving
  the cheaper fixes — no shipped voxel precedent found.

**For us:** a small, well-bounded engine milestone exists here — **BFS upgrades (direction
masks + angle test + step penalties + fog cut)**, with Vintage-Story raycasting as the
measured follow-up if M10/M8b show over-visibility cost. Front-to-back order from the BFS
already feeds our early-Z sort. *(Sources: tomcc.github.io visibility posts [S, verified
via raw repo]; CaffeineMC/sodium OcclusionCuller.java [V]; tyronx/occlusionculling [V];
GameTechDev/MaskedOcclusionCulling [V].)*

## 3. Streaming, upload & memory — the convergent shipped pattern

- **Job scheduling (Veloren, closest comp — Rust voxel):** distance-priority pick
  (`min_by_key(dist², started_tick)`), **stale-drop by tick compare** instead of hard
  cancellation, workers = cores−3/−4, and a **fractional per-frame upload budget**
  (`CHUNKS_PER_SECOND = 240` with carry ≈ 4 uploads/frame at 60). Sodium adds
  measured-duration budgeting + main-thread task stealing.
- **Generation amortization:** Veloren's per-column `ZCache` 2D pass before 3D fill (we
  have the generated-section cache — the 2D/3D split is the next refinement if worldgen
  shows in counters); Minecraft's staged proto-chunk ladder (a 2–3 stage version is worth
  copying if/when features need neighborhood radii).
- **Upload (wgpu-specific, load-bearing):** `Queue::write_buffer` allocates a fresh staging
  buffer per call, and unthrottled buffer creation causes **measured ~25 ms periodic
  hitches** even on desktop (wgpu#1242 — gpu-alloc growth). Use **`util::StagingBelt`**
  (ring of reusable staging buffers; chunk_size ≥ largest write, 1–4× total bytes/
  submission) or a Sodium-style **16 MB persistent-mapped ring** with fence reclaim +
  contiguous-copy consolidation. **Never create buffers mid-frame.**
- **GPU memory layout:** Sodium pools **region arenas** (8×4×8 sections per arena,
  best-fit segment allocator + compaction) instead of per-chunk buffers — kills
  fragmentation and bind/draw overhead.
- **Eviction/persistence:** plain distance-ring drop + regenerate beats LRU for a
  player-centric seeded world (Veloren does exactly this); if we ever persist edits beyond
  the share-codec, the Anvil shape (fixed sectors + offset table + per-chunk compression
  byte, LZ4) is the template.
- **WASM threads:** wasm-bindgen-rayon needs COOP/COEP headers GitHub Pages doesn't set
  (a coi-serviceworker shim exists) + nightly atomics — re-confirms our time-sliced
  main-thread web path as the sane v1; the budget-with-carry pattern is the upgrade.

**For us:** the single most actionable engine item from this whole pass: **audit our mesh
upload path against the StagingBelt/pool pattern** — if we call `write_buffer`/
`create_buffer` per chunk upload, we own a latent hitch identical to wgpu#1242. Second:
**region-pooled vertex arenas** are the structural answer if M10 shows draw/bind overhead.
Both are measurable-here engine milestones. *(Sources: veloren terrain/mod.rs + world
lib.rs [V via raw github]; CaffeineMC/sodium ChunkBuilder/MappedStagingBuffer/
GlBufferArena [V]; wgpu belt.rs [V] + issue #1242 [V]; RReverser/wasm-bindgen-rayon [V];
Vercidium/voxel-mesh-generation [V]; minecraft.wiki chunk format [S]; fastanvil region.rs
[V]; voxagon.se Teardown notes [S].)*

## 4. Storage — palette sections validated; the refinements we may be missing

The brickmap papers (van Wingerden 2015 — our namesake!), NanoVDB, Minecraft paletted
containers, and Teardown all converge on what we already do (palette-compressed fixed
sections, index-not-pointer residency). The refinements worth checking against our code:
- **Uniform-section fast path** (all-air / all-solid stores nothing, skips meshing+upload
  entirely — Minecraft's single-value palette).
- **Adaptive bits-per-index** (1/2/4/8 by palette size, 4-bit floor; global-palette escape
  hatch for pathological sections).
- **One shared occupancy bitset per section** (u64 columns — the binary-greedy-meshing
  primitive) reused by the mesher, cave-connectivity flood fill, raycasts, and `solid_at` —
  one representation, many consumers, 64 B per 8³.
- **NanoVDB's upload lesson:** flat, offset-based, pointer-free, immutable snapshots —
  the right shape for any GPU-side voxel buffer we ever add.
*(Sources: stijnherfst/BrickMap [V]; NanoVDB.h [V]; wiki.vg chunk-format mirror [V];
voxel-wiki palette-compression source [V]; SED Gustafsson interview [S].)*

## 5. Far-field LOD — the plan amendment

The strongest cross-source finding: **insert a downsampled greedy-mesh LOD ring before the
point regime.** Mesh-mips (2×/4× voxels via any-solid downsampling, vertex-color-only, no
textures, **skirts** at LOD borders) reuse our exact render path, cut vertex bandwidth ~4×
per level, and avoid the twinkle problem at mid distances; Distant Horizons proves the
beyond-that representation should be **column/span data** (~8 B/column runs, a separate
dumb render path) at 4096-chunk scales on weak hardware. Points take over only where quads
approach ~1 px — as the dissolve/mood layer and final ring, **not** the bulk far field.
The M7 `decimate_surface` core stays; its contract grows: **area-averaged palette colors**
(average resolved colors up the hierarchy — QSplat's "mipmapping for point clouds", the
single most important anti-twinkle measure), ~1 pt/pixel density target, ≥1 px size clamp
with fade, LOD-switch hysteresis. Cracks: skirts, not Transvoxel (smooth-terrain tool) and
not geomorphing (our dissolve already does the aesthetic work). Dreams' lineage (BVH cut at
~1 splat/pixel + stochastic alpha + TAA) confirms the architecture but needed TAA we don't
have. *(Sources: DH FullDataPointUtil/RenderUtil.java [V via mirror]; 0fps blocky-LOD;
QSplat; Schütz CLOD 2019; Dreams SIGGRAPH 2015 [S]; godot_voxel smooth_terrain.md [V];
Karis Nanite 2021 [S — voxels/points rejected on data size; ~1 primitive/pixel is where
representations converge].)*

## 6. Point stability & the screen-locked-Bayer verdict

Two independent passes examined our Bayer choice; the synthesis:
- **Validated as the no-TAA default.** Unity ships exactly this (screen-space Bayer
  CrossFade, no TAA dependency); screen-locked ordered dither is temporally *silent* for a
  static camera and maximally uniform per fade level. Animated noise (UE's
  DitherTemporalAA, Playdead-style per-frame hashes) is **disqualified without TAA** —
  full-amplitude binary coverage + animated pattern = shimmer.
- **The known weakness is pattern-swim under camera motion** (the surface slides under a
  screen-fixed pattern — worst on far content under rotation). The literature's escalation
  is **world-anchored per-point/per-quad hashed thresholds** (Wyman & McGuire hashed alpha:
  `hash(floor(world_pos/scale))` with log2-discretized, interpolated scale; Schütz CLOD's
  per-point `level + rand[0,1)` discard — shipped, VR-grade, pop-free), *not* blue noise
  (worse motion shimmer without TAA).
- **Pinned recommendation:** keep screen-locked Bayer as the default; **quantize the fade
  factor to the matrix's level count** (17 levels for 4×4) so slow fades don't ripple; use
  **complementary masks** for the mesh/points halves of the dissolve (each pixel shows
  exactly one LOD — the Unity/godot_voxel pattern); implement **per-point hashed fade
  behind a flag** as the motion-swim fallback, decided by eye on real content (a D6-norm
  toggle). For far points specifically, the per-point hash *is* SPT-style decimation made
  continuous and is world-anchored by construction — likely the right choice for the point
  ring even if the mesh half stays Bayer.
*(Sources: Wyman & McGuire I3D 2017 [S + patent]; Schütz et al. IEEE VR 2019 [V code];
Enderton stochastic transparency [S]; Unity SRP Common.hlsl LODDitheringTransition [V];
Cesium-for-Unreal blog + issue #1388; Playdead banding deck [V via LFS]; Bart Wronski
dithering series; StopThePop/Mip-Splatting/mobile-3DGS 2024-25 [S — sorts abandoned even
in photoreal; our opaque z-tested choice validated].)*
