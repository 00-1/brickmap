# G1 — Console & the economy spine (minimal block substrate)

> **Status: ready to build.** First gameplay slice for **Scraped Again**. Establishes the
> **block substrate** ([`../game-system.md`](../game-system.md)) *minimally* — because it's
> foundational, not a late feature — together with the data economy it carries. Built on the
> post-M9 workspace, in the **`scraped-again`** crate. Design: [`../game-mechanics.md`](../game-mechanics.md).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Stand up a **minimal block runtime + console UI** (blocks visible, **clickable
  to trigger**, one given pre-wired routine running), and the **economy spine**: collecting
  glyphs yields five typed **strata**, finds record in a **codex**, progress persists.
- **Demonstrable outcome.** The console shows your blocks; click `scan` / `collect` and it
  happens; a **given `on-scan → collect` routine** runs on its own and you can toggle/open
  it; the default `drift` autopilot is itself a (given) nav routine. Collecting ticks a
  **stratum** on the HUD and lands a **codex** entry. Reload from seed + progress → restored.
- **De-risks.** The whole game's spine: that **"everything is a clickable/wireable block"
  is viable and feels good**, and the *progress = seed + sparse log* save model. Proves the
  core collecting sensation *inside* the substrate, not bolted on later.

## Scope

**In:**
- **Minimal block runtime.** A block = a visible, named operation that can be (a) **clicked
  to trigger** once, and (b) **run by a routine**. A *routine* = a small fixed wiring
  (trigger → action[s]). G1 ships the runtime + **given routines only** — *no player editor
  yet* (that's G3).
- **Starter blocks** (game-system §11 Tier 0): `scan(item)` (ship — **starts as
  `scan(shards)`**), `collect` (shared), `spend` (shared), `goto(area)` (ship), nav `drift`
  (ship), trigger `on-scan`. **Shards** are the basic starter item / upgrade currency.
  Blocks take a *single* typed argument whose options are unlockable (G1 ships `shards`
  only; more items + `scanMany` come later).
- **Given pre-wired routines, shown as their blocks:** `scan(shards) → on-scan → collect`
  (the onboarding artifact — the ship opens auto-scanning shards) and `drift` (the default
  autopilot, now a nav routine — replaces the hardcoded auto-fly).
- **Console UI** — render the blocks/routines on the **E17 world-text / HUD text path**
  (terminal-styled, low-fi, controller/click friendly; no typing). Click a block to trigger;
  toggle a routine on/off.
- **The economy:** five **strata** + script→stratum mapping (game-mechanics §5); `collect`
  yields a stratum (pure fn of script + length); a **codex** (de-duplicated find set +
  metadata). Strata on the HUD; a codex list view.
- **Progress** = the `share` payload extended with `{ collected-set, strata }`, serialized +
  restored on load. Route `collect`/`spend` through the **`edit::Edit`/`apply` event seam**
  (serializable; N1 groundwork).

**Out:** the **composition editor** / player-authored routines (G3); conditions/filters,
`when`, budgets, control-meta (G3–G4); the **survey-beam** (G2); **more scan item-types +
`scanMany`** (G1 scans only `shards`); `scan`→map population + map-as-picker (G3);
auto-collect *selectivity* (G3 — G1's given routine grabs all shards in-corridor); the
tech-tree *unlock economy* (G4); decode (G4); codex RTT thumbnails; interiors / walking
branch (G5).

## Design sketch

- **Lives in `scraped-again`** over engine primitives: E17 `text` (inscriptions + the
  terminal text rendering), E14 **DDA pick** (aim for a manual `collect`), `share` (payload),
  `hud`, the `edit` seam, and the existing auto-fly (now driven by the `drift` block).
- **Block/runtime model** — `enum Block { Scan, Collect, Spend, Goto(Area), Drift, OnScan }`
  (or a small data-driven table); a `Routine` = `{ trigger, steps }`; a `Runtime` ticks
  active routines and dispatches block effects through the same paths a manual click uses.
  Keep it tiny and data-driven so G3+ grows the table, not the architecture.
- **`Strata`** — `{ records, schematics, rites, relics, signals: u64 }` + a pure
  `yield(script, len)`.
- **`Codex`** — de-duped set keyed by a stable find id (hash of seed + cell + glyph index);
  entry `{ script, text, world_pos, stratum, first_seen }`.
- **Autopilot reframe** — the cinematic auto-fly becomes the *execution* of the given `drift`
  nav routine; `goto(area)` lets you point it at a map area (map-as-picker is fuller in G3,
  but a basic "fly here" works now).

## Decisions to resolve (with recommended defaults)

1. **How much runtime in G1?** *Default:* **trigger-by-click + run given linear routines
   only**; the editor + conditions/flow are G3. Keeps G1 bounded but real.
2. **`decode`** *Default:* **out of G1** (raw strata accrue; decode/comprehension-unlocks land
   in G4). G1's strata are just counts.
3. **Manual `collect` trigger** *Default:* DDA pick + a click/button *is* "clicking the
   `collect` block aimed at a find."
4. **Console rendering** *Default:* reuse the **E17 text path** (terminal aesthetic, on-brand,
   cross-platform, no-typing) rather than DOM — even if minimal in G1.
5. **Payload versioning** *Default:* bump the `share` version; progress block append-only
   (E12 policy).

## Tests

- `Strata::yield` pure fn (all five scripts, length edges); codex **de-dup** (re-collect =
  no-op).
- **Runtime**: a given routine fires its trigger → runs its steps; clicking a block triggers
  the same effect path; toggling a routine stops/starts it.
- Progress **round-trip** (serialize→deserialize restores strata + collected-set); unknown
  future fields tolerated; **determinism** (same seed + same actions → same strata/codex).
- Native + wasm build green; console + HUD strata visible headless.

## Risks & mitigations

- **Console UX on the text path** (legibility, click/controller targeting). *Mitigation:*
  keep G1's console deliberately minimal (a list of blocks + the given routines); the rich
  editor is G3 where it gets real design.
- **Over-building the runtime.** *Mitigation:* Decision 1 — given linear routines only;
  data-driven block table so G3+ extends data, not architecture.
- **Payload growth.** *Mitigation:* compact id set (varint/RLE, like E14 deltas).
- **Find-id stability across worldgen versions.** *Mitigation:* derive from seed + stable
  cell/index; freeze per worldgen version (E12).

## Acceptance checklist

- [ ] Minimal block runtime: blocks visible + **clickable to trigger**; the given
      `on-scan → collect` and `drift` routines run and can be toggled/opened (shown as blocks).
- [ ] Starter blocks `scan(item)` (= `shards`)/`collect`/`spend`/`goto`/`drift`/`on-scan`
      implemented; the ship opens auto-scanning shards; the default autopilot is the `drift`
      routine (no separate hardcoded auto-fly).
- [ ] Five strata + script→stratum yield (tested pure fn); manual `collect` (aimed click) →
      strata++ on the HUD + a codex entry; de-dup works.
- [ ] `collect`/`spend` routed through the serializable `Event`/`apply` seam.
- [ ] Progress (strata + collected-set) saves into the `share` payload and restores on load
      (round-trip + determinism tested).
- [ ] Console + codex list render on the E17 text path (no typing; click/controller-driven).
- [ ] CI green (fmt / clippy -D / tests / wasm); existing systems unchanged.
- [ ] Docs in lockstep: game-mechanics §13 G1 ticked; game-system §11 Tier 0 ticked.
