# G3 — Cruiser auto-scan & the map opportunity surface

> **Status: ✅ landed (2026-06-07).** The cruiser auto-scans a forward cone while piloting
> (cool flicks via G2's overlay), marking nearby sites *known* without collecting; the explored
> map shows scanned-but-uncollected sites as amber rings (the opportunity surface), and the HUD
> shows "known / found". Scanned state rides the `pg=` save (v2). See the acceptance checklist.
> The **autopilot/idle** half of *Scraped Again* ([`../game-mechanics.md`](../game-mechanics.md)
> §6, §8.1): the cruiser **reads the world as it drifts** and the **map fills with the
> opportunity surface** you then triage. All in `scraped-again` over engine primitives —
> the only engine piece is reusing G2's overlay for a *second* (cool) beam colour, which
> is already generic. Follow the gates in [`../development.md`](../development.md).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Make **autopilot a real way to play**: as the cruiser drifts, it **auto-scans**
  what's ahead (a short, cool beam — distinct from G2's warm collection beam), **discovering**
  sites without harvesting them, and the **explored map (E10) becomes an opportunity surface**
  — scanned-but-**uncollected** inscriptions/colossi show as pins you can then steer to (manual)
  or, later, auto-collect (G4).
- **Demonstrable outcome.** Fly (or sit on autopilot); a cool scan beam flicks across the
  forward zone; the map (`N`) shows **distinct markers for scanned-uncollected vs collected**
  sites, and a count of "known / collected" on the HUD. Collecting one (G1 `T` / G2 beam)
  flips its marker from *uncollected* to *done*. Scanned state rides the same `seed + sparse
  log` save (extends G1's `pg=`).
- **De-risks.** That the **idle layer feels good and legible** (scan→map→triage loop) before
  the tech tree (G4) sits on top of it; and that two beams (warm collect / cool scan) read as
  clearly different through the palette.

## Scope

**In:**
- **Auto-scan**: each frame on autopilot (and manual flight), fire short-lived **cool** scan
  beams into the **forward on-screen zone** (a cone ahead of the camera; §6 — *not* a radius
  you'd never see). Reuses G2's `bm-render` overlay with a cool colour + a brief flick lifespan.
- **Discovery**: any inscription/colossus whose position falls in the forward zone (within a
  fixed basic **scan range/rate** — tech gating is G4) is marked **scanned/known** (a stable
  `find_id`, reusing G1's scheme) without collecting it.
- **Map opportunity surface**: extend the E10 map so a **scanned-uncollected** site draws a
  distinct marker from a **collected** one (and from the existing text/pristine/cruiser
  markers). The map becomes the triage surface; the key/legend updates.
- **HUD**: a small "known N · collected M" readout (sits with the G1 strata line).
- **Save**: add the **scanned set** to the `progress` payload (append-only, beside the codex).

**Out (later slices):** the **tech tree + Sensing upgrades** (range/rate, *Deep Scan*,
*Spectral Sight* for ethereal colossi, *Stratum Hints*) and **auto-collect** + **auto-route**
— all **G4**; map zoom/Cartograph tiers; any new render path (the scan beam reuses G2's overlay).

## Design sketch

- **Lives in `scraped-again`.** Reuses: G1 `progress` (`find_id`, the collected set, the
  `share` payload), G1 `collectible`/`structures` placement, G2 `bm-render::overlay` (the cool
  scan beam), E10 `map` (markers), the autopilot/`auto_fly` + camera.
- **`Scan` state** — a `HashSet<u64>` of **scanned** `find_id`s (sites known but maybe not
  collected), added to `progress` next to `seen` (collected). `scanned − seen = the opportunity
  set` the map highlights.
- **Forward-zone test** — a site at `p` is scannable when `(p − cam)·forward > 0`, within
  `SCAN_RANGE`, and inside a half-angle cone. Rate-limited (a few per second, a `scan_timer`)
  so it *reads* as the ship sweeping, not an instant flood.
- **Scan beam visual** — a short cool-coloured `beam::Beam`-like flick from the cruiser nose
  toward a scanned site (or a sweep across the forward zone), fed to `overlay` (cool vs G2's
  warm). Keep it cheap + brief.
- **Map markers** — `map`/`map.wgsl` gains a marker variant (or reuses the text marker with a
  different tint/shape) for **scanned-uncollected**; collected sites use the existing/》done tint.
- **Save** — `progress::encode/decode` bumps to carry the scanned set (varint id list; the
  collected set is already there). Round-trip + determinism tested as in G1.

## Decisions to resolve (with recommended defaults)

1. **What v1 scan detects.** *Default:* **inscriptions + colossi as site pins** (the things
   G1/G2 collect). Terrain/biome the map already shows; ethereal-only detection + buried/deep
   finds are **Sensing tech (G4)**.
2. **Scan beam form.** *Default:* a **brief cool flick** toward discovered sites in the forward
   zone (reads as "the ship noticed that"), not a continuous sweep — cheaper + less busy.
3. **Forward zone.** *Default:* a **cone ahead of the camera** (range + half-angle consts),
   matching §6's "zone just ahead, on-screen"; tuned on play.
4. **Map marker design.** *Default:* a distinct **hollow pin** for scanned-uncollected vs the
   filled/》marker for collected — must stay legible inside the dark palette (tone guardrail).
5. **Payload.** *Default:* extend the `pg=` blob append-only with the scanned-id set (bump its
   version), tolerant of older payloads (no scanned set → empty).

## Tests

- **Forward-zone predicate**: pure fn (in-cone within range vs behind / too far / off-angle).
- **Scan marks known, not collected**: scanning a site adds it to `scanned`, leaves `seen`
  (strata/codex) untouched; collecting later flips it in the opportunity set.
- **Opportunity set** = `scanned − collected`, recomputed correctly as both grow.
- **Payload round-trip + determinism**: scanned set survives encode→decode; same drift +
  scan sequence → identical scanned set (extends G1's tests).
- **Map**: a scanned-uncollected chunk reports the new marker; flips on collect. (Logic-level;
  the visual is eyeballed headless.)
- CI green on all targets; the `bm-render` overlay stays generic (no game dep — boundary check).

## Risks & mitigations

- **Two beams read the same through the palette.** *Mitigation:* pick a clearly **cool** hue
  (vs G2's warm) + a different cadence (brief flick vs held beam); verify headless over the
  palette (an A/B like G2's).
- **Scan flood / busy screen.** *Mitigation:* rate-limit (a few/sec) + brief lifespans; the
  forward cone bounds it to what you'd see.
- **Map clutter.** *Mitigation:* one new marker only; collected sites de-emphasised; lean on
  the existing legend.
- **Scope creep into the tree.** *Mitigation:* fixed basic scan range/rate in G3; **all**
  upgrades (range, deep, spectral, stratum hints, auto-collect, auto-route) are **G4**.

## Acceptance checklist

- [x] Cruiser **auto-scans** the forward cone while piloting (cool flicks via the G2 overlay),
      rate-limited (`scan::INTERVAL`) so it reads as a sweep.
- [x] Discovered inscriptions/colossi become **scanned/known** (stable `find_id`) without
      collecting; the forward-cone predicate (`scan::in_cone`) is pure + tested.
- [x] The **map** shows scanned-**uncollected** sites as a distinct **amber ring** (alpha-code
      200 + a `map.wgsl` icon); collecting flips it off (`forget_scanned_chunk`); legend + the
      HUD "known N · found M" updated.
- [x] Scanned set **saves/restores** in the `progress` payload (v2, append-only over v1;
      round-trip + determinism unit-tested).
- [x] CI green (fmt / clippy -D / tests / wasm); the `bm-render` overlay stays game-agnostic
      (the scan reuses G2's generic feed — boundary check green); golden voxel-hash unchanged.
- [x] `game-mechanics.md` §13 ticked for G3; this checklist complete.

> **Decisions:** defaults taken — v1 scan detects **inscriptions + colossi** (no deep/spectral
> tiers; those are G4 Sensing); a **brief cool flick** toward each freshly-scanned site (≤3 per
> pulse); a forward **cone** (`COS_HALF_ANGLE` ~53°, `RANGE` 150); a distinct **hollow amber
> ring** map marker; payload bumped to **v2** (append-only, tolerates v1). The map's
> opportunity/collected distinction is **per-chunk** (a v1 approximation — refine if chunks
> routinely hold several sites). The scan beam reuses G2's overlay (cool vs warm) — no new
> render path; the G2 A/B already verified the overlay reads vivid + occluded over the palette.

## Out of scope / follow-ups

- The **tech tree + in-engine tree/codex UI** (on the E17 text path) + **Sensing/Memory**
  upgrades + **auto-collect** + **auto-route** → **G4** (the next brief).
- Foot-collision interiors (caves / solid colossi as collectible) → with the Locomotion tech,
  later.
