# E1 — Aesthetic pass: expose the tech

> Status: **in progress** 🛠. Exploration rung (design §11–12). First rung that
> gives brickmap a face.

## Goal

Render the engine's *own artifacts* as the aesthetic — the §11 thesis made literal
— with two cheap, rasterized, weak-hardware-friendly effects:

1. **Vertex-quantization wobble** (PS1-style): snap each vertex's screen position to
   a coarse grid in the vertex shader. This renders the compressed-vertex
   quantization *as the look* (wobble as geometry moves).
2. **Ordered dithering** (fragment): quantize the shaded colour to a few levels with
   a Bayer pattern — exposing the palette/banding as deliberate dither instead of
   smoothing it away.

Both live in `shader.wgsl`, so they apply to the windowed *and* headless renders for
free (I can tune via headless PNGs).

## Scope

- **In:** the two shader effects, with tunable strength; tuned by eye via headless
  captures.
- **Out:** textures/materials (M4), a real post-process pipeline, bloom/CRT
  (later/maybe-never — CRT risks the retro-pastiche we ruled out).

## Design

- **Wobble** (`vs_main`): after the clip position, snap NDC: `ndc = round(ndc *
  snap) / snap`, re-multiply by `w`. `snap` = steps across NDC (lower = chunkier).
- **Dither** (`fs_main`): a 4×4 Bayer threshold indexed by the framebuffer pixel
  (`@builtin(position)`); `c = floor(c * levels + bayer) / levels`.
- Strength constants in the shader for now (a uniform/toggle can come later).

## Verification

Headless `screenshot` renders, viewed and tuned until it reads as *intentional
low-fi* (not broken). Snapshot to the gallery + post a render to chat.

## Acceptance checklist

- [x] Visible vertex wobble + ordered dithering; tuned to look deliberate
      (`WOBBLE_SNAP=85`, `COLOR_STEPS=4`, verified via headless renders).
- [x] Runs native; wasm builds. *(WebGL2 visual unverified headless — standard WGSL,
      low risk; confirm on the live build.)*
- [x] Snapshot to gallery + render posted to chat.
- [x] CI green; docs synced.

> Status: **done** ✅. The knobs are shader consts; easy to dial up/down later.
