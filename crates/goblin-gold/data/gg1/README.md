# GG1 content data (one-way synced from `00-1/halves:content/gg1/`)

A snapshot of Goblin Gold v1's content-as-data export — the cross-repo **data seam**
(research §"share DATA not code"). The brickmap port CONSUMES this; it never embeds GG's JS.

- `modes.json` — per-mode metadata (id/name/tag/group/expr/unlock + `masterSecs`) + the JS
  `transform` string (NOT executed here) + the raw `pool`.
- `parity-vectors.json` — the deterministic `{p,a}` set per mode (the behavioural contract). A
  re-implementation is correct iff it reproduces these exactly (`transforms.rs` does, for all 46).
- `transforms.md` — the source of every transform fn (reference for the Rust re-impl).
- `guides.json` (T230) — per-topic "how to beat it" help (intro/tips/example).
- `collectibles.json` (T230/T232) — the collectible catalogue: `total`, per-category counts, the
  **collector ladder** (`collectorLadder`), and the full `catalog`. `collector.rs` reads the
  ladder; the capstone must stay strictly below `total` to stay reachable.
- `balance.json` (T230/T232) — tuning constants: gold scalars + the Arena enemies/heroes data.
- `earning.json` (T233) — the earning **tunables** (init fraction, spark/speed thresholds, the 23
  ranks, gold/momentum/meta/topics/collector thresholds). The award **logic** itself is re-impl'd in
  `earning.rs` from `collectibles.js` and proven against:
- `earning-vectors.json` (T233) — the `{ctx → awarded keys}` behavioural contract (rank-index grid,
  46 modes × 13 scenarios, and the collector/topics/meta/gold/momentum families). A re-impl is
  correct iff it reproduces these (`earning.rs` does — set-equality, like the transforms test).

**Sync:** regenerate in halves (`node tools/content-export.js`), then re-copy `content/gg1/*`
here from `origin/main`. Halves' `test/content-parity.test.js` keeps the halves copy locked to the
live runtime; the brickmap re-impls (`transforms`/`progression`/`collector`) are gated against
these files. **Re-fetch + check `origin/main` before assuming a file is missing** (it isn't a
working-tree thing).
