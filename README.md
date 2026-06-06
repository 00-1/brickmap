# brickmap

A high-performance, cross-platform **voxel rendering engine**, written in Rust on
top of [`wgpu`](https://wgpu.rs/). The name nods to the *brickmap* sparse-voxel
brick-storage technique.

This is a **rendering** engine, not a game — the interesting problems here are
graphical: storing, meshing, culling, and drawing very large voxel worlds fast.
It is a personal-interest project, so it favours **interesting and correct** over
shipping speed.

> **Status: a dark, grimy, configurable look + per-seed doom audio, over an endless world.**
> An **endless, streamed** procedural voxel world you fly across — greedy-meshed,
> palette-compressed, frustum + cave-culled, off-thread (rayon) meshed — with a shared
> **splat pipeline** for wind-swept grass, undergrowth, and **point-cloud trees**, over
> **biomes**, **domain-warped + ridged** relief, **rivers**, stylised **water**, and
> **3D-noise caves**.
>
> The identity that *emerged* is low-fi, **"expose the tech"**: a **configurable palette
> post-process** (20 curated 1–2-hue palettes — a luminance gradient-map with ordered
> **dithering** that reads as a halftone at low resolution), **deep shadows** with an
> optional **sun-off, point-lit** mood (coloured emissive crystals the only light), and a
> per-seed **doom-drone** (Sleep/*Dopesmoker*-flavoured, cross-platform audio). Plus
> **shareable seeds**, in-world **voxel editing** (one command seam — multiplayer
> groundwork), an opt-in distance-dissolve "melt", baked AO, procedural materials, sky/fog,
> and bloom — on desktop, the web, **and a sideloadable Android APK**, one code path. See the
> [look journal](docs/look-journal.md).
> Next: in-world **text** + **colossal fallen bodies** (E17/E18), the **pixel-scale** halftone
> dial, profiling on weak hardware (M8b), multiplayer (N1). See
> [`docs/roadmap.md`](docs/roadmap.md).

## North star

**Performance on weak hardware first.** It must feel great on an **integrated
GPU** (no dedicated card) and a **mid-range phone**. These devices are
**bandwidth-starved**, so most design decisions are about *moving less data*, not
*computing less*. Cross-platform target is **PC + mobile + web**, but **web is
explicitly lower priority** — kept cheap to retain (wgpu makes it nearly free),
never allowed to dictate the design.

## Key decisions (the short version)

| Area | Choice | Why |
|---|---|---|
| Language | **Rust** | Flat arrays + tight loops; safety without a GC. |
| Graphics | **wgpu** | One renderer → Vulkan/Metal/DX12/GLES natively + WebGPU/WebGL on web. |
| Companions | **winit** (windowing), **glam** (math) | De-facto Rust gamedev stack. |
| Paradigm | **Rasterized greedy meshing** (not ray-marched voxels) | Ray-marching is bandwidth-hungry; it murders integrated/mobile GPUs. |
| Lighting | **Forward (→ forward+)**, not deferred | Mobile GPUs are tile-based; fat G-buffers are bandwidth-hostile. |
| Materials | **2D texture array** (not atlas) | No bleed, clean mips, cheap vertex packing. |
| Chunk size | **32³ sections**, palette-compressed | Amortises overhead; cuts RAM + meshing bandwidth. |

**Performance pillars:** binary greedy meshing · compressed (~4–8 byte) vertices ·
palette-compressed chunk storage · visibility-graph ("cave") occlusion culling ·
chunk LOD · async meshing.

**Visual identity:** deliberately **low-fi in the sense of *exposing the
underlying technology*** — quantization, banding, seams, stray pixels — *not*
retro pastiche and *not* photorealism. The look is meant to **emerge from the
tech** (the same artifacts that make it fast become the aesthetic), rather than
being pre-planned. See the design doc.

## Live preview

Every push to `main` auto-deploys to **GitHub Pages** as a small build gallery:
<https://00-1.github.io/brickmap/>. The current build is at
[`/latest/`](https://00-1.github.io/brickmap/latest/); frozen snapshots of past
milestones live under `/archive/`. See `docs/development.md`.

## Documentation

- [`docs/design.md`](docs/design.md) — goals/non-goals, target hardware + frame
  budgets, every decision with rationale, and the visual-identity pillar.
- [`docs/architecture.md`](docs/architecture.md) — module/crate boundaries and the
  sacred world-data ↔ renderer seam.
- [`docs/spikes.md`](docs/spikes.md) — what we de-risk first, and the spike backlog.
- [`docs/roadmap.md`](docs/roadmap.md) — milestone ladder and how we plan.
- [`docs/development.md`](docs/development.md) — workflow, code-quality gates, and
  testing strategy.

## Running

Requires a recent stable Rust toolchain (developed against 1.94).

### Desktop (native)

```sh
cargo run
```

You should get a window with a spinning, depth-tested, perspective cube on a dark
background. (`RUST_LOG=info cargo run` prints the selected GPU adapter.)

### Web (WASM)

The web build compiles the **library** to WebAssembly and uses
[`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/) to generate the JS glue
into `web/pkg/`. One-time tooling:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli   # version should match the `wasm-bindgen` crate in Cargo.lock
```

Build the bundle:

```sh
cargo build --lib --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript \
  --out-dir web/pkg \
  target/wasm32-unknown-unknown/debug/brickmap.wasm
```

Then serve the `web/` directory over HTTP (ES modules + WASM won't load from
`file://`) and open it:

```sh
# any static server works, e.g.:
python3 -m http.server --directory web 8080
# → visit http://localhost:8080
```

It runs on **WebGPU** where available and falls back to **WebGL2** otherwise. For
a smaller bundle, build with `--release` and point `wasm-bindgen` at the release
`.wasm`.

> Prefer a one-shot tool? `trunk serve` or `wasm-pack build --target web` also
> work; the manual `wasm-bindgen` flow above is what's documented because it's the
> verified path and has the fewest moving parts.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
