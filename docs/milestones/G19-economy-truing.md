# G19 — Economy truing (measured pacing + rarity gates + nav-wiring fixes)

> **Status: Part A landed (G19a) — Part B open.** The consequences of
> the first quantitative playtest ([`../pacing-analysis.md`](../pacing-analysis.md) — the real
> loop, 6 seeds, 60 sim-min each, driven through the D11 seam). Two parts: **(A) wiring
> bugs** that make shipped nav/expedition automation dead (fix first, own commit), and
> **(B) measured constant truing + the unimplemented half of the human's rarity decision +
> envelope regression tests.** Game-side; no engine change.

## A — URGENT wiring fixes (own commit; D11 asserts)

1. **`arrived_at` never fires in flight** (`lib.rs:1718`; babysitter-verified): 3D distance
   + `ARRIVE_RADIUS=12` vs `CRUISE_HEIGHT=22` → the vertical gap alone exceeds the radius,
   and Seek's turn clamp forces a ≥17 u orbit. `on-arrive`, automated `run(foot)`, and the
   expedition-from-air are **dead** (measured 0/hour). **Fix:** horizontal distance
   (`(t - pos)` with y zeroed) + `ARRIVE_RADIUS: 12.0 → 20.0` (> the 17.3 u orbit).
2. **Seek+collect deadlock:** with `seek` nav, once every in-cone site is known no scan
   event fires, so on-scan `collect` never runs — the ship orbits one uncollected site
   forever (economy flatline, measured). **Fix (pick the cleaner):** scan re-hits
   known-but-uncollected sites, **or** arrival dispatches a collect act. Must compose with
   fix 1 (seek → arrive → collect → move on: the loop the vocabulary advertises).
3. **Auto-`deposit` on the expedition's Return→Idle transition** (foot routines don't tick
   between back-to-back expeditions, so `when(carry) → deposit` can starve).
4. **D11 regression asserts:** an expedition-from-flight scenario that goes through the
   *real* `on-arrive` path (the gap the existing scenario missed — it kicked `run(foot)`
   directly); the seek→arrive→collect loop makes progress on a known-site field.

### As built (Part A, G19a)

- **Fix 1** as prescribed: `arrived_at` measures XZ distance (`(t - pos).with_y(0.0)`),
  `ARRIVE_RADIUS` 12 → 20 (> the 17.3 u seek orbit). One note against the brief: on main
  the function sat at `lib.rs:1719` (off by one from the quoted `lib.rs:1718`).
- **Fix 2 — option (a) chosen (scan re-hits known-but-uncollected sites):** a scan pulse
  now counts *any* uncollected site in the cone as a hit (flicks + map pins still fire only
  for newly-known ones, so the visible sweep stays calm). Chosen over (b) because it is the
  smaller mechanism (one filter in `scan_from`), it keeps `collect` authored rather than
  implicit (arrival dispatching a collect nobody wired would bypass the routine vocabulary),
  and it un-deadlocks *both* navs — under `drift` too, a site scanned once from beyond
  collect reach used to be permanently uncollectable. It composes with fix 1: seek closes on
  the site, the pulse re-hits it in the approach cone, `on-scan → collect` takes it
  (reach 45 > the ~30 u cruise-height slant), the target advances — arrive → collect → move
  on. Authored `on-arrive → collect` also works now, purely via fix 1.
- **Fix 3** as prescribed: `advance_expedition` deposits the walker's carry into the site
  cache on the Return→Idle edge (only when it carries something — the honest no-op stays).
- **Fix 4 (D11):** three e2e scenarios, each verified to FAIL against the pre-fix code:
  `expedition_from_flight_fires_through_the_real_on_arrive` (ship under `seek` at cruise
  altitude + an authored `on-arrive → run(foot)`; asserts deploy/harvest/return through real
  frames, the routine's own fire count, the Return→Idle auto-deposit, and the handshake
  banking value), `seek_collect_loop_progresses_on_a_known_site_field` (every streamed site
  force-marked known each frame; asserts ≥2 collects — pre-fix this deadlocks at 0), and
  `expedition_return_auto_deposits_the_carry` (fix 3 in isolation).
- Golden voxel-hash + headless render unchanged; goblin-gold and engine crates untouched.

## B — Truing (measured; arithmetic in the analysis doc)

1. **`FACULTY_COSTS` (progress.rs): `[25, 75, 200]` → `[150, 450, 900]`.**
   (Measured any-domain yield ≈16/min ⇒ L1 ≈ 9.4 min, L3 cumulative ≈ 1.6 h — in envelope
   on every measured seed.)
2. **Block research cost (progress.rs): `30 + 20 * s.byte()` → `25u64 << s.byte()`**
   (25/50/100/200/400). At ~3.2 domain-yield/min: SCH ≈15 min (onboarding), RIT 24–43 min,
   REL 50–87 min, SIG ≈1.6–2.9 h. Full ladder + faculties ≈ 5–6 h — a real arc.
3. **Rarity gates (the unimplemented half of the human's 2026-06-11 decision "rarer blocks
   demand rarer shards"):** research targets at Relics tier additionally require **≥4
   rare-tier shard pickups** credited to the target, Signals tier **≥8** (tracked per
   target; the bar shows both: fill and rare-count — structural UI). Numbers are
   placeholders; the *mechanism* is the decision. `pg=` bump append-only.
4. **Discovery variance:** flatten the colossus deep-label table (`DEEP`:
   `[RunFoot×3, Seek, Circle, Goto]` → `[Seek, Circle, Goto, RunFoot]`) — halves the
   Relics-first onboarding roulette and shrinks the 0.3→63-min per-block discovery tail.
   Ambient name-bearer rate `1/4 → 1/6` (nudges first discovery toward the 2–6 min
   envelope). Coverage + uniformity tests must stay green (re-pin if distributions shift).
5. **Dead code:** delete the orphaned `DECODE_COST`.
6. **Envelope regression tests** (the probe made permanent): port
   `docs/probes/pacing_probe.rs` (on the steering branch — copy it over) into a bounded CI
   test: seed 1337, shortened window, asserting **first discovery ≤ 6 sim-min**, **first
   comprehension ≤ 30 sim-min**, **income within ±50% of the measured band**; the full
   6-seed version env-gated (`PACING_FULL=1`). Recompute the asserted bounds *after* the
   truing lands (they move by design).

## Explicitly NOT in G19 (routed onward — recorded so they aren't lost)

- **Vocabulary gap** (no Records/Rites/Signals-gated blocks): filled by the Archive
  milestones — G21's sensing-ladder faculties are the natural **Signals-tier** research
  targets; G20+ add Rites-gated vocabulary. Do not shuffle `Goto`'s stratum now.
- **Expedition rationality** (walker income ≪ ship drift): needs on-foot-*only* value —
  G21's worn/⟦erased⟧ recovery on foot. Fix A/3 makes the loop *work*; G21 makes it *worth it*.
- **Strata-data sink**: candidate consumption in G22 (proto-language/cognate work).

## Tests / acceptance

- [x] A-fixes landed (own commit): horizontal `arrived_at` (+radius 20), seek/collect
      un-deadlocked, auto-deposit on Return→Idle; **D11 expedition-from-flight scenario
      passes through the real `on-arrive`**; golden voxel-hash unchanged.
- [ ] B-truing landed: new constants; rare-count gates (per-target tracking, `pg=`
      append-only, structural-UI display); DEEP table + 1/6 rate (coverage/uniformity
      tests re-pinned); `DECODE_COST` gone.
- [ ] Envelope CI test (bounded, seed 1337) green with post-truing bounds; full run
      env-gated. Probe credited from `docs/probes/pacing_probe.rs`.
- [ ] CI green (fmt / clippy -D / tests / wasm); boundary intact; roadmap G19; the
      pacing-analysis doc copied to main alongside.
