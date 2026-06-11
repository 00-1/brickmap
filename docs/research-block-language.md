# Research — language design for the block substrate

> 2026-06-11 research pass (web; HCI/PL literature, primary papers). What programming-
> language design, end-user-programming research, and tiny-language traditions teach the
> block console ([`game-system.md`](game-system.md)). Feeds [`game-depth.md`](game-depth.md)
> G12/G13+ and the G7+ console evolution. Sibling of
> [`research-automation-depth.md`](research-automation-depth.md).

## 1. The load-bearing empirical findings

- **Non-programmers think in events and aggregates, not loops** (Pane & Myers, Natural
  Programming/HANDS): 54% of natural descriptions of program behavior are event rules
  ("when X happens, Y"); **96.5% of multi-object operations treat the set as a whole**
  ("collect all the…") — explicit loops are rare; booleans are used inconsistently
  (natural "and" often means OR). HANDS' controlled study: queries+aggregates+visible data
  = 19 tasks solved vs 1 without. → **Demote `repeat`, promote richer triggers and
  aggregate parameters** (`collect(all)`, `match(each scanned)`); stacked filters
  implicitly intersect — never expose AND/OR; negation arrives late as its own found word
  (`avoid(…)` reads better than `not`).
- **The event/state confusion is real, measured, and interface-fixable** (Huang & Cakmak
  UbiComp 2015; Ur et al. CHI 2019 bug taxonomy — 10 classes incl. infinite loops,
  repeated triggering, flipped triggers): users systematically misread event triggers as
  states and vice versa because "if" covers both. The fix is **shape + grammar**: events
  read "ON …" (fire-flash feedback), states read "WHILE …" (live gauge). → This is
  *exactly* our `on-scan` vs `when` distinction — **consider renaming `when` → `while`**,
  give the two trigger kinds distinct console rendering, and show state-trigger values
  live on the block (a gauge). When two routines wake simultaneously, highlight both —
  the dominant real-world TAP bug class is rule interaction.
- **The ECA lineage** (HiPAC 1988): event (when to *consider*) vs condition (whether to
  *act*) vs action are deliberately separate; IFTTT collapsed event+condition into "if"
  and reaped the confusion. Our trigger + match-filter body already restores the split —
  keep it crisp.
- **The vocabulary problem** (Furnas et al. CACM 1987): two people name the same thing
  alike < 20% of the time — *guessable names are a lost cause*; but our palette is
  **recognition, not recall**, which sidesteps it entirely (Nielsen heuristic #6; Bau et
  al. "Blocks and Beyond"). What *does* matter (Landauer et al. 1983): **name-form must
  track role-form** — different syntax must never wear the same name shape. Every event
  trigger `on-X`, every state a bare condition, every parameterised action `verb(noun)`,
  across all five scripts. Congruent pairs (Carroll): inverse actions get lexical inverses
  (`land`/`ascend`).
- **Spreadsheets are the most successful EUP ever** (Nardi) because of task-specific
  high-level operations (SUM, not loops), visible concrete data, and **continuous feedback**
  ("twinkling lights"). → Every block is a SUM (world-task-named, never computational);
  the `continuous` trigger is the spreadsheet model — make its effects visibly twinkle on
  the console readouts (pairs with G11 telemetry).
- **Programming-by-demonstration died on inference** (Cypher's Eager; Lieberman
  post-mortems): generalizing from a demonstration is ambiguous and silently-inferred
  intent breaks trust; Eager's win was **anticipation highlighting** (showing what it
  expects *in the user's own modality*, verification for free). → For G12
  record-to-program: **record literally** (exact concrete blocks, zero inference); the
  *player* generalizes afterward via steppers — which is precisely Victor's
  "create by abstracting" (start concrete, generalize one step at a time). During routine
  playback, pre-highlight the agent's next target in the world (Eager's trick).
- **Victor's "Learnable Programming"**: read the vocabulary / follow the flow / see the
  state; "people understand what they can see." → The interpreter must be a *spectacle*:
  executing step highlighted while the ship visibly acts (G11 already plans this), live
  values on state blocks, found *routines* in the world as create-by-reacting material.

## 2. The tiny-language tradition (Forth / APL / Lisp / concatenative)

- **Forth** (Moore/Brodie): a program is a *vocabulary* grown toward the problem; factoring
  discipline ("a word should be a line long"); **the nameability test** — "if you can't
  assign a single name to a concept, it's not well-formed." → Our world-found vocabulary
  IS a Forth dictionary; the test for adding a new primitive block: *will it be reused
  across many routines?* If it'd appear once, it's a routine, not a word. Late-game
  routines should read like Moore one-liners made of named player routines.
- **Steele "Growing a Language"**: design a language that can grow; **user-defined words
  must look like primitives** ("no seams") — APL died partly because user code could never
  look first-class next to the glyphs. → G13's `run(routine)`-as-step + **player routines
  appearing in the palette as ordinary insertable blocks** is the no-seams requirement.
- **Iverson "Notation as a Tool of Thought"**: utility rises with range, falls with
  vocabulary size; economy comes from *grammatical rules for phrases over a small
  vocabulary*; suggestivity — forms from one problem suggest others; "harder to learn
  *because* suggestive" is depth, not a flaw. → Target ~15–25 block types with **one
  uniform parameterization grammar** so learning `scan(item)` pre-teaches
  `spend(faculty)`; orthogonality as content budget — N blocks × M argument values from
  N+M discoveries (the RISC lesson too: 10 instructions cover 80% of execution; compose,
  don't specialize).
- **Concatenative law** (Joy): any contiguous subsequence of a pipeline is itself a valid
  program — extraction/naming is cut-and-paste. The documented failure mode is **stack
  shuffling** (more than one implicit value in flight). → Preserve "any contiguous run of
  steps is liftable into a named routine" (G13), and pin: **exactly one implicit "current
  thing"** flows between blocks (the last-scanned/collected); a second implicit referent
  is forbidden — make it an explicit parameter instead.
- **Zachtronics/TIS-100**: a 13-instruction ISA framed as the *recovered manual of a dead
  machine* (Uncle Randy's annotated manual) is the value proposition, not a compromise;
  histograms over leaderboards; everything inspectable, single-steppable. → Our
  five-scripts-found-in-ruins IS the TIS-100 manual diffused into the world; lean into
  artifact-grade presentation over tutorials. (Patterson & Ditzel's design test transfers:
  for any candidate block — if it's rare and composable from primitives, what does
  *excluding* it cost?)

## 3. Block-environment craft (Scratch/Blockly literature)

- **Shape-as-syntax**: Scratch's C-blocks/ovals/hexagons make invalid programs
  *unrepresentable* — validation by physical impossibility, not error messages. Three
  shapes suffice for us: trigger (hat) / step (stack) / filter-param (inset). **Failsoft
  runtime** (Scratch swallows runtime errors and continues): a block that can't act does
  nothing and the routine continues — the dead machine shrugs, it never crashes. (G11's
  honest `blocked:` reason then *explains* the shrug — legibility without fail states.)
- **Blockly's folk lessons** (Fraser): conditionals/loops are the hard blocks (invest UI
  polish in triggers/match, not actions); conservative connection behavior; **code
  ownership** — players hate fill-in-the-blank; teach with whole working artifacts they
  copy and mutate (= our given routines + found routines).
- **Blocks' benefits are front-loaded** (Weintrop & Wilensky: gains diminish over time;
  viscosity is the cost): the notation's job is the first hour, the *domain's* job is the
  rest — depth must come from the world/economy, not block complexity. Anti-viscosity:
  single-press move/duplicate/swap of whole steps + "lift into routine" (G13).
- **Tinkerability** (Resnick): low floor / high ceiling / **wide walls**; "choose black
  boxes carefully" — what's primitive determines what players think *with* (`scan` as one
  block vs `detect`+`classify` decides the texture of play); a scratch-space of detached
  blocks on the console costs nothing and invites play.

## 4. Top-10 (ranked by leverage for us)

1. **Event vs state in shape + grammar** (`on-…` flash-hats vs `while-…` gauge-hats;
   consider renaming `when`); simultaneous-wake highlighting.
2. **Aggregates over repeat** — `collect(all)`/`each-scanned` parameters; implicit
   intersection of stacked filters; no AND/OR/NOT (negation as a late found word).
3. **One implicit register** — a single "current thing" between steps, never two.
4. **Execution as spectacle** (with G11): live step highlight, gauges on state blocks,
   Eager-style pre-highlight of the agent's next world target.
5. **Record literally, generalize manually** (G12 contract — kills the PBD trap).
6. **Player routines become palette words, no seams** (G13 + Forth/Steele).
7. **Failsoft interpreter semantics, named honestly** (with G11's blocked-reasons).
8. **Orthogonality as content budget** — uniform `verb(noun)` grammar; N+M finds → N×M
   power; the RISC exclusion test for new blocks.
9. **Name-form tracks role-form across all five scripts** (the one naming rule that
   matters; flavor/terseness is otherwise free under recognition).
10. **Whole-artifact teaching** — given + found routines to copy and mutate; never
    fill-in-the-blank; manual-as-artifact presentation.

## Sources (primary)

Pane & Myers IJHCS 2001 + HANDS CHI 2002 · Huang & Cakmak UbiComp 2015 · Ur et al. CHI
2014/2016 (224,590 recipes)/2019 (bug taxonomy) · AutoTap ICSE 2019 · HiPAC SIGMOD 1988 ·
Furnas et al. CACM 1987 · Landauer et al. CACM 1983 · Black & Moran CHI '82 · Carroll/
Rosenberg ACL 1984 · Nielsen heuristics · Nardi 1993 + Nardi & Miller · Cypher Eager
CHI '91 / Watch What I Do · Lieberman 2001 · Victor "Learnable Programming" 2012 ·
Tanimoto liveness L1–L6 · Moore POL 1970 + Brodie Thinking Forth (verified quotes) ·
Iverson Turing lecture 1980 · Steele "Growing a Language" 1998 · von Thun Joy papers ·
Factor docs · Resnick CACM 2009 / IDC 2005 / Tinkerability · Fraser "Ten Things… Blockly"
2015 · Maloney et al. TOCE 2010 · Weintrop & Wilensky TOCE 2017 · Bau et al. CACM 2017 ·
Meerbaum-Salant ITiCSE 2011 + Aivaloglou & Hermans ICER 2016 (the smell caveats) · Barth
TIS-100/SpaceChem interviews + postmortem · Patterson & Ditzel 1980 (verified text).
