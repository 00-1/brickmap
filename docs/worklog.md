# Worklog (autonomous run)

A running log of unattended work: what landed, decisions made without asking, and
open questions for the human to weigh in on later. Newest first.

> Conventions: **DECISION** = a choice I made and ran with. **QUESTION** = something
> worth the human's steer when they're back (I didn't block on it).

## 2026-06-05 — autonomous run begins

Mandate: work through the milestones unattended, snapshot visible builds, don't
pause for input; log decisions/questions here and keep moving.

- **D1 — headless render-to-PNG: done.** llvmpipe (software Vulkan) works in the
  container; `cargo run --bin screenshot` renders the demo scene to a PNG, verified
  to match the live build. Repro via `scripts/setup-env.sh` + a SessionStart hook
  installing `mesa-vulkan-drivers`. This means I can self-check renders during the
  run instead of working blind.
  - **DECISION:** the headless pipeline is *duplicated* from `gfx` (own
    Globals/palette/pipeline) to avoid touching the working windowed path. Sharing
    via a common `Renderer` is a noted follow-up.
  - **QUESTION (low):** an automated golden-image *test* (committed reference PNG +
    tolerance) is deferred — llvmpipe output may differ across versions, so I'll
    rely on capture-and-look for now. Worth adding once the look stabilises.

- Next: close M2 (criterion meshing bench) → M3 (palette storage + procedural
  generation + streaming), snapshotting the new terrain.
