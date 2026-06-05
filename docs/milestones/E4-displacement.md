# ✨ E4 — Sub-voxel surface displacement (cheap relief)

> Status: **done** ✅. Exploration rung in [`../roadmap.md`](../roadmap.md);
> backlog rationale [`../exploration-backlog.md`](../exploration-backlog.md) §E.
> Builds on M4 (the material detail texture) and E3 (lighting).

## Goal · Outcome · De-risk

- **Goal:** give faces *depth* — relief that catches the light — without adding
  geometry or shrinking voxels.
- **Outcome:** blocks stop looking flat; stone/dirt/grass read as bumpy under the sun.
- **De-risks:** the §E cost worry — true parallax-occlusion mapping marches per
  fragment, which is exactly the bandwidth/fragment cost we can't afford on weak
  hardware. So we take the **cheap** route.

## Scope

**In:**
- **Bump relief (no marching):** treat the material detail texture as a height field;
  sample it at small UV offsets to get a gradient, perturb the face normal in tangent
  space, and re-light the sun term with it. A few extra texture taps, no ray march.
- Tangent basis comes for free from the axis-aligned face (the two in-plane axes).
- **Toggle** (`relief`, per the D6 norm) + a strength constant. Distance-faded for
  free: the detail texture is mipped + nearest, so far fragments sample flat mips.

**Out (later / rejected for cost):**
- True parallax-occlusion / relief *mapping* (per-fragment marching) — the §E 🟡 risk;
  revisit only if it's cheap enough on the reference iGPU (M8).
- Per-material height maps authored separately — the detail texture doubles as height.

## Tests

- Mostly a shader effect — eyeballed via a close-up headless render + the look journal.
  The detail texture itself is already unit-tested (deterministic, grayscale).

## Acceptance checklist

- [x] Bump relief in the chunk shader (gradient of the detail texture perturbs the sun
      normal); cheap (no marching, 2 extra taps), toggleable (`relief`).
- [x] Visible relief close up; fades with distance (mips); look-journal entry.
- [x] Runs native + web; CI green; docs synced.

> Status: **done** ✅ — cheap bump relief from the detail texture's gradient; the 10th
> live toggle (`relief`). Deferred (cost): true parallax-occlusion marching (revisit on
> the reference iGPU at M8).
