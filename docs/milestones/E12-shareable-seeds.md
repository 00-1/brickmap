# ✨ E12 — Shareable seeds & permalinks

> Status: **done** ✅. Exploration/feature rung in [`../roadmap.md`](../roadmap.md);
> research in [`../exploration-backlog.md`](../exploration-backlog.md) §L. Pulled forward
> (ahead of the foliage pivot) — small, deterministic, high-delight.

## Goal · Outcome · De-risk
- **Goal:** share a world (and a view of it) via a URL or seed; type a seed; get a random
  one; restore on load. Reproducible because generation is deterministic.
- **Outcome:** "copy link" → paste → same world + camera; a seed box + 🎲 + seed-of-the-day.
- **De-risks:** runtime seed plumbing, and **cross-platform determinism** (the real risk).

## Scope (sliced)
1. **Runtime seed** — replace the `WORLD_SEED` const usage in the live app with an
   `App.seed` field, plumbed through the loader / sand / camera; a `set_seed` that resets
   streaming so the world regenerates. *(Headless demo keeps the fixed seed.)*
2. **`share` codec** (pure, no web-sys; shared web+native) — `ShareState { seed, pos, yaw,
   pitch, wobble, color_steps, toggles }` ⇄ a URL-fragment string `v=1&s=…&x=…&t=<hex>`.
   Robust decode (missing keys → defaults; tolerates leading `#`). `seed_from_text` (numeric
   or fold text→u32 via our hash). **Done first** — fully unit-testable.
3. **Web** — read `location.hash` on startup (before building the world); a seed `<input>`,
   🎲 random, seed-of-the-day, **copy-link / copy-seed** buttons; show the seed on the HUD.
   `set_seed`/`seed_from_text`/`current_share` wasm exports (reuse the `set_wobble` wiring).
   Copy-on-demand first; **defer** live URL updates (Safari `replaceState` throttle).
4. **Native** — `--seed <int|text>`, `--share <blob>`, `--daily`; print the share string on
   a keypress.
5. **Determinism guard** — a golden voxel-hash test; document the cross-platform caveat
   (the integer `hash` is portable; the `f32` noise path *may* drift but `height().round()`
   likely masks it). Fixed-point noise is the fallback only if the golden test fails.

## Out
- Live-as-you-fly URL updates (throttle-fragile) — copy-on-demand is enough for v1.
- Encoding edits/builds in the link — that's E14 (seed + sparse deltas).

## Tests
- `share`: encode→decode round-trips; decode tolerates missing keys + leading `#`;
  `seed_from_text` is deterministic and numeric-vs-text behave as specified.
- Runtime seed: two seeds produce different terrain (worldgen already tested per-seed).
- Determinism: a fixed seed hashes to a known voxel grid (golden) — guards accidental
  worldgen changes; cross-target check noted as a follow-up (needs wasm-in-CI).

## Acceptance checklist
- [x] Runtime seed (set/reset re-streams); headless still renders. *(`App.seed` +
  `ChunkLoader` seed/epoch; `set_seed` tears down + re-streams; stale-epoch meshes
  discarded. Headless keeps `WORLD_SEED`.)*
- [x] `share` codec + `seed_from_text`, unit-tested. *(+ `date_utc_from_unix_secs` so
  native `--daily` matches the web's UTC date.)*
- [x] Web: seed input / random / daily / copy-link / copy-seed; restore on load (hash →
  world + page controls); seed on HUD.
- [x] Native: `--seed <int|text>` / `--share <blob>` / `--daily`; `P` prints the share
  string, `R` reseeds random.
- [x] Golden voxel-hash test; determinism caveat documented. *(`golden_voxel_hash_is_stable`
  fingerprints a 5×5 chunk grid via FNV-1a over integer block ids; caveat in the test +
  below.)*
- [x] CI green; docs synced. *(green locally: fmt + native clippy + tests + wasm build;
  CI runs the same on push.)*

## Determinism caveat (what "identical worlds" actually promises)
The world is reproducible **on the same target**. Across targets (native ↔ wasm) the
**integer** path — `worldgen::hash`, block ids, the share codec — is portable, and the
golden test guards it. The **`f32` noise** path (`value_noise`/`fbm`) *could* drift
(FMA/libm/rounding differences), but `height()` ends in `.round().clamp(...) as u32`, which
masks sub-unit drift in almost all columns — so worlds should match in practice. The
remaining risk is a column whose height sits exactly on a `*.5` rounding boundary. **Not yet
proven cross-target** (needs wasm-in-CI); if it ever bites, the fix is fixed-point noise.
Policy: treat the `Event`/share format as append-only and freeze worldgen per seed version,
so old links keep reproducing old worlds.

> Status: **done** ✅ — runtime seed + codec + web/native wiring + golden determinism test
> all landed. (Live-web verification of the buttons is a human check; logic is unit-tested.)
