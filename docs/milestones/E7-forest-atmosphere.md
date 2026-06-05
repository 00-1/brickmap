# ✨ E7 — The forest & atmosphere

> Status: **planned** ⏳ (skeleton; fleshed out just before building, per the roadmap
> planning rule). Second rung of the point-cloud / foliage pivot — the *destination*
> look. Builds on **E6** (the splat pipeline + ground foliage). Reference: the Superbien
> point-cloud forest (glowing point trees, light shafts, ethereal density).

## Goal · Outcome · De-risk
- **Goal:** turn the lush ground (E6) into a **point-cloud forest with mood** — trees as
  point structures, layered vegetation, light through the canopy, a lush palette.
- **Outcome:** flying through the world feels like the Superbien reference: drifts of
  glowing points forming trees and undergrowth, atmospheric and alive.
- **De-risks:** density/perf at forest scale (lean on E6's bounded-count + LOD), and the
  aesthetic itself (this is where the *look* is won or lost — heavy look-journal use).

## Scope (provisional — refine before building)
**In:**
- **Trees / tall point structures:** procedural trunks + canopies emitted as splat
  clouds (reusing E6's pipeline), placed by a density/forest noise so woods cluster.
- **Layered vegetation:** a second foliage tier (ferns/bushes/taller grass) for depth.
- **Atmosphere:** light shafts / god-rays (cheap — fog + a screen-space gradient or
  additive streaks), brighter/emissive foliage tips so bloom gives the glow, and a
  **lusher terrain palette** to match.
- **Foliage LOD / distance density** so the forest stays bounded on weak hardware.
- A `forest` (or extend `foliage`) toggle.

**Out (later):**
- Seasons / day-night interplay with the canopy.
- Terrain distance-dissolve into points (reframed **M7**).
- Animated creatures among the trees (still parked).

## Acceptance (provisional)
- [ ] Point-cloud trees clustered into woods; canopy + trunk read clearly.
- [ ] Light-shaft / glow atmosphere; lusher palette; the Superbien mood is recognisable.
- [ ] Bounded + LOD'd on weak hardware; toggle; HUD count.
- [ ] Native + web; CI green; docs synced; snapshot + render in chat.

> Status: **planned** ⏳ — write the full brief once E6's splat path is proven.
