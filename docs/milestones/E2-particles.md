# E2 — Particles + destruction

> Status: **in progress** 🛠. Exploration rung (design §12; backlog §B).

## Goal

Voxels that *move* — the cheapest path to "alive on screen." Instanced **cube
particles** in the forward pass, spawned as **bursts of debris** that arc up, fall
under gravity, and fade. Emissive (glowing) so they read against the terrain — the
start of the atmosphere thread.

Because the run is unattended (no clicking), the demo is **ambient**: bursts erupt
continuously from random points on the terrain, so every render is lively. (Interactive
click-to-shatter can come later when someone's driving.)

## Scope

- **In:** a CPU particle system (spawn / gravity / integrate / lifetime / fade);
  **instanced** emissive cube rendering (a second pipeline); wired into the windowed
  loop (animated) *and* the headless capture (simulate a few steps, then shoot).
- **Out:** GPU-side simulation, collision against the world (Vercidium DDA — later),
  additive bloom (E3), debris settling back into voxels (a later bridge to E5).

## Design

- `particles::ParticleSystem` (pure logic, tested): `Vec<Particle { pos, vel, color,
  life, size }>`. `update(dt)`: `vel.y -= G·dt; pos += vel·dt; life -= dt`, drop dead
  ones. An emitter spawns bursts on a timer at given points with random outward+up
  velocity and a warm/emissive colour; size and brightness fade with `life`.
- Render (`gfx`): a small unit-cube mesh drawn **instanced**, one instance per
  particle carrying `(offset, size, color)` via an instance-step vertex buffer. A
  second pipeline with its own tiny shader; emissive (no lighting), depth-tested,
  opaque. Instance buffer re-uploaded each frame (grows as needed).
- App owns the `ParticleSystem`, updates it each frame with the camera dt, and passes
  the instances to `render`. Headless simulates a burst then captures.

## Tests

- Particle integration: a particle under gravity for `t` lands where ballistics say;
  lifetime expiry removes it; the live count tracks spawns − deaths.

## Acceptance checklist

- [ ] Particle system with gravity + lifetime + fade (tested).
- [ ] Instanced emissive cubes; ambient bursts over the terrain.
- [ ] Windowed (animated) + headless (static frame) both show particles.
- [ ] Snapshot to gallery + render to chat.
- [ ] CI green; docs synced.
