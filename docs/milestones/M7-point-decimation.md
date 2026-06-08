# M7 — Distance dissolve / point-decimation LOD

> **Status: ◑ (2026-06-08).** The **look** half (distance dissolve) shipped earlier; this slice
> lands the **point-decimation core** (the deferred perf half's reusable algorithm). The render-
> path integration that turns it into a true primitive-LOD is the remaining slice (notes below).

## Already shipped (the look)
- The `melt` toggle stipples distant terrain + foliage into a pixel haze (screen-locked Bayer).
- Solid relics dissolve **mesh → dots by distance in the shader** (E18) — mesh-near, points-far,
  uploaded once per cell (no rebuild hitch).
- Ethereal point-recession (foliage / colossi back away as you close in).

## Landed this slice (the decimation core)
- `bm_render::foliage::decimate_surface(section, cx, cz, stride)` → a `Vec<SplatInstance>`: samples
  the chunk's top surface on a `stride` grid and emits one billboard splat per sample, coloured by
  the surface material (`mat_color`), sized to cover its stride footprint. **Pure + deterministic**,
  unit-tested (count ≈ `(SIZE/stride)²`, scales with stride, sits on the surface, `stride 0`
  clamped, empty section → empty). This is the reusable system a far-LOD draws instead of the mesh
  — `~(32/stride)²` points vs the full greedy mesh, so *primitives* drop with distance.

## Remaining (the render-path integration — a shader/feel slice)
True mesh-near / **points-far** as a perf win needs, on top of the core:
1. a per-chunk decimated-point buffer (reusing the existing per-chunk splat upload path), and
2. a **distance fade-in** for that point layer in the splat shader, **paired with the mesh's
   existing distance dissolve** (`melt`) — so points fade in exactly as the mesh stipples out
   (no near-view overdraw), with mesh draws suppressed past the crossfade band for the actual
   primitive saving.
This is deferred because (a) it's a shader + render-path change whose **look needs visual
iteration** (the crossfade band, point size/coverage), and (b) its **value is perf on weak
hardware**, which overlaps the hardware-gated **M8(b)** profiling — the win can't be validated
here. The algorithm (the hard, testable part) is in place; gate any wiring default-off so the
golden render stays valid.

## Acceptance (this slice)
- [x] `decimate_surface` — pure, deterministic, stride-scaled, unit-tested; engine-generic
      (`bm-render`, no game dep). clippy `-D` / tests / wasm + golden hash green.
- [ ] Points-far render path + crossfade shader (deferred — shader/feel + overlaps M8b perf).
