# G21 — The sensing ladder (close reading, deep sensing, palimpsests)

> **Status: done ✅ (2026-07-10, four commits — sensing targets + close reading; deep
> sensing + erased reveal; palimpsests; riders + the rationality assert).** The Archive
> tranche's payoff milestone, modeled on the real
> recovery ladder ([`research-material-text.md`](../research-material-text.md) §2 — raking
> light → UV/multispectral → penetrating): **worn and ⟦erased⟧ text becomes recoverable,
> on foot, with researched sensing** — which simultaneously (a) cashes the G18 erasure
> hooks, (b) gives the walker/expedition its **economic reason to exist** (the pacing
> analysis: expeditions are currently income-negative), and (c) **fills the empty Rites and
> Signals research tiers** with the sensing faculties themselves. Plus two riders from the
> G20 review. Game-side; no engine change expected (all rendering rides existing paths).
> *As-built notes + deviations at the end.*

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

- [x] `close-reading` (Rites) + `deep-sensing` (Signals + 8-rare) research targets,
      lexicon-named, auto-discovered by frustration events; `pg=` v11 append-only.
- [x] On-foot worn recovery (full text/yield/frame credit) vs ship rung-0; erased reveal
      on foot resolves the logged gouge with deep-weighted content; palimpsests (~1/60,
      tell, deep-sensing reveals, stacked codex, both-layer yield).
- [x] Expedition rationality asserted (D11): close-reading expeditions obtain what
      drifting cannot.
- [x] Riders: given routines lexicon-named; English blocklist in the generator.
- [x] Envelope green (re-pinned + noted if shifted); golden voxel-hash unchanged; four-way
      CI green; boundary intact; roadmap G21 + brief as-built.

## As-built (2026-07-10)

Landed in four green commits: **(1)** sensing research targets + close reading, **(2)**
deep sensing + the erased reveal, **(3)** palimpsests, **(4)** riders + the D11
expedition-rationality assert + docs.

- **Sensing targets.** A `ResearchTarget::Sense` arm (rkeys `0xE0..`, disjoint from block
  codes and faculties): `close-reading` (Rites, cost 100 = `25 << 2`, no rare requirement)
  and `deep-sensing` (Signals, cost 400 = `25 << 4` + the standing **8-rare** own-domain
  gate — the gate's first natural object). Domain-matched fill like a block; comprehension
  **folds in the tier's legibility** like a block (researching deep-sensing is the first
  thing that can turn Galactic legible — noted; it's the deliberate "fills the empty tier"
  payoff). Lexicon-named (`VOCAB_KEYS` + `close-reading`/`deep-sensing`, appended), glyph-
  rendered in the gating stratum's script. Auto-discovered by the frustration events inside
  `Progress::apply` (first lacuna-bearing `Collect` / first `CollectErased`), so every
  collect route funnels the discovery deterministically; the console lists discovered
  instruments as research rows (`Sel::Sense`). `pg=` **v11** append-only.
- **On-foot = the agent, not the mode.** Every collect route funnels through one shared
  tail (`collect_collectible`) taking `on_foot`: manual walk-mode collect, the walk-mode
  survey-beam, foot on-scan, the away-walker's routines, and the expedition harvest pass
  it; every ship route (auto-collect, aim-collect while piloting, ship on-scan) stays
  rung 0 *even with the faculties researched* (Decision 2 held: the walker carries the
  instrument).
- **Close reading.** `Inscription`/`Collectible` carry the worn cell's `pristine`
  composition; `structures::recover_worn` merges it Leiden-bracketed (`[abc]` — the G20
  restore marks; structure, no yield) → full text, full unreduced yield, and frame credit
  as-if-intact (`sight_frame` judges the **banked** text, so a recovered exemplar teaches).
- **Deep sensing.** `structures::hidden_text(seed, cell)`: deterministic, fresh-salt
  (condition-independence tested), deep-weighted — ~1/4 name-bearers off a RunFoot-×3
  table (+ the Schematics nav words; there is still no Signals-gated *block*, so "deep
  names" = the deepest existing vocabulary), else Runic~2:1~Galactic data. The new
  `Event::RevealErased` resolves a **logged** gouge's codex entry *in place*
  (`⟦——⟧` → `⟦glyphs⟧`; stored as `revealed_text` = one leading gouge mark + content) and
  banks the yield; `update_inscriptions` re-lists logged-but-unresolved erasures once deep
  sensing is held — the logged-erasures list is literally the destination list. A revealed
  name discovers its block.
- **Palimpsests.** `structures::under_text(seed, cell)`: ~1/60 of *eligible* ambient cells
  (never names, frames, or erasures — an erasure hides even the under-layer; frames were
  excluded to keep sighting/restore semantics clean — deviation, see below), independent
  salt, deep-strata script, **ordinary lexicon** words (`phrase` under a palimpsest-salted
  seed; G22 re-sources). The tell is one new content-agnostic engine mark (`MARK_BASELINE`,
  U+E625 — two faint parallel base rules; renders in every script + on the HUD overlay,
  excluded from yield like all marks). `Event::CollectPalimpsest` banks both layers and
  logs them **stacked** (two codex entries, one `find_id`; rendered surface over a
  `└`-led under line).
- **Rationality (D11).** `expedition_rationality_close_reading_earns_what_the_ship_cannot`:
  on the same worn site, the ship — *even holding the faculty* — banks survivors only and
  keeps lacunae in its codex, while the authored `run(foot)` expedition (the real
  Deploy→Harvest→Return machine) banks the recovered-full yield and the lacuna-free
  bracketed text: the differential per collect, plus two more on-foot-only data classes
  (erased reveals, palimpsest under-texts) asserted in their own scenarios.
- **Riders.** The four givens display seeded lexicon names (`Console::routine_display_name`
  — `survey`/`prospect` joined `VOCAB_KEYS`; `drift`/`collect` reuse their block words;
  player routines keep `trace-N`/`routine-N` verbatim, `run(given)` resolves to the lexicon
  name). A ~220-word common-English blocklist joined the name generator's rejection filter
  (both the retry and the grow escape-hatch paths); determinism/collision/never-english
  re-verified, plus a 200-seed no-blocklist-word sweep.

**Deviations from the brief.**
1. Palimpsests exclude **frame cells** (brief said "ambient"): a frame's verbatim-recurrence
   and sighting/restore semantics don't compose cleanly with a second layer; the ~1/60 rate
   applies over the remaining ambient majority (still several per origin field).
2. A palimpsest collected at rungs 0–1 spends the site (surface only, under-layer lost) —
   the tell warns *before* collection; revisit-after-research applies to erased sites only.
   If "come back for the under-text" turns out to be wanted, the erased-revisit machinery
   generalises (noted for the feel pass).
3. Comprehending a sensing instrument folds in its tier's **legibility** (like a block).
   The brief didn't pin this; it follows the G15 rule "researching tier vocabulary cracks
   the tier" and gives Signals its first crackable object.
4. The envelope needed **no re-pin**: recovery only raises *on-foot* yield, and the pacing
   probe's autopilot is pure ship (rung 0) — measured band unchanged (see
   [`../pacing-analysis.md`](../pacing-analysis.md) addendum).
5. Gallery snapshot skipped, following G18–G20 precedent (the archive's last entry is the
   E-series; revisit when a visual milestone warrants it).
