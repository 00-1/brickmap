# G5 — Composition editor, `match` filter & nav vocabulary

> **Status: ✅ landed (2026-06-07).** The G4 console gains **picker/stepper editing** (↑↓ select a
> routine, ←→ change its parameter, Enter toggle — no typing): cycle the `drift` routine's nav
> block **drift → seek → circle**, and cycle the `survey` routine's collect **filter** none ↔
> `match(rare)`. `seek` steers the autopilot to the nearest known-uncollected site; `match(rare)`
> makes the auto-collect selective (Relics/Signals only). Routine edits persist in a `co=` share
> segment. As-built note below; golden voxel-hash + headless render unchanged.
>
> **As-built vs the original plan:** the v1 editor is **parameter steppers on the given routines**
> (cycle the nav block; toggle the `match` filter) rather than free-form insert/remove of an
> arbitrary step list — chunky, forgiving, controller-first, and enough to prove selective
> auto-collect + routing. Free-form multi-step composition grows with G6's richer vocabulary.
> The useful v1 `match` field is **`rare`** (the collectible set is already uncollected, so a
> `match(uncollected)` would be a no-op); the generic `match` infra + picker are in place for more
> fields in G6.

> _(Original brief follows.)_

> **Was: ready to build, after ✅ G4.** Turns the read-only console (G4) into an **editor**:
> the player authors routines by inserting/removing blocks (no typing), adds the generic
> **`match(field)`** filter, and gets nav **`seek`/`circle`** — the management game (selective
> auto-collect + routing you build). game-system §2 (L2–L3), §11 Tier 1. In `scraped-again`.
>
> **Assumptions / decisions taken solo:** (1) Editing is **cursor + insert-from-palette +
> delete** on the existing routine shape (`continuous` + `on_scan` step lists) — not free-form
> node graphs; chunky + forgiving. (2) `match` is **one generic block with a pickable field**;
> v1 fields = `uncollected` + `script(any of the 5)` (both already comprehensible from G1/G3) —
> rarity/range fields wait for G6's detection. (3) Routine **persistence**: routine edits ride
> the `pg=` payload (v3, append-only); given routines are the default if absent. (4) `seek` =
> head to the nearest *known-uncollected* site (reuses G3 scanned set); `circle` loiters around
> the current area — both drive the existing auto-fly executor (high-level nav only).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Author + edit routines from the console; a working `match` filter; nav blocks that
  steer the autopilot toward opportunities.
- **Demonstrable outcome.** In the console: open a routine, **insert `match(uncollected)`** into
  `scan → on-scan → … → collect`, and watch auto-collect become selective; swap the `drift` nav
  routine to **`seek`** and the ship heads for the nearest uncollected pin instead of wandering.
  Routine edits persist across reload.
- **De-risks.** The **composition UX** on the text path (insert/remove/reorder, value pickers)
  before G6 piles control/meta blocks on; that the runtime generalises from given→authored.

## Scope

**In:** an **edit mode** in the console (select routine → add block from a palette / remove /
toggle); the generic **`match(field)`** condition (fields `uncollected`, `script`); **parameter
pickers** (the `match` field; `scan`'s item — still just `shards` for now, but the picker exists);
nav **`seek(uncollected)`** + **`circle`** blocks driving the auto-fly; routine persistence in
the save. **Out:** `when`/`repeat`/`budget`/`priority`/meta + `decode` + the unlock economy +
legibility (G6); `scanMany`, more `scan` items, rarity/range match fields (G6); two-agent/hail (G7).

## Design sketch

- Extend `console`: `Routine.steps` already a `Vec<Block>`; add `Block::Match(Field)` and nav
  `Block::Seek`/`Block::Circle`. An `EditMode` cursor state: pick a routine, a palette of
  insertable blocks, insert at cursor / delete / move. A `Field` enum with a picker (cycle).
- Runtime: the `on_scan` step list is run as a **pipeline** — `match(field)` gates whether the
  subsequent `collect` runs for each just-scanned site (so `scan → on-scan → match(uncollected)
  → collect` only collects uncollected ones). `seek`/`circle` set an autopilot *intent* the
  existing wander executor follows (head toward a target vs loiter).
- Persistence: serialise the (small) routine set into the `pg=` blob (v3); decode tolerant of
  v1/v2 (→ given routines).

## Tests

- Editor model: insert/remove/move within a routine; the palette; field-picker cycling (pure).
- `match` predicate: `uncollected`/`script` filter a candidate set correctly.
- Pipeline: `scan → on-scan → match(uncollected) → collect` collects only uncollected (logic).
- `seek` target selection: nearest known-uncollected (pure); empty → falls back to drift.
- Routine round-trip in the payload (v3, tolerant of v1/v2). CI green; boundary intact; golden
  voxel-hash + headless render unchanged.

## Acceptance checklist

- [x] Console **edit** (picker/stepper): ↑↓ select a routine, ←→ change its parameter, Enter
      toggle — cursor-driven, no typing. (Free-form insert/remove → G6; see As-built.)
- [x] Generic **`match`** filter gating the on-scan→collect pipeline (v1 field `rare`); the
      picker cycles it; predicate + cycle tested.
- [x] Nav **`seek(uncollected)`** + **`circle`** steer the autopilot (high-level heading);
      `seek` target selection (nearest known-uncollected) wired + tested.
- [x] Routine edits **persist** in a `co=` share segment (lenient; absent → the givens);
      round-trip tested.
- [x] CI green (fmt / clippy -D / 140 tests / wasm); crate boundary intact; golden voxel-hash +
      headless render unchanged.
- [x] Docs in lockstep: game-mechanics §13 G5 + game-system §11 Tier 1 ticked.
