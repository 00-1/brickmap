# M11 — Render hygiene & cheap wins (vendor-doc pass)

> **Status: done (2026-06-11).** The distilled, here-verifiable engine actions from the
> 2026-06-11 research pass ([`../research-gpu-perf.md`](../research-gpu-perf.md),
> [`../research-voxel-rendering.md`](../research-voxel-rendering.md)): a latent hitch
> class in the upload path, silent bandwidth-compression disablers, splat draw-order
> discipline, dissolve-fade correctness, and a storage fast path. Every item is either
> byte-identical-verifiable headless or unit-testable; nothing needs the reference
> hardware. Engine crates; no game-visible change. **As built:** headless render proven
> **byte-identical** pre/post (worktree baseline at the prior commit vs this one, `cmp`
> on the PNGs); two items were audits that found the code already clean — recorded as
> passes per Decision 1, not invented churn.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Close the five vendor-documented gaps: (a) mesh-upload allocation churn,
  (b) render-target usage flags that disable AFBC/UBWC/CCS compression, (c) discard-draw
  ordering, (d) dissolve fade quantization, (e) uniform-section fast path.
- **Demonstrable outcome.** Headless render + golden voxel-hash **byte-identical**
  throughout; M10 counters show upload-bytes/frame steady (no per-chunk buffer creation);
  a new unit test proves uniform sections skip storage/mesh/upload; the fade is exact at
  every level.
- **De-risks.** wgpu#1242-class periodic ~25 ms hitches (measured upstream — unthrottled
  buffer creation); ~33–50% mobile color-write bandwidth lost to one wrong usage flag;
  early-Z degradation from interleaved discard draws; dissolve ripple.

## Scope

**In:**
1. **Upload-path audit (`bm-render`/`bm-scene`).** No `create_buffer`/fresh-staging
   `write_buffer` per chunk upload mid-frame: static chunk meshes via `mappedAtCreation`
   (write once, no staging hop) or a pooled/`StagingBelt` path for streamed updates;
   pre-allocate pools; never create buffers inside the frame. Keep Veloren's
   fractional per-frame upload budget pattern in mind if intake needs throttling (we have
   the M8a time budget — verify it bounds *uploads*, not just meshing).
2. **Render-target usage-flag audit.** Every offscreen target declares only
   `RENDER_ATTACHMENT | TEXTURE_BINDING` (no `STORAGE_BINDING`, no mutable-format) —
   one-line changes that preserve AFBC/UBWC/Apple compression on the targets we care
   about. Document the rule in `performance.md` §4.
3. **Discard draw-order discipline.** Verify (and pin with a comment + a draw-order test
   if cheap): all opaque depth-writers first (front-to-back — exists), **all
   discard-using draws (splats, dither passes) last among depth-writers, never
   interleaved**, blended/overlay after. Record the `melt` mode's early-Z tax in the
   toggle's doc (terrain-shader discard while active — opt-in look, known cost).
4. **Dissolve fade correctness.** Quantize every Bayer-threshold fade factor to the
   matrix's level count (17 for 4×4) so slow fades step cleanly; confirm the mesh/points
   halves of any crossfade use **complementary masks** (each pixel shows exactly one
   side). Headless A/B at fixed fade levels.
5. **Uniform-section fast path (`bm-world`/`bm-mesh`).** All-air/all-solid sections store
   a single palette entry and no index array; skip meshing (emit boundary faces only via
   neighbors) and skip upload where already implied. Unit-tested; memory win recorded.

**Out:** two-stream vertex split, fp16 shader variants, web render bundles, BFS culling
upgrades, region arenas, the mesh-mip far-LOD ring — all real, all researched, but their
wins are device-/browser-measurable, so they bundle with **M8b** (or an M12 once numbers
exist). Listed in `performance.md` §5 so they aren't lost.

## Decisions to resolve (pinned defaults)

1. If the upload path already pools correctly (possible — M8a touched streaming), items
   1 becomes a *verified-no-op*: record the audit result in the brief and move on.
   **An audit that finds nothing is a pass, not a failure** — don't invent churn.
2. Golden voxel-hash AND headless render must be byte-identical for items 1–3, 5; item 4
   may change dissolve-band pixels only at non-default fade settings (golden default
   untouched).

## Tests

Uniform-section storage/meshing fast path (round-trip, neighbor faces, memory size);
fade quantization levels exact; complementary-mask property; golden voxel-hash + headless
render byte-identical; M10 budget gates green (and upload-bytes steady on the HUD).

## Acceptance checklist

- [x] Upload path audited: no mid-frame buffer creation; static meshes `mappedAtCreation`
      or pooled; result recorded (including "already clean" if so).
      **As built — clean with one conversion + notes.** Chunk/structure meshes already go
      through `create_buffer_init` (= `mappedAtCreation`: write once, no staging hop) and
      intake is already throttled (`STREAM_UPLOADS = 4`/frame native; the M8a per-frame
      *time* budget on web bounds the whole mesh+upload step, uploads included). Splat,
      UI-rect, and particle buffers were already pooled (pow2-grow + `write_buffer`).
      **Converted:** the route-overlay path created a fresh vertex buffer per route edit —
      now pooled (`overlay::set_lines_pooled`, pow2 grow, count-0 clears). **Noted, not
      churned:** HUD text rebuilds its glyph texture on text change (~a few times/s in
      busy moments) — small (256×128), measured-not-hot, left for an M8b look.
- [x] All render targets minimal-usage; rule documented in the charter.
      **As built — verified-no-op (a recorded pass).** Audited every offscreen target:
      scene color + post ping-pong are `RENDER_ATTACHMENT | TEXTURE_BINDING`, depth is
      `RENDER_ATTACHMENT`-only, all texture binds are `TEXTURE_BINDING | COPY_DST`; no
      `STORAGE_BINDING`, no `view_formats` anywhere. Rule recorded as charter §4 rule 8
      (`performance.md`).
- [x] Discard draws ordered last among depth-writers (pinned + noted); melt tax documented.
      **As built.** Particles + ship hull (plain opaque depth-writers) were drawn *after*
      the discard-using splat passes — moved before them, so the depth-writer order is now:
      opaque terrain/structures (front-to-back) → particles → ship → all discard draws
      (foliage/structure/creature splats, text) → blended/overlay last. Discipline pinned
      with a comment block at the draw site (`gfx.rs`). The `melt` toggle's early-Z tax
      (terrain discard while active) documented at the toggle.
- [x] Fade quantized + complementary masks verified (headless A/B at fixed levels).
      **As built — with two recorded judgment calls.** The M7 distance-melt fade is
      quantized to the 4×4 Bayer's 17 levels in `shader.wgsl`, mirrored + unit-tested in
      Rust (`bm_render::quantize_fade`; the test also asserts the shader still carries the
      mirrored expression). Melt is **opt-in** (off by default) so the golden headless
      render is untouched — verified byte-identical. *Skipped on purpose:* the relic
      Bayer dissolve (`t*0.9` in the default path) — quantizing it would move
      golden-covered pixels for zero player-visible win; recorded here instead.
      *N/A:* complementary mesh/points crossfade masks — the M7 far-point fade-in isn't
      wired yet (M8b decides if it pays); the mask rule is noted in the M7 plan for then.
- [x] Uniform-section fast path landed + tested; memory delta recorded.
      **As built.** `bm-world`: a uniform section stores **one palette entry and zero
      index bits** (`bits == 0`, empty index vec) — `new()` starts there, `set` stays
      there while writes match, `palette_index` widens lazily on the first second block.
      Memory: ~8 bytes vs the old 4 KiB index array per uniform section (air oceans above
      ground, solid floors below). `uniform()` exposes the fast-path state. `bm-mesh`:
      `greedy_mesh_section_with` early-outs for uniform-air and for uniform-solid fully
      enclosed by uniform-solid neighbors — **byte-identical** results by construction
      (`ChunkMesh::default()` == the loop's output for those cases, AABB included; both
      asserted in tests).
- [x] Golden voxel-hash + headless render byte-identical; CI green (fmt / clippy -D /
      tests / wasm); boundary intact; roadmap M11 entry.
      **As built.** Headless PNG `cmp`-identical against a pre-M11 worktree baseline;
      golden voxel-hash test unchanged and green; 207 tests, fmt/clippy/wasm clean.
