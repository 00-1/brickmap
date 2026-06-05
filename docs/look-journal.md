# brickmap — Look journal

The running record of brickmap's **visual identity**: what the renderer does, which
artifacts we deliberately *keep*, and why. The thesis (design §11) is that the look
should be **emergent and low-fi by exposing the underlying technology** — not retro
pastiche, not photoreal, not the stock-engine look. So this isn't a style guide
written up front; it's a journal of decisions made as the tech produces them.

**How to read it:** newest entries on top. Each entry: *what we did · what it does to
the image · the call (keep / tune / drop) · why*. Snapshots live in the build gallery
(`/archive/<id>/`); milestone briefs hold the engineering detail.

---

## The standing thesis (design §11)

- **Expose the tech.** The compression, quantization, and meshing the engine does for
  speed are the *source* of the look — surface them instead of hiding them.
- **Cheap, not faked-expensive.** Every effect has to fit weak, bandwidth-bound
  hardware. If it needs GI / ray tracing / a photoreal budget, it's the wrong effect.
- **Crisp, not smoothed.** We don't add anti-aliasing or denoise the artifacts away.
  Aliased edges and greedy-quad seams are part of the signature.
- **Emergent, curated.** We let the tech make marks, then *choose* which to keep. This
  journal is the choosing.

---

## 2026-06-05 · E3 slice 3b — emissive crystals

**Glowing crystal blocks.** Rare (~0.4% of columns) emissive cyan crystals perched on
the surface; the shader renders material 6 unshaded and boosted past 1.0 so the
bright-pass catches it and it blooms. **Keep.** Now there are *two* kinds of glow — warm
ember particles and cool crystal blocks — which gives the dusk palette something to
play against and rewards exploring (you fly toward the glints). Density matters a lot:
1.3% read as cyan *frost* (a rash); 0.4% reads as rare and special. A good reminder that
"emissive" wants restraint or it stops being light and becomes texture.

## 2026-06-05 · E3 slice 3 — bloom

**Cheap LDR bloom.** The scene now renders to an offscreen target; a bright-pass
(luminance knee) → ¼-res separable Gaussian blur → additive composite gives a soft
glow around the bright things (right now: the warm ember particles). **Keep.** It's the
classic "pretty voxel demo" lever and it's what makes emissive content read as *light*
rather than just bright pixels. Kept conservative (high knee, modest intensity) so the
lit terrain doesn't haze — bloom should be for genuinely emissive things, not a global
soft-focus. Next: actual emissive *blocks* (glowing crystals) to give it something to
do across the whole world.

## 2026-06-05 · E3 slice 2 — sky gradient + horizon

**Screen-space sky gradient.** A fullscreen triangle behind everything: horizon band
(low) → deeper zenith blue (high), with the **fog colour retuned to the horizon** so
distant terrain dissolves into the haze rather than a flat void. **Keep.** Instant
atmosphere and a real sense of distance; the world no longer floats in a dark box. The
gradient is screen-space (correct under translation + yaw, i.e. the whole auto-fly
path) — a pitch-correct view-ray sky is a later upgrade if manual look-around needs it.

## 2026-06-05 · E3 slice 1 — hemispheric ambient

**Coloured hemispheric ambient.** Replaced the flat `0.35` ambient with a lerp
between a cool sky tint (from above) and a warm ground bounce (from below) by face
normal, plus a warm directional sun. **Keep, subtle for now.** It's the first
*coloured* light cue — up-faces lean cool, side/under-faces lean warm — so the
terrain reads with more tonal depth without any GI. Deliberately understated to dodge
the "stylised colour filter" trap; the louder mood levers (sky gradient, bloom,
flood-fill bleed) come in later E3 slices. Pure `shader.wgsl`, no pipeline change.

## 2026-06-05 · M4 — material & occlusion (snapshot `08-materials`)

**Baked ambient occlusion.** Corner darkening from the greedy mesher (the 0fps
3-neighbour method), `0.5..1.0` brightness. **Keep.** It's the single biggest "this is
solid, not a flat sprite" cue, and it's free at runtime (baked into the vertex's 2 AO
bits). It grounds the terrain — valleys and folds read as depth now. The greedy merge
splits at AO discontinuities, so you can occasionally *see* the triangulation near
occluders — that's fine, it's honest about the meshing.

**Procedural quantised textures.** Per-material grayscale detail tiles (hashed value
noise, posterised to 3–5 levels), tiled one-per-voxel, tinting the palette colour;
nearest-sampled with box-filtered mips. **Keep, lean further later.** The *posterisation*
is the point — it shows the same palette/quantisation thinking as the dithered shading,
rather than smuggling in smooth photo-textures. Mips use **nearest** filtering so distance
gives a chunky LOD step, not a smooth blur. Per-voxel tiling means the voxel grid stays
legible through the texture.

**Open tension to revisit:** AO (smooth, per-vertex) vs. dithering + texture
posterisation (deliberately stepped). Right now AO is the one *smooth* gradient in an
otherwise quantised image. Decide later whether to **dither the AO too** (fully commit
to stepped shading) or keep AO smooth as the one place the eye rests. Leaning toward
dithering it — but want to see it in motion first.

## Earlier (pre-journal, reconstructed)

- **E1 · Vertex-quantization wobble** (`04-aesthetic-pass`). Snap NDC to a coarse grid
  in the vertex shader (PS1-style affine jitter). **Keep** — it literally renders the
  compressed-vertex quantization as motion. Live-dial'd on the web (`wobble`).
- **E1 · Ordered (Bayer 4×4) dithering** (`04-aesthetic-pass`). Posterise shading into
  a few levels with a visible dot pattern instead of smoothing the banding. **Keep** —
  the clearest single signature so far. Live-dial'd (`dither`/colour steps).
- **M3 · Distance fog** (`07-endless-flight`). Fade terrain to the sky colour with
  distance. **Keep** — cheap atmosphere, and it does double duty hiding the streaming
  load edge. Tuned per-camera (low cruise vs. the far headless hero shot).
- **E2 · Emissive cube particles** (`05-particles`). Flat, unlit, glowing debris. **Keep**
  — the cheapest "alive on screen," and the flat glow fits the no-lighting-model look.

---

> Next look decisions queued: E3 (cheap coloured light & bloom — the big mood lever) and
> E4 (sub-voxel displacement). Add an entry per decision, not per milestone.
