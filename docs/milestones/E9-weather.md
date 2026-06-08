# E9 — Weather, water & sound

> **Status: ◑ (2026-06-08).** Backlog item. This slice lands the **global weather state machine**
> + **precipitation** (the most visible system); the heavier visual/audio couplings are noted
> follow-ups. Game-side; weather advances only in the live loop, so the headless/golden render
> stays dry (golden voxel-hash + image unchanged).

## Landed (the weather system + precip)
- `weather::Weather` — a seeded `Clear → Building → Precip → Clearing` cycle with seed-jittered
  phase durations, exposing `intensity() ∈ [0,1]` (0 dry, ramps over Building, full in Precip,
  ramps down over Clearing) and `phase()`. **Pure + deterministic**, unit-tested (cyclic order,
  bounded intensity, dry at t=0 so the golden frame is dry, deterministic in seed).
- **Precipitation** (`App::tick_weather`): during precip it spawns rain/snow particles around the
  camera through the existing particle system, scaled by intensity — **snow** in frost biomes
  (slow, white, drifting), **rain** elsewhere (fast, cool, thin). HUD shows the phase. Advanced
  only in the live loop → the static headless capture never precipitates.

## Deferred (noted follow-ups — visual/audio couplings)
- **Fog / wetness blend** (murk + a wet-surface sheen rising with intensity) — needs a runtime fog-
  density / surface-wetness uniform (engine-side); tune by eye.
- **God-rays** (volumetric shafts) — a post-pass; visual.
- **Stylised water** upgrades beyond today's surface.
- **Procedural ambient audio** for weather → folds into **E16** (the weather term feeds the drone:
  `weather.intensity()` → murk/heaviness).

## Acceptance (this slice)
- [x] A deterministic global weather cycle (unit-tested) driving **visible precipitation**
      (rain/snow by biome), HUD-surfaced; golden render unchanged (live-loop only, dry at t=0).
- [ ] Fog/wetness, god-rays, weather audio (deferred — engine/post/audio + feel).
