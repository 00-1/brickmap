# G4 — Block substrate & console (the retrofit)

> **Status: ready to build, after the ✅ G1–G3.** The **foundational block substrate**
> ([`../game-system.md`](../game-system.md)) — which G1–G3 shipped *before*, as direct
> keybind features. This milestone retrofits it: it re-expresses those existing actions as
> **blocks** and adds the **console**, *without changing what they do*. It **supersedes the
> old "G4 — first tech tree"** (a conventional spend-on-nodes menu); that approach is dead —
> its goals (in-engine menu, decipherment-legibility, auto-collect) return later via the
> substrate (the console here; composed routines in G5; the vocabulary/unlocks in G6). In
> the `scraped-again` crate. Design: [`../game-mechanics.md`](../game-mechanics.md).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Stand up the **minimal block runtime + console UI** and re-express the
  already-built actions (`collect`, `scan`, `fire-beam`, nav) as **visible, clickable
  blocks** wired into a couple of **given routines** — turning today's keybind features into
  the block interface.
- **Demonstrable outcome.** A **console** shows your blocks; clicking `collect`/`scan` does
  exactly what `T`/auto-scan do today; a **given `scan(shards) → on-scan → collect` routine**
  runs on its own (the current autoscan/collect behaviour, now *as a routine* you can toggle
  and open); the default `drift` autopilot is itself a given nav routine. Behaviour is
  unchanged; it's now expressed in blocks.
- **De-risks.** The whole game's spine — that **"everything is a clickable block" is viable
  and feels good** on the existing systems — *before* G5's editor and G6's vocabulary build on
  it. Catches the substrate's UX/architecture risk on known, working behaviour.

## Scope

**In:**
- **Minimal block runtime.** A block = a visible, named operation that can be (a) **clicked
  to trigger** once, and (b) **run by a routine** (a small fixed trigger→action[s] wiring).
  G4 ships the runtime + **given routines only** — *no player editor* (that's G5). Block
  effects dispatch through the **same code paths** the current keybinds use (a thin wrapper,
  not a rewrite).
- **Wrap the existing actions as blocks** (game-system §11 Tier 0): `scan(item)` (starts
  `scan(shards)`), `collect`, `fire-beam` (the G2 beam), `spend`, `goto(area)`, nav `drift`,
  trigger `on-scan`. Parameter values that exist today only (`scan` → `shards`); more
  items/fields are G5/G6.
- **Given pre-wired routines, shown as their blocks:** `scan(shards) → on-scan → collect`
  (re-expressing G1's collect + G3's autoscan) and `drift` (the default autopilot). The
  onboarding artifact: a working routine you can run, toggle, or *view as blocks*.
- **Console UI** on the **E17 world-text / HUD text path** (terminal-styled, low-fi,
  click/controller friendly; **no typing**): list the blocks + given routines; click a block
  to trigger; toggle a routine.

**Out:** the **composition editor** / player-authored routines, the generic `match` filter,
parameterised-block *unlocks* (G5); `when`/`budget`/`priority`/control-meta, `decode`, the
unlock economy, Decipherment legibility (G6); `scanMany`, more items (G6); two-agent/hail
(G7). No gameplay *behaviour* changes here — it's a representation/UI retrofit.

## Design sketch

- **Lives in `scraped-again`** over what G1–G3 already built (`progress`, `beam`, `scan`,
  `autoscan`, the map, the E17 `text` path, `hud`) + the `edit`/`Event` seam.
- **Block/runtime model** — a small **data-driven** table: `Block { id, kind, param }`;
  `Routine { trigger, steps, enabled }`; a `Runtime` that ticks enabled routines and
  dispatches each block to the existing effect fn (the one the keybind calls). Keep it tiny so
  G5/G6 extend the *data/table*, not the architecture.
- **Re-expression, not rewrite** — `collect` block → the existing G1 collect path; `scan`
  block / `on-scan` → the existing G3 autoscan/scan path; `fire-beam` → G2's beam; `drift` →
  the existing auto-fly. The given routines reproduce **current behaviour exactly**.
- **Console** — reuse the E17/HUD text rendering; a simple selectable list (blocks +
  routines), click/`A`-button to trigger, toggle to enable/disable. Minimal in G4; the rich
  editor is G5.

## Decisions to resolve (with recommended defaults)

1. **How much runtime in G4?** *Default:* **trigger-by-click + run given linear routines
   only**; editor/conditions/flow are G5. Bounded but real.
2. **Keybinds during/after the retrofit?** *Default:* keep the existing keybinds working
   (they become shortcuts that fire the same blocks) — no regression for current play.
3. **Console rendering** *Default:* the **E17 text path** (terminal aesthetic, cross-platform,
   no-typing), not DOM — even if minimal here.
4. **Data-driven vs hardcoded blocks** *Default:* **data-driven table** from the start, so
   G5/G6 add rows, not branches.

## Tests

- **Runtime**: a given routine fires its trigger → runs its steps; clicking a block triggers
  the *same* effect as today's keybind (assert parity); toggling a routine stops/starts it.
- **Behaviour parity**: collect/scan/beam/autopilot via blocks produce identical results to
  the pre-retrofit keybind paths (regression guard).
- Native + wasm build green; console renders headless; **golden voxel-hash + headless render
  unchanged** (pure game logic / UI).

## Risks & mitigations

- **Console UX on the text path** (legibility, click/controller targeting). *Mitigation:*
  keep G4's console deliberately minimal (a list + the given routines); the rich editor is G5.
- **Over-building the runtime.** *Mitigation:* Decision 1/4 — given linear routines only,
  data-driven table.
- **Regressing G1–G3 behaviour.** *Mitigation:* dispatch through the *existing* effect fns;
  the behaviour-parity tests above.

## Acceptance checklist

- [ ] Minimal block runtime: blocks visible + **clickable to trigger**; the given
      `scan(shards) → on-scan → collect` and `drift` routines run and can be toggled/opened.
- [ ] `collect`/`scan`/`fire-beam`/`spend`/`goto`/`drift`/`on-scan` exist as blocks dispatching
      to the existing G1–G3 effect paths (behaviour-parity tested); current keybinds still work.
- [ ] Console + routine view render on the E17 text path (no typing; click/controller-driven).
- [ ] No gameplay behaviour change; golden voxel-hash + headless render unchanged.
- [ ] CI green (fmt / clippy -D / tests / wasm); crate boundary intact.
- [ ] Docs in lockstep: game-mechanics §13 G4 + game-system §11 Tier 0 ticked.
