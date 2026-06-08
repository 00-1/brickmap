# Agent-review log (babysitter)

Running critical review of the **parallel agent** building the Scraped Again G-series (and
the rest of the roadmap) **on `main`**, while this branch
(`claude/core-mechanics-planning-0TpOA`) holds design + this log. Periodic: each pass fetches
`origin/main`, reviews anything new against [`game-system.md`](game-system.md) +
[`game-mechanics.md`](game-mechanics.md) + the milestone briefs, and records an entry.
**Newest entry on top.** Critical where criticism is due; praise where earned. This branch
only — never pushes to `main`.

**Reviewed through:** `a5f316f` (autopilot wander fix).

---

## 2026-06-08 · autopilot drift wanders, not circles (`a5f316f`)  ✅ playtest fix landed

The human's playtest note (drift = tight circle) — fixed cleanly, exactly per the directive.
**Diagnosis matched:** the shared `autopilot_step` heading was a low-freq two-sine sum → near-
constant turn → a loop. **Fix:** a slow **fbm of three incommensurate sines** (per-seed phase) so
the turn rate varies and *crosses zero* on a ~10–30 s scale → it meanders and covers ground.
Applied to the **shared `autopilot_step`** (piloted drift + the autonomous away-ship). Cheap,
deterministic, live-loop (golden hash untouched). **Nice verifiable test:** asserts the drift turns
*both ways* (meanders), not one way (circles) — a clean unit proxy for the human's "wanders" ask.

- **Correct scoping** (better than my directive): I'd said "+ the away-walker", but the walker uses
  directed `walk_toward` (toward sites), not free drift — so it never circled; the builder rightly
  applied the fix only where drift happens.
- **Still the human's call:** whether it *reads* as a purposeful survey sweep is motion-over-time —
  in-app confirm on your next look. The *mechanism* (meanders, covers ground) is verified.

**Verdict.** Quick-fix done right — on-directive, all drifting agents, smart test, correct scope.

---

## 2026-06-08 · D9 — phone touch controls (`ef2ac5a`)

**What landed.** `bm-platform::touch` (generic `TouchPoint`/`TouchPhase` + pixel→0..1 norm,
unit-tested, **no winit/game dep** — mirrors `PadInput`; re-exported by the `brickmap` facade) +
`scraped-again::touch` (a `Layout` + **pure, unit-tested** modal mapping `classify`/`slider_value`/
`button_tap`/`view_tap` over flight/walk/menu). App wiring: `WindowEvent::Touch` → router onto the
**existing** CameraController/mode/console paths — new input *source*, no new control logic. Sliders
steer (R=yaw) + climb/forward (L); buttons (1 console, 2 map, A cruise, B board/exit/hail); view-tap
casts the beam; menu-tap selects a console row. Golden hash + headless unchanged (overlay only after
a touch); boundary intact; tests/clippy/wasm/demo green.

**Strengths — followed the brief closely; all three watch-points met (verified in code).**
- **Engine boundary held:** touch events are generic in `bm-platform` (no game concepts), mapping
  lives in the game. ✓
- **Logic built + tested, not deferred-as-"needs-a-phone":** the touch→action mapping is a pure
  unit-tested function. ✓
- **Reuses existing paths** (new input source); **tap = the survey-beam** (the universal verb). ✓

**Minor notes (here-buildable refinements, lumped loosely with "deferred feel"):**
- **Tap casts the beam from screen-*centre*, not the tapped point** (per-pixel aim deferred). But
  per-pixel aim is **here-buildable** — reuse the desktop DDA-pick's screen→ray — so it's a small
  follow-up, *not* phone-gated. (v1 fire-at-crosshair is functional; low priority.)
- **Overlay is a HUD *text line*, not the dimmed edge-strip visuals** — also here-buildable (a HUD
  overlay), not device-gated. Function works; the visual is a follow-up.
- Neither warrants a push (v1 is functional + honest); just flagging they're buildable here, not
  truly "needs a real phone." On-device *feel*-tuning (sensitivity, sizes, targeting) genuinely is.

**Outstanding:** the **autopilot-wander quick fix** (drift = tight circle → meander) is **not** in
this commit — the builder did D9 first (fine; "around D9"). Directive still live in the channel;
**watching for it next.**

**Verdict.** Clean, on-brief D9 v1: solid engine/game split, pure-tested mapping, parity-safe. No
push. Watching for the wander fix + the two small here-buildable refinements.

---

## 2026-06-08 · 1a4a5e0 — wind-down ✅ confirmed (independent green-check: 176 tests, 0 failures)

The builder declared the M/E/D backlog pass complete. **This is a legitimate wind-down, not a
false pause — verified, not just trusted.**

**Independent verification.** I checked out `main`'s tip and ran the **full workspace suite myself**:
**176 tests pass, 0 failures, `cargo test --workspace` exit 0.** (First attempts red'd on
`libudev-sys`/`alsa-sys` build scripts — that was *my container* missing `libudev-dev` +
`libasound2-dev`, which CI installs per the D7/E16 setup; after installing them, all green. An
environment gap on my side, **not** a code defect.)

**Why it's a real end state (vs. the 9d0ed73 false pause).** There, ~5 buildable feature-areas were
*unbuilt* under a "feel-heavy" excuse. Here, **every non-blocked system is built, tested, and now
independently confirmed green**: E11 (water + live wiring), M8a (dynamic res), M7-core, E9 (weather
+ precip), E16 (reactive audio), E18 (voxelisation) — plus G7 + G8 (the composability core + two
agents). The remainder is **genuinely gated**, not dodged:
- **Hardware/secret:** M8b profiling + the M7 far-LOD it gates, D7/D8 device verification, D5
  browser, N1 server. (Same class that just blocked *my own* verification — these are real.)
- **End-of-run human review:** visual verify of the colossi; audio/visual *feel*-tuning (reverb
  size, weather depth, god-rays/water look) — needs eyes/ears the agent doesn't have.
- **Small coupled follow-ups** (noted, should land eventually): E18 solid-human live placement +
  asset-bake; the web weather→audio bridge; E9 post/shader polish.

**Verdict — the run is in a sound, complete end state.** No here-verifiable feature is left
unwired; the deferrals are all legitimately gated or feel/visual. **I confirm the wind-down** rather
than forcing busywork — the calibrated counterpart to fighting the earlier false pause. Strong run:
the escalation forced the core (G7), the false-pause steer kept it moving, and the per-commit pushes
held quality; it ends green and honest. Standing by for the human's end-of-run review + any
hardware-gated follow-ups.

---

## 2026-06-08 · E18 — solid voxelisation of the human colossus (`9766d16`)

---

## 2026-06-08 · E18 — solid voxelisation of the human colossus (`9766d16`)

**What landed.** `model::voxelize(mesh, res, seed)` → solid **surface-shell** voxels (deduped local
grid) from an area-weighted sampling of the CC0 human mesh — a shell (not filled) like the relics,
which is what an explorable giant wants. Deterministic, unit-tested (determinism, bounds, grid
extent). Pure logic, golden unaffected.

**Assessment — measured, no push.** Same algorithm-then-defer shape as M7/E11, but here the deferral
is **defensibly coupled**: live placement (wiring `voxelize → structure_draws`, like `relic_voxels`)
is parked **with** the asset-bake — and the bake is a real packaging need (live-placing the raw 19k-
vert OBJ would bloat the web build). Wiring + bake as one follow-up is reasonable, *not* a dodge —
closer to M7's defensible defer than E11's should-wire. Flagging it only as a **follow-up that should
land** so the solid human giant actually ships.

**Run state.** Much of E18 (relics ethereal+solid, human points, cached placement) is already built
and "pending **in-app visual verify**." That's the **legitimate end-of-run handoff** to the human —
*not* unbuilt work. With E18's buildable algorithm done, the run is now near a real wind-down: the
remainder is genuinely visual-verify + hardware/secret-gated (D5/M8b/D7/D8/N1) + feel-tuning. If the
builder stops here, that's the *correct* end state — I'll confirm it rather than force busywork (and
re-check there's no here-verifiable feature left unwired).

**Verdict.** Clean tested algorithm; reasonable coupled defer. No push. Watching for the wind-down to
confirm it's legitimate.

---

## 2026-06-08 · E16 — reactive-audio layer (`81c56be`)

---

## 2026-06-08 · E16 — reactive-audio layer (`81c56be`)

**What landed.** Three DSP systems on the drone: **Weather→audio** (`Drone::set_weather` pulls murk
down + leans the drive with E9's precip intensity — a lock-free atomic, smoothed per-sample — so
storms sound heavier; a nice E16×E9 cross-link), a **voice cap** (`MAX_VOICES` bounds polyphony →
fixed per-sample cost, keeps the ♭2 dread voice), and an **FDN reverb** (4-line, mutually-prime
delays, orthonormal Hadamard mix × feedback < 1 → contractive/stable). Audio separate from render →
golden unaffected.

**Strengths — correct and well-tested.** The reverb is a textbook *stable* FDN design, and crucially
it's **tested for the right property**: `fdn_reverb_is_stable_and_bounded` (instability/blowup is the
failure mode of a feedback net — verifying boundedness is exactly right). Voice cap + weather term
likewise finite/bounded/decay tested. And this is the **correct deferral shape**: build + test the
*systems* (stability, bounds, the weather→param mapping), defer **only** the actual *sound feel*
(reverb size, weather depth) — which genuinely needs the human's ear (the agent can't evaluate audio
feel). That's the *opposite* of the false pause: it built the feel-heavy thing and deferred only the
feel.

**Minor.** The **web** weather→audio param-bridge is a noted TODO (native has the full term) — a
small platform-parity gap, not worth a push.

**Verdict.** Clean, correct, on-design; the steering has clearly stuck (feel-heavy systems get built,
only the feel waits). Remaining directed work: **E18 remainder** — after which the backlog is down to
genuinely hardware/secret-gated + end-of-run feel-tuning (a *legitimate* wind-down to watch for, vs.
another false pause).

---

## 2026-06-08 · 805bb5e — M7 bundled with M8b (docs; channel working)

---

## 2026-06-08 · 805bb5e — M7 bundled with M8b (docs; channel working)

Docs-only sync: the builder read my M7 withdrawal, **reverted the speculative far-LOD wiring it had
started**, and bundled M7's render-path with M8b (hardware-gated), echoing the "wire here-verifiable
features (E11), M7 is the exception" principle. Nothing to critique — confirms the **steering channel
is read + respected mid-run** (the protocol works both ways: pushes *and* withdrawals land).

*Babysitter lesson noted:* my over-prescriptive M7 push caused a small build-then-revert churn.
Calibrate *before* pushing — reserve hard pushes for here-verifiable features / headline systems;
for ambiguous perf/feel slices, ask/observe before directing. (The withdrawal corrected it, but the
churn was avoidable.)

---

## 2026-06-08 · E9 v1 — weather state machine + precipitation (`00dc0a6`)  ✓ + M7 self-correct

---

## 2026-06-08 · E9 v1 — weather state machine + precipitation (`00dc0a6`)  ✓ + M7 self-correct

**What landed.** `weather::Weather` — a deterministic Clear→Building→Precip→Clearing cycle
(seed-jittered durations, exposes intensity 0..1 + phase). Pure, unit-tested (cyclic order, bounded
intensity, dry at t=0, deterministic). `App::tick_weather` spawns precipitation through the existing
particle system during precip, scaled by intensity — snow in frost biomes, rain elsewhere; HUD shows
the phase. Live-loop only → golden render never precipitates (hash + image unchanged). 

**Good — a real *shipped* feature, not an algorithm-only slice.** The weather state machine is
pure+tested *and* wired to visible precip in-game. Deferred (fog/wetness blend, god-rays, stylised-
water, weather→drone term) are genuinely engine-post/shader/audio follow-ups (the audio folds into
E16) — reasonable.

**M7 self-correction.** The builder *didn't* take my light push to wire M7 and moved to E9 — and on
reflection that's **defensible; I was over-prescriptive.** M7's integration value is *purely the
perf win*, which is genuinely **M8b/hardware-gated** — so unlike E11 (wiring delivers a here-
verifiable feature: water flows), M7's wiring delivers an *unmeasurable-here* optimisation. Bundling
the far-LOD integration with the M8b profiling it serves is the *right* grouping. `decimate_surface`
is tested and appropriately shelved. **Withdrawing the M7 wiring push** (softened the directive).

**Pattern check (the backlog-wide watch).** So far the builder is shipping mostly *real* features —
E13 (visible), E11-2 (water flows), M8a (real perf), E9 (visible precip) — with M7 the one
algorithm-only, and *defensibly* so. The "thin-v1s everywhere / never-wired" habit is **not**
materialising. Good.

**Verdict.** Clean E9 v1; honest defers; and a case where the builder's judgment was right and my
nudge wasn't — noted. Continue (E16 → E18).

---

## 2026-06-08 · M7 — point-decimation core (`9cdf089`)  ↩ light push to wire it

---

## 2026-06-08 · M7 — point-decimation core (`9cdf089`)  ↩ light push to wire it

**What landed.** `bm_render::foliage::decimate_surface(section, cx, cz, stride)` — samples a
chunk's top surface on a stride grid into ~(SIZE/stride)² material-coloured billboard splats. Pure,
deterministic, unit-tested (count-scales-with-stride, sits-on-surface, stride-0-clamped,
empty→empty), engine-generic. Clean algorithm.

**Same shape as E11-1: algorithm built, integration deferred.** The render-path wiring (per-chunk
point buffer + mesh suppressed past a distance + the splat-shader crossfade) is parked as "a
shader/feel slice whose look needs visual iteration AND whose win is weak-hardware perf (overlaps
M8b)."

**Calibration.** The deferral is *partly* legit — the **perf win** genuinely needs the reference
hardware to measure (M8b-gated), and the **crossfade feel** is real visual iteration. **But the
*system* isn't gated by either:** "draw distant chunks as decimated points, mesh suppressed past
distance X" is **buildable and headless-verifiable** (golden-render a scene with far chunks as
points), default-off + golden-safe — exactly how E11-2 wired the water. Deferring the *whole*
integration because its *measurement* + *crossfade feel* are gated is over-deferring. The far-LOD
should **exist** (default-off), with only the feel-tuning + on-hardware perf number deferred.

**Steered (light, in builder-directives):** wire the M7 far-LOD as a default-off, headless-
verifiable system; defer only the crossfade feel + the M8b perf measurement. Holds the "wire it,
don't leave a dead tested function" line (consistent with E11-2). Watching that the build-algorithm-
then-defer-integration shape doesn't become the backlog-wide habit.

**Verdict.** Good algorithm; integration over-deferred. Not egregious (real hardware-gating
exists), but the system half should land default-off. Light push, not an escalation.

---

## 2026-06-08 · E11-2 + M8a — resumed from the false pause (`15087c2`, `9a46a72`)  ✅ steer landed

---

## 2026-06-08 · E11-2 + M8a — resumed from the false pause (`15087c2`, `9a46a72`)  ✅ steer landed

The builder **resumed and built the backlog systems in the directed order** — the false-pause
steer worked end-to-end.

**E11-2 — water wired into the live world (`15087c2`).** The sim toggle now drives sand *and*
water: seeds both ahead of the camera (shared dropper; water on a slower cadence + lateral phase
so they read apart), steps `step_sand | step_water` (bitwise — water runs even when sand moved),
re-meshes dirty overlay sections → **water actually falls, runs downhill, and pools in-game.**
The golden-hash concern I flagged is **handled correctly**: the sim is toggle-gated (off by
default) and mutates the **overlay, not worldgen**, so the static golden world + E12 voxel-hash
are untouched. Sections leave the active set on settle (terminating). **My E11-2 watch is
satisfied — the CA shipped as a feature, not a dead unit-tested function.** Leveling/pressure +
a dedicated water look are fair later slices.

**M8a — dynamic resolution (`9a46a72`).** Frame-time-adaptive internal res: over a ~30 fps budget,
raise an extra divisor (on top of the art-directed `pixel_scale`) so weak hardware holds its rate.
`dyn_resolution_step` is **pure, unit-tested, hysteresis'd**; only ever makes the image *chunkier*
(on-thesis, never sharper), +0 / byte-identical on capable hardware; live-loop only (golden path
untouched). HUD `dynres +N`. **Good judgment call:** it **declined FSR1/EASU** — which I'd named —
because a *smoothing* upscale fights the crisp nearest-present that *is* the look (§11). That's a
correct, design-grounded rejection, not a dodge; I was over-prescriptive listing it. Vertex-quant/
quad-expansion deferred for genuine golden-byte risk — reasonable.

**Verdict.** The babysitter loop worked exactly as intended: caught the false pause → pushed →
builder resumed and is productively building systems, with sound on-thesis judgment (the FSR
rejection) and the determinism handled right. No concerns. Continuing the order (M7 → E9 → E16 → E18).

---

## 2026-06-08 · 9d0ed73 — backlog checkpoint = a FALSE PAUSE  ⚠ steered to resume

---

## 2026-06-08 · 9d0ed73 — backlog checkpoint = a FALSE PAUSE  ⚠ steered to resume

**What it is.** A docs-only "checkpoint": the builder cleared E13 + E11-1, correctly skipped D5
(no headless browser — a real external blocker) and the hardware/secret-gated items, and then
**paused the whole run** — arming a watcher for the babysitter to pick which to build, because
*"the remaining backlog (M7, M8a-rest, E9, E16, E18) is feel/visual/perf/audio whose quality bar
is human iteration."*

**This is exactly the false pause the run rules forbid.** "Feel-heavy / needs human iteration" is
**not** a blocker — those items are **buildable, testable systems**; only the *feel-tuning* waits
for end-of-run play. The builder is generalising "human review at the end" into "stop now". Per
the human's explicit instruction, that does not halt the run.

- **Legit skips (correct):** D5 (no browser + network-gated download), M8b/D7/D8/N1 (hardware/
  secrets). Skip + noted — fine.
- **NOT blockers (build them):** E11-2 (wire the water — my prior watch), M8a-rest (dynamic-res +
  FSR/EASU, vertex-quant/quad-expand, upload prioritisation — *measurable*, barely feel), M7
  (point-decimation LOD), E9 (weather state / precip / snow-blend / stylised water / god-rays /
  ambient audio — systems), E16 (reactive-audio layer + reverb — DSP), E18 (remainder).

**Steered (builder-directives):** do **not** pause — build the remaining systems in order, defer
only feel-tuning, and stop generalising end-of-run review into "stop now."

**Verdict.** The legit skips are right; the **pause is not**. This is the babysitter's core job —
the run would have stalled here with ~5 buildable feature-areas unbuilt under a "needs iteration"
excuse. Pushed it back to work.

---

## 2026-06-08 · E11-1 — flowing-water CA (`be37c39`)

---

## 2026-06-08 · E11-1 — flowing-water CA (`be37c39`)

**What landed.** `bm-world::sim::step_water` — water falls / slides diagonally / flows sideways
into air that can itself fall, so it runs downhill + pools. **Deterministic** (cell-parity
tie-break — matters for golden images), **mass-conserving** (water/air swaps), **terminating**
(sideways only toward a descent → no ping-pong). Engine-generic (no game dep), golden hash
unchanged. Tests: `water_falls_to_the_floor_and_is_conserved`, `water_flows_off_a_ledge_and_downhill`,
`resting_water_reports_not_dirty` — the right CA properties.

**Strengths.** Careful, correct sim work with a sound termination argument and conservation +
determinism verified by tests. And it's **engine work** (`bm-world`) — so the builder isn't
dodging engine changes (partly answers the E13 "engine-side follow-up" worry).

**Watch (not a steer).** This is the *rule*; the **live feature** (water actually flowing in the
world) is deferred to wiring — active-set seeding + re-mesh budget + handling the golden-hash
determinism (live flow mutates the deterministic world, which the E12 hash guards). Honest,
reasonable slicing — but **the wiring (E11-2) must land**, or this is a tested function that never
ships as a feature. Pressure/leveling (flat-puddle) is a fair later slice.

**Verdict.** Solid engine increment, properly tested, honest slice. No concerns; watch that the
live wiring follows. Continue.

---

## 2026-06-08 · E13 — photo / cinematic mode v1 (`431eb33`)

---

## 2026-06-08 · E13 — photo / cinematic mode v1 (`431eb33`)

**What landed.** `K` toggles a photo mode: a single `dt → 0` lever freezes every time-driven
system (sim/autopilot/expedition/auto-scan/clocks) while a free 6-DOF cam runs on real
frame-time; `-`/`=` zoom FOV; exit restores the exact prior camera + FOV. Interpreter tick +
movement skipped while paused (no world mutation); streaming/rendering continue. `adjust_fov` pure
+ clamped 20–100°, unit-tested. Game-side only, mode-off default → golden hash unchanged. 163 tests.

**Strengths.** The `dt→0` single-lever pause is elegant (one mechanism freezes everything coherently,
no per-system pause flags). Exact camera restore on exit is a nice touch. Clean, contained,
parity-safe; correctly kept on the game side.

**Deferral note (proportionate — backlog item, not a headline).** v1 is pause + free-cam + FOV;
the full E13 (exposure/vignette/roll **post-grade**, in-app **screenshot** via the RTT path,
**Catmull-Rom camera paths**) is deferred as "engine-side follow-ups." Two caveats: (a) the builder
*can* do engine work (it built the bm-render overlay in G2), so "engine-side" is a soft defer, not
a hard boundary; (b) **camera paths are pure game-side CPU** (cheap, testable — the backlog even
says so), so that one isn't engine-gated. Acceptable v1 scoping for a backlog polish item — **not
worth a steer** — but registering the **watch:** don't ship thin v1s across the whole backlog and
park all the meat as "follow-ups". If that pattern shows across E-items, I'll push.

**Verdict.** Good, clean v1 of a backlog feature. No concerns; light watch on backlog-wide
under-scoping. Continue.

---

## 2026-06-08 · G8c — persistent away-walker (`99fbf8b`)

---

## 2026-06-08 · G8c — persistent away-walker (`99fbf8b`)

**What landed.** The second system from the steer: a persistent **away-walker** mirroring the
away-ship. While piloting, the walker is the autonomous off-screen agent — a foot `walk` routine
steers it from its *own* position (`nearest_site_to(pos)` generalises seek per-agent) and its foot
acts bank what it reaches; the ship-commanded `run(foot)` expedition takes precedence when out.
`advance_away_walker` orchestrates it. Parity held; clippy/wasm/demo green.

**Assessment.** Good — completes **full two-agent symmetry**: away-ship while you walk (G8a),
away-walker while you pilot (this), ship-initiated expedition (G8c-2b). The builder finished the
*second* buildable system I named rather than skipping to the backlog — exactly the discipline the
steer asked for. Feel/visual (avatar, tuning) legitimately deferred.

**Minor note.** Test count held at 161 — `advance_away_walker` is thin orchestration over
already-tested primitives (`walk_toward`/`nearest_site_to`/collect), so it's covered indirectly,
but a direct test of the away-walker tick would be worth a line. Non-blocking.

**Verdict.** Clean close-out of the G8 two-agent pillar; full symmetry, on the interpreter, parity
preserved. Next: the M/E/D backlog.

---

## 2026-06-08 · G8c-2b — automated expedition + cross-agent run(foot) (`d00cf61`)  ✅ steer landed

---

## 2026-06-08 · G8c-2b — automated expedition + cross-agent run(foot) (`d00cf61`)  ✅ steer landed

**What landed.** The headline §11 Tier-3 payoff. `expedition.rs`: a pure phase machine
`Deploy → Harvest → Return` (idempotent `start`, one-shot harvest entry, `advance(at_site, home,
dt)`), three unit tests. `Block::RunFoot` ("run(foot)") — a **cross-agent SHIP block**,
rare-gated (Relics, Tier-3); `start_expedition` deploys the walker to the nearest known site,
`advance_expedition` walks it out (shared `walk_toward`), **collects via the G1 event seam**, walks
it back; autopilot holds while it's out; HUD shows the phase. So the full loop runs: `seek +
on-arrive → run(foot)` → ship reaches a site, holds, walker disembarks, harvests the ground finds
the cruiser can't reach, returns, ship cruises on. 161 tests, clippy/wasm/demo green, parity held.

**The steer fully landed — verified.** This is exactly what I pushed for across G8c-1 → 2a → 2b:
the buildable systems (the deploy/harvest/return entity + the cross-agent `run(foot)` interpreter
feature) are **built and tested now**; the **only** deferrals are genuinely feel/visual —
speeds/radii/dwell tuning + an in-world walker avatar — which legitimately wait for end-of-run
play. That's the *correct* application of "build the systems, tune the feel later."

**Verdict.** **G8 is systems-complete** (8a/8b/8c-1/2a/2b), all on the G7 interpreter, all tested,
parity preserved. The deferral concern from G8c-1/2a is **resolved** — the marquee feature got
built instead of parked. The two-step steer worked: caught the defer, pushed, got the systems.
Back to routine review; next is the M/E/D backlog.

---

## 2026-06-08 · G8c-2a — foot walk nav + auto-walk (`500c3b1`)  ↩ steer partly taken

---

## 2026-06-08 · G8c-2a — foot walk nav + auto-walk (`500c3b1`)  ↩ steer partly taken

**What landed.** A foot nav block `Block::Walk` (`walk(uncollected)`, the foot analog of the
ship's `seek`): on foot, a `walk` routine auto-walks the walker toward the nearest known site
when you're not steering (manual always wins), through the existing voxel-collision walk. With
G8c-1's `on-arrive → collect`, that's a composable on-foot auto-harvest loop. Pure `walk_toward`
unit-tested; 158 tests, parity held.

**Good — the steer was taken (in part).** This *is* a real, testable system (foot nav), built in
response to the G8c-1 push — not parked. Credit it.

**But still bundling buildable systems into the "end-of-run" defer.** G8c-2b now holds "a
persistent away-walker that banks while you pilot" **and** cross-agent `run(foot:…)` — flagged for
end-of-run because it "changes the board/exit flow." Two of those are **buildable, testable
systems**: the **away-walker entity** is a straight mirror of the already-built autonomous ship,
and **`run(foot:…)`** (a ship routine running the walker's routine) is a **pure interpreter
feature** — *and it's the §11 Tier-3 headline*: the automated expedition (ship → land → walker
runs a foot routine → return → fly on). Only the **board/exit transition *feel*** genuinely needs
play. Don't let "changes the board/exit flow" defer the marquee feature.

**Steered (builder-directives):** build G8c-2b's **systems** next — the persistent away-walker
entity + cross-agent `run(foot:…)`, tested; defer **only** the board/exit-flow feel-tuning.

**Verdict.** Genuine progress and a real response to steering — but the headline expedition
automation is still being held back behind a feel caveat it doesn't fully need. One more precise
push to land the systems; then G8 is actually done.

---

## 2026-06-08 · G8c-1 — on-arrive trigger (`25d2103`)  ⚠ DEFERRAL — steered

---

## 2026-06-08 · G8c-1 — on-arrive trigger (`25d2103`)  ⚠ DEFERRAL — steered

**What landed.** `Trigger::OnArrive` — fires once (rising edge) when an agent reaches the site
it's heading to (`ship_arrived` = nearest known site within `ARRIVE_RADIUS`); composable
(`seek → on-arrive → decode/hail`), persists in `co=`. A nice correctness fix: `Routine`'s custom
`PartialEq` excludes the transient `armed` edge-state so authored routines compare/round-trip
correctly. 157 tests, clippy/wasm/demo green, parity held.

**The primitive itself is clean and correct** — small, well-scoped, on the interpreter.

**But the deferral needs pushback.** The commit parks the *actual* **G8c-2 expedition** — a
second persistent walker entity, foot auto-walk/path, disembark/return choreography, cross-agent
`run(foot:…)` — to *"end-of-run play-iteration,"* citing the run rules ("build the testable
skeleton now, record what needs a human eye"). **That misreads the rule.** Those are **buildable,
testable *systems*** (mirror the autonomous-ship entity; a foot-nav integrator; a disembark/return
state machine; the interpreter running a foot routine on command) — the "needs a human eye" part
is the **feel-tuning** (speeds, radii, timing), *not* the systems. Deferring the whole slice
because its "payoff hinges on play-iteration" is the exact pattern the human flagged: don't park
buildable work behind end-of-run review.

**Steered (builder-directives):** build G8c-2's **systems now** (testable, with parity); flag only
the *feel-tuning* for end-of-run play. Don't skip the expedition to the M/E/D backlog.

**Verdict.** Good primitive; **mild but real deferral** of the headline G8 feature. Not egregious
(the builder is honest and the work is genuine), but it's precisely the "needs play → defer"
move to correct. The systems should land before moving on.

---

## 2026-06-08 · G8b — per-agent routine library (`eff8a82`)  ✓ watch-item satisfied

---

## 2026-06-08 · G8b — per-agent routine library (`eff8a82`)  ✓ watch-item satisfied

**What landed.** Routines are now genuinely **per-agent**: `enum Agent { Ship, Foot }` on each
`Routine`; `Block::agent()` classifies (ship: scan/nav/goto · foot: survey-beam · shared:
collect/decode/spend/hail + match/repeat). The editor's insertable `vocabulary(agent)` is scoped
by agent + shared; `Tab` flips a routine's agent; the agent tag shows + persists in `co=`.
`tick(agent, data)` / `on_scan_acts(agent)` run **only that agent's routines**, and the app ticks
**Ship** for the cruiser *and* **Foot** for the walker separately — so a foot routine runs as a
genuine second agent (e.g. a continuous `collect` harvests as you explore; `when … → decode`).
Givens stay ship routines → piloted parity. 156 tests, clippy/wasm/demo green; golden hash unchanged.

**Assessment.** This **satisfies the G8a watch-item** — verified directly: agent-scoped `tick`
(`if r.agent != agent { skip }`), agent-scoped `vocabulary`, and the app ticking Ship vs Foot on
separate lines (lib.rs 1816/1834). Tests `agent_scopes_which_routines_tick` +
`vocabulary_is_scoped_by_agent` back it. The two agents now each run their **own** routines on the
shared interpreter — the real "two agents" payoff, not shared-intent reuse. Clean, honest slice;
foot nav/pathing transparently held for G8c.

**Verdict.** On-design, well-tested, parity-preserving. No concerns — the run continues to deliver
real structural work on the G7 runtime. Next: G8c (expedition choreography + foot nav).

---

## 2026-06-08 · G8a — autonomous away-ship + hail (`d373d3a`)

---

## 2026-06-08 · G8a — autonomous away-ship + hail (`d373d3a`)

**What landed.** While on foot, the cruiser flies its own course (no longer parked) and
away-scans the cone ahead, filling the map — both agents active at once. A pure, GPU-free
`autopilot_step` is extracted and **shared** by the piloted autopilot and the away-ship (DRY +
unit-testable); `scan_pulse` generalised to `scan_from(origin, forward, do_on_scan)` so any
vantage can scan. A `hail` block + `H` key recalls the away-ship to the walker (wireable; rounds
through `co=`). Parity: piloted behaviour + golden hash + headless unchanged. 78 tests, clippy/
wasm/engine-demo green.

**Strengths.**
- **Sliced the former "G7+" catch-all into 8a/8b/8c** with explicit scope per slice — exactly
  the fix my G6 escalation asked for. 8a (ship-as-agent + hail) is the right first slice.
- **Builds on the G7 interpreter** — the away-ship advances under the interpreter's `nav_intent`
  (drift/seek/circle), not a bespoke path. The shared `autopilot_step` (tested) is a clean DRY win.
- Honest as-built note; cheap off-screen agent (no banking) per game-system §7.

**Watch-item.** 8a reuses the **single** `nav_intent` for the away-ship — there's **not yet a
per-agent routine library** (ship vs foot routine sets); that's explicitly deferred to **G8b**.
Hold 8b to delivering *genuine* per-agent routines (the ship runs its *own* routine while you
run yours), not just continued shared-intent reuse — that's the real "two agents" payoff.

**Verdict.** Healthy, transparent incremental progress on the right foundation. No concerns; the
slicing is honest and the engineering is clean. Continue.

---

## 2026-06-08 · G7 — routine runtime & free-form editor (`04ba341`)  ✅ ESCALATION RESOLVED

---

## 2026-06-08 · G7 — routine runtime & free-form editor (`04ba341`)  ✅ ESCALATION RESOLVED

**What landed.** `console.rs` rewritten: `Routine { trigger, body: Vec<Step> }` run by a real
**interpreter** (`Console::tick` / `on_scan_acts` emit nav/scan/collect **intents** the app
applies). `Trigger` = `Continuous` / `OnScan` / `When(cond)` (rising-edge); `Step` = `Do(Block)`
with `Match`/`Repeat` prefix modifiers. A **no-typing free-form editor**: create/delete routines,
insert/remove/reorder/param steps, cycle step & trigger, nudge when-threshold / repeat-count;
locked blocks can't be inserted. `Scan` parameterised (`ScanItem::Shards`). Authored routines
round-trip through `co=`. Auto-collect now uses a generous nearby reach so the hands-off loop
harvests at cruise. 150 tests (14 in console.rs), clippy clean, wasm, boundary intact; golden
voxel-hash + headless render unchanged. Roadmap re-scoped (G7 ✅; old G7+ → G8+).

**This clears every item I escalated.** Verified directly:
- The accessor hacks (`drift_enabled`/`survey_enabled`/`survey_autocollects`/`nav_block`/`filter`)
  are **deleted** (grep-confirmed gone from console.rs *and* lib.rs); behaviour is now driven by
  **interpreter intents** (`lib.rs` `.tick(...)` → nav/scan/collect).
- **No per-name special-casing** (`== "drift"`/`"survey"` gone); the givens are **plain data**
  instances (test `given_routines_are_plain_data` + `interpreter_runs_the_givens`).
- A genuine **editor** (test `create_insert_remove_reorder`), **`when` rising-edge** (test
  `when_fires_once_on_the_rising_edge`), **`repeat`** (test `repeat_multiplies_the_next_do`),
  **persistence** (test `routines_round_trip_through_co_segment`).
- It also fixed a **prior critique**: auto-collect was inert at altitude (G4/G6) — now reach-based.

**Strengths.** A comprehensive, well-tested delivery that hits the **full non-negotiable bar** in
one milestone, *and* mopped up an old critique. Honest as-built note. This is exactly the work
deferred at G4–G6 — the escalation + forcing brief worked.

**Minor notes (not blocking).**
- The body is **linear with prefix modifiers** (`Match`/`Repeat` affect following/next steps),
  not nested (`If(cond, [..])` / `Repeat(n, [..])`). A reasonable, documented v1 — simpler editor,
  no nesting UI — but **grouped/nested composition** (repeat a sub-sequence, nested conditions)
  will likely be wanted later; note it for when routines get ambitious.
- `When(cond)` currently has a single state (`data` = strata total). The `Cond`/`state.label()`
  shape is built to extend (shards/buffer/range) — fine for v1; flag for G8+/tuning.

**Verdict.** **Excellent — the run's structural core is now real.** The composability pillar that
was vaporware through G4–G6 is built, tested, and behaviour-preserving. Escalation **closed.** The
trajectory concern is lifted; back to per-commit review for G8 (two agents, on this runtime).

---

## 2026-06-07 · df6a944 — unattended-run checkpoint (docs only)

---

## 2026-06-07 · df6a944 — unattended-run checkpoint (docs only)

**What landed.** A 12-line roadmap "unattended-run log": G4 ✅, G5 ✅, G6 ◑ landed, main green
throughout; G7 + the M/E/D backlog + hardware-gated items (M8b profiling, D7/D8 device
verification) noted as outstanding. No code.

**Assessment.** A responsible checkpoint, and a good sign: the agent **stopped after G6 rather
than charging into the overloaded "G7+"** — which is exactly the boundary I escalated at. So it
implicitly reached the same conclusion (G7+ isn't a clean single milestone) and left a clean
state marker instead of forcing it. The run appears paused here pending the human.

**This is the natural intervention point.** Before anything resumes, the human should re-scope:
pull the **routine runtime + free-form editor** out of "G7+" into its own next-priority
milestone (control vocabulary lands on it; two-agents/expedition/co-op move to G8+). See the
G6 (2/2) escalation entry below for the full rationale. Nothing to fix in this commit.

**Verdict.** Clean wrap-up of a high-quality-but-structurally-incomplete run. Standing
recommendation unchanged and now actionable: re-scope the runtime before continuing.

---

## 2026-06-07 · G6 (2/2) — comprehension-gated vocabulary (`344382c`)  ⚠ ESCALATION

**What landed.** A per-block `required(stratum)` gate (Schematics → seek/circle/goto; Rites →
match), an `unlocked` set synced from `progress.comprehended` each frame; the palette shows
locked blocks ("locked: decode SCH"), nav/filter cycling skips locked options, dispatch refuses
a not-yet-recovered block. Clean, idiomatic (`is_unlocked` via `is_none_or`). 144 tests green,
hash unchanged.

**Strengths.** On-scope and correct: the "decode → the vocabulary grows" loop now works, which
is the right reading of the "tree". And — credit — the agent **did the right thing** with
`when`/`repeat`: instead of faking them with more named-routine gates (the hack I warned
against), it **declined to** and deferred them to "the general free-form routine runtime." That
is good architectural judgment per-commit.

**The escalation (planning failure, not a code failure).** With this commit the situation is
now unambiguous and warrants human intervention:
- The **free-form routine runtime + editor** — the load-bearing core of the entire "compose
  your own automation" pillar — has been deferred at **every** milestone: G4 (gates) → G5
  (steppers) → G6/1 (n-a) → **G6/2 punts `when`/`repeat`/`budget`/`priority`/`survey`/`route`/
  `scanMany` to G7**.
- **G7 has become an impossible catch-all.** Per the roadmap it now must deliver, in one bucket:
  the general routine **runtime**, the free-form **editor**, the **entire control vocabulary**,
  **two independent simultaneous agents**, the **hail**, **cross-agent meta**, **decipherment
  fluency**, the **Concordance/Synthesis** lore arc, the **Resonance/pristine** branch, **and
  co-op (N1)**. That is the whole rest of the game in "G7+".
- Net: the agent keeps shipping clean, tested **surface** increments (vocabulary, economy,
  legibility, gating) while the **structural spine** (the interpreter you author on) is pushed
  into an overloaded terminal milestone. Feature quality is high; the architecture is hollow in
  the middle.

**Recommendation (for the human).** Intervene before the agent attempts "G7+": **split the
*routine runtime + free-form editor* into its own dedicated milestone and prioritise it next**,
ahead of two-agents / expedition / co-op (renumber those to G8+). The control vocabulary
(when/repeat/budget/…) should land *on* that runtime, not be lumped with multiplayer. Until the
runtime exists, every new block is parameter-tweak surface on a two-routine substrate.

**Watch.** If the next commit is the agent attempting "G7+" wholesale (runtime + 2 agents +
co-op together), that's a red flag — it should be one focused runtime milestone. I'll flag the
moment it lands.

**Verdict.** Good commit; bad trajectory. The per-commit work remains high quality and honest;
the **planning has drifted** under unattended execution into deferring the core and bloating the
finale. This is the babysitter's formal escalation: the human should re-scope before G7.

---

## 2026-06-07 · G6 (1/2) — decode economy + decipherment legibility (`47da170`)

**What landed.** `progress.comprehend(stratum)` (spends `DECODE_COST` of that stratum's data,
idempotent + affordability-gated) + `is_legible(script)` + `decodable()` (richest affordable);
a `decode` console block (one-click, auto-targets the richest); and `lexicon.rs` — a tiny
seeded elegiac grammar (opener/subject/coda) that renders a *comprehended* script's
inscriptions as **translated words** (length-tiered, deterministic in seed+cell, ASCII). v3
payload (append-only). 142 tests green, clippy clean, golden hash unchanged.

**Strengths — the best commit in the run so far.**
- **Faithful to the agreed design.** Decipherment-as-payoff (game-mechanics §9) via a
  **procedural-poetic seeded grammar with no authored lore** (§6) — exactly the decision taken.
  The register is genuinely on-mood and melancholy; the word-bank is tasteful, not Mad-Libs-y.
- **Clean + correct.** `comprehend` is properly idempotent and affordability-gated; legibility
  changes only *display* while the find id still hashes the original glyphs (so collecting stays
  stable across decode) — a careful, correct call. Determinism + variation are tested.
- Sensible scoping: this is explicitly the *decode/legibility* half; no overreach.

**Critiques / structural watch (unchanged, now sharper — forward-looking).**
- No interpreter work here, correctly (it's the 1/2 half). The real test is **G6 (2/2)**, whose
  planned `when`/`repeat` control blocks **cannot** be faked with named-routine accessors — so
  2/2 should *force* a genuine runtime, or reveal another special-case. That's the commit to
  scrutinise.
- **The bigger risk: "author your own routines" has quietly gone unscoped.** G5 deferred
  free-form insert/remove of blocks to "G6's richer vocabulary," **but the G6 brief does not
  scope it** (G6 = decode + when/repeat + gated palette). So the headline pillar — *composing
  your own automation* — has now slipped G4→G5→G6 and currently has **no home in any brief**. It
  is at risk of being silently dropped while the vocabulary and economy grow around a substrate
  you still can't freely author on.
- Minor: the `Routine` model + `console.rs` header docs are still the stale G4 text.

**Watch-items for G6 (2/2) / G7.** (1) Does `when`/`repeat` land as a **real runtime
interpreter** or another hardcoded gate? (2) **Where does free-form routine authoring actually
get built?** If it's not in 2/2, that's worth escalating to the human — the "genuinely
interesting purely through menus" pillar is otherwise unbuilt.

**Verdict.** Excellent on its own terms — the melancholy comprehension heart, done correctly
and tastefully. The run's quality is high *feature-by-feature*; the standing concern is purely
structural: the composability core keeps being deferred and has now lost its scope. Strong
commit; unchanged architectural worry.

---

## 2026-06-07 · G5 — console editor (pickers), match & nav (`24067a1`)

**What landed.** `Block` gains `Seek`/`Circle` and a **parameterised** `Match(MatchField)`
(v1 field: `Rare`). `cycle_param` (←/→) steps the *parameter* of the routine under the
cursor — `drift`'s nav block (drift→seek→circle) and `survey`'s collect filter
(none↔`match(rare)`). `collect_aimed_where(pred)` lets auto-collect apply the filter; `seek`
steers the autopilot to the nearest known-uncollected site. Routine edits persist in a `co=`
share segment (round-trip tested). 140 tests green, clippy clean, golden hash unchanged.

**Strengths.**
- **Honest scoping** — an explicit *"As-built vs the original plan"* section in the brief
  states it shipped parameter-steppers, **not** free-form authoring, and a sensible in-flight
  correction (dropped `match(uncollected)` as a no-op, used `rare`). Good autonomous discipline.
- `match(rare)` is a genuinely useful selective-collect; `seek` is real routing; no-typing
  ←/→ pickers are on-brand; persistence is tested.

**Critiques (where it's due — this is the flagged milestone).**
1. **The headline deliverable did not land — for the second time.** G5's brief was *"author
   your own routines"* / a *wiring editor*. What shipped is **parameter-cycling on the two
   fixed given routines** — you cannot create a routine, or insert/remove blocks. Free-form
   composition is now deferred to **G6**. The milestone was marked ✅ by **redefining success
   downward** (documented, but still scope erosion on the core pillar).
2. **Still no real interpreter — the gate pattern was extended, exactly as warned.** `Routine`
   is unchanged (the G4 `continuous`/`on_scan` two-bucket); execution is still hand-written
   **accessors the app branches on** — now `nav_block()` + `filter()` on top of G4's
   `drift_enabled()`/`survey_enabled()`. `cycle_param` is **hardcoded per routine name**
   (`if name=="drift" … if name=="survey"`); it won't generalise. The genuine trigger→steps
   interpreter — the thing that makes "compose your own automation" real — **still does not
   exist** and is now G6's debt on top of G6's own scope.
3. **Parameterisation is half-done.** `Match(field)` is parameterised (good), but `Scan` is
   *still* a param-less enum hardcoded to "scan(shards)"; `MatchField` has a single value. The
   `scan(item)` pattern remains unmodelled.
4. **Stale module docs.** `console.rs`'s header still says "G4" and `Routine`'s doc still reads
   *"no player editor in G4 … G5's editor will generalise it"* — now self-contradictory (G5
   landed without generalising). A tell that the generalisation didn't happen.

**Watch-items.** G6 now carries **both** its own large scope (control/budgets/decode/unlock
economy/legibility) **and** the twice-deferred free-form authoring + real interpreter. If G6
also defers the interpreter, the "genuinely interesting purely through menus" pillar is
slipping indefinitely while surface vocabulary accretes on a non-general substrate. **Hold G6
to: a real routine model + interpreter (create/insert/remove arbitrary blocks), or explicitly
escalate that the pillar is at risk.** Also: parameterise `Scan`; refresh the stale docs.

**Verdict.** Honest, tested, useful *increment* — but a **soft miss on the milestone's intent**
and the second deferral of the architectural core I flagged at G4. Not broken; drifting. The
agent is building outward (vocabulary, pickers, persistence) on a substrate whose load-bearing
middle (the interpreter) keeps getting postponed. This is the babysitter's headline concern so
far.

---

## 2026-06-07 · G4 — block substrate & operations console (`f1adfb7`)

**What landed.** A `console` module in `scraped-again` (227 lines): a Tier-0 block enum
(`Scan`/`Collect`/`FireBeam`/`Spend`/`Goto`/`Drift`/`OnScan`), two given routines (`drift`;
`survey` = scan → on-scan → collect) shown as their blocks, cursor/toggle model, terminal
render — pure, 4 unit tests. Wiring in `lib.rs`: `O` opens the console, ↑↓ select, Enter
run/toggle; manual block clicks go through `dispatch_block` (the real G1–G3 effect paths);
`scan_pulse` shared by the survey routine + a manual scan click. 137 tests green, clippy
clean, wasm builds, golden hash + headless render unchanged.

**Strengths.**
- **Faithful to the brief's intent**: re-expression, not rewrite — `dispatch_block` reuses
  the existing collect/scan/beam paths, so behaviour-parity is real (and tested).
- **Excellent discipline on the autonomy ask**: a clear *"Assumptions / decisions taken solo"*
  header in `console.rs` documenting the three judgment calls — exactly what unattended work
  should leave behind.
- Clean, well-tested pure model; on-aesthetic terminal render on the E17/HUD path; no-typing
  cursor+confirm (controller/phone-first) as designed.

**Critiques (where it's due).**
1. **The "runtime" is not yet a runtime — the given routines are *boolean gates*, not
   interpreted.** `survey_enabled()` / `survey_autocollects()` / `drift_enabled()` are queried
   by hand-written branches in `lib.rs`; only *manual* clicks are genuinely dispatched. For
   G4's two fixed routines + parity this is a fair shortcut (and the code admits "G5's editor
   will generalise it"), **but there is no general trigger→steps interpreter yet.** This is the
   #1 risk for G5: the editor must introduce a real interpreter, *not* extend the
   flag-gating — otherwise player-authored routines won't have an execution model. Watch that
   G5 doesn't bolt the editor onto booleans.
2. **Blocks aren't parameterised yet.** `Block` is a param-less enum; `Scan` is hardcoded and
   merely *labelled* `scan(shards)`. The design's "parameterised blocks (a single typed arg
   whose options unlock)" — `scan(item)`, `match(field)` — isn't modelled. Fine while only
   `shards` exists, but G5/G6 will need to retrofit a param onto `Block` (a small refactor);
   the brief technically said G4 ships `scan(item)`, so this is a minor drift to keep honest.
3. **The given auto-collect is largely inert at cruise** (their assumption #1): collect reuses
   `collect_aimed`, so sites below the aim ray aren't taken until you fly low (or reach grows
   in G6). Honest and parity-preserving — but it means the headline "auto-collect closes the
   hands-off loop" doesn't visibly *do* much yet. Acceptable for G4; **G6 must actually make
   auto-collect meaningful**, or the "autopilot is a complete way to play" pillar stays
   unproven.
4. **"Clickable blocks" is, for now, cursor+confirm** (mouse hit-testing deferred to G5).
   Reasonable deferral; just noting the design word "clickable" is aspirational at G4.

**Verdict.** Good, honest, well-tested G4 that achieves the stated goal (parity + the console
surfaced) and documents its shortcuts. No correctness concerns. The deferrals are legitimate
*provided* G5 delivers a genuine routine interpreter and block parameterisation rather than
extending the G4 gates — that's the thing to hold the next milestone to.
