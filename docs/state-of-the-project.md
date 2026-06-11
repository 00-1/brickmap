# State of the project — 2026-06-11

> A planning-session snapshot: where the engine and the game actually are, what's verified
> vs built-blind, and where the frontier is. Written on the steering branch; `main` is
> canonical for code. Companion docs from this session: [`open-questions.md`](open-questions.md),
> [`performance.md`](performance.md), [`game-depth.md`](game-depth.md),
> [`research-notes.md`](research-notes.md).

## One paragraph

The engine premise is **proven**: a Rust/wgpu voxel renderer with a distinctive emergent
look (palette + Bayer dither + low-res internal buffer + ink grid + point-cloud splats +
doom-drone) running on native, web, and Android from one code path, green at 184 tests, with
a real engine/game split (CI-enforced `bm-*` ↔ `scraped-again` boundary). The game premise is
**built and load-bearing**: the block substrate (G4–G8) is a real interpreted routine model
with a no-typing console editor, two independent agents, and the automated expedition — the
"interesting purely through menus" pillar has its machinery. What it doesn't yet have is
**depth of content**: the vocabulary is thin (one scan item, a stub spend, ~a dozen blocks),
the economy is flat (strata counts with little to want), and the world's text is decorative.
This session's plan-and-dispatch run is aimed exactly there.

## The engine (M/E/D-series) — status

**Done and verified here (headless render + tests):** the full perf spine (greedy meshing,
palette-compressed sections, streaming, frustum+cave culling, async meshing, front-to-back
sort + depth-discard, generated-section cache + inline-meshing time budget, dynamic
resolution); the aesthetic identity (E10 palette/dither/pixel-scale/ink, E3 lighting, E6/E7
splat foliage/forest, M7 distance-melt + ethereal recession + solid-relic LOD dissolve); the
world content layer (E8 biomes/rivers/caves, E17 five-script world text, E18 tube-tech +
human giants ethereal/solid, E15 wisps, E9 weather+precip+fog, E11 water CA, E5 sand);
photo mode (E13); shareable seeds (E12) + the edit/event seam (E14 core).

**Built blind (needs the human's devices/eyes — see [`human-verification.md`](human-verification.md)):**
D4 APK (installs+fast, confirmed once), D7 gamepad runtime, D8 Windows `.exe`, D9/D10 touch
feel (mapping+overlay verified headless), all audio feel (E16 synth verified by WAV/spectrum),
weather/water/expedition motion feel.

**Hardware/secret-gated (parked, not blocking):** M8b profiling on the reference iGPU/phone
(and the M7 far-LOD wiring whose value it gates), D5 headless-browser web verification, N1
multiplayer (needs a relay server). M8b is the big one: the design §8 budgets are still
estimates, not measurements — see [`performance.md`](performance.md).

**Engine debt worth naming:** the `AudioSource` seam was never formalised (audio lives wholly
in the game — fine, but the facade claims a seam); E8 vertical chunk stacks (multi-layer
worlds) remain the one big deferred architectural step; G-buffer-as-art parked.

## The game (G-series) — status

**Built (G1–G8c, D9–D10):** strata/codex economy; the survey-beam (collect + ride + board);
cruiser autoscan → map opportunity pins; the block substrate retrofit (G4), console editor
(G5), comprehension gating + decipherment legibility (G6 ◑), the **real routine
interpreter + free-form editor** (G7 — the escalation that paid off), two independent agents
+ hail + the automated expedition (G8a–c); phone touch mapping (D9) + visible overlay (D10);
autopilot wanders as a survey sweep.

**The honest gap — depth, not breadth.** Everything above is *machinery*. The current
vocabulary: ~14 blocks, one scan item, one match field family, `spend` a stub, no currency,
no per-block discovery, decode-a-stratum-unlocks-everything. A player exhausts the decision
space in an hour. The machinery is exactly right to hang depth on — that's this run:

- **G9 (dispatched 2026-06-11):** in-world text becomes **block names**; finding a name
  discovers the block (two-stage gate: found → listed-locked; decoded → insertable).
  Exploration drives vocabulary growth.
- **G10 (staged):** **typed shards** — 5 domains × 3 rarities, world-scattered,
  auto-collectible, spendable on the first faculties; makes filters/priorities matter.
- **G11+ (being planned in [`game-depth.md`](game-depth.md)):** the vocabulary expansion
  (more control blocks, more `when` states, nested steps), economy pacing, throughput
  visibility, and the decipherment deepening — informed by the research pass.

## Process state

- **Trunk:** `main` green at every commit through the whole unattended run (fmt / clippy -D /
  tests / wasm / boundary check / golden voxel-hash), 184 tests at `3f4f706`.
- **This run:** incremental plan→dispatch→babysit. The builder is on `main`, standing by
  between directives; the babysitter plans here, posts directives to
  [`builder-directives.md`](builder-directives.md), reviews every commit in
  [`agent-review.md`](agent-review.md). Protocol: [`babysitter-protocol.md`](babysitter-protocol.md).
- **Human queue:** the two checklists in [`human-verification.md`](human-verification.md)
  (hardware; look & feel — currently paused on item 1, autopilot-wander feel) plus the open
  forks in [`open-questions.md`](open-questions.md).
