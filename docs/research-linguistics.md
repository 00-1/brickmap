# Research — linguistics, writing systems & real decipherment (for scripts/lexicon/names)

> 2026-06-11 research pass (web). Question: what real linguistics, writing-system typology,
> and the *actual history of decipherment* can teach the five scripts/strata, the seeded
> lexicon, and the G9 block-name system. Feeds [`game-depth.md`](game-depth.md) and
> [`open-questions.md`](open-questions.md). Sibling of
> [`research-decipherment.md`](research-decipherment.md) (games) — this doc is the
> real-world counterpart. *(§1 complete; further sections land as the research pass
> finishes.)*

## 1. The great decipherments — how it actually worked

Verified case studies (Champollion/Egyptian, Kober+Ventris/Linear B, Knorozov+
Proskouriakoff/Maya, Hrozný/Hittite, Grotefend+Rawlinson/Old Persian). The headline for us
is the **recurring structure** — the same machine ran in every successful case, in nearly
the same order:

1. **A parallel text / bilingual anchor.** Rosetta's Greek; Behistun's trilingual; Landa's
   mangled Maya "alphabet" (a bilingual nobody recognized as one for 90 years). Linear B —
   the exception — lacked one and required the most purely internal analysis.
2. **Proper names as the first phonetic footholds** — names cross languages with their
   sounds roughly intact: Ptolemy/Cleopatra/Ramesses **in cartouches** (a visual marker
   that pre-segments the text!), Darius/Xerxes in the royal titulary, Knossos/Amnisos on
   the Knossos tablets.
3. **Distributional/structural analysis precedes phonetic guessing** — the methodological
   heart. Kober's "triplets" proved inflection and Ventris's grid fixed sign
   *relationships* (same consonant / same vowel) before any sound was assigned; the
   sign-count heuristic classified systems before anyone read a word (≲36–40 signs →
   alphabet, ~50–90 → syllabary, hundreds → logosyllabic).
4. **Formulaic text as crib** — Grotefend's "X, great king, king of kings, son of Y";
   Proskouriakoff's birth/accession date-glyph patterns at Piedras Negras (which proved
   the inscriptions recorded *history* — lifespans — not just astronomy).
5. **The known-language gamble** — decipherment closes only when the sound-skeleton binds
   to a language someone knows: Coptic for Champollion, Greek (against Ventris's own
   Etruscan expectation), Indo-European cognates for Hrozný (*watar* = water), Yucatec for
   Knorozov. Linear A and Indus resist because this step has nothing to bind to.
6. **Confirmation by prediction on fresh data** — Blegen's tripod tablet: Ventris's values
   applied to a newly excavated tablet read *ti-ri-po-de* beside a picture of tripods.
   Prediction-on-new-data converts a self-consistent scheme into an accepted decipherment.

Sociological coda: in every case the correct **mixed** (logosyllabic) reading was resisted
by an authority committed to a "pure" model (Young's foreigners-only phoneticism,
Thompson's pure ideography). Real scripts are deliberately mixed.

**Applications for us:**
- **The cartouche principle (high leverage, cheap).** Mark block names with a visual
  enclosure in every script — players learn to *spot* names before they can read anything,
  exactly mirroring real decipherment's first foothold. Retrofit to G9's name-bearing
  inscriptions: a thin frame/cartouche glyph pair around the transliterated name.
- **Formulae as cribs.** The lexicon should emit *recurring formulaic frames* (the
  epitaph/titulary register) so distribution-watching players get the Grotefend foothold:
  the same 3-glyph frame around varying contents = "that frame means something fixed."
- **The Kober/Ventris lesson** for any future hypothesis mechanic: reward *structural*
  observations (this sign alternates with that one) before/independently of meaning —
  e.g. the survey log auto-grouping inscriptions that share stems (already planned, P13 of
  the games research) is the real method, not a gamification of it.
- **Prediction-as-confirmation** (the Blegen beat): the strongest possible confirmation
  moment is using a derived reading on a *fresh* find and having it work — aligns exactly
  with the compositional-names/knowledge-gate fork (open-questions §A) where a player
  *derives* a block name they've never seen and the console accepts it.
- **Mixed scripts are realistic** — a stratum can mix logographic site-marks with
  phonetic name spellings without being "unfair"; purity is the ahistorical choice.

Sources (verified, confidence high unless noted): Britannica "Rosetta Stone" · Smithsonian
Magazine on Champollion (cartouche cross-check P-T-O-L; Ramesses 1822; Coptic) · Wikipedia
"Lettre à M. Dacier" / "Jean-François Champollion" / "Linear B" / "Michael Ventris" /
"Decipherment" (sign-count heuristic) / "Decipherment of cuneiform" / "Behistun
Inscription" / "Yuri Knorozov" · Cambridge Faculty of Classics "The Decipherment of Linear
B" (classics.cam.ac.uk PDF — Kober triplets, the grid) · thearchaeologist.org on Ventris
(place-name test; Blegen PY Ta 641) · National Geographic + RFE/RL on Knorozov/Thompson ·
SLUB Dresden on the Dresden Codex · Archaeology Magazine + AIA on Proskouriakoff (Piedras
Negras 1960) · Radio Prague International ×2 on Hrozný (the NINDA sentence, 24 Nov 1915)
· historyofinformation.com + CSMC Hamburg on Rawlinson. *(Method note: direct page fetches
were largely 403-blocked; claims rest on substantive search excerpts + the standard
literature — Pope, Robinson, Coe.)*

## 2. Why scripts resist — and the statistical honesty checklist for the lexicon

The undeciphered cases teach by failure. **Linear A** (~1,400 inscriptions, ~7,400 signs):
the signs are *readable* (shared with Linear B) but the language binds to nothing known —
and yet parts are functionally readable anyway: tablets are structured **name + commodity
logogram + numeral** lists, and a recurring **libation formula** appears across sites.
**Indus** (~4,500 objects but average text length ~5 signs, longest ~17): corpus *shape*
matters more than corpus size — texts too short for internal analysis; the
Farmer/Sproat-vs-Rao entropy fight shows even "is it language at all?" is contested.
**Rongorongo** (~26 objects; reverse boustrophedon — rotate the tablet each line!): the
only accepted structural identification is a *calendar* on one tablet. **Voynich**: passes
Zipf but fails deeper tests — character conditional entropy too low (~2 bits/char vs 3–4
for real language), word lengths too-narrowly binomial, rigid positional glyph grammar,
words suspiciously similar to nearby words (the Timm–Schinner "self-citation" generator
signature), line-position artifacts. *(Sources: Wikipedia Linear A/Rongorongo/Indus
script/Voynich; SigLA database paper; Rao et al. Science 2009 + Language Log/Sproat
rebuttals; Montemurro & Zanette PLOS ONE 2013; Timm & Schinner Cryptologia 2020;
voynich.nu; Altmann & Gerlach "Statistical laws in linguistics".)*

**Applications:**
- **The lexicon honesty checklist** — for player pattern-hunting to be *rewarded*, the
  generated corpus should pass what Voynich fails: (1) Zipf rank-frequency (~slope −1);
  (2) Heaps' law (sublinear vocabulary growth); (3) character conditional entropy in the
  3–4 bits/char band (not too predictable); (4) unimodal-but-not-tight word lengths +
  frequent-words-are-shorter (Zipf abbreviation); (5) a small consistent affix inventory
  with stable slot order (real morphology, not random syllables); (6) bursty topic-words +
  uniformly-spread function-words; (7) no adjacent-word similarity signature; (8) no
  line-position artifacts. An LCG-plus-syllable-table generator fails 2/5/6/7 first —
  those need actual machinery (finite morphology, a topic/state process, a global lexicon).
  *Practical: these are unit-testable properties of the generator.*
- **Corpus-shape rule:** inscriptions must include some *longer* texts (the Indus lesson —
  all-short-texts makes structure undiscoverable) and **structured lists** (the Linear A
  lesson — name+logogram+numeral tables are readable *before* the language is); duty
  rosters and inventories are both on-tone and mechanically right.
- **The libation-formula pattern** — one mostly-fixed ritual frame recurring across sites
  with one varying slot = the single most decipherable thing a corpus can contain. Give
  Rites (Hiragana) exactly this.

## 3. Typology — five scripts as five genuinely different puzzles

Daniels' six types (logosyllabary / syllabary / abjad / alphabet / abugida / featural)
each pose a structurally different decipherment problem: an **alphabet** is a small
substitution puzzle; an **abjad** underdetermines (no vowels — one skeleton, many words);
an **abugida** is compositional (each glyph factors into base consonant + vowel diacritic
— treating CV forms as atomic wildly inflates the apparent inventory); a **syllabary** is
a grid-completion puzzle (the Ventris method); a **logosyllabary** mixes phonetic signs,
logograms, and unpronounced **determinatives**; a **featural** script (Hangul — strokes
encode articulator features) lets a solver *predict unseen letters*. Determinatives are
the standout stealable mechanism: Egyptian classifiers and cuneiform DINGIR (𒀭 before any
deity name, unpronounced) let a reader **classify a word before reading it** — and, being
word-final/initial, double as word dividers. Layout is its own pre-puzzle (boustrophedon
mirroring, rongorongo's rotate-per-line). *(Sources: Daniels Blackwell Handbook chapter;
Goldwasser & Soler 2024 on Egyptian classifiers (Sage); Wikipedia Dingir/Abugida/Abjad/
Kangxi radicals/Origin of Hangul/Cherokee syllabary/Boustrophedon; Valério & Ferrara on
rebus/acrophony.)*

**Applications — a concrete five-stratum assignment** (refines the games-research P4):
- **Latin → Records:** plain alphabet, interpunct word dividers — the substitution on-ramp.
- **Greek → Schematics:** alphabet + **compositional technical compounds** (Greek is the
  real-world scientific-compounding language) — the morpheme/derivation stratum (feeds
  compositional block names).
- **Hiragana → Rites:** true syllabary + **formulaic liturgical frames** (the libation
  formula) — the grid-completion + formula-crib stratum; repetition is the clue.
- **Runic → Relics:** treat as an **abjad-like** terse lapidary register (vowel-poor,
  order-free epitaph formulas) + **determinative marks** for categories (relic-class
  classifier glyphs) — classify-before-reading.
- **Galactic → Signals:** degraded/noisy channel (the corpus-condition dial) + boustrophedon
  or rotation as the layout pre-puzzle — the script you must *re-orient* before reading.
- **Determinatives generalized:** a small set of unpronounced classifier glyphs across all
  strata (deity/place/machine/person) — cheap to generate, hugely legible to pattern-hunters,
  and a second "cartouche-like" foothold.

## 4. Morphology — the design recipe for compositional block names

The literature converges on a concrete recipe for a *guessable* constructed vocabulary
(directly serving the knowledge-gate fork, open-questions §A):

- **Strictly agglutinative**: one morpheme = one meaning, invariant form, fixed slots —
  exactly the property that let Kober detect inflection in an unreadable script from
  "triplets" alone. Players will be able to *Kober* the vocabulary by lining up names
  sharing roots and differing in one affix. (Fusional/portmanteau endings are the
  anti-pattern.)
- **Consistent right-headedness** (German/English/Sanskrit tatpuruṣa): MOVE+AGAIN = a kind
  of moving → *iterate*; the rightmost element fixes what the word IS. Reserve exocentric
  (bahuvrīhi) names for rare flavor — they're precisely the non-guessable ones.
- **Small root inventory, relentless recombination**: neoclassical/ISV scientific
  compounding shows ~30–60 roots + a linking convention covers thousands of concepts while
  staying learnable.
- **Sound symbolism for free first impressions**: bouba/kiki is robust across cultures
  (Ćwiek 2022, 25 languages, ~72%); Sapir's mil/mal size effect (~96%); /gl-/-type
  phonesthemes. Rounded vowels + sonorants for soft/large/slow blocks; voiceless stops +
  high front vowels for sharp/small/fast — most players guess polarity before learning a
  rule. (Beware collisions with real words — the Romanian *buba* lesson.)
- **Deliberate Zipf abbreviation**: most-used blocks get the shortest names, with a few
  eroded "irregular" short forms at the top of the frequency curve and a fully
  compositional long tail — the equilibrium real lexicons settle into; reads as
  verisimilitude, not inconsistency.

*(Sources: Wikipedia Morphological typology/Alice Kober; Snyder UConn compounds chapter
(Williams 1981 right-hand head rule); Wikipedia Sanskrit compound/Neoclassical compound;
Ćwiek et al. 2022 Phil Trans R Soc B; Sapir 1929 via Wikipedia Sound symbolism + Winter &
Perlman; Blasi et al. 2016 PNAS; Kanwal et al. 2017 Cognition (Zipf abbreviation);
Plag 2003 via semantic-transparency literature.)*

## 5. Registers, epitaphs & the conlang method

**The three-register system** of the real epigraphic record maps directly onto gameplay:
- **Monumental** (lapidary capitals, runestones, ḥtp-dj-nswt stelae): maximally formulaic —
  60–80% of a Roman epitaph is recoverable from the D.M. … H.S.E. … S.T.T.L. template;
  Viking runestones are near-universally "X raised this stone in memory of Y". Monuments
  are the player's crib: slot-and-filler frames where only names/numerals/kinship vary —
  teaching numbers and name-recognition almost free.
- **Graffiti** (Pompeii scratchings, the 30k+ Safaitic desert corpus with its "by So-and-so
  son of So-and-so" openings + grief notes): low-formula, vernacular, emotionally direct —
  and in a *different letterform register* (Vindolanda cursive vs monument capitals — a
  second letterform set for the same script as a puzzle layer).
- **Ledger/administrative** (the Sumerian lexical-list tradition — which is literally how
  Sumerian was recovered): tabular, repetitive, quantity-heavy — cracks units, commodities,
  taxonomies. Our Linear A-style name+logogram+numeral lists belong here.
- Registers gate each other: monuments yield names+formulas, ledgers nouns+numbers,
  graffiti verbs+feeling. **Curse formulae and appeals-to-the-living address the player
  directly** ("cursed be he that moves my bones") — a free crib *and* a diegetic warning
  system, perfect for relic interiors.

**The conlang method** (Tolkien/minimal-langs): derive everything from a small proto-root
set with regular sound changes (the *Etymologies* discipline) so etymology is *solvable*
— the five strata become **sister scripts of one proto-language with detectable cognates**
(a late-game epiphany: the strata were one people). Tengwar/Cirth model the
register split *inside* the fiction (penned vs carved forms). Toki Pona (~120 words) shows
a tiny compositional lexicon carries meaning through paraphrase; Esperanto's *mal-* prefix
shows one antonym-affix halves the vocabulary to learn. Partial player knowledge then
degrades gracefully into Dorian's "semi-speaker" experience — knowing words but not
speech, which *is* our player's diegetic condition.

*(Sources: Wikipedia Sit tibi terra levis / Jelling stones / A Secret Vice / The
Etymologies / Toki Pona / Esperanto vocabulary / Lexical lists / Nancy Dorian; Ashmolean
Latin Inscriptions teacher guide; OCIANA Safaitic corpus (OSU); CREWS Cambridge on
Vindolanda cursive; World History Encyclopedia on Pompeii; CSMC Hamburg on Sumerian
lexical lists; natmus.dk on Jelling.)*

## 6. The synthesis — one proto-language, five daughters, budgeted corpora

The full pass's three structural recommendations, refining §3's assignment:

1. **One proto-lexicon, five daughter forms (the Tolkien method, implementable).**
   Generate the seeded lexicon ONCE at the proto level; derive each stratum's surface
   forms by a per-stratum *ordered list of deterministic sound changes* (Records: \*k→k;
   Rites: \*k→s before front vowels; Relics: drop final vowels; Signals: unchanged — the
   proto preserved in the machine layer). Cheap (a rule cascade over phoneme strings),
   guarantees **detectable cognates** (proto `tarn` → `tarn`/`sarn`/`tar`/`tarn`), and
   turns five script skins into **one family tree the player can reconstruct** — the
   deepest possible endgame puzzle for ~5 rule lists. Bonus: Signals/Galactic being a
   trivially-cracked substitution cipher (as SGA always is — community decoders exist) is
   *fine*: the easy crack opens the hard comparative puzzle, because what it spells is the
   proto-language.
2. **Refined stratum assignment** (sharpens §3): Records/Latin = the readable-early
   anchor (the player's "Coptic"); Schematics/Greek = alphabet *used as an abjad*
   (engineers' vowel-dropped shorthand — reconstruct vowels from known names);
   Rites/Hiragana = syllabary with real inflection (Kober triplets, grid-buildable);
   Relics/Runic = **the Maya trap** — looks alphabetic, actually logo-morphemic (rune
   clusters = morphemes; a determinative rune marks artifact class; punishes the obvious
   prior, as Thompson's assumption did for decades); Signals/Galactic = cipher onto the
   proto. Steal **dual spellings** (Landa's accidental bilingual): occasionally render a
   block name logographically in one stratum and spelled out in another — a deliberate
   gift to attentive players.
3. **Budget corpus per stratum as the difficulty dial.** Linear B (~6 k texts) cracked;
   Linear A (~1.4 k, list-heavy) didn't; Indus (avg 4.6 signs/text) can't. A stratum meant
   to crack early needs volume + long connected text; one meant to stay mysterious gets
   Indus-like stats. And the **Linear A trap is a feature**: a stratum can be
   *phonetically solved but semantically opaque* mid-game (players pronounce Relics text
   via Runic letter values long before meanings arrive) — a real historical state that
   makes "readings firm up over time" natural.

*(Sources for §6: Wikipedia Languages constructed by Tolkien/Quenya; Tolkien Gateway;
Toki Pona (~120–137 words, compounding); Linear A/B corpora; Minecraft SGA decoders
(minecraft.wiki, dcode.fr); plus the §1–2 decipherment sources.)*
