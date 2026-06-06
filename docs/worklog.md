# Worklog (autonomous run)

A running log of unattended work: what landed, decisions made without asking, and
open questions for the human to weigh in on later. Newest first.

> Conventions: **DECISION** = a choice I made and ran with. **QUESTION** = something
> worth the human's steer when they're back (I didn't block on it).

## 2026-06-06 — milestone review + next unattended run

Reviewed the whole ladder before going heads-down again. **The roadmap is the source of
truth and is current** — recently captured: E15 point-cloud creatures (drifting wisps, live),
the splat **ethereal recession** (lagged-camera + per-splat stagger, M7), and **varied-size +
dither-transparent foliage** (E6). Brought the lockstep docs back in line (README "Next" line,
architecture current-vs-target module list + pipeline row, and this run's open questions in
`unattended-questions.md`).

**Where things stand:** M0–M6, E1–E8 (bar vertical stacks), E10 core, E12, D1–D4, D6 are done;
E14/E16/E17/E18/M7/M8/D7/D8 are partially landed. The renderers for text (E17) and colossi
(E18) exist; what's left there is content/placement + finishing solid voxelisation.

**Planned unattended order (verifiable-first; revisit if the human steers):**
1. **E16 reactive-audio layer** — speed/biome/weather → cutoff/level tweens, voice cap, one FDN
   reverb. Pure-logic-heavy, testable, no device needed. (Synth + I/O already shipped.)
2. **E18 finish** — solid voxelisation of the human mesh + live placement; bake a compact asset
   so the web build needn't ship the raw OBJ. Verify relics/figures density + perf headless.
3. **M7 / M8a perf** — the *output-neutral* engine-perf systems (vertex pooling, load/store
   discipline, dyn-resolution/upscale groundwork) + the general point-decimation far-LOD.
4. **E8 vertical chunk stacks** — the one big architectural step left in E8 (single→multi-layer
   streaming); warrants its own brief, written just before building.
- **Gated on the human:** E17 text *content/placement* (decision logged), E10 ink-outlines /
  G-buffer-as-art (opt-in, awaiting your eye), M8b profiling (needs the reference iGPU + phone),
  E11 flowing water (wants the proper Margolus/active-set substrate, not a naive CA).

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

- **D2 — live dials: done.** Wobble + dither are runtime-adjustable via page sliders
  (`#[wasm_bindgen]` setters → globals uniform). Reusable mechanism for more dials.

- **QUESTION — native app + auto-update (follow-up):** the human wants a *real* native
  Android app (not a web wrapper) that auto-updates on push. **Assessment:** the build
  is doable but I'd build it *blind* (no device/emulator here), and crucially the
  **auto-update half is blocked on the human** — a sideloaded APK can't auto-update;
  the real options are Google Play internal-testing (needs a $25 Play account + signing
  key + CI publish) or Firebase App Distribution (needs a Firebase project). Expanded
  **D4** with this; **deferred** until the human stands up a channel and native
  *performance* is the goal. Not attempting it in the unattended run (unverifiable +
  partly blocked). Web-on-mobile remains the preview.

- **M3 — procedural world (parts 1–2 of 3 done):**
  - **Part 1 — noise terrain:** dependency-free fractal value noise; a 5×5 world of
    rolling hills, snowy peaks, sandy lows. Snapshot `/archive/06-procedural-world/`.
  - **Part 2 — palette storage:** `Section` now palette-compressed (~4 KiB vs 64 KiB
    for low-diversity chunks). Transparent swap, tested. **Note:** palette `get` is bit
    extraction → slows meshing; the fix (decompress section to a dense scratch buffer
    before meshing) is queued with the binary-greedy follow-up.
  - **Part 3 — streaming (next):** load/unload chunks around a *travelling* camera for
    an endless world. Needs a dynamic renderer (chunks keyed by coord, add/remove at
    runtime) + a forward-drifting auto-fly. Bigger and harder to verify (load/unload is
    a behaviour over time, not one frame) — proceeding carefully.

- **QUESTION (logged, not blocking) — D5 web-render verification:** to verify the
  *actual* deployed web build (WebGL2 fallback + browser integration), drive a headless
  Chromium against a locally-served build (localhost dodges the sandbox network block).
  Heavier; my native llvmpipe render already covers the WebGPU path. Candidate D5.

- Next: **M3 part 3 — streaming**, then M4 (materials). Renders to chat as they land.
