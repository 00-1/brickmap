# M11 — Render hygiene & cheap wins (vendor-doc pass)

> **Status: planned — do NOT build until the channel directs it** (queued after G11). The
> distilled, here-verifiable engine actions from the 2026-06-11 research pass
> ([`../research-gpu-perf.md`](../research-gpu-perf.md),
> [`../research-voxel-rendering.md`](../research-voxel-rendering.md)): a latent hitch
> class in the upload path, silent bandwidth-compression disablers, splat draw-order
> discipline, dissolve-fade correctness, and a storage fast path. Every item is either
> byte-identical-verifiable headless or unit-testable; nothing needs the reference
> hardware. Engine crates; no game-visible change.

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

- [ ] Upload path audited: no mid-frame buffer creation; static meshes `mappedAtCreation`
      or pooled; result recorded (including "already clean" if so).
- [ ] All render targets minimal-usage; rule documented in the charter.
- [ ] Discard draws ordered last among depth-writers (pinned + noted); melt tax documented.
- [ ] Fade quantized + complementary masks verified (headless A/B at fixed levels).
- [ ] Uniform-section fast path landed + tested; memory delta recorded.
- [ ] Golden voxel-hash + headless render byte-identical; CI green (fmt / clippy -D /
      tests / wasm); boundary intact; roadmap M11 entry.
