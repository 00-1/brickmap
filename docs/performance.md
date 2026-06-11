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
- **Cadence:** budgets reviewed when M8b lands, then at every content milestone that adds a
  streamed layer or splat consumer.

## 6. Budget table — M10 measured actuals (M8b will re-baseline on device)

**Measured 2026-06-11** (M10; seed 1337, the three reference scenes from `budgets.rs`, via
`cargo run -p scraped-again --bin stats`). Counters are **full streamed-set totals**
(camera-independent: the whole 13×13 chunk keep set + in-range structures/labels — culling
only *reduces* them live, and content regressions move *these*):

| Scene | tris_total | splats_total | mesh_draws | labels |
|---|---|---|---|---|
| default (spawn) | 1,794,750 | 123,358 | 169 | 7 |
| forest (densest cell ±2 km) | 1,888,548 | 78,309 | **832** | 8 |
| giant (nearest colossus) | 1,739,366 | 104,294 | 169 | 9 |

**CI-gated budgets** (worst actual + ~40% headroom; `crates/scraped-again/src/budgets.rs` —
a failing gate is adjusted *with a recorded reason*, never deleted):

| Counter | Budget | Worst actual | Basis |
|---|---|---|---|
| Triangles (terrain + solid structures) | ≤ 2.6 M | 1.89 M | measured (replaces the 1.5 M estimate) |
| Live splats (foliage + structures + wisps) | ≤ 170 k | 123 k | measured |
| Mesh draw instances (chunks + structure sections) | ≤ 1,200 | 832 | measured — **solid giants = 663 sections in the forest scene**; a real M8b lever (merge/section-cap candidate) |
| Inscription labels in range | ≤ 16 | 9 | measured |
| Foliage splats (consumer budget) | ≤ 170 k | 121.5 k | measured |
| Structure points (consumer budget) | ≤ 4 k | 2.8 k | measured |
| Wisps (consumer cap) | ≤ 64 | ~7–28 live | cap |

**Live-only counters** (HUD via the engine `DrawStats`; not CI-gated — timing-dependent, per
the M10 brief's Decision 4): upload bytes/frame (`kB/f` on the HUD; the ≤ ~1 MB steady-cruise
design figure stands as the watch level), draw calls incl. passes (`dc`), internal-res divisor
(`÷N`, dynamic-res visible), inline mesh time (web, ≤ 3 ms — the M8a budget, already enforced
in code).

M8b re-baselines all of this on the reference devices and tightens the headroom.
