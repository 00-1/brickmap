# G2 — The survey-beam (the active verb + the engine capability it needs)

> **Status: ready to build, after [G1](G1-data-strata-codex.md).** The signature manual
> verb of **Scraped Again** ([`../game-mechanics.md`](../game-mechanics.md) §6), and the
> one slice that needs a **new engine capability** — so it also de-risks the post-palette
> overlay flagged in [`M9-engine-game-split.md`](M9-engine-game-split.md). Split across
> `bm-render` (the capability) and `scraped-again` (the beam itself).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** The **warm survey-beam**: a heavy, deliberate, vivid energy line you cast that
  **persists then fades**, **collects every glyph along its path** (feeding G1's strata +
  codex), is a **rideable rail** (attach + slide, any angle), and is the **universal
  interaction verb** (contact boards the cruiser). Plus the **engine capability** it
  requires: a **post-palette, depth-aware, non-palettised emissive overlay** draw.
- **Demonstrable outcome.** Cast a beam: it draws as a vivid line *over* the palette yet is
  *occluded* by terrain in front of it; glyphs along it are collected; you attach and ride
  it up a cliff / across a gap; if it fades while you're on it you drop; aim it at the
  parked cruiser and you're reeled in and board.
- **De-risks.** The engine's **scene-depth-through-the-post-chain** capability (needed by
  any future overlay), and that the beam *feels* good (heavy, readable, on-aesthetic). Also
  fixes the "you get stuck on foot too easily" problem.

## Scope

**In — engine (`bm-render`):**
- Retain **scene depth through the post chain** (don't discard it when an overlay needs it
  — interacts with the M8 depth-discard note).
- A **final overlay pass** the game feeds line/segment primitives into: drawn **after the
  palette/dither present**, **non-palettised** (keeps raw colour), **depth-tested** against
  the scene, with a cheap own-glow (it misses the pre-palette bloom). Exposed as an engine
  API (a small "overlay primitives this frame" feed, like the splat feed).

**In — game (`scraped-again`):**
- **Cast**: from the player, in the aim direction (free, any angle — DDA gives the aim/
  direction), a straight segment of a max **Length**; **heavy/deliberate** (wind-up, not a
  zap).
- **Persist + fade**: a **Lifespan** timer, visual fade until it stops functioning.
- **Collect-on-cast**: one-shot sweep — every glyph the segment intersects is collected via
  G1's `collect` path (incl. *through* ethereal colossi).
- **Ride**: **attach + 1-DoF slide** along the segment (parametric position; integrate with
  the `player` walker; collide via the `solid` oracle for arrival). **Drop on expire**
  (walker gravity) and **detach on demand**; **fire-and-attach mid-fall** works.
- **Board the cruiser**: beam-contact with the parked cruiser fires the **E19 enter/exit
  mode-machine "enter"** — *zip-then-board* (reeled along, not teleport), with **light
  lock-on** toward the ship and **reach-gating** (only if within beam reach).
- **Warm colour** (distinct from G3's cool scan beam).

**Out:** the cruiser **auto-scan** + map (G3); **auto-collect** + multi-beam **Capacity**
+ tree-driven Lifespan/Length upgrades (G4 — start with a single beam + fixed basic
values); the cruiser-mounted (aerial) beam (flight branch, later).

## Design sketch

- **Engine seam.** Mirror the existing splat feed: `bm-render` gains `set_overlay_lines(&[
  OverlayLine{ a, b, color, glow }])` (or similar), drawn in a new pass after `palette.wgsl`
  presents, sampling the retained depth target to occlude. Keep depth alive that frame.
  Document it as an engine capability (the M9 note becomes real).
- **Game `beam` module.** `struct Beam { seg: (Vec3,Vec3), born, life, attached_t }`; a
  small system: cast → register overlay line + run the collect sweep; tick → fade + expire;
  while attached → drive `t` from input, set walker position along the seg, drop on expire.
- **Aim** reuses the E14 DDA (direction + first-hit for lock-on/board targeting).
- **Board** calls into the existing mode machine in the game's app/`player` glue.

## Decisions to resolve (with recommended defaults)

1. **Beam length model.** *Default:* **fixed max Length** cast in the aim direction
   (reach = how far you ride before fade); not a grow-while-held beam.
2. **Aim.** *Default:* **free direction, any angle** (DDA gives direction + a hit point for
   board lock-on); not anchor-to-surface-only.
3. **Overlay API shape.** *Default:* a **per-frame line-primitive feed** on `bm-render`
   (engine owns the *capability*; the game owns *what* lines). Avoid a beam-specific engine
   type — keep it a generic overlay-line feed.
4. **Depth retention cost.** *Default:* keep scene depth only **when an overlay is active**
   (no overlay → keep the M8 discard optimization).

## Tests

- **Lifespan/fade** timing; **expire → drop** transition (pure where possible).
- **Collect-on-cast** line↔glyph intersection (a segment vs nearby inscription positions),
  feeding G1's tested collect path (no double-count).
- **Attach/slide** parametric position; **reach-gate** predicate for board.
- **Headless render** (D1): a cast beam is **vivid post-palette** *and* **depth-occluded**
  by terrain in front — an A/B that would catch "drawn as a flat HUD line" or "palettised".
- All four targets build; the `bm-render` overlay capability has no game dependency
  (boundary intact).

## Risks & mitigations

- **Depth-through-post touches the hot path / tilers** (M8). *Mitigation:* gate retention
  to frames with an active overlay; measure on the HUD; it's the engine's call where to
  store/load depth.
- **Ride physics feel** (clipping, arrival, drop). *Mitigation:* 1-DoF parametric slide
  (not surface-walking); collide only on arrival/detach via the `solid` oracle.
- **Engine/game boundary** — the overlay must stay a generic engine capability, not a
  beam-aware one. *Mitigation:* feed generic line primitives; CI boundary check stands.

## Acceptance checklist

- [x] `bm-render`: scene depth retained through the post chain (only when an overlay is
      active — keeps the M8 discard otherwise); a **post-palette, depth-aware, non-palettised**
      overlay pass with its own additive glow, composited over the frame; generic (an
      `OverlayVertex` feed + `set_overlay`, no game dep — CI boundary check still green).
- [x] Cast → persist → **fade** (eased); **collect-on-cast** along the path (feeds G1's
      strata/codex via the shared collect seam); **attach + ride** any angle (1-DoF slide along
      the segment); **drop on expire** (walker gravity resumes); **mid-fall re-cast** re-attaches.
- [x] Beam-contact **boards the cruiser** (lock-on: cruiser near the beam line; **reach-gated**
      to the beam length) → the E19 "enter" as a ranged alternative to walk-up-and-press-E.
- [x] Headless A/B (opt-in `SCRAPED_BEAM`) confirms the beam is **vivid over the palette**
      *and* **occluded by terrain** it rises through.
- [x] CI green (fmt / clippy -D / tests / wasm); crate-graph boundary intact.
- [x] `game-mechanics.md` §13 ticked for G2; the M9 "upcoming render capability" note marked
      realised; this checklist complete.

> **Decisions:** defaults taken — fixed max **Length** cast in the aim direction; free aim
> (any angle); a **generic per-frame overlay-vertex feed** on `bm-render` (the game expands its
> beam into camera-facing ribbons — the engine stays beam-agnostic); depth retained **only when
> an overlay is active**. Cast is **left-click** (once the pointer is captured), so it works on
> web too. The zip-then-board reel is instantaneous for now (the lock-on + reach-gate are the
> load-bearing parts); a visible winch animation is a polish follow-up.
