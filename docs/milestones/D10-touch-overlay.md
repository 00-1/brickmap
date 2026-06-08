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

- [ ] Two edge sliders (track + value-tracking handle) + four labelled buttons (1/2/A/B) render,
      dimmed, from `touch::Layout`.
- [ ] Press/active highlight; labels reflect the context-current action (flight/walk/menu).
- [ ] Shown on touch targets / after first touch; hidden on desktop+pad.
- [ ] Geometry derived from `touch::Layout` (unit-tested it matches the hit-zones); headless A/B
      shows the overlay.
- [ ] Any new HUD rect primitive is **generic** in `bm-render` (no game concept); boundary intact.
- [ ] CI green (fmt / clippy -D / tests / wasm); golden hash + render unchanged (overlay touch-gated).
- [ ] On-device feel-tuning (size/opacity/placement) noted as the human follow-up.
