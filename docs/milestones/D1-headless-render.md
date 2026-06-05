# D1 — Headless render-to-PNG

> Status: **in progress** 🛠. Dev tooling (see [`../roadmap.md`](../roadmap.md)).

## Goal · Outcome · Unlocks

- **Goal:** render the scene **offscreen** (no window/display) to a PNG.
- **Outcome:** a `screenshot` command that captures the current demo scene to an
  image file.
- **Unlocks:** (1) **golden-image regression tests** (render a known scene, compare
  to a committed reference) — already promised in `development.md`; (2) **supervised
  autonomous runs** — Claude can render a PNG and *look at it* to catch visual bugs
  instead of working blind.

## Key finding

The container has **no GPU and no display**, but wgpu can use **llvmpipe** (Mesa's
software Vulkan rasterizer, `device_type: Cpu`). Installing `mesa-vulkan-drivers`
provides the `lvp_icd.json` ICD and `wgpu::Instance::default()` then finds a
headless adapter. Verified via `cargo run --example gpucheck`.

## Scope

**In:**
- A native `headless` render path: instance → adapter (no surface) → device →
  render the meshed scene into an offscreen `Rgba8UnormSrgb` texture → read pixels
  back → write a PNG.
- A `screenshot` binary that captures the demo world from the default camera.
- Reproducibility so it works in fresh sessions / CI (the ICD isn't preinstalled).

**Out (later):**
- A full golden-image **test** harness with committed references + tolerance
  (follow-up once the capture path is trusted).
- De-duplicating the render pipeline between the windowed and headless paths into a
  shared `Renderer` (noted debt; first cut may duplicate the pipeline setup to
  avoid risking the live windowed path).

## Design sketch

- `headless::capture(width, height, out)` (native-only): builds the same demo
  scene (`demo_world` + greedy meshing + packed vertices) and camera framing as the
  app, sets up a software-Vulkan device, draws with the **same `shader.wgsl`** into
  an offscreen texture, copies it to a mappable buffer (row-padded to 256 B), and
  writes a PNG (`png` crate).
- A `screenshot` bin wraps it: `cargo run --bin screenshot -- out.png`.
- **Reproducibility:** a `SessionStart` hook + a CI step run
  `apt-get install -y mesa-vulkan-drivers` so the adapter exists everywhere.

## Tests / verification

- Manual first: run `screenshot`, open the PNG, confirm it matches the windowed
  build (Claude can view the PNG directly).
- Later: a golden-image test that renders a fixed scene/camera at a small size and
  asserts it matches a committed reference within tolerance.

## Acceptance checklist

- [ ] `headless::capture` renders the demo scene to a PNG via llvmpipe.
- [ ] `screenshot` binary produces an image that matches the live build.
- [ ] Reproducible in fresh sessions (SessionStart hook) and CI.
- [ ] Pipeline shared with the windowed path *or* the duplication is documented as
      debt with a follow-up.
- [ ] CI green; docs synced.
