# brickmap — Spikes

> A *spike* is a small, throwaway-friendly experiment that **de-risks one
> uncertainty** before we commit to building on it. Spikes are judged by what we
> learn, not by code we keep. Read `docs/design.md` and `docs/architecture.md`
> for context.

## Why spike before engine code

The design (forward-rendered, rasterized, wgpu, Rust, web-as-cheap-extra) makes
several bets. The cheapest bet to invalidate — and the one everything else sits
on — is: **can a single wgpu code path actually clear the screen and draw
geometry on desktop *and* in a browser?** If that path is painful or impossible,
the "web is nearly free" premise (design §5) is wrong and we want to know on day
one, not after building a mesher.

So: planning docs + this first spike come **before** any voxel-specific code.

---

## Spike 1 — Cross-platform render path  ✅ (this kickoff)

**Question:** Does one wgpu/winit/glam code path clear the screen and draw a
single 3D cube, running on **native desktop** and **web (WASM)**, with depth and
an MVP transform?

**Why this and not a voxel:** a cube exercises every moving part of the render
path we will reuse forever — instance/adapter/device, a surface configured for
the platform's preferred format, a vertex + index buffer, a uniform bind group,
a depth attachment, a render pipeline, and the resize/lost-surface handling — but
contains **zero** voxel-specific complexity. If the cube renders on both targets,
the cross-platform foundation is proven and meshing can proceed.

### Scope (intentionally tiny)
- Clear to a dark colour, draw one spinning, depth-tested, perspective cube.
- One source path; platform differences only at the edges (`cfg`):
  - native blocks on async GPU init (`pollster`); web delivers the ready state via
    a winit user-event because it cannot block.
  - web binds to a `<canvas>`; native opens a window.
  - web enables wgpu's `webgl` feature as a WebGL2 fallback for browsers without
    WebGPU.
- No clock: rotation advances a fixed amount per frame, so behaviour is identical
  everywhere and we avoid pulling in time/instant deps.

### Explicit non-scope
- No voxels, chunks, meshing, palettes, culling, LOD, textures, or lighting.
- Not performance-tuned; fat 24-byte vertices are fine here.
- Not the final module layout (it's one flat crate — see architecture §7).

### How to run
See the **Running** section of the top-level `README.md` (native + web).

### Acceptance criteria
- [x] `cargo build` (native) compiles clean (no warnings).
- [x] `cargo build --lib --target wasm32-unknown-unknown` compiles clean.
- [x] `wasm-bindgen --target web` produces a loadable `web/pkg/` (`brickmap.js`
      + `brickmap_bg.wasm`). Verified during kickoff.
- [ ] Cube visibly renders in a desktop window. *(Verify on a machine with a
      display/GPU; the kickoff container is headless so this is checked locally.)*
- [ ] Cube visibly renders in a WebGPU browser, and in a WebGL2-only browser via
      the fallback. *(Verify locally.)*

### What we expect to learn / decide
- Confirms (or kills) the "web is nearly free" premise behind choosing wgpu.
- Establishes the async-init-on-web pattern we reuse for the real engine.
- Surfaces any wgpu-version / winit-version friction early.

> Note on this environment: the kickoff container is headless (no display/GPU), so
> the two visual checkboxes are verified on a real machine. The build/compile
> criteria for both targets are checked here.

---

## Spike backlog (planned, not yet started)

Ordered by how much they de-risk the design. Each gets its own section when picked
up.

### Spike 2 — Render one *meshed* chunk
Hand-build a 32³ section, run a first (even naïve, not yet greedy) mesher, draw it
with the **packed-vertex** layout (design §9–10). De-risks the world↔render
contract (architecture §4) and the vertex-packing budget.

### Spike 3 — Binary greedy meshing correctness + speed
Replace the naïve mesher with binary greedy meshing; verify quad-merging
correctness and measure meshing throughput against the async budget (design §8).

### Spike 4 — Texture-array materials
Prove the 2D-texture-array material path (design §9) end-to-end: per-face texture
id → array layer, mipmapping, no bleeding.

### Spike 5 — Visibility-graph (cave) occlusion culling
Flood-fill chunk connectivity + frustum culling; measure draw-call/triangle
reduction on a cave-y scene — the pillar that makes the world feel light
(design §7.4).

### Spike 6 — Async meshing on a thread pool (+ web fallback)
rayon pool on native; confirm meshing stays off the critical path. Scope the web
story (single-thread vs `wasm-bindgen-rayon`).

### Spike 7 — Weak-hardware profiling pass
Run spikes 2–6 on the reference iGPU and reference phone (design §8) and check the
frame-time budgets hold. This is where targets get tightened with real numbers.
