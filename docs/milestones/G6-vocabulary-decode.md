# G6 — Decode, the unlock economy & decipherment legibility (the "tree")

> **Status: ✅ landed (2026-06-07).** The progression-as-comprehension spine: a `decode` block
> spends a stratum's data to **comprehend** it, which (a) **unlocks blocks** — the console
> palette/nav/filter are gated by comprehension, so the "tree" *is* the growing vocabulary — and
> (b) makes that **script legible**: its inscriptions render **translated** via a seeded elegiac
> grammar (`lexicon`). Comprehension persists (`pg=` v3). The `when`/`repeat` control blocks are
> **folded into G7** (they need the richer free-form runtime/editor that lands there). Golden
> voxel-hash unchanged. As-built note below.
>
> **As-built / decisions taken solo:** `decode` is **one-click**, auto-targeting the richest
> affordable un-comprehended stratum (no picker — keeps it controller-first); the method is
> `progress::comprehend` (`decode` is the payload codec). Legibility changes only the **display**
> — find ids still hash the original glyphs, so collecting stays stable. The vocabulary gate is a
> small per-block `required(stratum)` table (Schematics → seek/circle/goto; Rites → match);
> `when`/`repeat`/`budget`/meta move to G7 with the general routine runtime.

> _(Original brief follows.)_

> **Was: ready to build, after ✅ G1–G5.** The progression spine reframed as the **growing
> block vocabulary** (game-system §4): spend collected strata via **`decode`** to *comprehend*
> the dead machine — which **unlocks blocks** and turns a **script legible** (its inscriptions
> render **translated**, game-mechanics §9). Plus the first **control blocks** (`when`/`repeat`).
> In `scraped-again`. The "tech tree" is this — no spend-on-nodes menu.
>
> **Assumptions / decisions taken solo:** (1) `decode(stratum)` is the `spend` action made real
> — spend a fixed cost of that stratum's data to **comprehend** it; the strata→block gating is a
> small data table. (2) **Legibility** swaps a comprehended script's glyphs for **words from a
> seeded grammar** (procedural-poetic, §6) — deterministic in seed + cell, no authored lore. (3)
> Comprehension is stored in `progress` (the `pg=` payload, v3, append-only). (4) Control blocks
> `when(state)` / `repeat` land as routine-level wiring the runtime honours; the full
> `budget`/`priority`/meta set is illustrative and grows after. (5) Unlocking gates the
> *vocabulary* (which blocks the console offers) — recovered, not bought from a menu.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Make collected data *mean comprehension*: `decode` a stratum → the matching **script
  renders translated** and **new blocks unlock**; a couple of **control blocks** deepen routines.
- **Demonstrable outcome.** Collect Records; open the console; **`decode(records)`** (spends
  Records); Latin inscriptions in the world now read as **words** instead of glyphs, and a new
  block (e.g. `scanMany`/`when`) appears in the palette. Comprehension persists across reload.
- **De-risks.** That progression-as-vocabulary + the seeded-grammar legibility *feel* like
  comprehension (the melancholy payoff) — the design's heart — and that control blocks compose.

## Scope

**In:** `decode(stratum)` (the real `spend`): spend a cost of a stratum → mark it **comprehended**
in `progress`; a small **strata → unlocked-blocks** table (comprehension grows the console
palette); **decipherment legibility** — a comprehended script's inscriptions render as seeded
**words** (a `lexicon` grammar over the existing E17 text path); the control blocks **`when(state)`**
(a threshold trigger, e.g. `shards ≥ N`) and **`repeat`** honoured by the runtime; comprehension
in the save. **Out:** the full `budget`/`priority`/meta vocabulary + cross-agent (G7); two-agent/
hail (G7); authored lore over the grammar (later); rarity/range match-fields beyond G5's.

## Design sketch

- `progress`: add a `comprehended: set<Stratum>` + `decode(stratum)` (checks/spends cost, marks
  it); serialise in `pg=` v3 (tolerant of v1/v2). `is_legible(script)` = its stratum comprehended.
- `lexicon` module (pure, tested): `word(seed, cell, glyph_index) -> &str` over a small elegiac
  word-bank + a tiny grammar; `phrase(...)` composes 1–2 words. Deterministic.
- `text`/inscription build: when `progress.is_legible(script)`, replace the glyph string with the
  lexicon phrase (Latin font), else today's glyphs. (Engine `text` stays generic; the *strings*
  are the game's — boundary intact.)
- console: `decode`/`spend` dispatch → `progress.decode`; the palette is filtered by
  comprehension (locked blocks hidden/greyed until their stratum is decoded); `when`/`repeat`
  added to the routine model + runtime tick.

## Tests

- `decode`: spends exactly, marks comprehended, idempotent; gating table maps strata→blocks.
- `lexicon`: deterministic word/phrase; distinct per cell; stable across runs.
- legibility: a comprehended script yields words, an un-comprehended one yields glyphs (logic).
- `when` predicate + `repeat` honoured (runtime logic). Comprehension round-trips (`pg=` v3).
- CI green; boundary intact; golden voxel-hash unchanged; a legibility screenshot headless.

## Acceptance checklist

- [x] `decode` spends data → **comprehension** (`progress::comprehend`, persisted `pg=` v3);
      cost + idempotence + round-trip tested.
- [x] Comprehension **unlocks blocks** (a per-block `required(stratum)` gate grows the console
      vocabulary — palette shows locks, nav/filter cycling + dispatch gated); gating tested.
- [x] **Decipherment legibility:** a comprehended script's inscriptions render **translated**
      (seeded `lexicon` grammar); legibility + determinism tested.
- [~] Control blocks **`when`** + **`repeat`** — **deferred to G7** (need the general routine
      runtime/free-form editor that lands there; recorded as a solo decision).
- [x] CI green (fmt / clippy -D / 144 tests / wasm); boundary intact; golden voxel-hash unchanged.
- [x] Docs in lockstep: game-mechanics §13 G6 + game-system §4/§11 ticked.
