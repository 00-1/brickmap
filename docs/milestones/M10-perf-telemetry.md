# M10 — Perf telemetry & budget regression gates

> **Status: planned — do NOT build until the channel directs it** (queued after G9, before
> G10, so the gates exist before the next streamed-content layer lands). From the
> [performance charter](../performance.md) §5: make the frame's cost **legible and
> CI-assertable** without the reference hardware. Engine-side counters in `bm-render`/
> `bm-scene` (generic), game-side budget pins in `scraped-again` tests.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Frame-cost **counters** (draw calls, triangles, live splats, upload bytes/frame,
  meshing-queue depth / inline-mesh ms, sim-active cells) collected every frame, shown on
  the M5 HUD, queryable headless — and **CI budget tests** that pin them at reference scenes
  so a content regression fails the build here, months before a human feels a stutter.
- **Demonstrable outcome.** The HUD shows the counter block live; `cargo test` includes
  budget assertions that render/walk the reference scenes headless and fail if any counter
  exceeds its pinned budget; the recorded actuals land in `performance.md` §6 (replacing the
  estimate column with measured-on-llvmpipe numbers + headroom).
- **De-risks.** Threat #1 in the charter: game-content accretion silently eating the weak-
  hardware headroom. Also makes the eventual M8b hardware session far more productive (the
  human reads numbers off the HUD).

## Scope

**In:**
- **Counters (engine, generic):** a small `RenderStats`/frame-stats struct populated by the
  existing draw path — draw calls, triangles (sum of index counts), instance/splat count,
  buffer-upload bytes this frame, mesh-queue depth + inline-meshing ms (the M8a budget is
  already tracked — surface it), dynamic-res current scale. No game concepts; `bm-render`/
  `bm-scene` own what they already know.
- **Game counters:** live splat budget per consumer (foliage / wisps / giants / shards
  later) — each call site declares a const budget; a game-side aggregate.
- **HUD:** extend the M5 HUD block (behind the existing HUD toggle; full-res text as today).
- **Headless query:** the `screenshot` tool (or a sibling `stats` bin) prints the counter
  block after N warm frames at a given camera — machine-readable (one `key=value` per line).
- **CI budget tests:** for 2–3 reference scenes (seed 1337 default vantage; a forest-dense
  vantage; a giant close-up), assert each counter ≤ budget. Budgets live in one
  `budgets.rs` (or toml) with the charter table; failing message says which counter and by
  how much.
- **Record actuals:** update `performance.md` §6 with measured llvmpipe actuals + chosen
  headroom in the same commit (docs-in-lockstep).

**Out:** GPU timestamp queries (wgpu timestamps are flaky across backends — wall-clock +
counts suffice here); any optimisation work the numbers might suggest (separate, directed);
M8b device profiling (hardware-gated, unchanged).

## Design sketch

- `bm-render`: accumulate into a `FrameStats` reset per frame (counts incremented where
  draws/uploads already happen — cheap adds, no timing syscalls in hot loops beyond what
  M8a already does); expose `stats() -> FrameStats`.
- Splat consumers: the game's `set_*_points` call sites already know their counts — collect
  into a game-side table keyed by consumer name.
- Reference-scene tests: reuse the headless harness (D1 path) — pump N frames at a fixed
  camera, read stats, assert. Keep them in the game crate (they know the scenes); mark
  `#[ignore]`-not — they should run in normal CI (llvmpipe is in CI already for the golden
  render).
- Determinism: counts at a fixed camera/seed are deterministic (same stream set, same
  meshes); upload-bytes settles after warmup — assert on the post-warmup steady frame.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Counts + wall-clock only, no GPU timestamps** (portability; CI determinism).
2. **Budgets pinned at measured-actual + ~40% headroom** (loose first pins — they exist to
   catch *step changes*, not to be a straitjacket; tighten at M8b).
3. **Three reference scenes** (default vantage / forest-dense / giant close-up), chosen by
   the builder from the existing headless vantage repertoire, documented in `budgets.rs`.
4. If a counter is genuinely non-deterministic in CI (e.g. async mesh timing), assert a
   range or skip *that counter* in CI with a note — don't drop the whole gate.

## Tests

The milestone *is* tests, plus: `FrameStats` unit coverage (counters reset/accumulate);
the headless stats query emits parseable output; budget failure message is actionable.
Golden voxel-hash + headless render unchanged (counters are read-only).

## Risks & mitigations

- **Flaky CI gates** → post-warmup steady-frame assertion + ranges where needed + one place
  to adjust budgets (a failing gate must never be "fixed" by deleting it — adjust with a
  recorded reason).
- **Counter overhead** → increments only; no per-draw syscalls; HUD formatting only when
  visible.

## Acceptance checklist

- [x] Engine stats (generic — the existing `DrawStats`, extended): draw calls (exact in-pass +
      one per active stage), triangles, splats, **upload bytes/frame** (accumulated at every
      buffer/texture setter, harvested per `render()`), internal-res divisor (dyn-res visible);
      surfaced on the M5 HUD (`… dc · … kB/f ÷N`). Headless query = the `stats` bin (see As-built).
- [x] Game-side per-consumer splat budgets declared (`budgets.rs`: foliage / structures / wisps);
      the aggregate is the HUD `splats` counter + `SceneStats::splats()`.
- [x] CI budget tests on 3 reference scenes (`default` spawn / densest-`forest` cell / nearest-
      `giant`) against `budgets.rs`; a failure names the scene, the counter, and the overage.
- [x] `performance.md` §6 actuals recorded in the same commit (measured table + pinned budgets);
      roadmap M10 entry.
- [x] Golden voxel-hash + headless render unchanged (counters are read-only); CI green (fmt /
      clippy -D / 194 tests / wasm); boundary intact (no game concept in engine stats).

## As-built (2026-06-11) — decisions + findings

1. **The headless query is content-counters, not a frame loop.** The CI-assertable counters
   (chunks/tris/splats/labels) are computed **CPU-side** from the same builders the renderer is
   fed (`build_chunk_instance_cached`, the shared `structure_geometry`, `inscriptions_near`) —
   fully deterministic, no GPU/llvmpipe needed, no warm-frame flake. The live-only counters
   (upload bytes, draw calls incl. passes, dyn-res) are HUD-only and **not CI-gated** (the
   brief's Decision 4); `stats` prints the content block per scene as `key=value`.
2. **The gate found a real number on day one:** the charter's 1.5 M-triangle *estimate* was
   already exceeded by the actual streamed set (1.79–1.89 M). Budgets are pinned at **measured
   actual + ~40%** per Decision 2 (tris ≤ 2.6 M, splats ≤ 170 k, mesh-draws ≤ 1,200, labels
   ≤ 16) — recorded in `performance.md` §6 with the measured table.
3. **Finding for M8b:** solid colossi dominate mesh-draw instances — the forest scene carries
   **663 structure sections** (vs 169 terrain chunks). A merge/section-cap on solid-giant
   meshing is a real future lever; recorded in §6.
4. `structure_geometry` was factored out of `update_structures` so the budget counters count
   **exactly** what the app renders (one source of truth); the cached chunk builder was
   promoted from test/wasm-only to all targets for the same reason.
5. **Cost:** the 3-scene gate adds ~40 s to debug CI (meshing ~500 chunks + structures). Within
   tolerance; revisit (e.g. release-profile tests) if CI time becomes a problem.
