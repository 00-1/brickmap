# brickmap — Architecture

> Status: **target architecture**, seeded at kickoff. The current code is only the
> spike (see `docs/spikes.md`), which is a single flat crate. This document
> describes where the module/crate boundaries are *heading* so the spike can be
> refactored into them without surprises. Read `docs/design.md` first.

## 1. Guiding principles

- **Weak-hardware-first, bandwidth-aware** (see design §4). Module boundaries
  must not force extra data copies on the per-frame hot path.
- **Clean world-data ↔ renderer seam.** The voxel world must not know how it is
  drawn; the renderer consumes a well-defined mesh/draw contract. This is the one
  abstraction we protect, so a future renderer swap stays *possible* (design §5).
- **Don't over-abstract.** No speculative trait towers. Boundaries earn their keep.
- **Platform differences live at the edges**, behind `cfg`, not smeared through
  the engine. Native vs web differs in *windowing, threading, and asset loading* —
  nowhere else if we can help it.

## 2. Layer / module map

Data flows **left → right**; the renderer never reaches back into world internals.

```
            ┌──────────────────────────────────────────────────────────┐
            │                         app                                │
            │  event loop, frame scheduling, wiring (native + wasm)      │
            └───────────┬───────────────────────────────┬──────────────┘
                        │                                │
              ┌─────────▼─────────┐            ┌─────────▼──────────┐
              │     platform      │            │       scene        │
              │ window, surface,  │            │ camera, frustum,   │
              │ input, timing,    │            │ view/proj, the     │
              │ threading shim    │            │ floating origin    │
              └─────────┬─────────┘            └─────────┬──────────┘
                        │                                │
   world  ─────────────┼───────► mesh ──────────────────┼──────► render
   ┌──────────────┐    │     ┌──────────────────┐       │   ┌──────────────────┐
   │ voxel data   │    │     │ greedy mesher     │       │   │ wgpu device/queue │
   │ chunk/section│    │     │ packed vertices   │──────────►│ pipelines, passes │
   │ palette store│────┴────►│ (the draw contract)│      │   │ forward(+) frame  │
   │ world coords │          │ visibility graph  │       │   │ texture array     │
   └──────────────┘          └──────────────────┘        │   └──────────────────┘
                                                          │
                                                       (culling feeds
                                                        draw lists)
```

## 3. The crates (planned workspace)

The spike is a single `brickmap` crate. As it grows we split into a Cargo
**workspace** so compile times stay sane and boundaries are enforced by the crate
graph (you *cannot* accidentally call a renderer internal from the world crate if
they are separate crates). Proposed split:

| Crate | Responsibility | Depends on |
|---|---|---|
| `bm-core` | Shared math glue (re-export glam types), IDs, error types, small utils. | — |
| `bm-world` | Voxel data model: `Voxel`, `Section` (32³), palette-compressed storage, chunk coords, floating-origin math, world container + streaming hooks. **Knows nothing about wgpu.** | `bm-core` |
| `bm-mesh` | Binary greedy mesher; emits the **packed-vertex draw contract**; builds the per-chunk **visibility graph**; LOD/octree-mip generation. Pure CPU, no GPU types. | `bm-core`, `bm-world` |
| `bm-render` | wgpu abstraction: device/queue/surface management, pipelines, the **forward(+)** frame graph, texture-array material system, GPU buffer pools that ingest `bm-mesh` output. **Knows nothing about how the world is generated.** | `bm-core`, `bm-mesh` (contract only) |
| `bm-platform` | winit windowing/surface, input, timing, and the **threading shim** (rayon pool on native; single-thread / Web Worker fallback on web). | `bm-core` |
| `bm-scene` | Camera, view/projection, frustum extraction; owns culling policy that combines frustum + the visibility graph into draw lists. | `bm-core`, `bm-world`, `bm-mesh` |
| `brickmap` (bin/lib) | The `app`: glues the above, owns the event loop, frame scheduling, and the wasm `start` shim. | all |

Until the split lands, these are **modules** inside the one crate with the same
names and the same dependency rules enforced by discipline + review.

## 4. The world-data ↔ renderer contract

This seam is sacred. Concretely:

- `bm-world` exposes voxel data and notifies when a section becomes dirty.
- `bm-mesh` turns a (section + its neighbours) into a **`ChunkMesh`**: a tightly
  packed vertex/index blob in the format from design §9–10 (≤8 byte face
  vertices, texture id = array layer), plus its bounding box and the visibility
  bitset used by occlusion culling.
- `bm-render` consumes `ChunkMesh` blobs into GPU buffer pools and draws them. It
  never sees `Voxel`, `Section`, or palettes — only the contract type.

Because the contract is *data* (a byte layout + small metadata), an alternative
renderer (e.g. a ray-marched experiment) could consume the **world** directly and
ignore `bm-mesh` entirely, without `bm-world` changing. That is the renderer-swap
escape hatch from design §5, paid for only by keeping this seam clean.

## 5. Frame flow (steady state, target)

1. **`app`** drives the winit event loop; on redraw it ticks `scene` then asks
   `render` to draw.
2. **`scene`** updates the camera, extracts the frustum, and walks the
   **visibility graph** from the camera's chunk (cave culling) intersected with
   the frustum to produce a **draw list** of visible chunk LOD meshes.
3. **`render`** records a **single forward pass** (minimal render-target
   switches — design §5): bind the texture array + per-frame uniforms, then issue
   the draw list from persistent GPU buffer pools.
4. **Async, off-thread:** dirty sections from `world` are handed to `mesh` on the
   `platform` thread pool; finished `ChunkMesh`es are uploaded to `render`'s pools
   between frames. Meshing is **never** on the critical path (design §8).

## 6. Platform / web boundary

Everything platform-specific is isolated in `bm-platform` (+ small `cfg` shims in
`app`):

- **Windowing/surface:** winit on all targets; web uses the canvas + WebGPU
  (WebGL2 fallback via wgpu's `webgl` feature). The spike already proves this path.
- **Async GPU init:** native blocks (`pollster`); web cannot, so init runs on the
  microtask queue and the ready state is delivered via a winit user-event. (This
  pattern is already in `src/lib.rs`.)
- **Threading:** native uses a rayon pool for meshing; web starts single-threaded
  with an upgrade path to `wasm-bindgen-rayon` Web Workers. Per design §4/§7.6,
  web meshing throughput is allowed to be worse.
- **Time:** a platform timing source feeds animation/movement; the camera uses a
  `web-time` clock for frame-rate-independent movement (native + web).

## 7. Current vs target (honesty section)

| Concern | Today (M4) | Target |
|---|---|---|
| Crate layout | one `brickmap` crate, modules `world` / `worldgen` / `mesh` / `scene` / `particles` / `textures` / `gfx` + app | the 7-crate workspace in §3 |
| Geometry | greedy-meshed chunks, **8-byte packed** face vertices (pos/dir/material/ao + block light) | greedy-meshed chunks, ≤8-byte packed face vertices |
| World model | palette-compressed 32³ sections, **procedural + streamed** around the camera | palette-compressed sections, streaming |
| Culling | frustum culling (per-chunk AABB) | frustum + visibility-graph cave culling |
| Pipeline | forward pass → bloom post; baked AO + texture-array materials + hemispheric ambient + flood-fill block light + sky/fog | forward(+), texture-array materials, LOD |
| Threading | single-thread (synchronous streaming + meshing on the main thread) | rayon (native) / workers (web) async meshing |

Through M4 the data → mesh → GPU pipeline, the `world`↔`render` seam, streaming,
and the first material/aesthetic layers are real. The remaining gaps above
(occlusion culling, async meshing, LOD, the crate split) are the later pillars.
