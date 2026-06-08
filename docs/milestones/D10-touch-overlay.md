# D10 — Touch control overlay (visible sliders + buttons)

> **Status: ready to build.** Follow-up to D9 (touch *mapping* works well in playtest, but the
> on-screen visual was deferred to a HUD text line — **the player can't see where to touch**). This
> draws the two edge sliders + four buttons as **visible on-screen controls** with feedback. Game-
> side HUD overlay in `scraped-again`; reuses the **existing `touch::Layout`** (already the source
> of truth for hit-testing), so the visual and the hit-zones stay in sync by construction.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Render the D9 controls — left/right vertical **sliders** + buttons **1, 2** (left) and
  **A, B** (right) — as visible, dimmed on-screen controls so the player sees *where* to touch and
  gets press/position feedback.
- **Demonstrable outcome.** On a touch device (or once a touch is seen): two dimmed edge strips with
  a slider **track + handle** (handle at the current value), four **labelled** buttons; the handle
  tracks the finger; buttons **highlight on press**; labels show the **context-current** action
  (e.g. `A` reads "cruise on/off", `B` "board"/"exit"/"hail" per mode). Visible in a headless
  screenshot.
- **De-risks.** Discoverability — the controls are currently invisible; this is the gap between
  "works" and "usable by someone who didn't build it."

## Scope

**In:**
- Draw the controls **from `touch::Layout`** — the strip rects, slider tracks, button rects are
  *already* defined there for hit-testing; render them so visual == hit-zone by construction.
- **Slider handle** at the current value; **button press/active highlight**; **context labels**
  (the per-mode action from the D9 mapping — flight/walk/menu).
- **Dimmed / translucent** so the game shows through the edge strips (per the human's sketch).
- Appears on **touch-capable targets / once a touch is seen** (no clutter on desktop+pad).

**Out:** the touch *mapping* itself (D9 ✅); per-pixel beam aim (separate D9 follow-up); deep
theming / final visual polish (on-device eye-tuning — see below).

## Design sketch

- A **HUD overlay** over the scene: reuse the `hud` text path for the labels + a **translucent
  filled-rect** draw for the strips / tracks / handles / buttons.
- If a filled-rect/quad HUD primitive isn't already in `bm-render::hud`, add a **generic** one
  (engine-generic — rect + colour + alpha; **no game concept**); the game composes the control
  shapes from `touch::Layout`. Keeps the `bm-*`→game boundary intact.
- **Single source of truth:** the overlay geometry is derived from `touch::Layout` (not duplicated),
  so it can never drift from the hit-testing.

## Decisions to resolve (pinned defaults — styling is the human's later call, per "save Qs")

1. **Look:** subtle **dimmed/translucent** HUD (palette-consistent), *not* the vivid post-palette
   beam treatment — controls should sit quietly under the world. *(Final colour/opacity = eye-tuning.)*
2. **Visibility:** show on touch-capable targets / after the first touch; hide on desktop+pad.
3. **Labels:** short, context-current (the active mapping per mode); icons are a later polish.
4. **Feedback:** handle tracks value; pressed button brightens briefly. Haptics/sound deferred.

## Tests

- **Geometry from `Layout`:** assert the rendered control rects are derived from `touch::Layout`
  (the same source as hit-testing) — a unit test that the draw-list matches the layout regions.
- **Headless render** shows the overlay (sliders + four labelled buttons) — a screenshot A/B
  (opt-in flag, like `SCRAPED_BEAM`, so the golden default stays clean).
- Native + wasm + clippy -D + tests green; golden voxel-hash + render unchanged (overlay
  default-off / touch-gated); boundary intact (any HUD rect primitive generic).

## Risks & mitigations

- **Visual feel needs a real screen** (placement, size, opacity, thumb-reach) → that's **on-device
  eye-tuning** (the human, on a phone). Build the visible, correct overlay now (headless-verifiable);
  defer the *feel*-tuning — don't block on a phone.
- **Drift from hit-zones** → derive the visual from `touch::Layout`, never a parallel copy.

## Acceptance checklist

- [x] Two edge sliders (track + value-tracking handle) + four buttons (1/2/A/B) render, dimmed,
      from `touch::Layout` (`Layout::overlay_rects` → the generic `RectOverlay`).
- [x] Press/active highlight (a button brightens for ~0.18 s after a tap); labels reflect the
      context-current action via the HUD line (`A`=cruise · `B`=board/back per mode). *(Per-button
      glyph labels on the rects are a noted polish — see As-built.)*
- [x] Shown after the first touch (`touch_seen`); hidden on desktop+pad (no touch → empty rects).
- [x] Geometry derived from `touch::Layout` (unit-tested `overlay_rects_derive_from_layout_hit_zones`
      asserts track == slider region, button rects == button regions, handle in track); headless A/B
      (`SCRAPED_TOUCH=1`) shows the overlay (byte-differs from the clean render).
- [x] The new HUD rect primitive (`hud::RectOverlay` + `UiRect`) is **generic** in `bm-render` (just
      rects + rgba; no game concept); the game composes the control shapes; boundary intact.
- [x] CI green (fmt / clippy -D / tests / wasm); golden hash + render unchanged (overlay touch-gated;
      headless A/B opt-in).
- [x] On-device feel-tuning (size/opacity/placement) noted as the human follow-up.

## As-built (2026-06-08)

- **Generic engine primitive:** `bm_render::hud::RectOverlay` draws a list of `UiRect`
  (`0..1` screen + rgba, alpha-blended) over the frame, under the HUD text — engine-generic, no
  game concept. `State::set_ui_rects` feeds it.
- **Single source of truth:** `touch::Layout::overlay_rects(left, right, pressed)` builds the rects
  from the *same* `Layout` rects used for hit-testing (track = the slider rect; handle at the value;
  button rects = the button regions; pressed = brighter). Unit-tested that the draw geometry matches
  the hit-zones, so they can't drift.
- **Labels:** the context-current action labels stay on the existing HUD text line (`[1]console
  [2]map [A]cruise [B]board`, relabelled per mode) — the player sees the strips/handles/buttons +
  reads the action names. **Per-button glyph labels drawn *on* the rects** would need positioned
  text quads; deferred as polish (the HUD line + fixed button positions convey it for v1).
- **Feel deferred:** colours/opacity/size/placement are the on-device eye-tuning (pinned defaults
  here); a real-phone pass tunes them.
