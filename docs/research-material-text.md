# Research — material text: palimpsests, epigraphy, ruins & archives

> 2026-06-11 research pass (web; classics/archaeology/conservation/media-theory). The
> real-world practice and theory of reading dead civilizations' writing — mapped to
> mechanics, display conventions, content rules, and tone guards. The game's title is,
> pleasingly, a literal translation of *palimpsestos* — "scraped again." Feeds
> [`game-depth.md`](game-depth.md) and the fork-gated decipherment tranche
> ([`open-questions.md`](open-questions.md) §D). Sibling of
> [`research-linguistics.md`](research-linguistics.md).

## 1. The Leiden Conventions — a ready-made display grammar for uncertain readings

Agreed at Leiden in 1931 (refined Krummrey–Panciera 1980; digitized as EpiDoc/TEI), the
epigraphers' sigla distinguish **why** text is uncertain — three separate axes: *what
happened to the stone*, *what the ancient writer did wrong*, *what the editor did*:

| Siglum | Meaning |
|---|---|
| `[abc]` | lost to damage, **restored by the editor** (loss, not doubt — certainty lives elsewhere) |
| `[...]` / `[- - -]` | lacuna, length known (one dot per letter) / unknown — unrestored |
| `⟨abc⟩` | the **carver omitted** it; editor supplies (the *writer* failed) |
| `{abc}` | present but superfluous/erroneous — editor says ignore (dittography) |
| `(abc)` | **abbreviation expanded** — the most misread siglum: it means *certain*, not guessed |
| `⟦abc⟧` | **deliberately erased in antiquity** — damnatio; an *event* in the object's life |
| `ạ` underdot | traces survive but ambiguous — readable only from context (the perception layer) |
| `vacat` | deliberately blank — intentional silence is data |

**Application (the single highest-leverage humanities find):** adopt Leiden wholesale as
the survey log's display system. A logged inscription is never "37% translated" — it's
`⟦the third reactor⟧ was [never] finished v. forgive {forgive} us`. Each siglum maps to a
game state: `[…]` = needs condition/parallel-text; underdots = firms up on re-scan; `( )`
= the growing lexicon auto-expands formulae; **`⟦ ⟧` = someone erased this on purpose — an
entire quest category in one siglum.** The editor-voice never editorializes; it only
brackets — "lore implied, never read" gets typographic enforcement. (Also real: stoichedon
grid layout means a lacuna's letter-count is *known* — why `[..5..]` works.)

## 2. Palimpsests & the sensing ladder (the Archimedes case)

The real recovery ladder, each tier defeating the previous tier's failure mode — a
ready-made faculty/upgrade ladder: **(0)** raking/transmitted light (scraped strokes leave
grooves; an upside-down spolia inscription in a wall is a palimpsest readable with zero
tech — just attention); **(0.5)** chemical reagents — *one dramatic read, then permanent
damage* (Angelo Mai's tinctures; historically authentic destructive cheap tier);
**(1)** UV fluorescence (washed iron-gall text); **(2)** multispectral + computation
(12 bands, PCA — the Sinai project industrialized it: 74 palimpsests, ~305 erased texts,
incl. **Caucasian Albanian — a whole language existing only as undertext**, splitting
"sense it" from "read it"); **(3)** synchrotron XRF — reads iron ink *through* forged gold
overpaint, ~10 h/page; **(3.5)** **better math on old data** — the Aristotle commentary
recovered in 2009 from 2003 photons; re-processing archives is a free late-game power
spike. Key structural facts: erasure *method* gates recovery (washed = cheap tier,
scraped = expensive); one surface hides *many* unrelated undertexts (the Euchologion
buried seven books); and **damnatio memoriae is erasure that stays legible** — Geta's
chiseled-out name on the Arch of Septimius Severus is still readable *as a murder*: the
gap names its maker. De Quincey supplies the tone thesis (the mind as palimpsest:
"not one [layer] has been extinguished"); Genette the layer-relations typology (a later
inscription can quote, frame, critique, or parody the one beneath — not just bury it).

## 3. What dead civilizations actually write — the content-mix law

- **Epitaphs are ≥75% of all surviving Latin inscriptions**; at Oxyrhynchus (~500 k papyri
  from a *rubbish dump*) **~90% is documentary** — tax rolls, leases, petitions, letters.
  The lost literature is statistical noise; **the ledger is the signal.** → Hard corpus
  ratio: ~85% documentary/formulaic, ≤15% monumental; ban the expository diary — every
  text has an in-world addressee who is not the player.
- **Vindolanda** (the strongest single tone anchor): the birthday invitation where a
  scribe wrote the body and Claudia Severa added, in her own shakier hand, "farewell,
  sister, my dearest soul" — **officialdom plus one autograph deviation, detectable
  materially** (a hand-change, not a semantic flag); the "send more socks and underpants"
  note; the strength report (752 nominal, 265 fit — every man an absence or ailment);
  "the wretched little Britons" in one draft memo. Several hundred distinct hands over
  ~40 years.
- **Prosopography** grounds the attributable-hands mechanic: persons assembled from
  scattered attestations via name + title + date-window + place + associates + **hand**;
  Trismegistos' ratio — ~569 k attestations to ~375 k persons — means **most people in
  history are attested exactly once**; linking even two records (Masclus begging for beer
  ~AD 100, Masclus requesting leave years later) is the payoff. → Implement persons as
  **factoid graphs** (atomic, source-anchored, Leiden-confidence-carrying assertions) in
  the survey log; the player performs the join and grieves someone who exists only as a
  join across their own catalogue.
- **Formula grammar:** D.M. … vixit annis … S.T.T.L. — mostly frame, little filler; once a
  frame cracks, every instance becomes a name-extraction machine; abbreviations readable
  only after the full monumental form is known = "readings firm up" for free. **Curse
  formulae address the player directly** ("cursed be he that moves my bones") — free crib
  + diegetic warning system. Registers: monumental (the crib) / graffiti (different
  letterforms — Vindolanda cursive vs lapidary capitals; verbs, feeling, misspellings as
  phonology clues) / ledger (units, numbers, taxonomies — how Sumerian was actually
  recovered).
- **Dethlefsen & Deetz gravestone seriation** (death's-head → cherub → urn-and-willow,
  battleship curves, diffusion ~1 mile/year from Boston): a complete, historically real
  **dating minigame** — log motif frequencies per site, order time *without reading
  anything*, calibrate with the few dated stones. Pairs with palaeography (letterforms as
  strata — and the three-bar-sigma controversy as a built-in plot: a survey culture that
  over-trusts one letterform rule gets its history wrong).

## 4. Ruins, value conflicts & the right melancholy

- **Riegl's value matrix** (age-value / historical-value / use-value / newness-value /
  intentional-commemorative): every interesting monument beat is a *conflict between two
  values* — the inscription that demands restoration vs the lichen that forbids it; a
  perfectly preserved chamber in a dead world reads *uncanny* (newness-value as horror).
  Age-value is the one value legible to everyone with zero lore — the renderer carries it
  (erosion is information, never cleanable grime); the survey mechanics carry
  historical-value; the endgame question is Riegl's: arrest the decay or witness it?
- **Simmel's line:** a ruin is poignant only while the human intention is still legible —
  break colossi into 3–6 large *orientable* pieces whose original assembly is inferable;
  pure rubble is punctuation (`[- - -]`), not default. Ruins of *time* read differently
  from ruins of *violence* — choose per site. Nothing in the world maintains ruins for the
  player's benefit (no follies).
- **Wabi-sabi/kintsugi:** repair-as-visible-history → **repaired inscriptions in a
  different hand** (sometimes a different script — translation as repair): one stone, two
  dates, two hands, an act of care across generations, zero readable lore. Kenkō's rule:
  show *just before* and *long after*, never the civilization at its peak.
- **Boym's two nostalgias as the master tone guard:** the game is *reflective* (lingers on
  ruins and patina, dreams "futures that never took place"); *restorative* nostalgia
  (rebuild it, complete the record, "truth and tradition") is the temptation — or the
  antagonist ideology. **Hauntology names our exact register:** a spacefaring civilization
  is one that *announced* its future — the grief object is the **cancelled future**, so a
  large fraction of inscriptions should be forward-facing: launch manifests for launches
  that never happened, phase-two cornerstones, countdowns that counted to nothing. The
  Ozymandias triangle: inscriptions never mean what they meant; let texts boast and the
  terrain refute them.
- **The Caretaker** (Everywhere at the End of Time) is the production bible for
  decay-as-content: staged degradation (the same inscription re-encountered *worse* — the
  player's earlier log entry becomes the only surviving witness: the catalogue outlives
  its object); corrupted fragments of already-logged material resurfacing elsewhere,
  recognizable only because the player logged them; one terminal clarity beat.

## 5. Archives & conservation — the player's log as a designed artifact

- **Provenance vs subject** (respect des fonds, 1841; original order): records derive
  meaning from context-of-creation; rearranging by subject destroys evidence permanently.
  → The log's primary organization is **find-context** (site/stratum/adjacency — ground
  truth); subject views are derived indexes, visibly flagged as *the player's
  interpretation*. An inscription's neighbor is part of its content.
- **Jenkinson vs Schellenberg** (keeper vs selector) + **Derrida's Archive Fever** (the
  archive works against itself; "archivization produces as much as it records the event")
  give the role its weight: with finite attention, what you don't catalogue never existed
  — the catalogue is the machine that produces what the civilization *will have been*.
- **Conservation ethics as mechanics:** Venice Charter Art. 9 — restoration "must stop
  where conjecture begins" and additions "must bear a contemporary stamp" → reconstruction
  overlays render conjecture as unmistakable ghost-voxels, never blended (= Leiden `[ ]`
  for architecture). Wheeler: "at the best, excavation is destruction" + Valletta's
  preserve-in-situ default → collecting can degrade context; **even examination alters**
  (squeeze bans, cumulative lux — a rule that's part superstition, part real, for the
  player to distinguish); AIC's reversibility→**retreatability** ("don't foreclose the
  future") → interventions narrow the option set, never toggle cleanly. The scan-surrogate
  question (Nefertiti/Parthenon): is scanning care or extraction — and can the *scan*
  itself be hoarded? All periods are valid (anti-scrape): later graffiti and repairs on a
  monument are heritage too — "cleaning back to the original" destroys evidence.
- **Media regimes** (Kittler "media determine our situation"; dead media; digital decay):
  five scripts as five **survival physics** — stone (weathering, loses paint), cast metal
  (salvaged — *absence* as evidence), ceramic (shatters legibly), electronic (almost
  wholly dead; rare powered sites; format obsolescence = bits perfect, no interpreter —
  the cruelest failure), ephemeral paint/chalk (sheltered microclimates only). Survival
  bias becomes learnable world literacy; LOCKSS's poignant inverse — a redundancy system
  that worked until no one was left to run the audit. Menkman: each glitch should *reveal*
  the medium's workings, never be seasoning.

## Top-10 (ranked by leverage)

1. **Leiden Conventions verbatim** as the inscription display/state grammar.
2. **⟦Deliberate erasure⟧ as a first-class content category** (damnatio — the gap names
   its maker).
3. **The sensing ladder = the Archimedes history** (raking → UV → multispectral →
   penetrating; destructive cheap tier; re-process old scans as a late free win).
4. **The Vindolanda ratio + the autograph postscript** (~85% documentary; ban expository
   diaries; hand-changes as material emotion).
5. **Factoid-graph prosopography** for attributable hands (most people attested once;
   the join is the payoff).
6. **Formula-driven decoding** (10–20 abbreviations/script; frames auto-restore corpus-
   wide; curse formulae address the player).
7. **Five scripts = five media regimes** with different survival physics.
8. **Riegl/Simmel/kintsugi as art-direction law** (value conflicts; legibility-side ruins;
   visible repair in a different hand).
9. **Hauntological content slant** (cancelled-future texts) + Caretaker staged
   degradation (the log outlives its objects).
10. **Provenance-first survey log**; subject views as confessed interpretation; Venice
    Art. 9 ghost-overlays; examination-costs as the ethics-tree.

## Sources (condensed)

Leiden/EpiDoc: Wikipedia Leiden Conventions · GRBS/Dow conventions PDF · Ashmolean
conventions · EpiDoc Guidelines + Krummrey-Panciera appendix · Ancient Graffiti Project.
Squeezes/RTI: IAS (Tracy) · OSU epigraphy center · CHI/PTM (Malzbender) · Historic England
multi-light guidance · Krateros. Palimpsests: archimedespalimpsest.org · SLAC/SSRL
(Bergmann) · Sinai Palimpsests Project (~305 texts; Caucasian Albanian) · Vatican Library
on Mai · De Quincey Suspiria · Genette Palimpsests · Huyssen Present Pasts. Epitaphs/
prosopography: Oxford Handbook of Roman Epigraphy (≥75% figure) · ASGLE abbreviations ·
Wikipedia S.T.T.L./Tomb of Eurysaces ("her ashes are in this breadbasket") · Vindolanda
tablets (RIB TabVindol 291/346/154/628/892) · Trismegistos People · PIR/Zenon archive ·
Dethlefsen & Deetz 1966 + Deetz summaries. Ruins/theory: Riegl 1903 (Forster/Ghirardo) ·
Simmel 1911 · SEP Japanese Aesthetics (Kenkō, mono no aware) · Koren wabi-sabi · Wikipedia
Kintsugi · Boym Future of Nostalgia · Fisher Ghosts of My Life/Film Quarterly · Derrida
Spectres/Archive Fever · Wikipedia Everywhere at the End of Time · Macaulay Pleasure of
Ruins · ruin-porn critiques (Tate Ruin Lust). Conservation: Ruskin Seven Lamps §XVIII–XX
(verified) · SPAB Manifesto · Viollet-le-Duc Dictionnaire · Venice Charter Arts. 9/11/12/
15/16 (verified) · Nara Document 1994 · AIC Code 1994 · Wheeler 1954 · Valletta 1992 ·
Nefertiti/Parthenon scan sagas (Hyperallergic/Artnet). Media: Kittler GFT · Parikka ·
Ernst · Sterling Dead Media Manifesto · Kuny 1997 · Menkman Glitch Studies Manifesto.
