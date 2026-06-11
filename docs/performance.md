# Performance charter — weak-hardware-first, enforced

> The standing contract for keeping the engine radically performant **while the game grows**.
> Targets and budgets originate in [`design.md`](design.md) §8; this doc is the *operational*
> side: what protects the budgets today, what threatens them next, the rules every new piece
> of work must follow, and the measurement plan. 2026-06-11 planning session.

## 1. The contract (restated)

Reference hardware, in priority order — if it's slow here, it's broken:
1. **No-dGPU PC** — Intel Iris Xe (or AMD 660M) @ 1080p: **60 fps target, 30 floor**.
2. **Mid-range phone** — Pixel-6a-class (Mali-G78), native APK: 60 target, 30 floor.
3. **Web browser** — WebGPU, WebGL2 fallback; slower accepted, **never broken**.

The binding constraint is **bandwidth** (tiler GPUs + iGPUs sharing system RAM), then CPU
frame cost (web's main-thread meshing), then raw ALU — in that order. Every design choice
already follows this (greedy meshes, palette sections, packed vertices, low-res internal
buffer, splats-for-far); every *future* choice must too.

**Honest status: the budgets are still estimates.** M8b (profiling on the reference devices)
is hardware-gated; the one real-device datapoint is "release APK is fast" on the human's
phone. Until M8b lands, our defence is (a) designs that are cheap *by construction* and
(b) the regression gates below.

## 2. What protects the budget today

- **Geometry:** greedy meshing; packed 4–8-byte vertices; front-to-back opaque sort
  (early-Z); frustum + cave culling; solid-structure Bayer LOD dissolve; distance melt.
- **Bandwidth:** palette-compressed sections; depth `storeOp: Discard`; the **pixel-scale
  dial** (low-res internal buffer — the biggest single lever); **dynamic resolution**
  (frame-time-adaptive, hysteresis'd).
- **CPU/streaming:** rayon meshing off-thread (native); generated-section cache + per-frame
  inline-meshing time budget (web); per-cell cached + once-per-frame-budgeted structure
  generation (the E18 lesson); active-set CA sim.
- **Process:** golden voxel-hash + headless render on every commit; per-feature runtime
  toggles (D6 norm) so any feature's cost is A/B-able; clippy/CI green-trunk discipline.

## 3. What threatens it next (ranked)

1. **Game-content accretion** — the new frontier. Shards (G10), denser inscriptions (G9),
   giants, wisps, wind, weather, water, two agents: each adds per-frame streamed scans,
   splat counts, and live-loop work *in the game crate*, where engine discipline doesn't
   automatically reach. Death by a thousand cheap features is now likelier than any single
   expensive one.
2. **Unmeasured budgets** — without M8b numbers we can't see headroom shrink; we'd learn
   from a human "it stutters" report months late.
3. **Splat overdraw** — the one known sharp edge of the points path: big near billboards.
   Bounded today (ethereal recession pushes points away; sizes shrink with distance), but
   every new splat consumer (shards!) must respect the budget.
4. **Web main-thread meshing** — structurally capped; fine while per-frame budgets hold,
   but content growth eats the same budget.

## 4. The rules (every new feature, engine or game)

1. **Streamed world content** is per-cell seeded, cached on entry, generated under a
   per-frame budget, and dropped on exit — never regenerate-per-frame, never all-at-once
   (the E18 pattern is the template; G9/G10 briefs already require it).
2. **Splat consumers declare a count budget** at the call site (a const, asserted in tests);
   the total live splat budget is tracked in the M10 counters.
3. **No new full-resolution passes.** Post works at internal res; overlays/HUD are the only
   full-res draws. Anything else needs a charter amendment with numbers.
4. **Live-loop features ship with a toggle** (D6 norm) and idle at ~zero cost when off.
5. **Per-frame game logic is O(near set), never O(world)**; distant agents run abstracted
   (the G8 away-agent pattern).
6. **Golden-neutral by default**: new visuals flag-gated or content-keyed so the golden
   voxel-hash + headless render stay the regression anchor.
7. **Measure before optimising, but design cheap by default** — no speculative complexity
   chasing unmeasured wins (the FSR lesson: it fought the aesthetic and the numbers weren't
   there to justify it).
8. **Render targets declare minimal usage** (M11): offscreen colour targets are
   `RENDER_ATTACHMENT | TEXTURE_BINDING` only — never `STORAGE_BINDING`, never mutable
   `view_formats` — so AFBC/UBWC/CCS framebuffer compression stays enabled (one wrong flag
   silently costs ~33–50% colour-write bandwidth on mobile). Audited 2026-06-11: all current
   targets comply.

## 5. Measurement plan

- **M10 — perf telemetry & budget gates (dispatchable now, no hardware needed):** make the
  frame's cost *legible and CI-assertable* — counters for draw calls, triangles, live splats,
  buffer-upload bytes/frame, meshing-queue depth, sim-active cells; exposed on the HUD
  (extending the M5 HUD) and queryable headless; **CI budget tests** pin them at reference
  scenes so content growth that blows a budget fails the build *here*, before any human
  feels it. Brief: [`milestones/M10-perf-telemetry.md`](milestones/M10-perf-telemetry.md).
- **M8b — the real measurement (hardware-gated, unchanged):** profile on Iris Xe + the
  phone, record real frame times into design §8, tune dynamic-res thresholds, then wire the
  M7 far-LOD if the numbers say it pays. The M10 counters make that session dramatically
  more productive (the human reads numbers off the HUD instead of guessing).
- **M11 — render hygiene (dispatchable now):** the here-verifiable engine actions from the
  2026-06-11 vendor-doc research ([`research-gpu-perf.md`](research-gpu-perf.md),
  [`research-voxel-rendering.md`](research-voxel-rendering.md)): upload-path audit (the
  wgpu#1242 hitch class), render-target usage-flag audit (AFBC/UBWC compression
  preservation), discard-draw ordering discipline, dissolve-fade quantization,
  uniform-section fast path. Brief: [`milestones/M11-render-hygiene.md`](milestones/M11-render-hygiene.md).
- **Banked for M8b / a numbers-gated M12** (researched, real, device-measurable only):
  two-stream vertex split (position-first — Mali IDVS + Intel binner); fp16-first shader
  variants (2× ALU on Mali/Xe; dual-path — Adreno browsers lack `shader-f16`); web render
  bundles keyed on chunk-set (the big web-submission lever); BFS culling upgrades (Sodium
  direction masks + angle test + step penalties; then Vintage-Story perimeter raycasts if
  over-visibility shows); region vertex arenas; and the **mesh-mip far-LOD ring** (2×/4×
  vertex-color skirted downsamples *before* the point regime — the research's amendment to
  the M7 plan; per-point hashed fade for the point ring, screen-Bayer kept for the mesh
  half). FSR-class upscalers are now **closed on technical grounds** (EASU's input
  contract bans dither; temporal upscalers' jitter destroys pixel stability; ~2–6 ms cost
  on our hardware class) — nearest blit stands.
- **Mobile bandwidth budget (vendor planning figure):** DRAM ≈ 80–100 mW per GB/s against
  a ~1 W mobile GPU budget ⇒ target **well under ~100 MB/frame** total at 60 fps; ground
  truth = Streamline "Output External Read/Write Bytes" on the Pixel 6a at M8b. Iris Xe
  ceiling ≈ 0.7–0.8 GB/frame, *shared with our meshing threads*.
- **Cadence:** budgets reviewed when M8b lands, then at every content milestone that adds a
  streamed layer or splat consumer.

## 6. Budget table — M10 measured actuals (M8b will re-baseline on device)

**Measured 2026-06-11** (M10; seed 1337, the three reference scenes from `budgets.rs`, via
`cargo run -p scraped-again --bin stats`). Counters are **full streamed-set totals**
(camera-independent — culling only reduces them, and content regressions move *these*):

| Scene | tris_total | splats_total | mesh_draws | labels |
|---|---|---|---|---|
| default (spawn) | 1,794,750 | 123,358 | 169 | 7 |
| forest (densest cell ±2 km) | 1,888,548 | 78,309 | **832** | 8 |
| giant (nearest colossus) | 1,739,366 | 104,294 | 169 | 9 |

*(G10 added the shard consumer — counted + gated in `budgets.rs` as `shard_splats`,
budget ≤ 400.)*

**CI-gated budgets** (worst actual + ~40% headroom; `crates/scraped-again/src/budgets.rs` —
a failing gate is adjusted *with a recorded reason*, never deleted):

| Counter | Budget | Worst actual | Basis |
|---|---|---|---|
| Triangles (terrain + solid structures) | ≤ 2.6 M | 1.89 M | measured (replaces the 1.5 M estimate) |
| Live splats (foliage + structures + shards + wisps) | ≤ 170 k | 123 k | measured |
| Mesh draw instances (chunks + structure sections) | ≤ 1,200 | 832 | measured — **solid giants = 663 sections in the forest scene**; an M8b merge/section-cap lever |
| Inscription labels in range | ≤ 16 | 9 | measured |
| Foliage splats (consumer) | ≤ 170 k | 121.5 k | measured |
| Structure points (consumer) | ≤ 4 k | 2.8 k | measured |
| Shard splats (consumer, G10) | ≤ 400 | ~175 | measured |
| Wisps (consumer cap) | ≤ 64 | ~7–28 live | cap |

**Live-only counters** (HUD via the engine `DrawStats`; not CI-gated — timing-dependent):
upload bytes/frame (`kB/f`), draw calls incl. passes (`dc`), internal-res divisor (`÷N`),
inline mesh time (web, ≤ 3 ms — enforced in code).

M8b re-baselines all of this on the reference devices and tightens the headroom.
