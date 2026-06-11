# Open questions for the human (2026-06-11 session)

> The "answer later" doc: every fork I hit during the planning/dispatch run that's genuinely
> yours to call. I kept working past each one with a pinned conservative default (noted), so
> nothing here blocks the run — your answers redirect later milestones. Ordered by how much
> design they swing. The hardware/feel checklists stay in
> [`human-verification.md`](human-verification.md); this doc is *decisions*.

## A. The headline fork — knowledge-gates ("the console always listened")

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
4. **Name readability** (from the G9 side): pinned: a found block name is readable at
   collect time. Alternative: names render unreadable until the script's stratum is
   comprehended (stronger decipherment arc, slower opening). The hypothesis-state design
   (D below) is the richer version of this same instinct.

## C. How numeric should the upgrade layer be?

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

## Resolved-by-default during this run (FYI, no action needed)

- M10 budget numbers: pinned loose (measured-actual +40% headroom), tightened at M8b.
- G10 economy numbers (costs 25/75/200, rarity 85/13/2, yields 1/3/9): placeholders for
  the feel pass — flagged in the brief, not worth your time until playable.
- The old [`unattended-questions.md`](unattended-questions.md) items: M8b → H4 here;
  build-blind targets → human-verification checklist 1; E10/E16/E17 → resolved (shipped);
  E13 flythrough refactor → still parked (protecting the only in-container verifier);
  E11 substrate → H2 here.
