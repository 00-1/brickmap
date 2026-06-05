# M6 — Off the main thread (async meshing)

> Status: **done** ✅ (native verified by test; live frame-time win observed on the
> deployed build). Part of the ladder in [`../roadmap.md`](../roadmap.md).
> Builds on M3 (streaming) and M5 (the HUD that makes the hitches visible).

## Goal · Outcome · De-risk

- **Goal:** stop the frame from blocking on chunk generation + meshing while flying.
- **Outcome:** no hitches on native as the world streams; the HUD's frame time stays
  flat instead of spiking when chunks load.
- **De-risks:** threading, and the known web threading constraint (no SharedArrayBuffer
  on GitHub Pages → no `wasm-bindgen-rayon`).

## Scope

**In:**
- **Native:** a **rayon** thread pool meshes chunks off the critical path. Each job is
  self-contained — given a chunk coord + the world seed it **regenerates** the section
  and its 4 horizontal neighbours, greedy-meshes, and computes the connectivity graph,
  returning a `ChunkInstance` over an `mpsc` channel. No shared `World`, so no locks.
- **Main thread:** each frame, *request* missing chunks (cheap — just dispatch jobs),
  then *drain* finished meshes within a budget and upload them to the GPU. Meshing
  never runs on the main thread, so it can't stall a frame.
- **Web fallback:** same interface, but `request` enqueues and `drain` does the meshing
  inline, time-sliced to a few chunks/frame (today's behaviour). Accepted as slower.
- HUD shows the in-flight (`meshing N`) count so the async pipeline is visible.

**Out (later):**
- Double-/triple-buffered GPU upload rings — a single per-frame upload budget is enough
  for now; revisit if uploads themselves spike.
- `wasm-bindgen-rayon` web workers — blocked on COOP/COEP headers Pages can't set.
- Threaded *light* propagation across chunks (the cross-chunk light follow-up).

## Design sketch

- A `ChunkLoader` with a target-split implementation:
  - native: `rayon::spawn` per request → `mpsc::Sender<ChunkInstance>`; `drain` does
    `try_recv` up to a budget.
  - web: a `VecDeque<ChunkCoord>`; `drain` meshes up to a budget per frame.
- `build_chunk_instance(coord, seed)` is the pure worker: generate + mesh + connectivity.
  It's `Send` (just `Vec`s and POD), so it crosses the thread boundary freely.
- The app drops its `World` entirely for streaming — workers regenerate. Eviction is
  just removing GPU draws + the loaded-set entry; far results are dropped on drain.

## Tests

- Meshing correctness is already covered (M2 oracle tests). M6 is plumbing; the win is
  measured on the HUD (flat frame time while streaming on native). `build_chunk_instance`
  determinism falls out of `worldgen` + `mesh` already being deterministic + tested.

## Acceptance checklist

- [x] Native: generation + meshing run off-thread (rayon); the main thread only
      dispatches + uploads. No meshing on the render thread. (Tested: a request meshes
      off-thread and returns via the channel.)
- [x] Web: still functions, meshing time-sliced on the main thread (`drain` budget).
- [x] HUD shows in-flight mesh jobs (`meshing N`).
- [x] CI green; docs synced.

> Status: **done** ✅ — `ChunkLoader` (rayon on native, inline time-sliced on web),
> self-generating `build_chunk_instance` worker, request/drain in the stream loop, HUD
> `meshing N`. The app dropped its `World` for streaming. Deferred: triple-buffered
> upload rings; `wasm-bindgen-rayon` web workers (blocked on Pages COOP/COEP headers).
