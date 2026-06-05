# ✨ E3 — Light & atmosphere: cheap, no GI

> Status: **done** ✅. Exploration rung in [`../roadmap.md`](../roadmap.md);
> the backlog rationale is [`../exploration-backlog.md`](../exploration-backlog.md) §C.
> Builds on M4 (materials + baked AO).

## Goal · Outcome · De-risk

- **Goal:** mood and glow — the thing that makes "pretty voxel" demos pretty —
  **faked** without GI or ray tracing, on weak hardware.
- **Outcome:** a world with directional warmth, soft sky/ground ambient, glowing
  emissive blocks, a horizon, and (the headline) light that visibly **bleeds around
  corners** via flood-fill — a cheap *fake GI*.
- **De-risks:** the "lighting data path" the design hand-waved, and proving beauty is
  reachable with cheap fakes.

## Scope

**In (slices, cheapest first):**
1. **Hemispheric ambient** — cool sky tint from above, warm ground-bounce from below
   (by face normal), replacing the flat constant ambient. Shader-only. ✅
2. **Sky gradient + tinted fog** — a horizon the fogged terrain dissolves into, and
   a day-leaning palette. A cheap fullscreen sky.
3. **Emissive materials + bloom** — emissive block ids output unshaded brightness; a
   cheap down-sample + blur + add gives the glow. Pairs with the emissive particles.
4. **Flood-fill light (the headline fake-GI)** — BFS block + sky light, `−1` per step,
   baked per-vertex in the mesher; **coloured** propagation so light bleeds round
   corners. Needs a wider vertex (≤8 bytes, design-sanctioned) — the 4-byte packed
   vertex is full, so light gets its own attribute.

**Out (later):**
- Real-time relight on edits (E5 territory) — bake on mesh for now.
- Shadow maps / any view-dependent lighting — off-brand and not cheap.

## Design sketch

- **Hemispheric ambient/sun (slice 1):** `light = mix(ground, sky, n.y·0.5+0.5) +
  sun_col · diffuse`. Coloured, so faces pick up sky vs. bounce — the first cheap
  depth-from-light cue. Pure `shader.wgsl`; no pipeline change.
- **Sky (slice 2):** a fullscreen-triangle gradient (or fold into fog) so the
  background isn't a flat clear; fog colour samples the same gradient at the horizon.
- **Bloom (slice 3):** render emissive to the same target, then a small mip-blur and
  additive composite. Keep the blur cheap (few taps, half-res).
- **Flood-fill light (slice 4):** CPU BFS over the section (+ borders) on mesh; store a
  per-vertex light byte (rgb light + sky). Greedy merge keys on it like AO. This is the
  big one; spike it separately and budget the re-mesh cost (M6 absorbs it later).

## Tests

- **Ambient:** an up-facing face is sky-tinted, a down-facing face is ground-tinted
  (pure-logic check of the mix if it moves to Rust; otherwise eyeballed headless).
- **Flood-fill (slice 4):** BFS falloff is correct (`−1`/step, clamped at 0); a lit
  cell propagates round a one-voxel corner; greedy merge respects light like AO.
- Texture/colour content stays eyeballed via headless renders + the look journal.

## Acceptance checklist

- [x] Hemispheric coloured ambient (sky/ground) replacing flat ambient.
- [x] Sky gradient / horizon; fog tinted to match.
- [x] Cheap bloom (bright-pass → ¼-res separable blur → composite) + emissive crystal
      blocks — both the ember particles and the crystals glow.
- [x] Flood-fill coloured light baked in the mesher; visible corner bleed. *(v1:
      per-section, so light clips at chunk borders — cross-chunk light is a follow-up.)*
- [x] Runs native + web (wasm builds; the texture-array + 8-byte vertex paths are
      GL2-compatible — web *visual* check still pending D5); look-journal entries per
      decision; snapshots `09`/`10`/`11` + renders in chat.
- [x] CI green; docs synced (lockstep rule).

> Status: **done** ✅ — hemispheric ambient, sky/horizon, bloom, emissive crystals, and
> flood-fill block light. Deferred follow-ups: cross-chunk light (v1 clips at borders),
> day–night palette shift, and a pitch-correct view-ray sky.
