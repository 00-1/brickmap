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

- **M2 — done.** Greedy mesher + neighbour-aware seam culling + chunk grid +
  frustum culling + stats + 4-byte packed vertex, all landed and verified.
  - **Meshing bench (criterion, terrain-like section, single thread):**
    greedy ≈ **1.18 ms/section**, naïve ≈ **0.54 ms**. So greedy is currently
    *CPU-slower* than naïve even though it slashes GPU triangles (6 quads vs 6144
    for a solid section). Cause: it reallocates a per-slice mask (192×/section) and
    does extra merge work.
  - **QUESTION/FOLLOW-UP (meshing perf):** the named pillar is *binary* greedy
    meshing (bitmask columns) — it'd be both correct *and* faster than naïve. Plus a
    trivial mask-buffer reuse. Deferred as a tracked perf task (good fit for the M6
    threading/perf pass); the GPU bandwidth win is already achieved, which was M2's
    point.
  - **CORRECTION:** I started drifting toward M3, but per the roadmap **E1 then E2**
    come first (the inlined exploration rungs). Reordered — doing E1 (aesthetic
    pass) next. The M3 brief is written and waits.

- **E1 — aesthetic pass: done.** Two shader effects expose the tech as the look
  (design §11): PS1-style **vertex-quantization wobble** (NDC snapped to a coarse
  grid) + **ordered Bayer dithering** (posterised shading). Tuned by eye via headless
  renders to a fine low-fi grain (`WOBBLE_SNAP=85`, `COLOR_STEPS=4`). Both live in
  `shader.wgsl`, so windowed + headless match. The headless loop (render → look →
  tune) paid off immediately.
  - **QUESTION (low):** dialled fairly subtle at 960×720; easy to push bolder if you
    want it more aggressive. Knobs are shader consts.

- **E2 — particles + destruction: done.** A CPU particle system + a second instanced
  pipeline render ambient bursts of warm emissive cube debris fountaining off the
  terrain (windowed animates; headless captures mid-burst). First "alive on screen."
  - **DECISION:** ambient continuous bursts (not click-to-shatter), since the run is
    unattended — every render is lively. Interactive shatter + world collision later.

- **New asks from the human (queued):**
  - **D2 — live dials:** make params (wobble/dither/gravity/spawn) adjustable at
    runtime via web sliders, not recompiles. *(Requested while watching E1/E2.)*
  - **D3 — auto-fly / mobile:** can't navigate without keyboard/mouse, and on-screen
    sticks are fiddly. Plan: **auto-fly orbit on by default** (mobile just watches),
    desktop input takes over, toggle to resume. **Doing D3 next** (it gates the human
    being able to view the live build at all on mobile).

- **D3 — auto-fly: done.** Cinematic auto-orbit on by default, so mobile/hands-off
  just watches it circle the scene. Keyboard/click takes manual control; `F` resumes.
  (Headless renders the static framed view; the orbit is a live-build behaviour.)

- **QUESTION — native mobile (APK):** the human asked about previewing the *native*
  Android build with push-auto-update. **DECISION/assessment:** the **web build is the
  mobile preview for now** — it runs in the mobile browser and already auto-updates each
  push (auto-fly makes it watchable hands-free). A native APK is a real chunk
  (`android-activity` entry point + NDK cross-build + signing + a CI job), and
  "auto-update" really means *sideload-per-push* (no app-store update path). Queued as
  **D4**, deferred until the look settles and native *performance* is the point — not a
  good use of an unattended run yet. Logged so it's not lost.

- Next: **D2 (live dials)** → then **M3 (procedural world)**. Renders to chat as they land.
