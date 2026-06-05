# M4 — It starts to look like brickmap

> Status: **done** ✅. Part of the ladder in [`../roadmap.md`](../roadmap.md).
> Builds on M3 (a streamed world) and E1 (the wobble/dither aesthetic pass).

## Goal · Outcome · De-risk

- **Goal:** give the world *material* — baked ambient occlusion that grounds the
  geometry, and per-face texture-array materials — and make the first deliberate
  aesthetic decisions (start the "look journal", design §11).
- **Outcome:** a textured, AO-shaded world with an emerging, opinionated look — not
  flat-shaded palette blocks any more.
- **De-risks:** the material path (texture array on web GL2 + native) and the first
  concrete aesthetic calls.

## Scope

**In:**
- **Baked vertex AO** in the greedy mesher: the classic 3-neighbour corner darkening
  (0fps), packed into the 2 `ao` bits the vertex already reserves, applied in the
  shader. Greedy merging respects AO (only merges faces whose corner-AO matches).
- **Texture-array materials**: a per-face texture id (from the material), a small
  **procedurally-generated** texture array (dependency-free, fits the low-fi look),
  nearest sampling + mips. UVs derived from the greedy quad size so textures tile.
- **Look journal**: a docs page that curates what we keep and why (the §11 thesis
  made concrete), with a snapshot.

**Out (later):**
- Real authored textures / a texture atlas pipeline — procedural is enough to prove
  the path and set the look.
- Per-face damage/crack decals (backlog E) — a cheap follow-up once the path exists.
- Smooth/per-pixel relief (that's ✨E4, sub-voxel displacement).

## Design sketch

### Baked AO (`mesh`)
- For each face corner, sample the 3 voxels around it in the layer just outside the
  face (two edge neighbours + the diagonal). AO level `0..3` = `if both sides solid
  { 0 } else { 3 - (side1 + side2 + corner) }`.
- The greedy mask keys on `(block, [ao; 4])` so a quad only merges cells with an
  identical corner-AO pattern — AO discontinuities (near occluders) break the merge,
  flat-lit areas still merge big. Quad triangulation flips to the diagonal between
  the darker corners to avoid the interpolation artifact (culling is off, so the flip
  is free).
- Cross-chunk AO is correct across the 6 face seams; diagonal samples at chunk
  *corners* fall back to "air" (a tiny, accepted darkening loss at corners).

### Texture-array materials (`render` + `mesh`)
- A `texture_2d_array` bound in group 0; the fragment samples `layer = material`.
- Generate the layers on the CPU at startup (per-material noise/pattern) so there's
  no asset pipeline yet and the textures are reproducible for golden images.
- The greedy quad spans `w×h` cells, so pass quad-local UVs (0..w, 0..h) and let the
  sampler tile with `repeat` + nearest + mips.

## Tests

- **AO:** an isolated face is fully unoccluded (`ao == 3`); a neighbour block darkens
  the adjacent face corners; an occluder splits a greedy merge (more quads than the
  un-occluded slab).
- **Materials/UV:** quad UVs match the quad extent (tiling); texture-layer selection
  maps material→layer. (Texture *content* is eyeballed via headless renders.)

## Acceptance checklist

- [x] Baked AO in the greedy mesher; AO tests pass; visible corner darkening.
- [x] Texture-array materials (procedural), per-face, nearest + mips; tiling UVs.
- [x] Runs native + web (texture_2d_array is core in WebGL2/GLES3; wasm builds — web
      *visual* check still pending D5).
- [x] Look-journal page started ([`../look-journal.md`](../look-journal.md));
      snapshot `08-materials` + renders in chat.
- [x] CI green; docs synced (lockstep rule).

> Status: **done** ✅ — baked AO + procedural texture-array materials (nearest + mips);
> look journal started. Deferred to follow-ups: real authored textures, damage decals,
> and the AO-vs-posterisation tension noted in the journal.
