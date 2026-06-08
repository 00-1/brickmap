# E13 — Photo / cinematic mode

> **Status: ✅ landed (2026-06-08).** A bounded, additive exploration milestone from the backlog
> (roadmap E13). Game-side only (`scraped-again`); no engine/shader changes, so the golden
> voxel-hash + headless render are untouched (the mode defaults **off** and the golden path never
> enters it).

## Goal · outcome

A **cinematic free-camera** for capturing the world: toggle it on to **pause** the living world,
**detach** the camera into free 6-DOF flight (independent of pilot/walk), and **zoom** (adjust
FOV). Toggle off to restore exactly where you were. This is the tool the gallery/snapshots want —
line up a shot without the autopilot/sim moving under you.

## Scope

**In (this slice):**
- A **photo toggle** (`K`). Entering saves the live camera; leaving restores it (+ FOV).
- **Pause** — freeze the time-driven world (sim, autopilot, the expedition, auto-scan, particles,
  animation clocks) while photo mode is active, via a single `world_dt = 0` lever. Streaming +
  rendering keep running so you can fly the free-cam to frame a shot.
- **Free-cam** — manual 6-DOF flight (the existing controller) on the real frame `dt`, regardless
  of pilot/walk mode; no autopilot.
- **FOV zoom** — `-` / `=` adjust the vertical FOV (clamped ~20°–100°); restored on exit.
- HUD line shows the mode + current FOV.

**Out (noted follow-ups):** post-grade (exposure / vignette / film-grain) and **camera roll** —
both need `bm-render` shader / up-vector changes (engine-side), deferred to keep this slice
game-side + golden-safe. A timeline/dolly path is a later polish.

## Tests
- `adjust_fov` clamps to the range + steps by the given degrees (pure, unit-tested).
- clippy `-D` / tests / wasm green; golden voxel-hash + headless render **unchanged** (mode off by
  default).

## Acceptance
- [x] `K` toggles a cinematic mode that **pauses** the world (a single `dt → 0` lever), gives a
      **free 6-DOF camera** (on real frame-time), and **zooms** (`-`/`=` FOV), restoring the prior
      camera + FOV on exit. HUD shows `PHOTO <fov>°`.
- [x] Game-side only; golden voxel-hash + headless render unchanged; `adjust_fov` unit-tested
      (163 tests); clippy `-D` / tests / wasm green.
