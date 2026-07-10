# G22 — Proto-language & the comparative method (lexicon v3 + the collation sink)

> **Status: ready to build.** The Archive tranche's endgame puzzle, the §E pin cashed:
> the five strata become **daughter forms of one proto-language** (the Tolkien method —
> [`../research-linguistics.md`](research-linguistics.md) §5/§6: one seeded proto-lexicon,
> five ordered deterministic sound-change cascades), which (a) gives the pinned **script
> difficulty shapes** their cheap lexicon-level half, (b) creates **detectable cognates**
> — the late-game structural epiphany "the strata were one people" (never prose; the
> family tree is glyph-structure, per the no-lore constraint), and (c) supplies the
> **strata-data sink** the pacing analysis demanded: **collation** — comparative work
> *consumes* banked strata data. Game-side; **no engine change expected** (all rendering
> rides existing transliteration/marks/cartouche paths).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** One proto-lexicon; five daughters via per-stratum sound-change cascades;
  rare **dual spellings** (Landa's accidental bilingual — a name written in two strata at
  once) seed **cognate candidates**; the player spends banked strata data to **collate**
  (confirm) a pair; enough confirmed pairs make a stratum-pair **correspondence known**,
  unlocking **comparative restoration** — the knowledge-crib that restores worn names in
  a sister stratum. Knowledge travels; instruments don't (G21's sensing stays on-foot;
  the comparative method works from the ship — it's in your head).
- **Demonstrable outcome.** Collect a dual-spelling name-bearer (one cartouched name,
  two scripts, visibly the *same-but-shifted* word) → the codex logs a cognate candidate
  → spend strata data from both strata to collate → a **family-tree codex panel** grows
  (structural: proto slot + the daughter forms as glyphs, no translation) → after 3
  confirmed pairs between two strata their correspondence is known → a worn name-bearer
  in either stratum whose partner form is attested now **restores** (Leiden `[abc]`,
  full yield) even from the ship. Strata balances visibly *go down* for the first time.
- **De-risks.** Whether the endgame comparative puzzle lands as structure-not-prose;
  whether a data sink feels like investment rather than tax; the §E shapes' foundation
  (all later typology work builds on the cascades).

## Scope

**In:**
1. **Lexicon v3 — proto + daughters.** Vocabulary/corpus generation moves to a **proto
   level** (phoneme strings; same morphology/statistical-honesty machinery), and every
   stratum's *surface* form derives via an **ordered deterministic sound-change cascade**
   (research §6): **Records** = mild shifts (the on-ramp stays plainest); **Schematics** =
   vowel-dropped shorthand (abjad-like — engineers' register); **Rites** = palatalization
   (\*k→s before front vowels) + CV re-shaping (syllabary-friendly); **Relics** = final
   vowels dropped (terse lapidary); **Signals** = **identity — the proto preserved** (the
   machine layer speaks the mother tongue; the hardest-*looking* script is secretly the
   root of the tree — and SGA being a trivially-cracked substitution is *fine*: the easy
   crack opens the hard comparative puzzle). All existing surface consumers re-source
   (block/vocab names, phrases, records, frames, palimpsest under-texts — G21 said G22
   re-sources them); transliteration downstream unchanged. **Guarantees move to the
   surface forms:** per-script post-transliteration distinctness, ≥4-glyph, English
   blocklist + keys/function-words rejection — now checked on *every daughter form* of
   every word. Plus a **cognate-detectability statistical test** (mean edit distance
   within cognate sets ≪ between unrelated words) and the honesty suite re-run
   per-stratum surface corpus.
2. **Dual spellings (the Landa gift).** A deterministic minority of name-bearing
   inscriptions (~**1/12**, independent salt, standing correlation discipline) carry the
   name **twice**: its own stratum's form + the cognate form in one sister stratum's
   script (both cartouched, stacked — reuse the G21 stacked-render path). Collecting one
   logs a **cognate candidate** (the pair of forms; both strata credited normally).
   v1: dual spellings are the *only* candidate source (root-mining the ambient corpus is
   later polish — recorded, not lost).
3. **Collation — the strata-data sink.** A discovered candidate can be **collated** from
   the console (a research-adjacent action, structural UI): costs banked **strata data
   from both strata of the pair** (cost scales with the deeper stratum — sketch numbers
   below; tune against the measured ~3.2/min per-domain income so a collation ≈ a few
   minutes' banked data, and note the arithmetic in the brief's as-built). Confirmed →
   **family-tree codex panel** entry: the proto slot (rendered in Signals script — it *is*
   the proto) + daughter forms, glyphs only. `pg=` **v12** append-only.
4. **Known correspondence → comparative restoration.** At **3 confirmed pairs** for a
   stratum pair, the correspondence is **known** (codex marks the tree edge). Payoff verb:
   a **worn name-bearer** whose name's partner form is **attested** (its dual spelling
   collected, or the block discovered in the partner stratum) **restores** — lost glyphs
   render Leiden-bracketed via the existing G20 restore machinery, full unreduced yield —
   and this works **from the ship** (rung 0): the comparative method is knowledge, not an
   instrument. No double-pay when stacked with frame-restore or close reading (one full
   yield, best-instrument-wins).
5. **D11 scenario:** dual spelling → candidate → collate ×3 → correspondence known → a
   ship-collected worn name-bearer restores comparatively (asserting the ship *can* — the
   deliberate inversion of G21's on-foot assert).
6. **Worldgen-version policy** applies (all inscription surface content shifts; golden
   voxel-hash untouched; coverage/uniformity/`name_of_text`/pixel-identity tests
   re-target — the G20 precedent).

**Out:** determinatives/classifier glyphs, register split (monument/graffiti/ledger),
layout pre-puzzles (boustrophedon/rotation), the Relics "Maya trap" logo-morphemic deep
shape, corpus-budget difficulty dials, prosopography/hands (G23), ambient root-mining,
any readable prose anywhere (the family tree is glyphs + tree structure only).

## Design sketch

- `lexicon`: `proto_word(seed, key)` (today's generator, renamed inward);
  `derive(word, stratum) -> String` — an ordered rule list per stratum over the phoneme
  string (pure, ~5 rules each, unit-tested rule-by-rule); `surface_word(seed, key, stratum)
  = derive(proto_word(..))`; `vocabulary(seed)` returns per-stratum surfaces (collision
  loop rejects a candidate if **any** daughter violates any guarantee — the retry/grow
  escape hatch machinery survives unchanged). Cognate sets are free: same key, five forms.
- `structures`: name-bearers render `surface_word(.., block.stratum())`; dual spellings
  pick the partner stratum deterministically from the cell and stack the second cartouched
  line; phrases/records/frames/under-texts derive through the cell's script's stratum.
- `progress`: `cognate_candidates: Vec<(key-ish id, StratumPair)>` + `collated:
  HashSet<..>` + correspondence-known derivation; `Event::CollectDual`,
  `Event::Collate { pair, cost }` (the sink — subtracts strata); comparative-restore
  consult in the same worn-collect path close reading/frames use (explicit precedence:
  close-reading = frames = comparative → one full yield).
- Costs (sketch, tune in-build): collate cost = `30 × (1 << deeper.byte())` from **each**
  side (SCH pair ≈ 60+60 … SIG pair ≈ 480+480) — at ~3.2/min/domain that's minutes-scale
  early, tens-of-minutes deep; must not starve research (research spends *shards*, collation
  spends *data* — disjoint currencies, no direct competition; note it in the as-built).
- Console: candidates + collate action live beside research rows (structural UI, glyphs +
  gauges only); the family tree is a codex panel (existing codex render machinery).

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Signals = the proto** (identity cascade; the machine layer is the mother tongue).
2. Dual spellings ~**1/12** of name-bearers, sole candidate source v1.
3. **3 confirmed pairs** → correspondence known (placeholder; feel pass tunes).
4. **Comparative restoration is ship-available** — knowledge vs instrument is the G21/G22
   axis (instruments walk, knowledge flies).
5. Collation costs **data, not shards** (the sink must not compete with research's currency).

## Tests

Cascade determinism + per-rule units; per-stratum surface guarantees (distinctness
post-transliteration per script, blocklist/keys/function-words, ≥4 glyphs) across all five
daughters; cognate-detectability statistic; honesty suite per-stratum; dual-spelling rate
/determinism/independence (correlation discipline) + stacked render; collation gating
(candidate required, funds required), sink accounting (strata decrease), pg= v12
round-trip + migration; correspondence-known threshold; comparative restoration (render
+ full yield + ship-path) and **no double-pay** with frames/close-reading; D11 scenario;
envelope pacing green (collation is player-initiated — autopilot band should not move;
re-pin with a note if it does); golden voxel-hash unchanged; four-way CI; boundary intact
(no engine change); roadmap G22 + brief as-built.

## Acceptance checklist

- [ ] Lexicon v3: proto + five cascades (Signals = identity); all surface consumers
      re-sourced; surface-form guarantees + cognate-detectability + honesty suite green.
- [ ] Dual spellings (~1/12, independent, stacked cartouched render) log cognate
      candidates on collect.
- [ ] Collation spends strata data from both sides (structural UI; family-tree codex
      panel, glyphs only); `pg=` v12 append-only.
- [ ] 3 pairs → correspondence known; comparative restoration (Leiden render, full yield,
      **ship-available**, no double-pay); D11 scenario passes.
- [ ] Worldgen-version policy honored; golden voxel-hash unchanged; envelope green
      (re-pinned + noted if shifted); four-way CI green; boundary intact; roadmap G22 +
      brief as-built.

## Standing notes for the build agent

- **Push per commit** (the G21 landing batched four commits into one push — only the tip
  got CI; per-commit pushes keep bisection cheap).
- The Signals-gated *block* gap (noted in the G21 review) stays open — do **not** invent
  a block here; G22's Signals-tier content is the proto twist itself.
- Splittable: (1) lexicon v3 + re-source, (2) dual spellings + candidates, (3) collation
  + tree + comparative restoration + D11. Land each green.
