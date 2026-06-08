# D9 — Touch controls (phone)

> **Status: ready to build.** A native **touch UI** for phones — sibling to D7 (gamepad). Today
> mobile input is the D7 digital pad; there's no touchscreen handling. This maps **all core
> controls** onto a 2-slider + 4-button + tap-the-display layout, exploiting that *Scraped Again*
> is **autopilot-first** and **tap-native** (the console is clickable blocks; the survey-beam is
> the universal interaction verb). Engine-side touch-event plumbing in `bm-platform`; the overlay +
> control mapping in `scraped-again`.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Play the whole core on a phone: fly (steer + altitude + auto-forward), act (tap = beam/
  collect/interact), and run the console + map — via an on-screen overlay, no physical controller.
- **Demonstrable outcome.** On a touchscreen: left/right sliders steer + change altitude; **A**
  toggles cruise (auto-forward); **tap the view** fires the beam at that point (collect/board);
  **1** opens the console and you **tap blocks** to run/edit them; **2** opens the map; **B** lands/
  boards. (Verified: the overlay renders headless + the touch→control mapping is unit-tested; *feel*
  needs a real phone.)
- **De-risks.** That the full control set fits a thumbs-only layout — which it does *because* of the
  autopilot-first + tap-native design, not in spite of it.

## The layout (the human's sketch)

Landscape: a central **display** (the game) with a **control strip** down each side (game shows
through, dimmed). Left strip: vertical **slider** + buttons **1**, **2**. Right strip: vertical
**slider** + buttons **A**, **B**. Left thumb = altitude + menus; right thumb = steering + cruise/
board.

## Control mapping

| Input | Flight | Walk | Menu open (console/map/codex) |
|---|---|---|---|
| Right slider | turn / yaw | turn | — |
| Left slider | climb / descend | forward / back | — |
| **A** (right) | **cruise** (auto-forward) on/off | **auto-walk** | — |
| **B** (right) | **land & exit** (→walk); **hail** ship when away | **board** / **hail** | back / close |
| **1** (left) | **Console** | Console | — |
| **2** (left) | **Map** | Map | — |
| **tap display** | **fire survey-beam at point** = collect / board / interact | same | **tap a block/routine** = trigger / select / edit (+/− param) |
| **drag display** | *(unused — view follows heading)* | — | scroll / drag-reorder |

**Coverage of the existing control set:** forward → `A` cruise (autopilot-first ⇒ no manual
thrust); yaw/altitude → sliders; collect/cast-beam/board/interact → **tap** (the beam = the
universal verb); autopilot → a console routine toggle (no button needed); enter/exit/hail → `B`;
console/map → `1`/`2`. **Secondary toggles** (codex, mute, ink, biome, edit, the D6 feature
toggles, pixel-scale, photo) go into a **settings panel reached from the console**, *not* onto
buttons — a phone player needs *fly · tap · console · map · board*; the rest is menu depth.

## Design sketch

- **Engine (`bm-platform`):** surface winit **touch events** (`WindowEvent::Touch`) as a generic
  normalised stream (id, phase, position) alongside the existing input — **content-agnostic**, no
  game concepts. (Mirrors how gamepad input is a generic `PadInput`.)
- **Game (`scraped-again`):** an **on-screen overlay** (two sliders + four buttons) drawn on the
  HUD/text path, dimmed over the edges; a **touch-router** that turns touches into the same
  `CameraController`/mode/console actions the keys/pad already drive (reuse those paths — no new
  control logic, a new *input source*). Tap-on-view → the existing beam-cast/DDA-pick at the hit
  point. Menu-open → taps hit-test the console/map UI directly.
- **Modal:** the four buttons + tap reinterpret by context (flight / walk / menu) — the table above.
  Pure mapping function (touch state → action), unit-testable without a screen.

## Decisions to resolve (pinned defaults — veto any)

1. **Look:** *view follows heading* in play; **no drag-to-look** while flying (frees the thumbs).
   Free-look (drag) only in **photo mode**. *(The clean fix for "two thumbs can't steer + look.")*
2. **Tap = beam:** a tap **casts** the survey-beam at the point (collect/board/interact in one).
   **Hold-to-extend** / drag-aim is a later enrichment, not v1.
3. **Walk left-slider:** forward/back (no altitude on foot). Jump deferred.
4. **Secondary toggles:** a **settings panel** off the console; not buttons.
5. **Autopilot vs cruise:** `A` = cruise (you still steer); full self-steer is the console
   `drift`/`seek` routine. Touching a slider overrides (as auto-fly already yields to input).

## Tests

- **Pure mapping:** touch state (slider positions, button taps, view-tap point) → actions, across
  flight/walk/menu contexts (deadzones, slider→rate, context switch). Unit-tested, no screen.
- **Overlay renders** on the headless path (sliders + buttons visible, dimmed).
- Native + wasm build green; golden voxel-hash + headless render unchanged (overlay is additive,
  default-off when no touch device / behind a flag).

## Risks & mitigations

- **Final feel needs a real touchscreen** (device-gated, like D7/D8): build + test the *logic* and
  *overlay* now; **defer the on-device feel-tuning** (slider sensitivity, button size/placement,
  tap targeting) to the human's phone. Don't block the build on it.
- **Engine boundary:** keep `bm-platform` touch events generic (no game concepts); all
  mapping/overlay in `scraped-again` (CI boundary check stands).
- **Web touch:** the Pages build can also receive touch — wire the same path; web is lower-priority,
  fine to follow native.

## Acceptance checklist

- [x] `bm-platform` surfaces generic normalised touch events (`touch::TouchPoint`/`TouchPhase`,
      pixel→`0..1` normalisation, unit-tested) — engine-generic; no winit/game dep.
- [x] `scraped-again` overlay (2 sliders + 4 buttons, context-labelled) on the HUD/text path —
      `touch::Layout::overlay` (pure, unit-tested), appended once a touch device is in use.
- [x] Touch-router maps to the existing camera/mode/console actions per the table (flight/walk/menu),
      via a **unit-tested pure mapping** (`classify`/`slider_value`/`button_tap`/`view_tap`);
      tap-view casts the beam (at centre in v1 — per-pixel aim is deferred feel).
- [x] `A` cruise (toggle auto-fly), `B` board/exit/hail (`touch_board`), `1` console, `2` map,
      tap-blocks-in-console (Home-view y→row) all wired onto the existing paths.
- [x] CI green (fmt / clippy -D / tests / wasm); `bm-*`→game boundary intact; golden hash +
      headless unchanged (overlay only shows after a touch; engine touch type is generic).
- [x] On-device feel-tuning (slider sensitivity `TOUCH_TURN`, button size/placement, per-pixel tap
      targeting, edge-strip visual + dimming) noted as the device-gated human follow-up.
- [x] Docs: roadmap D9 entry + this checklist ticked.

## As-built (2026-06-08)

Pinned defaults followed (no fork). v1 deviations are all explicitly the deferred *feel*-tuning:
- **tap-view casts the beam at centre** (`cast_beam`), not the unprojected tap point — per-pixel
  tap targeting is in the brief's deferred feel list; the beam-along-forward path already exists.
- **`A` = toggle `auto_fly`** (the existing cruise/autopilot, matching the pad's A) — the
  cruise-vs-full-self-steer nuance (Decision 5) rides the console `drift`/`seek` routine.
- **Overlay is a compact HUD text line** (sliders as `·●·` bars + labelled buttons) on the existing
  text path; the edge-strip placement + dimming is the deferred on-device visual.
- **`B`** boards if next to the parked ship, **hails** it if away (on foot), lands & exits while
  piloting (`touch_board`, reusing `toggle_cruiser` + `hail_ship`).
- **Menu tap** maps `y`→console Home-row + confirm (approximate; precise targeting deferred).
