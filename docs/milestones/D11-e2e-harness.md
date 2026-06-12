# D11 — End-to-end play harness (headless integration + soak)

> **Status: ready to build — next directive.** Everything G9–G16 is unit-tested + golden-hash
> + render-checked, but **the per-frame orchestration in `App` that wires the modules together
> over time has never been driven end to end** — autopilot, scan→on-scan→collect→shard-intake→
> research-fill→comprehend, streaming, the two-agent expedition, persistence round-trips. This
> builds a **durable headless integration harness** (a CI regression asset) that drives the
> real core loop and asserts it progresses + holds invariants, plus a **bounded soak/fuzz**
> driver. The human explicitly asked for autonomous end-to-end testing. Game-side (+ whatever
> minimal seam is needed to drive `App`/sim headlessly); keep any engine touch generic.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** A headless test that **plays the game**: from a fresh seed, drives the actual
  update/tick loop with scripted inputs through a full progression (discover a block →
  allocate → domain shards fill it → comprehend → author + run a routine → an expedition →
  serialize/restore) and **asserts the loop works**, plus a bounded **soak/fuzz** that runs
  many ticks under autopilot + random-valid edits asserting no panic / no NaN / no overflow /
  honest progress-or-blocked.
- **Demonstrable outcome.** `cargo test` includes an `e2e` integration test (the scripted
  playthrough + invariant asserts) that runs in CI; a `soak` test (bounded iterations,
  seeded) that fuzzes the loop; both green. A regression that breaks the *integration* of the
  systems (not just a unit) now fails the build.
- **De-risks.** The whole "does it actually play?" question — the integration bugs unit tests
  structurally can't see (per-frame ordering, the systems' interplay over time, persistence
  fidelity, determinism), now guarded permanently.

## The central design problem (solve first, document the choice)

`App` (lib.rs) owns the wgpu device/surface, so it likely can't be constructed headlessly.
Pick the most faithful drivable approach and **document why**:
1. **Extract a sim-core** — separate the game-logic update (console interpreter tick, progress/
   economy, shards, structures/inscription streaming, autopilot, agent/expedition, persistence)
   from the GPU/render/upload side, so the logic half can be `new()`'d and `tick()`'d with no
   GPU. *Preferred if the seam is clean* — it makes the loop testable forever and is good
   architecture (render reads sim state; sim never needs the GPU). Keep the split behaviour-
   preserving (golden voxel-hash + headless render unchanged).
2. **Headless App mode** — if extraction is too invasive now, add a headless construction path
   for `App` (no surface; a stub/null render target) drivable tick-by-tick. Less clean but
   lower-risk.
3. Whichever: the harness must drive the **same tick sequence** the live frame uses (don't
   re-implement a parallel loop that drifts from reality — that would test a fiction).

## Scope

**In:**
- **Scripted playthrough test** (deterministic, seeded): fresh world → autopilot/scripted
  inputs over N ticks → **assert progression milestones reached**: a block discovered, a
  research allocated + filled + comprehended, an authored routine enabled + emitting the
  right acts, an expedition cycle (ship→walk→return) completing, and a **state round-trip**
  (serialize → deserialize → equal). Run across a few seeds.
- **Persistence fidelity:** serialize a played-forward state (discovered/comprehended/
  in-progress research/authored routines/shard banks) through `share`/`pg=`/`co=`,
  deserialize, assert meaningful-state equality; assert old payloads (v1..v5) load to the
  documented migration default; a malformed-payload fuzz that must **fail gracefully, never
  panic**.
- **Determinism:** same seed + same scripted inputs on two instances → identical state
  (the E12 share-link/golden-hash promise at the *economy/world* level, not just voxels).
- **Soak/fuzz (bounded for CI):** M ticks (sized to a few seconds CI budget; a longer count
  behind an env flag for local) under autopilot + random *valid* console edits/allocations,
  asserting: no panic; no NaN/inf in any float (camera/economy/sim); no integer overflow
  (the u64 economy — exercise large banks/costs); bounded growth (no unbounded Vec/counter);
  the active research is **progressing or honestly blocked** (never silently stuck-while-
  claiming-running — cross-check the G11 state against actual fill delta).
- **Render-robustness sweep** (reuse the D1/screenshot path): render headless at several
  vantages + after key state changes (console open, research mid-fill, beam/touch/console
  flags) — assert no panic / no NaN, output non-degenerate. (Hash-compare only where stable.)

**Out:** a visual/feel pass (that's the human's eye-pass); new gameplay; turning the soak
into an unbounded fuzzer in CI (keep CI bounded + seeded; longer runs are local/opt-in).

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Sim-core extraction preferred** (Decision/Design 1) if the seam is clean + behaviour-
   preserving (golden hash unchanged); else headless-App mode. Document the choice + why.
2. **CI cost:** the scripted playthrough + persistence + determinism + a *bounded* soak run
   in normal CI; the heavy soak is env-gated. Keep total added CI time reasonable (note it).
3. **Faithful tick:** drive the real update sequence, not a parallel re-implementation.
4. **Findings feed the asserts:** a separate adversarial bug-hunt is running in parallel; its
   confirmed bugs will arrive as fix directives — **encode each fixed bug as a regression
   assert in this harness** as they land.

## Tests

The milestone *is* tests. They must be deterministic + seeded (no flake), bounded in CI,
and assert *integration* outcomes (progression reached, invariants held, round-trip equal),
not just "it didn't crash." Golden voxel-hash + headless render unchanged by any
extraction/seam (behaviour-preserving). CI green (fmt / clippy -D / tests / wasm); boundary
intact.

## Risks & mitigations

- **Sim-core extraction is invasive** → if it can't be done behaviour-preserving quickly,
  take the headless-App path (Decision 1) and note the extraction as a follow-up; don't
  destabilise the render path. The golden hash + headless render are the guard.
- **Flaky soak** → seeded + bounded in CI; assert invariants (ranges/finiteness), not exact
  values; the heavy run is local/opt-in.
- **Parallel-loop drift** → drive the real tick (Decision 3); if extraction, the live frame
  must call the *same* sim-core tick.

## Acceptance checklist

- [x] A headless way to drive the **real** game loop (sim-core extraction *or* headless-App),
      choice documented; live frame and harness share the same tick.
- [x] Scripted playthrough (seeded, multi-seed): discovery → research fill → comprehend →
      authored routine emits acts → expedition cycle → state round-trip — all asserted.
- [x] Persistence fidelity (round-trip equality; v1..v5 migration; malformed → graceful);
      determinism (same seed+inputs → identical state).
- [x] Bounded seeded soak/fuzz: no panic / NaN / overflow / unbounded growth; research
      progresses-or-honestly-blocked (cross-checked vs G11 state). Heavy run env-gated.
- [x] Render-robustness sweep (no panic/NaN across vantages + state changes).
- [x] Golden voxel-hash + headless render unchanged; CI green; boundary intact; roadmap D11.
- [x] (Ongoing) each confirmed bug from the parallel hunt encoded here as a regression assert.

## How it landed (2026-06-12)

**Seam — Decision 2 (headless-App), documented per the central design problem.** The
`RedrawRequested` body was extracted verbatim into one shared method, `App::run_frame(real_dt)`,
called by **both** the live window arm and the harness — so the harness drives the *same* tick the
window does (Decision 3: no parallel loop). `App::headless(seed)` builds the app with `state: None`
(no GPU/window/audio) through the same `assemble_app` the live entry uses; every GPU/window touch
inside `run_frame` was already `state`-gated, and `update_inscriptions` was split so its
gameplay half runs headlessly while only the label *upload* stays GPU-gated. **Sim-core extraction
(Decision 1) was deferred** as too invasive to do behaviour-preserving right now; the headless-App
path is the brief's sanctioned lower-risk fallback. Behaviour-preservation verified: the golden
voxel-hash is unchanged and the headless render is **byte-identical** (stash-isolation `cmp`).

**Tests (`crates/scraped-again/src/e2e.rs`, 8):** `real_loop_plays_and_progresses` (multi-seed —
the headline "does it run?"), `scripted_progression_…` (discover via a real world collect →
allocate → fill → comprehend → legibility → authored routine emits a Collect act → expedition
Deploy→Harvest→Return cycle → share round-trip), `determinism_same_seed_same_inputs_same_state`,
`persistence_round_trip_and_malformed_is_graceful`, `soak_random_valid_edits_holds_invariants`
(bounded; `E2E_SOAK_TICKS` for the heavy run), `economy_survives_large_shard_intake` (u64
overflow), `bug1_extreme_coords_clamp_and_dont_crash_the_loop`, and an `#[ignore]`
`render_robustness_sweep` (needs a Vulkan adapter — local/opt-in, not CI). CI runs the seven
CPU-only tests under `cargo test --all` (no GPU).

**Faithfulness vs determinism note.** Filling a research bar uses the canonical
`progress::Event::CollectShard` seam (the exact event the loop's auto-collect emits), not a gamble
on world shard-luck along an autopilot wander (which would flake in CI); that the *live loop* drives
that seam is asserted independently in `real_loop_plays_and_progresses`.

**Bug surfaced + fixed (the harness doing its job).** The soak/extreme-coord assert found a latent
integer overflow in `biome::vnoise` — the lattice `+1` overflowed when `x.floor() as i32` saturates
to `i32::MAX` at `f32::MAX`-class coords. That's beyond the ±`POS_BOUND` (1e7) share clamp (so
unreachable in real play) but a gap in BUG1's "any extreme cam coord is panic-free" defense-in-depth.
Fixed with wrapping lattice indices (`xi.wrapping_add(1)`), byte-identical to `+1` at every reachable
coordinate (the hash already mixes with `wrapping_mul`); golden + render unchanged.
