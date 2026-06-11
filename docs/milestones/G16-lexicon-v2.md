# G16 — Lexicon v2 (statistical honesty)

> **Status: ready to build — next directive.** The fork-free first step of the **Archive
> tranche** (the human green-lit the tranche 2026-06-11, with the binding constraint **no
> readable lore** — comprehension is *structural*, never prose). This milestone makes the
> seeded lexicon generator produce text that **passes the statistical tests pattern-hunting
> players will actually run** ([`../research-linguistics.md`](research-linguistics.md) §2;
> [`../research-material-text.md`](research-material-text.md)) — so the world's nonsense-word
> inscriptions reward analysis instead of reading as noise. Pure game-side logic
> (`scraped-again::lexicon` + worldgen wiring); CI-testable; **no forks** (it commits to no
> contested Archive design — it's the substrate the later, fork-gated milestones sit on).
> No engine change.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** The generated corpus (across the five scripts) has the statistical fingerprints
  of real language — Zipf, Heaps, natural conditional entropy, Zipf abbreviation, consistent
  morphology, bursty content-words — each property a **unit test**, so the lexicon is
  honestly language-shaped, not Voynich-shaped noise.
- **Demonstrable outcome.** A `lexicon`-stats test/bin reports the corpus's Zipf slope,
  Heaps β, char conditional entropy (bits/char), word-length distribution, function-word
  share, and adjacent-word-similarity — all within the natural-language bands; CI asserts
  them. The in-world inscriptions read the same (still nonsense words) but now *cohere*
  under analysis. Golden-neutral (content-keyed; the golden default scene unaffected, or the
  one golden image that shows inscriptions updated once, noted).
- **De-risks.** The whole Archive direction: that "comprehension through structure" is
  *possible* — players can only find names/frames/cognates if the text has real structure to
  find. This is the no-regrets foundation; every later Archive milestone (cartouches,
  formulaic frames, proto-language, prosopography) assumes a statistically honest corpus.

## Scope

**In — make the generator pass the honesty checklist (each a unit test):**
1. **Zipf rank-frequency**: token frequency ∝ 1/rank^α, α ≈ 1.0 over a couple of decades —
   needs genuine high-frequency **function words**, not uniform sampling of roots.
2. **A function-word layer**: ~5–10 grammatical particles dominating the frequency top and
   sitting in **positionally constrained** slots (not bursty; uniformly spread).
3. **Heaps' law**: vocabulary grows sublinearly with corpus size (V ∝ N^β, β ≈ 0.5–0.8) —
   not "every word a hapax", not a tiny closed list.
4. **Character conditional entropy** in the natural band (~3–4 bits/char) — *not* the Voynich
   ~2 (too predictable) nor uniform-random (too high). Implies a real phonotactic grammar.
5. **Word-length distribution** unimodal but not narrow-binomial; **frequent words shorter**
   (Zipf's law of abbreviation).
6. **Consistent morphology**: a small affix inventory, **same affix = same string**, stable
   slot order — morpheme boundaries recoverable by segmentation (reward, don't punish, a
   player doing Morfessor-style analysis).
7. **Bursty content-words + uniform function-words**: content tokens cluster by topic/site
   over a span; particles spread evenly.
8. **No autocopy signature**: adjacent words not systematically more edit-distance-similar
   than distant words (the Timm–Schinner Voynich tell).
9. **No layout artifacts**: word statistics independent of line/paragraph position.

**Corpus-shape rules (cheap, high-leverage — from the material-text research):**
- The world's text includes some **longer connected strings** (not only ~5-glyph
  fragments — the Indus lesson: all-short makes structure undiscoverable).
- **Structured records**: name + logogram + numeral list forms (the Linear A lesson — these
  are partially "readable" structurally before any language is) — at least as a generable
  inscription *shape*.
- At least one **recurring fixed frame** with one varying slot (the libation-formula / Rites
  pattern) — the single most analysable thing a corpus can contain.

**Out:** the **proto-language / five-daughter-scripts** architecture (a real design
commitment + sound-change cascades — its own Archive milestone, likely with a human
fork-check); the **Leiden display grammar**, **cartouches**, **prosopography**, the
**sensing ladder** (later Archive milestones); any change to G9 transliteration semantics
(this is about the *lexicon*'s phrases, not block names — block names stay glyph per G12);
new engine capability.

## Design sketch

- `lexicon.rs`: introduce a small **phonotactic grammar** (e.g. (C)V(n) syllables over fixed
  consonant/vowel inventories), a **function-word set** (closed, high-frequency, positional),
  and a **morphology** (a few invariant affixes with fixed slots). Word choice = a topic/state
  process (so content-words burst) + uniform function-word insertion. Phrase/corpus assembly
  honours the corpus-shape rules. All deterministic in the seed (+ the existing per-cell hash).
- A `lexicon::stats` module computing the metrics above over a generated sample — used by
  both the tests and an optional `cargo run --bin lexstats` for inspection/tuning.
- Keep the **rendered output still nonsense words in the five scripts** (no English, no
  lore) — only the *statistical structure* changes. Existing inscription placement (E17/G9)
  and the ambient-vs-name distinction (G12) are unchanged.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Scope = statistical honesty + corpus-shape only.** Proto-language and the display/
   decipherment-mechanic milestones are deliberately *out* (they carry forks). *Pinned.*
2. **Test bands** are the natural-language ranges above; pick concrete thresholds with a
   tolerance (assert ranges, not exact values) so seed variation doesn't flake. *Pinned.*
3. **Golden:** keep inscriptions content-keyed / flag-gated so the golden default is
   unaffected; if one golden image shows inscriptions, update it once, noted. *Pinned.*

## Tests

- One test per checklist item (1–9) over a seeded sample corpus, asserting the metric's
  band (ranges + tolerance, deterministic per seed). A meta-test that a *deliberately broken*
  generator (e.g. uniform sampling) **fails** the Zipf/Heaps tests — proving the tests bite.
- Corpus-shape: long strings exist; a name+logogram+numeral record shape is generable; a
  recurring frame with one varying slot recurs verbatim across sites (n-gram repeat above
  shuffled baseline).
- Determinism: same seed → same corpus (share-link safe); golden voxel-hash unchanged;
  headless render unchanged (or the one inscription golden updated once, noted).
- CI green (fmt / clippy -D / tests / wasm); boundary intact; roadmap G16 entry.

## Risks & mitigations

- **Flaky statistical tests** → assert *bands with tolerance* on a fixed-seed sample, not
  exact values; size the sample so the metrics are stable. (Decision 2.)
- **Over-reach into Archive design** → Decision 1 fences the scope; proto-language/display
  are separate fork-gated milestones.
- **Determinism/share-links** → fully seed-derived; the E12 golden-hash + share-link
  reproducibility must hold (tested).

## Acceptance checklist

- [ ] The lexicon generator passes the 9-item statistical-honesty checklist — each a unit
      test asserting a natural-language band; a broken-generator meta-test fails them.
- [ ] Corpus-shape rules: some long strings, a name+logogram+numeral record shape, a
      recurring one-varying-slot frame.
- [ ] Output is still nonsense words in the five scripts (no English/lore); G9 names + the
      ambient/name distinction unchanged.
- [ ] Deterministic in seed (share-link safe); a `lexstats` query for tuning.
- [ ] Golden voxel-hash unchanged (inscription golden updated once if needed, noted);
      CI green (fmt / clippy -D / tests / wasm); boundary intact; roadmap G16.
