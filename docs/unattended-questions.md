# Unattended-run questions & blockers

Logged during autonomous runs when a decision is genuinely the human's, or a
step can't be completed/verified in the container. I keep working past these —
this is the queue to resolve when you're back.

Format: `## [date] area — question`, then context + what I did instead.

## 2026-06-05 · M8(b) profiling — needs your real weak hardware
The engine-perf systems (M8a: front-to-back sort, depth-store discard; more later) are
landing, but the **profiling half** (M8b) needs the reference devices to measure: the
target iGPU (Intel Iris Xe) and a mid-range phone. I can't measure real frame times here
(headless llvmpipe isn't representative, and there are no GPU perf counters). **When you're
back:** run the build on those devices, read the HUD (fps/ms/chunks/tris/splats), and tell
me the numbers so I can tighten to the design §8 budgets. Until then I'm doing only the
*output-neutral, logically-safe* perf changes and verifying them byte-identical headless.

## 2026-06-05 · Build-blind targets — D4 (Android APK), D8 (Windows .exe), D7 (gamepad)
**Partly resolved (2026-06-06).** All three are now *built* (CI + entry shims landed):
- **D4 (Android APK)** — ✅ **device-verified**: installs, launches, release build is fast on a
  real phone. Lifecycle robustness (surface recreate on suspend/resume) is a follow-up.
- **D7 (gamepad)** — built for web/native/Android; **runtime still built blind** (no pad here),
  on-device tuning expected from your reports.
- **D8 (Windows .exe)** — CI workflow lands; the Linux release binary is verified locally, the
  **Windows `.exe` is built blind** — you verify it runs.
Standing caveat: I can't exercise a phone / Windows / a controller / a live browser in the
container, so these advance by you verifying on your devices and us iterating from reports.

## 2026-06-05 · E11 flowing water / fire — wants a proper substrate, not a naive CA
I held off on a flowing-water cellular automaton. A naive "spread into air" rule either
doesn't conserve water or **oscillates forever** (water shuffling sideways on flat ground →
the dirty-chunk re-mesh fires every tick → perf sink). Doing it right needs the E11 §J
substrate (Margolus/block CA + active-set/dirty-AABB, and pressure-water rendered as a
*separate vertex-displaced pass, not re-meshed*). That's a real milestone, not a quick add,
and worth your steer on scope. Sand (E5) stays the shipped CA; the E14 `edit` seam is ready
to carry sim triggers when we build it.

## 2026-06-05 · E13 headless flythrough — deferred to protect my only verifier
E13's deterministic camera-path flythrough (Catmull-Rom → a clip/contact-sheet) needs the
headless renderer refactored to render N cameras per setup. The headless renderer is my
*only* way to verify renders in-container, so I didn't want to risk breaking it unattended.
The pure camera-path math is easy + testable; the render-loop refactor I'd rather do
carefully (or with you watching). Parked.

## 2026-06-05 · E10 aesthetic spine — wants your eye before I commit the look
**Mostly resolved (2026-06-06).** The core of E10 landed and you embraced it as *the* identity:
the **configurable palette post-process** (20 curated 1–2-hue palettes, luminance gradient-map),
**Bayer dithering**, the deliberate **low-res "pixel-scale" buffer** (halftone), and the
**deep-shadow / sun-off point-lit** mood. Defaults are set to the look you chose (palette on,
`bruise`, pixel-scale 2). **Still open (opt-in, your call):** depth/normal **"ink" outlines**
(voxel creases as a blueprint grid) and a **G-buffer-as-art** mode — I'll add these as live
toggles like `melt` if you want to A/B them; holding until you say.

## 2026-06-06 · E16 reactive audio — flight-reactive intensity built blind (tune by ear)
The drone now reacts to flight: camera speed + altitude → `Drone::set_intensity`, opening the
cutoff + lifting the swell (native atomic + web per-frame; smoothed; tested bound/finite). I
**can't hear it in the container**, so the modulation is deliberately conservative — when you're
back, fly it and tell me if the reaction should be stronger/weaker or mapped to something else
(biome, proximity to colossi). Remaining E16: a voice cap and one FDN reverb (also ear-tuned).

## 2026-06-06 · E17 in-world text — needs your direction on *content* before live placement
The text **renderer** is done and verified (`src/text.rs` + `text.wgsl`: multi-script —
Latin/Greek/Hiragana/Standard Galactic — camera-facing emissive billboards, palettised + fogged
in the scene pass). What's missing is a **content/aesthetic decision I shouldn't make alone**:
**where** inscriptions appear in the streamed world, in **which script**, and **what they say**.
Options I can run with unattended if you give a steer: (a) sparse glyphs/markers seeded near
colossi + points of interest, abstract Standard-Galactic/Greek (decorative, no semantics);
(b) short fixed phrases; (c) leave it dark until you author content. Default if you stay silent:
**(a)** — abstract, seed-placed, decorative, low density — since it's reversible and on-aesthetic.

