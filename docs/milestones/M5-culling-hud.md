# M5 — Light to draw (occlusion culling + HUD)

> Status: **in progress** 🛠. Part of the ladder in [`../roadmap.md`](../roadmap.md).
> Builds on M2 (frustum culling) and M3 (streaming).

## Goal · Outcome · De-risk

- **Goal:** stop paying for geometry you can't see, and make the cost *visible* so we
  can steer perf — the pillar that keeps large worlds light.
- **Outcome:** an on-screen perf HUD (frame time, FPS, draw calls, triangles) and a
  visibility-graph "cave" cull layered on the existing frustum cull.
- **De-risks:** the perf-visibility loop, and the connectivity primitive the bigger
  worlds (3D terrain, caves) will lean on.

## Scope

**In:**
- **Perf HUD**: frame time + FPS (smoothed), drawn/total chunks, triangles, particles.
  On the web an overlay element; on native the window title (an in-render bitmap-font
  HUD is a later nicety). Throttled to a few updates/sec.
- **Visibility-graph connectivity**: per-section, a 6×6 "which faces connect through
  air" graph (flood-fill the air cells; two faces connect if their air regions touch).
  Pure logic, tested — including a cave scene where it culls hidden chunks.
- **Cave-cull traversal**: a BFS from the camera's chunk through the loaded set,
  stepping into a neighbour only via connected faces and within the frustum. Layered
  on frustum culling. **Safe fallback:** when the camera is outside the voxel volume
  (e.g. flying *above* a surface world — our current case), fall back to frustum-only,
  so it can never wrongly cull visible terrain.

**Out (later):**
- An in-render (bitmap-font) HUD on native — title bar is enough for now.
- GPU/Hi-Z occlusion — off-brand cost; the connectivity graph is the cheap win.

## Honesty note (current world)

Our world today is a **single surface layer** (`cy = 0`) viewed from a camera that
mostly flies **above** it. The visibility graph shines on caves / thick 3D worlds; on
an open surface seen from above, almost everything floods as visible, so the cave-cull
adds little over frustum culling *right now*. We still build + test the primitive (it's
correct and ready) and wire it with the safe fallback, but the **measurable** draw-call
reduction the HUD shows today is mostly **frustum** culling. The cave-cull pays off when
3D terrain / caves arrive. The connectivity reduction is proven in a unit test instead.

## Tests

- **Connectivity:** an empty section → all 6 faces mutually connected; a solid section →
  none; a section split by a solid wall → the two sides' faces don't connect; a straight
  air tube connects exactly its two ends.
- **Cave cull:** a synthetic scene (an enclosed, meshed-but-sealed chunk behind rock) →
  the traversal does not mark it visible; an open scene → all in-frustum chunks visible.

## Acceptance checklist

- [x] Perf HUD on screen (web overlay + native title), smoothed frame time + counts.
- [ ] Visibility-graph connectivity computed per section; logic tested (incl. a cave).
- [ ] Cave-cull traversal layered on frustum, with the safe above-world fallback.
- [ ] HUD shows the draw-call reduction (frustum today; connectivity proven in tests).
- [ ] Runs native + web; CI green; docs synced.

> Status: **in progress** 🛠 — HUD first, then the connectivity graph.
