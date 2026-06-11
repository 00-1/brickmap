# G9 — Names in the world (block discovery)

> **Status: ready to build — FIRST DIRECTIVE of the 2026-06-11 run.** A human-decided
> direction: **the in-world text becomes the block names** — "you need to find the names to
> start unlocking the code block." Today inscriptions are ambient glyph-noise and decoding a
> stratum unlocks *all* its blocks at once. After G9, a share of inscriptions **carry a block's
> name**; **collecting one *discovers* that block** in the console (visible, named, still
> stratum-locked); comprehension (the existing decode) then unlocks it. Vocabulary growth
> becomes **exploration-driven** — the world's text is load-bearing, not decorative. In
> `scraped-again`; no engine changes expected.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Make finding a block's **name in the world** the first stage of acquiring the
  block: *found* (the name collected → the block listed, locked) → *comprehended* (its stratum
  decoded → insertable). The two-stage gate replaces "decode unlocks everything in the stratum
  at once."
- **Demonstrable outcome.** Fly to a glowing inscription rendered in Runic; collect it (`T`,
  the beam, or an auto-collect routine); the HUD announces **NAME RECOVERED — `priority`**;
  the console now lists `priority` (previously absent), dimmed with its stratum tag; decode
  Relics and it becomes insertable. The opening is unchanged (starter set pre-discovered,
  given routines intact).
- **De-risks.** The exploration↔vocabulary loop — that wandering a dead world *reading its
  text* is how your console grows. This is the core of "deepen the mechanics" and everything
  later (typed shards, per-block economies) sits on it.

## Scope

**In:**
- **Name-bearing inscriptions.** A deterministic minority of `structures::inscriptions_near`
  cells carry a **block name** instead of ambient noise: the text is the block's name rendered
  in **its stratum's script** (`stratum_of` already maps script↔stratum — use the inverse), as
  a **stable per-block transliteration** (same glyphs every time for the same block — it must
  read as *a name*, recognisably recurring, not fresh noise per cell).
- **Colossus monument labels name the deep blocks.** `colossus_label` draws from the Tier-2/3
  vocabulary (Relics/Signals strata) — the fallen giants are *named after* the deepest
  operations. Evocative and mechanically right: rare names live at rare landmarks.
- **Discovery model.** A new append-only progress `Event` (e.g. `Discover(Block)`) applied
  through the existing `progress` event path; round-trips in the `pg=` payload (next version,
  append-only; absent → starter-only discovered).
- **Console gating becomes two-stage.** `is_unlocked(b)` = *discovered* AND *stratum decoded*.
  Undiscovered blocks are **absent** from the vocabulary/palette listing; discovered-but-locked
  blocks are **listed dimmed with their stratum tag** (the "I've seen this name" tease);
  discovered+decoded = insertable. **Tier-0 starter set is pre-discovered** so the opening and
  the given routines are untouched.
- **All collect routes discover**: manual `T`, beam collect-along-path, and routine-driven
  auto-collect. A collect that discovers shows a distinct HUD line; re-collecting an
  already-discovered name yields normal stratum data (no dupe event — still worth collecting).
- **Coverage guarantee.** The seeded scatter must surface **every** block's name: weight
  name-cells by stratum rarity but test that the full vocabulary appears within a bounded cell
  radius of any start (no discovery softlock).

**Out:** the typed-shard economy (**G10**, brief to follow); gating name *readability* on
script comprehension (a live design fork — logged for the human; v1 names are readable on
collect); changes to strata yields/decode costs; any new engine capability.

## Design sketch

- `structures::compose` grows a name-bearing arm: cell hash → (is-name? → pick a `Block` by
  stratum-weighted hash) → text = `transliterate(block.label(), script_of(stratum))`, where
  `transliterate` maps the label's letters deterministically into the script's glyph pool
  (stable, collision-checked across the vocabulary). `Inscription` carries an optional
  `names: Option<Block>` so the collect path knows what it found.
- `progress`: `Event::Discover(Block)` + a `discovered: HashSet<Block>` in the derived state;
  `pg=` bumps a version, append-only.
- `console`: `is_unlocked` consults discovery; `vocabulary()` filters absent-undiscovered,
  renders discovered-locked dimmed + tagged. Starter set seeded as discovered at init.
- The collect paths (`T`/beam/auto) already resolve an `Inscription`; on a name-bearer, emit
  `Discover` before the normal yield.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Two-stage gate** (found → listed-locked; decode → insertable), starter pre-discovered.
2. **Name fraction:** ~1 in 4 inscriptions name-bearing; the rest stay ambient. Colossus
   labels always name-bearing (deep vocabulary).
3. **Names are readable at collect time** in v1 (the script-comprehension-first variant is the
   human's open fork — don't build it).
4. **Stable transliteration** per block (deterministic, distinct across the vocabulary —
   test for collisions).

## Tests

- Transliteration: deterministic, distinct for every `Block` in the vocabulary (no collisions).
- Discovery: `Event::Discover` applies + round-trips `pg=` (and old payloads still load).
- Console visibility states: absent / dimmed-locked / insertable; starter pre-discovered;
  locked still uninsertable in the editor.
- Coverage: every block's name occurs within N cells of origin across seeds (pick N to bound
  the walk; statistical, deterministic per seed).
- Golden voxel-hash unchanged (terrain untouched). The headless inscription render changes
  *content* (names instead of noise) — if a golden image covers inscriptions, update it
  intentionally, once, noted in the commit.

## Risks & mitigations

- **Discovery softlock** (a needed name unreachable) → the coverage test + starter
  pre-discovery; rarity-weighting tuned so Tier-1 names are common.
- **Transliteration mush** (names not recognisably distinct) → collision test + favour
  length/letter-structure preservation over aesthetics.
- **Ambient feel lost** (every inscription a pickup) → the ~1-in-4 fraction; ambient majority
  keeps the melancholy noise.

## Acceptance checklist

- [ ] A deterministic minority of inscriptions carry stable per-block names in the block's
      stratum script; colossus labels name deep (Relics/Signals-tier) blocks.
- [ ] Collecting a name-bearer (T / beam / auto-collect) emits `Discover(block)`; HUD announces
      it; dupes yield normally without re-discovering.
- [ ] Console: undiscovered absent · discovered-locked dimmed+tagged · discovered+decoded
      insertable; Tier-0 pre-discovered; given routines/opening unchanged.
- [ ] Discoveries persist (`pg=` next version, append-only; old payloads load).
- [ ] Coverage test (full vocabulary findable), transliteration collision test, console-state
      tests; golden voxel-hash unchanged; CI green (fmt / clippy -D / tests / wasm); boundary
      intact (no engine change).
- [ ] Roadmap G9 entry + this checklist ticked on `main`.
