# G21 — The sensing ladder (close reading, deep sensing, palimpsests)

> **Status: ready to build.** The Archive tranche's payoff milestone, modeled on the real
> recovery ladder ([`../research-material-text.md`](research-material-text.md) §2 — raking
> light → UV/multispectral → penetrating): **worn and ⟦erased⟧ text becomes recoverable,
> on foot, with researched sensing** — which simultaneously (a) cashes the G18 erasure
> hooks, (b) gives the walker/expedition its **economic reason to exist** (the pacing
> analysis: expeditions are currently income-negative), and (c) **fills the empty Rites and
> Signals research tiers** with the sensing faculties themselves. Plus two riders from the
> G20 review. Game-side; no engine change expected (all rendering rides existing paths).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** A three-rung ladder: **rung 0** (today's beam/scan — reads what survives);
  **rung 1 — close reading** (a researched faculty, Rites-tier): the *walker*, in reach of
  a worn inscription, recovers its lost glyphs — full text, full yield, frame-sighting
  credit; **rung 2 — deep sensing** (a researched faculty, Signals-tier): the *walker*
  recovers an ⟦erased⟧ inscription's underlying text (the gouge finally speaks) — and
  reveals **palimpsests**: rare inscriptions carrying an older under-text beneath the
  surface writing.
- **Demonstrable outcome.** Research close-reading (Rites shards) → land, walk to a worn
  inscription → it resolves fully on collect (codex: `[recovered]`-clean, full yield).
  Research deep-sensing (Signals shards + its 8-rare gate — the gate's natural object at
  last) → return to a logged ⟦erased⟧ site on foot → the gouge yields its hidden name/text
  (Relics/Signals-weighted data). A rare palimpsest cell shows a second, older under-text
  only deep-sensing reveals (codex renders both layers, stacked). **The expedition becomes
  rational:** an authored expedition to a worn/erased-rich site now returns data the
  drifting ship *cannot* obtain.
- **De-risks.** The walker's purpose (the automation game's biggest degeneracy), the
  Signals-tier vocabulary gap, and whether "revisit old finds with better instruments"
  lands as the loop-multiplier the research promised.

## Scope

**In:**
1. **Two sensing faculties as research targets:** `close-reading` (gated **Rites** —
   domain-matched Rites shards; no rare requirement) and `deep-sensing` (gated **Signals**
   — Signals shards + the standing 8-rare gate). One level each v1 (binary unlocks — the
   ladder rungs; multi-level sharpening is later polish). Lexicon-named + glyph-rendered
   like everything (they're *recovered instruments* of the dead machine). They appear as
   research targets once **discovered** — pin: discovered automatically on the player's
   **first worn collect** (close-reading) / **first erased log** (deep-sensing): the
   frustration teaches the need; the console then offers the remedy. `pg=` v11 append-only.
2. **Close reading (rung 1):** with the faculty comprehended and the player **on foot**
   within collect reach, a worn inscription collects **fully recovered** — full text
   (codex renders recovered glyphs in a distinct clean-restored state), full unreduced
   yield, frame-sighting credit (intact-equivalent). Ship-based collects stay rung-0
   (worn = reduced). Auto-collect via the *walker's* routines counts (it's the agent, not
   the hands, that carries the instrument).
3. **Deep sensing (rung 2, on foot):** an ⟦erased⟧ inscription collects its **hidden
   content** — compose the underlying text deterministically from the cell (it always
   existed; the gouge hid it): weighted toward name-bearers of **deep strata** and
   Relics/Signals data (erasures were deliberate — what was erased *mattered*). The codex
   erasure-log entry resolves (`⟦——⟧` → `⟦recovered glyphs⟧`); the logged-erasures list
   thus becomes a **destination list** — the G18 tease paying off. Without the faculty,
   erased stays silent (as today).
4. **Palimpsests:** ~1-in-60 ambient cells carry an **under-text** (an older, second
   composed text — deep-strata-weighted script) beneath their surface text. Invisible at
   rungs 0–1 (a faint doubled-baseline mark at most — pin: yes, a subtle tell, so they're
   findable before they're readable). Deep-sensing reveals: collecting yields both layers;
   the codex renders them stacked (surface / under). Deterministic, independent bits
   (the standing correlation test discipline). *Splittable to its own commit.*
5. **Expedition rationality, asserted:** extend D11/pacing — an authored expedition with
   close-reading to a worn-rich site yields data-classes/rates the drifting ship cannot
   (assert the differential, not just activity). The "worth it" test the pacing analysis
   demanded.
6. **Riders (G20 review):** (a) the four **given routines get lexicon names** (authored by
   the dead machine; player-created routines keep instrumentation defaults like
   `trace-1`); (b) an **English-word blocklist** (a small common-word list) added to the
   name-generator's rejection filter (kills "sorrel"-class accidents; re-verify
   determinism/collision tests).

**Out:** multi-level sensing sharpening; palimpsest *chains* (one under-layer only);
proto-language/cognates (G22 — though palimpsest under-texts should use the ordinary
lexicon so G22 can later re-source them); any new engine capability.

## Design sketch

- `progress`: `Faculty`-like sensing targets (or a `Sensing` variant in `ResearchTarget`)
  with stratum gating + the standing rare-gate machinery (Signals ⇒ 8 via
  `rare_requirement`); discovery events wired at the worn-collect / erased-log funnels.
- `structures`: `under_text(cell)` for palimpsest cells (independent salt); erased cells
  gain `hidden_text(cell)` (deterministic compose, deep-weighted).
- Collect paths: the walker's on-foot collects consult the sensing tier → full-recover /
  erased-reveal / palimpsest-both; ship collects unchanged. Codex render states extend
  (recovered-clean, erased-recovered, stacked layers) via existing mark machinery.
- Keep every new render state on the existing glyph/mark paths; no new engine glyphs
  expected (reuse lacuna/gouge/underdot/cartouche marks + layout).

## Decisions to resolve (pinned defaults — veto via the channel)

1. Sensing = **binary research targets** at Rites/Signals, auto-discovered by the
   frustration events (first worn collect / first erased log).
2. **On-foot only** for rungs 1–2 (the walker carries the instrument; ship stays rung 0) —
   this *is* the expedition's reason; don't soften it.
3. Palimpsest rate ~1/60, subtle doubled-baseline tell before revelation.
4. Erased hidden content is deep-weighted (names + Relics/Signals data) — erasure implies
   significance.
5. Given routines → lexicon names; player routines keep instrumentation defaults.

## Tests

Sensing research targets (gating, rare-gate on Signals, discovery events, v11 codec);
on-foot vs ship recovery differential; erased reveal resolves the codex log; palimpsest
determinism/independence + tell + stacked render + both-layer yield; the D11 expedition
**rationality assert** (data the ship can't get); riders (given names lexicon +
blocklist, determinism/collision re-verified); envelope test green (recovery raises
on-foot yield — re-pin with a note if the band shifts); golden voxel-hash unchanged;
four-way CI; boundary intact; roadmap G21.

## Acceptance checklist

- [ ] `close-reading` (Rites) + `deep-sensing` (Signals + 8-rare) research targets,
      lexicon-named, auto-discovered by frustration events; `pg=` v11 append-only.
- [ ] On-foot worn recovery (full text/yield/frame credit) vs ship rung-0; erased reveal
      on foot resolves the logged gouge with deep-weighted content; palimpsests (~1/60,
      tell, deep-sensing reveals, stacked codex, both-layer yield).
- [ ] Expedition rationality asserted (D11): close-reading expeditions obtain what
      drifting cannot.
- [ ] Riders: given routines lexicon-named; English blocklist in the generator.
- [ ] Envelope green (re-pinned + noted if shifted); golden voxel-hash unchanged; four-way
      CI green; boundary intact; roadmap G21 + brief as-built.
