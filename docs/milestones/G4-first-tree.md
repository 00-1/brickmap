# G4 — The first tech tree (+ the in-engine menu) & auto-collect

> **Status: ready to build, after [G1](G1-data-strata-codex.md) · [G2](G2-survey-beam.md) ·
> [G3](G3-cruiser-scan-map.md).** The **economy payoff** of *Scraped Again*
> ([`../game-mechanics.md`](../game-mechanics.md) §8, §9, §12): spend the five strata on a small
> **tech tree of comprehension**, shown through the first **in-engine menu** (on the E17 text
> path — no DOM UI), with **Decipherment** turning a script legible and **auto-collect** closing
> the hands-off loop. Mostly `scraped-again`; the only plausible engine touch is a small
> text/quad UI-draw helper in `bm-render` *if* the E17 path doesn't already suffice (keep it
> generic). This is the game's **one substantial new system** (§12) — so v1 is deliberately
> small and tuned, with the *shape* designed to expand.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Make the collected strata **mean something**: a small, tuned tree you spend them on,
  a quiet **in-engine tree/codex screen** to spend at, and the first **Decipherment** payoff (a
  script becomes legible and renders translated). Plus **auto-collect** so an autopilot-only run
  climbs the tree hands-off (the load-bearing "autopilot is a complete way to play" claim).
- **Demonstrable outcome.** Open the tree (a key); see the five strata and a handful of nodes
  across **Sensing / Memory / Decipherment**; spend strata to unlock one (e.g. *Scan Range II*,
  *Auto-Collect I*, *Legibility: Records*); the effect is **immediately visible** — a wider scan,
  the cruiser auto-firing the collection beam in its corridor, or **Latin inscriptions rendering
  as words** instead of glowing nonsense.
- **De-risks.** The whole economy's *feel* (costs/curve) and the **in-engine menu** approach
  (the no-DOM-UI, cross-platform-parity promise) — the riskiest, biggest build in the game.

## Scope

**In:**
- **A small tree** (`tree` module): ~3 branches × ~3 nodes, **data-driven** (a node table:
  id, branch, cost in a stratum, prereqs, effect). v1 branches: **Sensing** (scan range/rate),
  **Memory** (storage cap, **Auto-Collect I**), **Decipherment** (**Legibility: Records** +
  one more script). Nodes are illustrative; the *table is the system*, expandable in G6+.
- **Spend/unlock** via the serializable seam: an `Event::Unlock { node }` through
  `progress::apply` (mirrors G1's `Collect`), checked against strata + prereqs; unlocked set
  saved in the `pg=` payload (v3, append-only).
- **Node effects** wired to existing systems: Sensing → `scan::RANGE`/`INTERVAL`; Memory
  Auto-Collect → the cruiser auto-fires the **warm** collection beam (G2) into the forward zone
  while piloting (the visible idle harvest); Decipherment Legibility → flip a script to
  **translated rendering** (see below).
- **The in-engine menu**: a quiet, palettised tree/codex screen on the **E17 world-text path**
  (glowing, dithered, in-aesthetic — *not* a flashing idle dashboard, per the tone guardrail).
  Toggle key; navigate nodes (keys/pad); shows cost, locked/affordable/unlocked; spend.
  Reuse/extend the G1 codex screen as the "archive" tab.
- **Decipherment rendering**: legibility swaps a script's glyphs → **meaning** (the E17 path
  already *chooses* glyph strings; legibility maps glyph→word via a **small seeded word-bank /
  grammar**, §6 "procedural-poetic, real grammar"). v1: one or two scripts become legible.

**Out (later):** the deep "to-the-extreme" arcs (Concordance/Synthesis), Resonance/pristine
branch, Locomotion (flight/foot) branches + interiors (**G5**); RTT codex thumbnails; auto-route
(can ride in G4 if cheap, else G5); authored lore layer over the grammar (decide separately).

## Design sketch

- **Lives in `scraped-again`.** Reuses: G1 `progress`/strata/`Event`/`apply` + the `pg=` payload;
  G2 beam (auto-collect fires it); G3 `scan` (Sensing tunes it); E17 `text` (the menu + translated
  rendering); G1 codex screen (the archive tab); `hud` for utilitarian text.
- **`tree`** — `struct Node { id, branch, cost: (Stratum, u64), prereqs: &[NodeId], effect }`;
  a static `NODES` table; `Tree`/unlocked-set in `progress`. Pure: `can_unlock(node, strata,
  unlocked)`, `apply_unlock` (spend + mark), `effects()` (derived modifiers the systems read:
  scan range mult, auto-collect on, legible scripts). All unit-tested.
- **Effects are read, not pushed** — systems query `progress.effects()` (e.g. `scan` reads the
  range mult; the app's auto-collect checks `effects.auto_collect`; `text` label-building checks
  `effects.legible(script)` and substitutes meaning). Keeps the tree a thin policy layer.
- **The menu** — a `menu` module drawing on the E17 text path: a column of nodes per branch,
  a cursor, cost/affordability colouring (within the palette). Input: a toggle key + arrows/pad
  to move, a key to buy. Drawn as world-text-style glyphs composited like the HUD/codex.
- **Word-bank/grammar** — a tiny seeded grammar (`lexicon` module): per script, glyph→word maps
  composing fragmentary elegiac phrases (§6). Legibility unlock flips a script from glyphs to
  its grammar output. Deterministic in seed.

## Decisions to resolve (with recommended defaults)

1. **Menu rendering path.** *Default:* **extend the E17 world-text/HUD path** (no new engine
   pipeline) — draw the menu as composited text quads. Only add a small generic `bm-render`
   text-UI helper if E17 genuinely can't lay out a menu; keep it content-free.
2. **Tree size for v1.** *Default:* **~3 branches × ~3 nodes** (Sensing, Memory incl.
   Auto-Collect, Decipherment incl. one Legibility) — enough to prove the loop; expand in G6+.
3. **Decipherment content.** *Default:* **procedural-poetic grammar** (a seeded word-bank), one
   or two scripts legible in v1; an authored layer is a later option (§6) — decide separately.
4. **Auto-collect scope.** *Default:* Memory **Auto-Collect I** fires the warm beam into the
   forward zone while piloting, taking the **common layer in the corridor** (rares stay manual) —
   so a hands-off run progresses. Auto-route deferred to G5 unless trivial.
5. **Payload.** *Default:* bump `pg=` to **v3** (append-only: the unlocked-node set), tolerant of
   v1/v2 (older saves → no unlocks).
6. **Balance.** *Default:* hand-tune small integer costs now; the *feel* needs live iteration —
   design the shape, tune on play (game-mechanics §8 "depth, honestly").

## Tests

- **Tree logic (pure):** `can_unlock` respects strata + prereqs; `apply_unlock` spends exactly
  and is idempotent (no double-spend); `effects()` reflects the unlocked set; determinism.
- **Event/apply:** `Unlock` round-trips through the payload (v3) + restores; tolerates v1/v2.
- **Decipherment:** an unlocked script maps glyphs→words deterministically; locked stays glyphs.
- **Effects wiring (logic-level):** scan range scales with Sensing; auto-collect flag flips with
  Memory; legible-set drives `text` substitution.
- **Menu** logic (cursor/affordability) unit-tested; the *visual* eyeballed headless (a menu
  screenshot, like the codex).
- All four targets build; `bm-render` stays game-agnostic (boundary check); golden voxel-hash
  unchanged.

## Risks & mitigations

- **The in-engine menu is the big unknown.** *Mitigation:* lean entirely on the E17 text path
  (it was always the stated UI substrate); v1 menu is utilitarian (a list + cursor), polish later.
- **Economy feel.** *Mitigation:* tiny tuned v1 + the idle/active multipliers from G1–G3 already
  in place; iterate costs on play; keep it no-fail (the tree is the only sink).
- **Grammar reads as Mad-Libs.** *Mitigation:* invest in the grammar, keep phrases short +
  elegiac; allow an authored layer later (§6).
- **Tone (a flashing idle dashboard).** *Mitigation:* the menu sits *inside* the doom palette —
  quiet, dithered, archival; no popups/score (§14 guardrail).

## Acceptance checklist

- [ ] A small **data-driven tree** (Sensing/Memory/Decipherment, ~3×3) with a pure
      `can_unlock`/`apply_unlock`/`effects` core (unit-tested).
- [ ] **Spend/unlock** via a serializable `Event::Unlock`/`apply`; unlocked set saves/restores
      (`pg=` v3, tolerant of v1/v2); determinism tested.
- [ ] An **in-engine tree/codex menu** (E17 text path, palettised) — toggle, navigate, see
      cost/affordability, buy; the archive tab reuses the G1 codex.
- [ ] Node effects **visibly** wire to systems: Sensing → wider scan; Memory **Auto-Collect** →
      the cruiser auto-harvests its corridor (hands-off progress); Decipherment **Legibility** →
      a script renders **translated** (seeded grammar).
- [ ] CI green (fmt / clippy -D / tests / wasm); `bm-render` still game-agnostic (boundary);
      golden voxel-hash unchanged; menu screenshot captured headless.
- [ ] `game-mechanics.md` §13 ticked for G4; this checklist complete.

## Out of scope / follow-ups

- **G5** — autonomy + interiors: "ship learns to seek" auto-route; Descent/Hull foot-collision so
  caves + solid colossi become collectible interiors; the Locomotion (flight/foot) branches.
- **G6+** — expand each branch toward the late **Concordance / Synthesis** arcs; the Resonance/
  pristine branch; RTT codex thumbnails; co-op shared archive (N1); an authored-lore layer.
