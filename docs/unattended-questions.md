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
These can't be verified in this Linux container (no Android device/emulator, no Windows, no
gamepad, no live browser). I'm leaving them parked rather than build blind; say the word and
I'll do the CI-packaging work (they're mostly CI + entry-shim), but you'd verify on your
devices. Not started yet — prioritising verifiable milestones first.

## 2026-06-05 · E10 aesthetic spine — wants your eye before I commit the look
E10 (indexed-palette colour-cycling, depth/normal "ink" outlines, a deliberate low-res
internal buffer, banded lighting, G-buffer-as-art) is a set of **strong, opinionated**
aesthetic forks that could clash with the lush point-cloud-forest direction you set. These
feel like calls you should make, not me unattended. I can implement any/all as opt-in
toggles (like `melt`) so you can A/B them live — tell me which to try. Holding off for now.

