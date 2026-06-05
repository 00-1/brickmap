# M1 — One real chunk on screen

> Status: **draft for review** (no engine code written yet). Part of the ladder in
> [`../roadmap.md`](../roadmap.md). When this is agreed, it becomes the build plan.

## Goal · Outcome · De-risk

- **Goal:** stand up the real data → mesh → GPU pipeline behind a clean module
  seam, and replace the spike's hardcoded cube with a *meshed voxel chunk* you can
  fly around.
- **Demonstrable outcome (in the live preview):** a single hand-built 32³ chunk —
  shaped into something obviously voxel (e.g. a hollow box or a stepped pyramid) —
  that you can **fly around** with keyboard + mouse. Faces between two solid voxels
  are not drawn; faces exposed to air are.
- **De-risks:** the world↔mesh↔render contract (architecture §4), basic camera +
  cross-platform input, and proves the pipeline end-to-end before we invest in the
  *fast* mesher (M2).

This milestone is **correctness-first, not performance-first.** It deliberately
uses a naïve mesher and an un-packed vertex; M2 makes both fast. Resisting the
urge to optimize here keeps the pipeline easy to debug.

## Scope

**In:**
- A minimal voxel data model: `BlockId`, `Section` (dense 32³).
- A **naïve** mesher: per solid voxel, emit a quad for each face exposed to air
  (cull shared faces between solids). No greedy merging.
- A `ChunkMesh` contract type (CPU-side vertex/index data + AABB) — the seam
  between `mesh` and `render`.
- Render path generalized from "one cube" to "upload and draw a `ChunkMesh`",
  with simple per-face flat shading so the geometry is readable.
- A **fly camera** + input (move + look), working on native and web.
- Module carve-out inside the single crate: `world`, `mesh`, `scene`, `render`
  (the existing `gfx`).

**Explicitly out (later milestones):**
- Greedy meshing, packed/compressed vertices → **M2**.
- Multiple chunks, inter-chunk face culling, frustum culling → **M2**.
- Palette compression, procedural generation, streaming → **M3**.
- Textures/materials, ambient occlusion → **M4**.
- Any aesthetic decisions — M1 shading is purely functional, *not* "the look".

## Design sketch

### `world` module
```rust
/// 0 is reserved for empty/air. Real block semantics come with the palette (M3).
pub struct BlockId(pub u16);
impl BlockId { pub const AIR: BlockId = BlockId(0); pub fn is_air(self) -> bool { self.0 == 0 } }

pub const SECTION_SIZE: usize = 32;

/// Dense 32^3 storage. Palette compression replaces the backing store in M3;
/// the get/set API is designed to survive that change.
pub struct Section { blocks: Box<[BlockId; SECTION_SIZE.pow(3)]> }
impl Section {
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId;
    pub fn set(&mut self, x: usize, y: usize, z: usize, b: BlockId);
    // linear index helper; bounds via debug_assert
}
```
- Dense cost is 64 KB/section (32³ × 2 B) — fine for one chunk; M3 shrinks it.
- Keep the indexing convention documented (e.g. `x + SIZE*(y + SIZE*z)`).

### `mesh` module
```rust
pub struct Vertex { /* position, face normal, color — see decision D1 */ }
pub struct ChunkMesh { pub vertices: Vec<Vertex>, pub indices: Vec<u32>, pub aabb: Aabb }

/// Naïve mesher: a face is emitted when a solid voxel borders air (or the chunk
/// boundary). Treats out-of-bounds as air for M1 (single chunk).
pub fn mesh_section(section: &Section) -> ChunkMesh;
```
- For each solid voxel and each of the 6 axis directions: if the neighbour is air
  (or outside the section), push a quad (4 vertices, 6 indices) with that face's
  normal and a colour from the block id.
- Consistent **CCW winding** per face so M2 can switch on back-face culling.

### `render` (current `gfx`)
- Replace the const cube buffers with buffers built from a `ChunkMesh`.
- Shader gains simple Lambert shading from a fixed sun direction using the face
  normal, so faces are distinguishable (flat, not smooth). This is *legibility*,
  not the visual identity.
- Keep depth buffer, resize, lost-surface handling as-is.

### `scene` module
- `Camera { position, yaw, pitch, fov }` → `view_proj()` via glam.
- Fly controls: **WASD/EQ** (or arrows) to move along camera axes; **mouse drag**
  to look (see decision D3). Per-frame integration with a fixed move speed; we
  have no clock yet (spike uses per-frame steps) — introduce a minimal frame
  delta-time here (decision D4).

### Module/seam rules
- `world` knows nothing about `mesh` or `render`.
- `mesh` depends on `world`, produces `ChunkMesh`, knows nothing about wgpu.
- `render` consumes `ChunkMesh`, never sees `Section`/`BlockId`.

## Decisions to resolve (with recommended defaults)

- **D1 — Vertex format v1.** *Recommend:* keep it simple and un-packed —
  `position: [f32;3]`, `normal: [f32;3]` (or a 0–5 face index), `color: [f32;3]`.
  Correctness over bytes; M2 introduces the real ≤8-byte packing with round-trip
  tests. *(Alternative: pack now — rejected, slows debugging the pipeline.)*
- **D2 — Block colours without textures.** *Recommend:* a tiny hardcoded
  `BlockId → RGB` table in `world` (or a `mesh` helper) for a handful of test
  blocks. Textures arrive in M4.
- **D3 — Look controls.** *Recommend:* **mouse-drag-to-look** + keyboard move for
  M1 — works identically on native and web with no pointer-lock plumbing. Pointer
  lock is a later polish item. *(Alternative: pointer lock now — more immersive but
  more cross-platform edge cases.)*
- **D4 — Time source.** *Recommend:* add a minimal per-frame delta-time
  (native `Instant`; web `performance.now()` via `web-time` or a small shim) so
  camera speed is frame-rate independent. Keep it tiny.
- **D5 — Test chunk shape.** *Recommend:* a stepped pyramid or hollow box built in
  code — visually unambiguous about which faces should/shouldn't appear, so the
  culling is verifiable by eye as well as by test.

## Tests

Pure-logic, in `mesh` and `world`:
- Single solid voxel in an empty section → **6 faces** (12 triangles, 24 verts).
- Two face-adjacent solids → **10 faces** (shared face culled both ways).
- A fully solid 2×2×2 block → only the **24 outer faces**, no interior faces.
- A fully solid section → only the surface faces; interior fully culled (count =
  the section's outer shell).
- Every emitted index references an existing vertex; index count is a multiple of 3.
- `Section` get/set round-trips; linear-index helper matches a reference formula.

(GPU upload / camera matrices are validated by the thing running, not unit tests —
per the testing strategy.)

## Risks & mitigations

- **Cross-platform input** (mouse capture differs) → mitigated by D3 (drag-look).
- **Web canvas focus / event wiring** → verify in the preview early; keep input
  handling in one place.
- **Scope creep toward greedy/packed** → explicitly deferred to M2; this brief is
  the guardrail.
- **Naïve mesher is slow on a full section** → acceptable for one chunk; it exists
  to be replaced and to serve as the M2 correctness oracle.

## Acceptance checklist

- [ ] `world`: `BlockId`, `Section` (dense 32³) with tested get/set + indexing.
- [ ] `mesh`: naïve `mesh_section` producing a `ChunkMesh`; face-count tests pass.
- [ ] `render`: draws an uploaded `ChunkMesh` with flat per-face shading; the
      hardcoded cube is gone.
- [ ] `scene`: fly camera with move + drag-look; frame-rate-independent speed.
- [ ] Runs on **native and web**; the preview shows the fly-around chunk.
- [ ] Module seams respected (`world` ⊥ wgpu; `render` ⊥ voxel types).
- [ ] CI green; docs/roadmap status for M1 flipped to done.
