# Human-verification pass (end-of-run)

After the unattended build runs wound down (all non-blocked **systems** built + green —
**243 tests** at the 2026-06-12 run's end, independently re-run; see
[`agent-review.md`](agent-review.md)), what's left needs a **human** or **hardware**. The
babysitter did the part it *can* (headless render passes + the D11 E2E harness, which now
drives the real loop in CI); the rest is the checklists below — **hardware**, the original
**look & feel** list, and the **2026-06-11/12 run's new items** (Checklist 3).

> **How to look:** the live web preview auto-deploys from `main` →
> <https://00-1.github.io/brickmap/latest/> (already current). Or run a native build
> (`cargo run` in the workspace). Controls: `O` console · `K` photo mode · `N` map · `M` mute ·
> `T` collect · `F` autopilot · sim/weather/feature toggles per the HUD.

---

## What the babysitter already render-verified (headless, llvmpipe @ seed 1337)

Captured + eyeballed via the `screenshot` tool (images dumped in chat). **No broken renders.**

- ✅ **World look** — palette + ordered dither, foliage point-cloud, distance melt, the cruiser.
- ✅ **Solid relic (E18)** — the voxelised tube-tech giant renders shaded/AO/palettised (close-up).
- ✅ **Inscriptions / world-text (E17)** — the glowing in-world labels render.
- ✅ **Survey-beam (G2)** — renders **vivid (gold) over the palette**, the post-palette-overlay
  claim, confirmed visually.
- ◑ **Human-figure giant (E18)** — *renders* as a reclining ethereal point cloud; whether it
  reads recognisably **human** is a feel call → look-&-feel list.

**Not headless-verifiable (live-loop only)** → on the look-&-feel list: weather/precip, the
operations console UI, live water flow, dynamic-resolution, **all audio**, the two-agent/
expedition motion, the cruiser-redesign feel.

---

## Checklist 1 — HARDWARE (needs your devices; do only if/when you have the kit)

Not on the critical path — these can sit indefinitely. Each: *do · check · report back*.

- [ ] **D7 — gamepad/controller.** Plug a USB/Bluetooth pad (desktop) or a USB-C pad (Android
      APK). *Check:* left stick moves, right stick looks, A toggles autopilot, B enter/exit, the
      mapped buttons work; no drift/deadzone issues. *Report:* which mappings feel wrong.
- [ ] **D8 — Windows desktop build.** Download the `dev`-release `.exe`, run on Windows. *Check:*
      it launches, renders, runs at a good rate; controller works natively. *Report:* launch/crash.
- [ ] **D4 — Android APK.** Sideload the `dev`-release APK. *Check:* installs, launches, runs;
      audio plays; touch/pad input. *Report:* install/run/audio issues.
- [ ] **M8b — profiling on weak hardware.** Run on the reference **Intel Iris Xe** (or AMD 660M)
      @ 1080p and a **Pixel-6a-class** phone. *Check:* sustained FPS vs the design §8 budgets
      (60 target / 30 floor); does dynamic-resolution kick in and hold the rate? *Report:* the
      frame-time numbers (these update `design.md` §8 + unblock the **M7 far-LOD** wiring).
- [ ] **D5 — web build (WebGL2 path).** Open the Pages preview in a browser *without* WebGPU.
      *Check:* it falls back to WebGL2 and renders. *Report:* console errors / broken visuals.
- [ ] **N1 — multiplayer.** *(Not built — needs a relay/signaling server + hosting + secrets.)*
      Decide if/when to stand up the server; the seed + edit-delta groundwork already exists.

---

## Checklist 2 — LOOK & FEEL (needs your eyes/ears + preference)

Best via the **web preview** or a desktop run. The *systems* are built + green; you're judging
whether they **look/feel right**, and tuning. Each: *look at · the call · note any tweak*.

**Render-verified — you're only judging *feel/preference*:**
- [ ] **Survey-beam** — cast it: is the vivid-gold-over-palette right, or too bright/dim? Ride
      feel (lifespan/reach), the cruiser-board/hail. *(Confirmed it renders vivid.)*
- [ ] **Solid relic + inscriptions** — do the giants read as monumental? inscription density/glow ok?
- [ ] **Human-figure giant** — does the ethereal point cloud read as a *recognisable human*, or
      too diffuse? (The one render-pass ◑.) Worth a tweak to sampling/density?
- [ ] **Palette / dither / melt** — the overall murky look + distance dissolve: keep, or tune?

**Live-only — verify it works *and* judge feel:**
- [ ] **Operations console (`O`)** — open it: do blocks/routines read clearly? cursor+confirm
      editing (create/insert/remove/reorder, `←→` params) usable? on-aesthetic? *(The G7 core —
      worth the closest look.)*
- [ ] **Two-agent expedition** — wire `seek → on-arrive → run(foot)`; watch the ship land + the
      walker harvest + return. *Tune:* speeds / radii / dwell. And the **away-walker** while piloting.
- [ ] **Weather (E9)** — let a precip cycle run: do rain (cool/thin) and snow (frost biomes,
      drifting) read right? intensity/pacing? *(Deferred v2: fog/god-rays/water-look.)*
- [ ] **Audio (E16)** — *listen:* the doom-drone; does the **weather term** (storms heavier) +
      **FDN reverb** (size) + the flight-reactive intensity feel right? *Tune by ear.* The whole
      audio feel is unverifiable without you.
- [ ] **Live water (E11)** — toggle the sim: does seeded water fall / run downhill / pool well?
- [ ] **Dynamic resolution (M8a)** — under load, does the image chunkify gracefully (never blurry)?
- [ ] **Photo mode (`K`)** — pause + free-cam + FOV: does the freeze + framing feel clean?
- [ ] **Cruiser** — the redesigned faceted dart: does it read well in flight?

---

## Checklist 3 — THE 2026-06-11/12 RUN (G9–G17: discovery, economy, console depth)

The new gameplay arc. Systems are built + green + E2E-tested headlessly (D11); you're
judging **feel, pacing, and legibility**. Best on the web preview or a native run.

**The core loop (the big one — play ~20 minutes hands-off-ish):**
- [ ] **G9/G12 — names in the world, glyphs in the console.** Fly until you spot a
      name-bearing inscription; collect it; the console lists the block as glyphs. Does the
      world↔console recognition *land* (you recognise the cluster you saw carved)? Is
      pure-glyph + learn-by-clicking intriguing or impenetrable? (Fallback if impenetrable:
      an after-use gloss — flagged, not built.)
- [ ] **G10 — shards.** Do the domain-tinted clusters read well (density, rare glints)?
      Too busy / too sparse?
- [ ] **G15 — research.** Click a discovered block to allocate it; watch the fill as
      shards arrive. Does allocate-and-fill *feel* like research? Pacing: too fast/slow
      (numbers are placeholders — report what feels right)? Is the lit-goal (`◆`) helpful?
- [ ] **G11 — telemetry.** Open the console while routines run: do the state lines
      (`running`/`waiting`/`blocked: …`), counters, and the live step highlight make the
      machine legible? Anything that claims to run while visibly doing nothing?
- [ ] **G13 — trace → routine.** Do a few manual collects, then turn the trace into a
      routine. Does the ticker make the feature discoverable? Is the draft what you'd expect?
- [ ] **G14 — subroutines/groups.** Author a `run(other-routine)` + a repeat-group in the
      editor. Usable with cursor + steppers? The `G`-descend/`O`-back navigation OK?
- [ ] **G17 — the handshake** *(once landed)*: wire walker deposit + ship pickup; watch a
      full cache cycle. Does the failed-handoff state read as a legible vignette?

**Carried look-and-feel edges (from reviews, deliberate scope calls):**
- [ ] Routine names + faculty names render **English** (author labels / instrumentation) —
      right call, or should givens be glyphs too?
- [ ] Discovery toast wording ("NAME RECOVERED" + glyphs) — instrumentation-English OK?
- [ ] G16 — ambient inscriptions are now statistically-honest nonsense; does the ambient
      text still *feel* right at a glance (it should look unchanged-ish)?

**Standing items that gained new relevance:**
- [ ] Touch (D9/D10) over the new console depth (allocate, group-editing) on a phone.
- [ ] Autopilot wander (the original item 1) — now also: does `prospect` (shard scanning)
      visibly do its job as you fly?

## After you review

For anything that needs a change, tell the babysitter (or write it under a directive) and it
becomes a builder task — the steering loop spins back up. "Looks/feels good" items just get
ticked. The hardware list waits on your devices; it's not blocking.
