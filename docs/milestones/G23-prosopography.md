# G23 — Prosopography: hands, persons & the factoid survey log

> **Status: ready to build** (finalized 2026-07-10 against G22's as-built: lexicon v3's
> `derive()` and the dual/stacked render exist; glyph layout confirmed engine-side, so the
> ductus mechanism is pinned below). The Archive tranche's elegiac layer, from
> [`../research-material-text.md`](research-material-text.md) §3/§4:
> **hands** (every inscription attributable to a writer, detectable *materially*),
> **persons** (assembled by the player from scattered attestations — Trismegistos' law:
> most people in history are attested exactly once; linking even two records is the
> payoff), and **repairs** (kintsugi — re-cut glyphs in a *different hand*: one stone, two
> dates, two acts of care, zero readable lore). This is the milestone where grief arrives
> through structure alone. Game-side, with **one permitted engine touch** (pinned below):
> a content-agnostic per-glyph baseline-offset capability in `bm-render`'s text path —
> generic typography, the same permitted class as the G18/G20 mark glyphs.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Every composed inscription carries a **hand** (a writer identity from a small
  per-world pool, rendered materially — a per-glyph ductus, not a UI label); a new
  **epitaph/graffito register** carries **person names** (a new lexicon class, proto-derived
  like everything post-G22); the survey log gains a **factoid panel**: atomic,
  source-anchored observations the player can **join** into person hypotheses
  (Provisional → Confirmed via corroboration, the G18 attestation machinery generalized);
  rare worn inscriptions carry a **repair in a different hand**.
- **Demonstrable outcome.** Two epitaphs at different sites share a name-form and a
  shaky ductus; the codex's factoid panel lets the player join them; a third attestation
  confirms — a *person* now exists in the catalogue, assembled entirely by the player from
  glyph-structure (no prose, no translation, no biography — a name, a hand, three places).
  A repaired inscription visibly changes hands mid-text. Telemetry reports the join in
  honest state lines.
- **De-risks.** Whether the game's emotional thesis ("grieve someone who exists only as a
  join across your own catalogue") lands mechanically; whether hands are readable as
  *material* signal without any readable language; the factoid model (which E-series
  panels and later content build on).

## Scope

**In:**
1. **Hands.** A deterministic per-world pool of hands (size ~ the corpus warrants:
   most hands attest once or twice — the Trismegistos ratio is the *point*; a few
   recurrent hands are the discoveries). Every composed inscription draws a hand
   (per-cell salt; geographically clustered — a hand attests at nearby sites, never
   world-wide). **Material rendering:** a per-hand ductus applied at glyph-layout time —
   small deterministic per-glyph vertical offsets / baseline wobble (±1 px class), so a
   shaky hand *looks* shaky in any script. No hand-id is ever shown as a UI judgment;
   the player recognizes ductus the way they recognize cartouches. **Mechanism (pinned;
   layout is engine-side):** `bm-render`'s rasterize path gains an optional caller-supplied
   per-glyph baseline offset (a `&[i8]` or equivalent — content-agnostic, knows nothing of
   hands); the *game* computes offsets from `(hand, glyph_index)`. Engine tests: offsets
   bounded, deterministic, default-empty path byte-identical to today (golden-neutral).
2. **Person names + the epitaph/graffito register.** A new lexicon class
   (`person_name` — proto-derived daughters like all v3 vocabulary; collision/blocklist
   guarantees extend to it; never colliding with block/vocab words). A small family of
   **epitaph frames** (formulaic — the G16/G20 frame machinery; the varying slot is a
   person name) and a **graffito register** (short name+verb scratchings; recognizable by
   *format and density*, not reading — different composition parameters, not a new
   renderer). Epitaph-register share follows the content-mix law loosely (formulaic
   dominates; epitaphs are common among *monumental* pieces).
3. **The factoid survey log.** Each collect auto-records atomic **factoids**:
   (name-form, hand, site/cell, register, condition) — source-anchored, nothing inferred.
   A codex **prosopography panel** lists unjoined factoids; the player **joins** two that
   share a name-form or a hand into a **person hypothesis** (Provisional); a third
   corroborating factoid (same name-form or hand again) **Confirms** — reusing the G18
   attestation semantics. **The machine never auto-joins** — the player performs the join;
   the panel renders persons as glyphs + a join-graph structure only. Joins are free
   (attention is the cost — the archival theme); mis-joins can be **unpicked** (dissolve a
   hypothesis; retreatability, never a punishment). `pg=` **v13** append-only.
4. **Repairs (the kintsugi beat).** ~**1/40 worn** inscriptions carry a **repair**: a run
   of lost glyphs re-cut **in a different hand** (the ductus visibly changes mid-text);
   occasionally (~1/4 of repairs) the repair is in the **sister stratum's cognate form**
   (translation-as-repair — riding G22's derive + stacked-render machinery). Repaired
   glyphs read as present (they were re-cut, not lost): they count toward yield and frame
   matching; the two hands each leave a factoid (one stone → two writers → a join hint).
5. **Telemetry:** honest state lines for factoid capture, joins, confirmations, and
   hand-recurrence sightings (the `◆` lit-goal grammar where it fits).
6. **D11 scenario:** collect two same-hand epitaphs at different sites → join →
   third attestation confirms → the person round-trips through `pg=` v13; a repaired
   inscription yields both hands' factoids.

**Out:** the gravestone-seriation dating minigame (banked — it wants motif frequencies we
don't compose yet); per-hand *letterform* palaeography (needs per-hand glyph sets — engine
work; ductus is the v1 material signal); curse formulae / appeals-to-the-living; media
survival physics; powered/electronic sites; any auto-biography, any prose, any readable
names (person names are lexicon nonsense like everything).

## Design sketch

- `structures`: `hand_of(seed, cell) -> HandId` (pool + geographic clustering via
  site-anchored salt); the game computes `ductus(hand, glyph_index) -> i8` per glyph and
  hands the offset slice to the engine's rasterize path (see the pinned mechanism —
  `bm-render` stays hand-ignorant); `repair_of(seed, cell)` (~1/40 of worn, independent salt) replaces the
  worn-lost run with re-cut glyphs under the second hand (cognate-form variant via
  `lexicon::derive`).
- `lexicon`: `person_name(seed, idx)` — same candidate/rejection machinery, disjoint key
  space; epitaph frames join the frame family with a person-name slot.
- `progress`: `Factoid { name_form, hand, cell, register, condition }`,
  `Event::RecordFactoid`, `PersonHypothesis { factoids, attestation }` with
  join/unpick/confirm transitions; `pg=` v13 append-only.
- `console`/codex: the prosopography panel (existing codex render machinery; join-graph as
  indented glyph rows — no new engine text capability).

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Ductus, not labels:** hands are only ever visible as glyph-layout material signal
   (±1 px class); no hand-code UI. (Recurrence is *discovered*, like cartouches were.)
2. **Most hands attest once** — the pool is sized so recurrent hands are rare finds
   (Trismegistos ratio as a tuning target, not a hard invariant).
3. **Joins are player-performed, free, and unpickable** (retreatability); confirmation
   needs a third corroborating factoid (G18 semantics).
4. **Repairs count as present text** (re-cut, not lost): full yield + frame credit; ~1/40
   worn, ~1/4 of those translation-as-repair.
5. Person names are per-world (per-seed) like all v3 vocabulary.

## Tests

Hand pool determinism + clustering (a hand's attestation sites are near one another) +
recurrence distribution (most once — statistical); ductus determinism + bounded offsets +
render into every script (pixel test at compose level); person-name guarantees (disjoint
from vocabulary, blocklist, per-script post-transliteration distinctness); epitaph frame
family + graffito register composition parameters; factoid capture on every collect route;
join/unpick/confirm state machine + attestation semantics; repair rate/independence
(correlation discipline), two-hands factoids, cognate-form repairs derive correctly,
repaired = full yield + frame credit; `pg=` v13 round-trip + migration; D11 scenario;
envelope pacing green (repairs slightly raise worn yield — re-pin with a note if the band
shifts); golden voxel-hash unchanged; four-way CI; boundary intact; roadmap G23 + brief
as-built.

## Acceptance checklist

- [ ] Hands: deterministic pool, geographic clustering, once-dominant recurrence; ductus
      as the only signal, rendered in all five scripts.
- [ ] Person names (new lexicon class, all guarantees) + epitaph frames + graffito
      register (format-recognizable, not readable).
- [ ] Factoid log + prosopography panel: player-performed joins, unpick, third-attestation
      confirm (G18 semantics); glyphs + structure only; `pg=` v13 append-only.
- [ ] Repairs: ~1/40 worn, different hand, ~1/4 translation-as-repair via G22 derive;
      full yield + frame credit; two factoids per repaired stone.
- [ ] D11 scenario; telemetry lines; envelope green (re-pinned + noted if shifted); golden
      voxel-hash unchanged; four-way CI green; boundary intact; roadmap G23 + brief
      as-built.

## Standing notes for the build agent

- **Push per commit** (per-commit CI; the G21 lesson).
- Splittable: (1) hands + ductus, (2) person names + registers + factoid log + panel,
  (3) repairs + D11 + docs. Land each green.
- The engine touch is pre-approved but minimal: per-glyph baseline offsets on the text
  rasterize path, caller-computed, default-empty = byte-identical output (assert it).
  Nothing about hands, persons, or the game enters `bm-*`.
