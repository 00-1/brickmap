# Open questions for the human (2026-06-11 session)

> The "answer later" doc: every fork I hit during the planning/dispatch run that's genuinely
> yours to call. I kept working past each one with a pinned conservative default (noted), so
> nothing here blocks the run — your answers redirect later milestones. Ordered by how much
> design they swing. The hardware/feel checklists stay in
> [`human-verification.md`](human-verification.md); this doc is *decisions*.

## A. The headline fork — knowledge-gates  ✅ ANSWERED (2026-06-11): superseded

**The human's call: block names stay UNREADABLE too** — no English layer at all. Blocks
render by their glyph names everywhere (world *and* console); function is learned by
clicking and observing (L0 of the ladder is the teacher; the vocabulary starts at 1–2
blocks so learning is gradual, one block at a time at acquisition). The readable-derivation
version of knowledge-gates is therefore moot. A glyph-composition verb (derive a compound
by picking morpheme glyphs you've learned *by function*) remains a possible Archive-tranche
layer — arguably stronger now — but is NOT needed; obvious-from-clicking suffices.

**Build implication (the de-Anglicization retrofit):** G9/G11 currently show English block
labels in the console and HUD ("NAME RECOVERED — `priority`"; routine rows). A milestone is
needed to render block identity as glyphs everywhere before more English surfaces accrete.
**Pinned scope default (veto if wrong):** *block names and name-derived strings* go glyph;
*structural/meta UI* (numbers, gauges, ×fires, the ▶ marker, state glosses like `waiting`)
stays minimal-English for usability — the machine's *diagnostics* read as instrumentation,
its *vocabulary* as a dead language. Full-glyph-everything is a later eye-pass call.

*(Original question kept below for the record.)*

The decipherment research's top recommendation
([`research-decipherment.md`](research-decipherment.md) P11+P6, the Tunic Holy-Cross trick):
make block names **compositional** (built from glyph morphemes — `MOVE`, `MOVE+AGAIN`,
`MOVE+NOT`) and make blocks **knowledge-gated, not flag-gated** — the console executes any
block whose name you can *compose* (via a glyph picker, no typing), even before you've found
it in the world. Finding a name teaches you it exists; deriving a name you've never seen and
having it *work* is the strongest emotion this design can produce, and it fuses the scripts,
the block system, and the theme into one mechanic.

**What I did instead:** G9 ships the conservative two-stage gate (found name → listed locked;
stratum decoded → insertable). It does not foreclose this — discovery data and per-block
names are exactly the substrate knowledge-gates would need.

**The question:** do you want to evolve toward knowledge-gates after G9/G10? It's a real
investment (morphemic name design across five scripts + a compose-a-name picker UI +
balance implications: a clever player skips the gating), and it partially *replaces* the
decode-unlocks economy rather than adding to it. My take: it's the most exciting idea in
the research, and the right time to decide is after you've played G9 — but it changes how
much we should invest in the decode-economy (G14) in the meantime, hence asking now.

## B. The shard model (G10 ships pinned defaults — veto any)

You decided: shards exist, auto-collectible, types + rarities. Four sub-forks I pinned:

1. **Are shards a sixth thing, or consolidation of the five strata?** Pinned: shards are
   their own currency, *typed by* the five domains (tinted to match), separate from strata
   data. Alternative: shards ARE chunks of strata (collecting a shard = strata points) —
   fewer currencies, but then `spend` competes with `decode` for the same pool.
2. **Spend model: one-time spend vs gradual fill?** Pinned: classic spend (bank N, buy a
   faculty level). Alternative: shards *fill* a faculty gradually as collected (no banking
   decision, more idle-ambient).
3. **Domain-matched costs?** Pinned: v1 spend takes the generic total; types exist to feed
   `match` filters. Alternative: faculties cost specific domains (sensing wants Rites
   shards…) — more decisions, more grind risk.
4. **Name readability** — ✅ superseded by §A's answer: names are never readable;
   the fork dissolves.

## C. How numeric should the upgrade layer be?  ✅ RESOLVED (2026-06-11) by the research-economy model

The human's "find words → research blocks → add shards to unlock/upgrade" reframe
(recorded in [`game-depth.md`](game-depth.md) G14) settles this: progression is
**vocabulary funded by shard-research**, not a +% layer; faculties fold into the same pipe
as ordinary research targets; numbers serve unlocking verbs. Forks B.1 (shards = research
substrate, not a sixth currency), B.2 (gradual fill, pacing TBD in prose), and B.3
(domain-matched research cost) converge here too. **One open sub-point: research pacing**
(allocate-and-fill vs bank-then-commit vs hybrid) — to decide in prose; default
allocate-and-fill.

--- original question kept below ---

### How numeric should the upgrade layer be? (superseded)

Both research packs converge on a warning: numeric +% layers (faculties, yields) are the
genre's hedonic treadmill, and Outer Wilds' lesson (P12) is that a *second* progression
currency dilutes comprehension-as-progress. But you explicitly want shards/spend, and a
light numeric layer gives the idle loop a pulse. **Pinned:** faculties stay small (3
faculties × 3 levels in G10) and all *further* progression is vocabulary/expressiveness
(new blocks, new argument options, new triggers). **The question:** is that cap right, or
do you want a bigger numeric layer (more faculties, deeper levels) accepting the
treadmill risk?

## D. The uncertainty layer (hypothesis states, erosion, behavioral confirmation)

The research's strongest *cheap* resonance wins (Sennaar/Heaven's Vault — P1/P3/P5/P7/P10):
found names/readings as **provisional hypotheses** (rendered degraded until confirmed by a
second sighting or first use); **eroded inscriptions** as partial data; the console giving
**in-script soft errors** instead of UI judgments; the survey log **re-rendering old finds**
as vocabulary grows ("this entry's reading changed"). None of this is built or dispatched —
it would slot in as a G-milestone after the economy tranche. **The question:** green-light
designing this tranche? It deepens the archival mood a lot for modest engineering, but it
adds friction to the currently-instant collect→know loop — a tone/pacing call that's yours.

## E. Script difficulty shapes (the five strata as five puzzles)

Today all five scripts are cosmetically different but structurally identical. The research
(P4/P17) says at most one should be a plain substitution (Latin/Records, the on-ramp), and
each other stratum should have one structural property a lookup table can't capture
(compositional Greek, formulaic Hiragana, order-free Runic, noise-degraded Galactic).
This mostly shapes the **lexicon generator + G9 transliteration** and only pays off if we
build D (and especially A). **The question:** worth committing to as the long-term shape?
(Cheap to start steering now, pointless if A/D are vetoed.)

## F. Touch overlay — per-button label styling (carried from the playtest)

D10 landed visible sliders/buttons with context labels. Your earlier note ("nothing visual
to see where to click" — addressed) left one styling fork: keep iterating label/icon
polish blind, or park it for your on-device eye-pass? **Pinned:** parked for the eye-pass;
it's pure feel.

## G. E9 god-rays — build blind or skip?

The one substantive buildable renderer item left from the backlog: a ¼-res post pass whose
entire deliverable is the look (unverifiable headless). **Pinned:** not dispatched; waiting
for you to either request it (builder does it blind, you tune) or drop it.

## H. Engine direction calls (no urgency, shaping the next engine tranche)

1. **E8 vertical chunk stacks** (multi-layer worlds — the one big deferred architectural
   step). The depth plan's "region conditions" content (P7) and taller giants/interiors
   would pair with it. When do you want this prioritized vs more game depth?
2. **E11 full dynamics substrate** (Margolus CA, pressure water, fire/heat field) — flowing
   water shipped in its simple form; the §J substrate is a real milestone. Steer on appetite.
3. **N1 multiplayer** — still parked on "needs a relay server + your hosting/accounts";
   the determinism groundwork keeps it cheap to start whenever. Unchanged.
4. **M8b profiling** — the budgets in [`performance.md`](performance.md) stay estimates
   until you run the reference iGPU/phone session; M10 (dispatching this run) will make
   that session a read-numbers-off-the-HUD exercise.

## I. New from the deep research pass (2026-06-11, second wave)

1. **The Archive tranche as a direction.** ✅ **ANSWERED (2026-06-11): green-lit, with a
   binding constraint — NO readable lore.** No letters-from-the-dead, no translated
   English sentences, no text dumps: inscriptions stay nonsense words (the seeded
   lexicon) in the five writing systems, forever. What the player "comprehends" is
   **structure, never prose**: this glyph-run is a *name* (cartouche) · this is a
   *formula frame* with a varying slot (a name / a count) · this was *deliberately
   erased* · this is the *same hand* as that repair · these two words are *cognates*.
   (Updated by §A's answer: block names are unreadable too — no readable layer at all; function is learned by clicking.) The Leiden
   brackets annotate glyph-text, not translations. The Vindolanda/mundane-text research
   survives as *registers* (ledger vs monument vs graffiti — recognizable by format,
   layout, and density, not by reading). Grief arrives through structure: hands,
   erasures, repairs, numbers, cancelled-future *formats* (a countdown grid, a manifest
   table) — never through sentences. Design docs to absorb this when the tranche briefs.
   *(Original question kept below for the record.)*

   **The original question.** The non-game research (real decipherment
   history, epigraphy, palimpsests, archival theory —
   [`research-linguistics.md`](research-linguistics.md),
   [`research-material-text.md`](research-material-text.md)) assembled into something
   bigger than §D's uncertainty layer: a Leiden bracket display grammar, cartouched
   names, formulaic-frame decoding, a palimpsest **sensing ladder**, attributable-hands
   prosopography, and **one proto-language with five daughter scripts** (cognates as the
   endgame puzzle). Coherent, mostly cheap per piece, deeply on-tone — but it's a
   *direction*, several milestones long, and it interacts with §A (knowledge-gates) and
   §B.4 (readability). **The question:** green-light the Archive direction as the
   post-economy tranche (after G14/G15), or keep decipherment minimal and spend that
   budget on more automation/economy depth? My take: this is the game's soul and the
   research makes it concretely buildable — but it's the biggest single commitment in the
   plan. (A lexicon v2 passing the statistical-honesty checklist is the no-regrets first
   step either way — pure CI-testable logic.)
2. **`when` → `while` rename.** The trigger-action studies (Huang & Cakmak; the CHI 2019
   bug taxonomy) show event/state confusion is the genre's #1 authoring bug and "when" is
   precisely the ambiguous word; the fix is distinct wording + shapes (`on-…` fires,
   `while-…` holds, rendered as a live gauge). **Pinned:** the shape/gauge distinction
   folds into G11/G12 regardless; the player-facing *rename* waits on your nod.
3. **Audio direction.** Lament-bass ostinato + per-stratum sound identity + **phone-mode
   virtual bass** ([`research-audio.md`](research-audio.md)) would turn the drone from a
   reactive pad into a composed grief signature — and fix the fact that our sub is
   inaudible on phone speakers. All buildable blind behind toggles, ear-tuned later — but
   it's the game's voice. **Pinned:** queued as a build-with-toggles milestone late in
   this run unless you'd rather hear the current drone first and steer.

## Resolved-by-default during this run (FYI, no action needed)

- M10 budget numbers: pinned loose (measured-actual +40% headroom), tightened at M8b.
- G10 economy numbers (costs 25/75/200, rarity 85/13/2, yields 1/3/9): placeholders for
  the feel pass — flagged in the brief, not worth your time until playable.
- The old [`unattended-questions.md`](unattended-questions.md) items: M8b → H4 here;
  build-blind targets → human-verification checklist 1; E10/E16/E17 → resolved (shipped);
  E13 flythrough refactor → still parked (protecting the only in-container verifier);
  E11 substrate → H2 here.
