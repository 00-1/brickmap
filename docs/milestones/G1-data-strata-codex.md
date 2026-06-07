# G1 — Data, strata & the codex (the economy spine)

> **Status: ready to build.** First gameplay slice for **Scraped Again** (the design:
> [`../game-mechanics.md`](../game-mechanics.md)). Built on the post-M9 workspace, in the
> **`scraped-again`** crate, over engine primitives — no engine changes needed here.
> Follow the milestone template + quality gates in [`../development.md`](../development.md)
> and [`../roadmap.md`](../roadmap.md).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Make glyphs *collectible* into **five typed data strata**, record each find in
  a **codex**, and persist progress as a serializable log — the resource spine every later
  slice (tree, scan, beam) plugs into.
- **Demonstrable outcome.** Aim at an in-world inscription and collect it: its **stratum
  counter ticks up on the HUD**, and the find lands in a **codex list**. Reload the world
  from the same seed + progress and the strata/codex are restored.
- **De-risks.** The typed-currency economy and the *progress = seed + sparse log* save
  model (the thing G4's tree and multiplayer's shared archive depend on). Proves the core
  collecting *sensation* with the cheapest possible trigger.

## Scope

**In:**
- The **five strata** and the **script → stratum** mapping (game-mechanics §5): Latin→
  Records, Greek→Schematics, Hiragana→Rites, Runic→Relics, Galactic→Signals.
- A **`collect(glyph)`** path that yields the glyph's stratum (amount = a pure fn of script
  + word length) and appends to the codex.
- **HUD readout** of the five strata counts (reuse the `hud` text overlay).
- A **codex** model (set of finds + metadata: script, text, where, when) and a minimal
  **codex list screen** (text path; no thumbnails yet).
- **Progress state** = the existing `share` seed payload extended with `{ collected-set,
  strata }`, serialized + restored on load. Route the collect through the **`edit::Edit`/
  `apply`-style event seam** so it stays the serializable mutation path (multiplayer/undo
  groundwork, roadmap N1).
- A **manual collect trigger**: aim (the E14 DDA pick) at a nearby inscription and press a
  key/button to collect it.

**Out (later slices):** the survey-beam (G2); cruiser auto-scan + map opportunity surface
(G3); the tech tree + in-engine tree UI + auto-collect (G4); translation/decipherment
rendering; codex RTT thumbnails; pristine/interior gating.

## Design sketch

- **Lives in `scraped-again`.** Reuses: `text`/E17 (inscriptions already exist in-world
  with a `Script`), `structures::inscriptions_near` (placement), the E14 **DDA pick**
  (aim → hit inscription), `share` (payload), `hud` (readout), the `edit` seam (mutation).
- **`Strata`** — `struct Strata { records, schematics, rites, relics, signals: u64 }` with
  a pure `add(script, len)`/yield fn (unit-tested).
- **`Codex`** — a de-duplicated set keyed by a stable find id (e.g. hash of seed + cell +
  glyph index), each entry `{ script, text, world_pos, stratum, first_seen }`.
- **Collect event** — `Event::Collect { find_id, script, len }` → `apply` mutates
  `Strata` + `Codex`; serializable; deterministic.
- **Which inscriptions are collectible** — the ones `inscriptions_near` already streams;
  collecting marks the find id so it isn't re-collected (and so the map can later show it
  done, E10).
- **HUD/codex UI** — start with the existing bitmap-font overlay; the *fancy* in-engine
  menu (on the E17 world-text path) is G4's job. Keep G1's UI utilitarian.

## Decisions to resolve (with recommended defaults)

1. **Collect trigger.** *Default:* **manual DDA pick + a key/pad button** — simplest and
   deterministic; proximity/auto-collection arrive with scan (G3) and the tree (G4).
2. **Yield formula.** *Default:* `base[script] * f(word_len)`, small integers, tuned later
   — just needs to be a pure, tested fn now.
3. **Codex thumbnails.** *Default:* **defer** — text entry only in G1; RTT thumbnails are a
   later polish (heavier; reuse the headless RTT path then).
4. **Payload versioning.** *Default:* bump the `share` payload version and treat the
   progress block as append-only (matches the E12 worldgen-versioning policy).

## Tests

- `Strata::add` / yield: pure fn, exhaustive over the five scripts + length edge cases.
- Codex **de-dup**: collecting the same find twice is a no-op on strata + codex.
- Progress **round-trip**: serialize → deserialize restores strata + collected-set
  identically; unknown future fields tolerated.
- **Determinism**: same seed + same collect sequence → identical strata/codex.
- Native + wasm build green; HUD readout visible headless.

## Risks & mitigations

- **Payload growth** (collected-set could grow large). *Mitigation:* store collected as a
  compact id set (varint/RLE like the E14 edit deltas); it's sparse vs the world.
- **Find-id stability across worldgen versions.** *Mitigation:* derive ids from seed + a
  stable cell/index scheme; freeze per worldgen version (E12 policy).
- **Scope creep into the tree/UI.** *Mitigation:* G1's UI is deliberately utilitarian;
  the real menu is G4.

## Acceptance checklist

- [x] Five strata + script→stratum mapping implemented; yield is a tested pure fn
      (`progress::{Stratum, Strata, stratum_of, yield_amount}`).
- [x] Manual collect (aim ray + **`T`**) → strata++ on the HUD + a codex entry; de-dup works
      (`App::collect_aimed` — nearest in-reach collectible to the view ray).
- [x] Collect routed through the serializable `progress::Event::Collect` / `Progress::apply` seam.
- [x] Progress (strata + codex) saves into the share payload (its own `pg=` key, ignored by
      `ShareState`) and restores on load (`initial_progress`); round-trip + determinism unit-tested.
- [x] Codex list screen shows finds (text only) — toggle **`J`** (`App::codex_text`).
- [x] CI green (fmt / clippy -D / tests / wasm); existing behaviour unchanged (golden voxel-hash
      + headless render untouched — G1 is pure game logic + HUD text).
- [x] `game-mechanics.md` §13 build plan ticked for G1; this checklist complete.

> **Decisions:** all four took their recommended defaults (manual DDA pick + key; yield
> `base[script]·(1+glyphs)`, glyph count clamped at 32; thumbnails deferred; append-only
> payload — a versioned binary `pg=` blob that also carries the full codex entries, so a reload
> restores the *list*, not just the id-set). Collect/codex are keyboard **`T`/`J`** (work on web
> via the shared key path; no engine or gamepad change, per scope).
