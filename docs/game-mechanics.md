# Scraped Again — Core game mechanics (design)

> *Scraped Again* is the **game**; **brickmap** is the **engine** it's built on. The
> title is the plain-English sense of *palimpsest* ("scraped again") — a dead,
> overwritten world you read back into legibility, where the endless re-seeded world
> is itself one more scraping. (Workspace crate `scraped-again`; see
> [`milestones/M9-engine-game-split.md`](milestones/M9-engine-game-split.md).)
>
> **Core architecture:** the progression / management / automation system is a single
> **block substrate** (the "tech tree" reframed) — see [`game-system.md`](game-system.md).
> Where this doc says "Auto-Collect upgrade", "the tech tree", or "scan tiers", read those
> as **blocks you compose**; `game-system.md` is authoritative on how they work.
>
> **Status: living design.** The direction below is **decided** (2026-06): a
> **melancholy-archival exploration game** built on the brickmap engine, with an
> **autopilot-as-idle data-collection loop feeding a deep tech tree of
> comprehension**. The engine/game *separation* that gives this a home has **landed**
> ([`milestones/M9-engine-game-split.md`](milestones/M9-engine-game-split.md) ✅ — the
> game now lives in its own `scraped-again` crate); this doc is the **game's** design
> and will keep evolving. The tree's
> exact node lists here are **illustrative and expandable** — the structure is the
> commitment, the specific nodes are not.
>
> *(Historical note: an earlier draft weighed three directions and recommended a
> no-fail "cartographer/decipherer" loop. That decoding idea **survives** — it is now
> the Decipherment branch of the tree, §8 — but the spine is now the autopilot/data/
> tech-tree loop below.)*

## 1. The pitch

You are the last surveyor of a dead machine-world — a patient automaton drifting an
endless, doom-quiet ruin of **fallen mechanical giants** and **ancient inscriptions**.
You don't conquer it; you **understand** it. **Autopilot is a real way to play**: the
cruiser drifts on its own, and the world streaming past is *read* — glowing glyphs and
the corpses of giants logged as **data**. You spend that data on a vast, intricate
**tech tree** — not weapons, but **faculties of comprehension and reach**: sharper
senses, a ship that learns to seek, the slow **decipherment** of a lost language until
the dead world's writing becomes legible and tells you what happened to it. **Manual
flight and walking** are the high-agency mode — targeted runs to a specific giant, down
a cave, into a pristine pocket — for the rare data and the keys autopilot can't sweep.

The mood stays **melancholy**: numbers go up quietly, in service of mourning-by-
understanding. The endpoint isn't power. It's knowing what was lost.

### Three ways to play (one economy, three experiences)

The same strata economy and the same tech tree underlie all three, but **what you can
reach and which branches pay off depend on how you engage** — so each mode is a genuinely
different game, and you can live in one, or blend them.

- **Autopilot — the *management* game.** Playable **entirely through menus**: manage
  strata, buy upgrades, set routes, and watch the ship scan / auto-collect / seek. **The
  whole core completes hands-off** (the tree *and* the decipherment payoff). Its branches:
  **Memory** (storage / indexing / auto-collect / auto-route) + **Sensing** (scan). The
  *abstract, distant* register of the mourning.
- **Manual flight — the *navigation* game.** Take the stick: choose headings, reach
  off-corridor, make precise approaches, collect from the air. Its branch: **Locomotion
  (flight)** — handling, ceiling, a cruiser-mounted beam. Finds the autopilot corridor
  never sweeps.
- **Walking — the *exploration* game** (the richest divergence). Land and go on foot:
  interiors, cave depths, climbing solid colossi, the survey-beam as a traversal rail,
  reading inscriptions up close. Its branch: **Locomotion (foot)** + beam-rail traversal +
  on-foot collection. The **deepest, most resonant lore lives here**. The *intimate,
  on-the-ground* register.

The load-bearing rule: **the core spine completes on autopilot; flight and walking are
*additional, parallel* experiences and upgrade paths — never *gates* on the core.** A pure
idler, a pilot, a walker, or any mix is a complete way to play.

## 2. What we already have to build with

The world has, live and on-aesthetic (see [`roadmap.md`](roadmap.md)):

- **Fallen colossi** (E18) — seed-placed giants; ethereal point-clouds you drift
  through *and* solid voxel bodies you land on and walk. Ancient *mechanical* tube-tech
  relics + toppled human figures. → the **machines whose tech you reassemble.**
- **Inscriptions & monuments** (E17) — abstract writing scattered on the ground and
  labelling each colossus, in **five writing systems** (Latin, Greek, Hiragana,
  Standard Galactic, Runic), glowing and lo-fi. → the **collectibles / data.**
- **Pristine pockets** — rare places with a **choral pad** (relief from the drone) and
  a map icon. → the **high-value sites + the game's quiet payoff beats.**
- **A per-seed doom dirge** (E16) that swells as you near a giant. → a **direction
  sense** the Resonance branch turns into a mechanic.
- **Autopilot + manual flight + on-foot walking** (E19, D3). → the **two play modes:
  idle survey vs. targeted expedition.**
- **An explored map** (E10) that fills in as you travel. → the **survey display.**
- **Shareable seeds** (E12) + **the edit/event seam** (E14) + the headless **render-to-
  texture**. → **save/share = seed + a sparse progress log**, and **codex thumbnails**.
- **Biomes** that crossfade; **sim/edit/particles**. → texture and future systemic depth.

Almost nothing in the loop below needs new *render* tech. The one genuinely new system
is the **tech-tree + codex UI** (§12).

## 3. Design pillars

1. **Comprehension, not conquest.** Every upgrade is a faculty of *understanding or
   reach*. No combat, no health, no score popups. The tree's terminus is knowing, not
   winning.
2. **Autopilot is a complete way to play — not a punished one.** The passive layer is
   genuinely rewarding and time-respecting (drift, tab away, return); an autopilot-only
   run still climbs the tree and earns the decipherment payoff. Manual flight and walking
   are **additive, never gates** — both a *multiplier* on the shared economy *and* their
   own distinct experiences + upgrade paths (the three modes of §1), never the only path
   to core progress.
3. **Stay cheap and on-brand.** Weak-hardware-first (design §4) + "expose the tech"
   (§11). Reuse the splat path, world-text, map, audio, and event seam; add ~no render
   cost.
4. **No-fail, melancholy.** The world is grief, not threat. The only "sink" is the tree
   itself. Pacing pressure, if any, is gentle (idle storage caps), never loss.
5. **Determinism-friendly, multiplayer-shaped.** State = seed + a sparse progress log
   (collected glyphs + tree state + decoded scripts). No world syncing; a shared
   **archive** is the natural co-op layer (roadmap N1).

## 4. The core loop

```
        ┌──────────────────────── AUTOPILOT (idle / broad) ───────────────────────┐
        │  drift the endless world ▸ the ship auto-SCANS what it passes → fills    │
        │  the map ▸ (unlocked) auto-COLLECTS the common layer into a buffer ▸     │
        │  and learns to seek dense/rare/undiscovered sites (Locomotion tech)      │
        └───────────────┬──────────────────────────────────────────────────────────┘
                        │ data accrues
                        ▼
        ┌──────── THE TECH TREE (spend data on faculties of comprehension) ────────┐
        │  Sensing · Decipherment · Locomotion · Resonance · Memory                │
        │  → better sweep, legible scripts, autonomous autopilot, pristine sense,  │
        │    bigger buffers — the passive layer improves *itself*                  │
        └───────────────┬──────────────────────────────────────────────────────────┘
                        │ unlocks reach + reveals where the rare things are
                        ▼
        ┌──────── MANUAL / FOOT (active / targeted / high-yield) ──────────────────┐
        │  take the stick ▸ fly/walk to a *specific* giant, cave, pristine pocket  │
        │  ▸ grab optional rares + speed up decoding + read interiors it skips     │
        └───────────────┬──────────────────────────────────────────────────────────┘
                        │ rare data + keys
                        ▼
        ┌──────── DECIPHERMENT PAYOFF (the melancholy) ────────────────────────────┐
        │  a script becomes legible ▸ inscriptions render *translated* ▸ a fragment │
        │  of what killed the world surfaces ▸ and that script's yield rises        │
        └───────────────────────────────────────────────────────────────────────────┘
                        ↺ set autopilot, drift on, deepen
```

## 5. The resource model — five scripts, five data strata

The five E17 scripts become five **data strata**, roughly tiered by rarity. The player
never sees "Latin/Greek" — they see distinct glyph-systems; we map script → stratum
internally. A collected glyph yields its stratum's data (amount scaled by word length /
source rarity). Strata are **typed currencies**, so the tree branches naturally.

| Script (engine) | In-world feel | Data stratum | Mainly feeds | Rarity |
|---|---|---|---|---|
| Latin | mundane labels / signage | **Records** (survey logs) | Sensing, Memory (baseline) | common |
| Greek | technical / engineering marks | **Schematics** | Locomotion, Sensing | uncommon |
| Hiragana | soft / ritual inscriptions | **Rites** | Resonance | uncommon |
| Runic | ancient grave-marks on the giants | **Relics** (deep lore) | Decipherment, foundations | rare |
| Standard Galactic | alien / non-human signal | **Signals** | Resonance (strange), late-game | rarest |

This is the roguelike "letters are items" idea unbound from single glyphs: **script =
category/rarity, the word = the specific sample.** And it makes all five scripts
mechanically load-bearing.

## 6. Collection & the survey-beam

There are **two tiers of information**, made visible as **two beams** (both drawn
post-palette so they read as vivid against the muted world — distinguished by colour):

The cruiser's beams fire into a **zone just ahead of the ship (in view)** — *not* a
radius you'd never see. So the idle layer is **on-screen**, your **heading** decides what
gets read (and "the ship learns to seek" steers things into that forward cone), and the
harvest is **naturally bounded** to your flight corridor — no artificial "you may not take
this" rule needed.

- **Scan (auto, the cruiser, a cool colour).** As you drift, the ship **automatically
  fires short-lived scan beams** at what's ahead — they **update the map** (a thing
  *exists / what kind* it is), but **don't harvest** it. The idle layer made diegetic: you
  *see* the ship reading the world, and the map fills as you go. *What the scan can detect*
  grows with Sensing tech (terrain/biome → inscriptions → ethereal colossi → buried/cave
  finds → rare-stratum hints). The map becomes a live **opportunity surface** (scan pins
  *uncollected* sites; you triage).
- **Collect (the warm beam).** Your survey-beam (below) **extracts the data/lore** into
  the strata + codex. Scan tells you *where*; collection takes *the thing*.
- **Auto-collect (a *composed routine*, not a toggle).** Auto-collection isn't a hardcoded
  upgrade — it's the block routine `on-scan → if matches(…) → collect` you assemble in the
  console ([`game-system.md`](game-system.md)). The cruiser then auto-fires the collection
  beam at the forward zone; you choose what it grabs via the filter, and budget/priority
  blocks tune it. It removes the *chore*, not the *expedition*.

**Autopilot is a complete way to play — not a punished one.** Manual is a **multiplier,
not a gate**: it's *faster*, it reaches the corridor's edges, and it gets the **optional**
depth (interiors, pristine pockets, specific giants, "Rosetta" finds that *accelerate* a
script). An **autopilot-only run still climbs the tree and still earns the decipherment
payoff** — just at a relaxed pace, missing optional lore. The only asymmetry is
**physical, not punitive**: some content simply *lives* where autopilot doesn't go (inside
a cave, deep in a solid colossus); skipping it costs *optional lore and a slower curve*,
never *blocked progress*. Crucially, **decipherment is reachable on autopilot** — fed by
the data you scan-collect of each script — with manual keys *speeding it up*, not
unlocking it.

- **What manual still uniquely reaches** (optional depth, not mandatory):
  - **interiors** — glyphs inside caves / solid-colossus bodies (foot collision + the
    descent/hull tech, E19 follow-ups),
  - **pristine-pocket** inscriptions (Rites/Signals + the choral beat),
  - a **named colossus's** monument inscription up close (E17 `colossus_label`),
  - anything **off your flight corridor** the forward beams never swept.
- **The codex.** Every collected glyph is recorded in a growing **archive** (catalogue +
  a headless-RTT thumbnail per find). Collection pays out twice: **quantity** (feeds the
  tree) and **understanding** (fills the archive — the melancholy payoff).

### The survey-beam — the active verb (collection *and* traversal)

Manual play's signature move, and the cheapest, most on-theme way to do both jobs at
once. This is the **warm collection beam** (distinct from the cruiser's cool auto-**scan**
beam, §6). You start with a basic one (it's never gated behind the tree); the tree only
deepens it — and Memory's **Auto-Collect** later lets the cruiser fire it for you.

**What it is.** A straight, solid, vivid **light/energy beam** you cast from the player
toward an aimed point (the E14 DDA pick gives the aim). It is **heavy and deliberate** —
a ponderous instrument, not a snappy zap — and once cast it **persists in space for a
while**, then **fades** until it stops working.

**It does two things with one cast:**
- **Collects along its whole path.** On cast, every glyph the line passes through is
  harvested at once (a one-shot sweep) — so you line shots up *through* clusters of
  inscriptions, and *through* the **ethereal** drift-through colossi (an energy beam
  reads what you can't stand on — this is how the untouchable giants get collected).
- **Becomes a temporary rail.** While it's up, you **attach to it and move along its
  single axis** (1 DoF — not walking a catwalk; a rail you clip onto), at **any angle**:
  vertical = an ascent out of a pit, diagonal = a climb up a cliff, horizontal = a
  crossing. This is the answer to *"you get stuck too easily"* — fire a ramp, ride it.

**Lifespan is the reach budget.** Your reach is how far you get before the beam dies.
Still attached when it expires → you **drop** from that point (walker gravity; a gentle
consequence, not a fail — and you can **fire-and-attach mid-fall to save yourself**). You
can also **detach on purpose** to drop precisely onto a ledge or a glyph cluster.

**Multi-segment routes.** With more than one beam up at once (an upgrade — see below),
you lay out a **path through space** and ride it segment to segment, racing each one's
fade. Chaining beams *is* the skill expression and the tension.

**It's the universal interaction verb.** Beyond collect-and-traverse, the beam is *how
you reach out and engage* — its **contact** triggers a context action depending on what
it touches: a glyph → collect, the **cruiser → board it**, and (extensible) relic
anchors / colossus doors / data-cores later. One verb, point the light.

**Boarding the cruiser (the save).** Touch the beam to your parked cruiser and you're
**rapidly reeled along it to the ship and board on arrival** (zip-then-board — consistent
with the rail, not a teleport). This plugs straight into the **E19 enter/exit mode
machine** as a *ranged* alternative to walk-up-and-press-E. Two rules make it feel right:
- **Light lock-on** — aiming roughly at the parked ship snaps the beam to it, so a clutch
  mid-fall recall doesn't need a pixel-perfect shot (the one moment forgiveness matters).
- **Reach-gated** — recall only works if the cruiser is within beam reach, so wandering
  far on foot has consequence (you drop and walk, no-fail); **Length/Lifespan** extend how
  far you can recall from. The headline emergent beat: **falling → beam the distant ship →
  the winch hauls you home** — relief, the light pulling you back from the dark.

**Hailing the ship.** Because the cruiser can be **off flying its own routine** while you
walk (two agents run independently — [`game-system.md`](game-system.md) §7), it may not be
parked within beam reach. So you can **hail** it — recall the autonomous ship to your
position, where the beam-board then takes over. (Hail is itself a block: clickable by hand,
or wired — e.g. `when on-foot buffer full → hail`.)

**Rendering (note for the engine).** The beam draws **after the palette post-process**,
so it keeps its **raw vivid colour** — the one thing in frame *not* mapped onto the
world's muted ramp, reading as artificial / yours / technology cutting through the murk
(thematically: comprehension is the one vivid thing in a desaturated grave). To still
feel *in* the world (occluded by terrain it passes behind, riding its true path), it must
be **depth-tested against the scene** even though it draws post-palette — i.e. the engine
keeps scene depth through the post chain and exposes a **post-palette, depth-aware,
non-palettised emissive overlay** draw. That's a small new engine capability the
engine/game seam should own (flagged in [M9](milestones/M9-engine-game-split.md)); it
also won't get the scene's pre-palette bloom, so it wants its own cheap glow.

**Cost.** Cheap: one emissive segment, a lifespan timer + fade, a line-vs-glyph
intersection on cast (a handful of nearby inscriptions), and a 1-D parametric
attach/slide for the walker. No rope/swing physics. Lives in the `scraped-again` crate over
engine primitives (DDA pick, the `solid` oracle, the walker).

## 7. Pressure / failure

**None.** No death, no threat. Data is always spent *toward comprehension*, so the tree
is the sink. The only pacing lever is the **idle storage cap** (Memory tech) — a *gentle*
reason to return and process, never a punishment. If playtesting wants more pull, the old
"pilgrim" idea returns as flavour, not threat: the cruiser's drift sips a charge
replenished at **pristine pockets**, making them rhythmic waypoints rather than fail
states. Default: keep it pure.

## 8. The tech tree — faculties of comprehension

> **Reframed (see [`game-system.md`](game-system.md)):** the "tech tree" is really the
> **library of blocks** you recover — action-blocks, control/meta-blocks, and spend-targets
> (upgrades). The branches below are the *domains* that vocabulary spans; read each "node"
> as a block or upgrade you unlock and then **compose**. `game-system.md` owns the how.

Five branches. Nodes are **illustrative** (the structure is the commitment); each branch
is meant to deepen "to the extreme" over time, terminating in a long, expensive
late-game arc. **Every node is comprehension or reach — never force.**

The branches **lean toward the three playstyles** (§1): **Memory + Sensing** ↔ the
autopilot/management game; **Locomotion (flight)** ↔ manual flight; **Locomotion (foot) +
the survey-beam** ↔ walking. **Decipherment** and the strata are shared by all. The
autopilot-completable core lives in the shared + Memory/Sensing nodes; the flight and foot
nodes are the *other two games' own* paths.

### 8.1 Sensing — *the scan beam: what the ship can read* &nbsp;(feeds: Records, Schematics)
The cruiser's auto-scan (§6) and what it reveals on the map.
- **Scan Range / Rate I–V** — widen and quicken the auto-scan as you drift.
- **Scan: Inscriptions** — pick up glyphs/monuments as sites on the map (not just
  terrain/biome, which the map shows from the start).
- **Deep Scan** — detect *buried / sub-surface* finds (cave inscriptions from above).
- **Spectral Sight** — reveal **ethereal** colossi you'd otherwise drift through unseen.
- **Stratum Hints** — the scan flags *which strata* a pinned site likely holds.
- **Cartograph I–III** — map resolution; pin *uncollected* sites (the opportunity surface).

### 8.2 Decipherment — *the lore spine* &nbsp;(feeds: each script's own data + Relics)
- **Legibility: Records → Schematics → Rites → Relics → Signals** — five unlocks bought
  with **that script's collected data** (which autopilot gathers too, so legibility is
  reachable hands-off); an unlocked script renders **translated** instead of glowing
  nonsense. Manual **"Rosetta" finds** *accelerate* a script's legibility — they don't gate it.
- **Fluency I–III** (per script) — *progressive* legibility: first fragments resolve,
  then full phrases. Watching the world become readable **is** the progression curve.
- **Cross-Reference** — auto-translate newly found glyphs of an already-known script.
- **Concordance** (late) — synthesise scattered fragments into the world's actual
  history: what the giants were, why they fell, what killed the place. The terminal
  lore. *Effect, beyond lore:* a deciphered script's **yield rises** (you extract more
  once you understand it) — a multiplier with a diegetic reason.

### 8.3 Locomotion — *reach* &nbsp;(feeds: Schematics, Relics) — the *flight* and *foot* games' own paths

*Autopilot (shared with Memory):*
- **Ship nav blocks** — autopilot *is* a composed nav routine ([`game-system.md`](game-system.md)):
  the default is `drift`; unlocking `seek(criteria)` / `survey(region)` / `route(sites)` is
  how *"the ship learns to seek"* (route toward dense / rare / undiscovered sites). High-level
  maneuvers over map areas/targets, not raw direction.

*Manual flight:*
- **Drift Efficiency / Range** — go further between stops.
- **Handling / Banking** — responsive piloting for precise approaches and off-corridor reach.
- **Atmospheric Ceiling** — altitude to reach high giants and overview vantage.
- **Cruiser-mounted beam** — collect from the air without landing.

*Walking (the survey-beam is the foot game's core verb):*
- **Survey-beam: Lifespan I–V** — the beam lasts longer before it fades (the reach
  budget; see §6).
- **Survey-beam: Capacity I–III** — how many beams you can hold up *at once* → longer
  multi-segment routes through space.
- **Survey-beam: Length / Reel Speed / Re-cast** — cast farther, ride faster, fire again
  sooner (candidates; tune on play).
- **Descent Rig** — safely enter **caves** on foot (consumes the E19 foot-collision
  follow-up).
- **Hull Attunement** — walk **solid-colossus interiors** (the explorable giants).

### 8.4 Resonance — *the strange / audio / pristine branch* &nbsp;(feeds: Rites, Signals)
- **Drone Attunement** — the dirge's swell-toward-significance (E16) becomes a usable
  **direction sense**: you can *feel* where something important lies.
- **Pristine Sense** — detect pristine pockets at range; **Pocket Resonance** — harvest
  them for **Signals/Rites** and the choral relief beat.
- **Echo** — locate distant *ethereal* colossi by their resonance.

### 8.5 Memory — *the idle spine + meta* &nbsp;(feeds: Records, all)
- **Storage I–V** — how much ambient data the buffer holds while you drift / are away
  (the idle-accrual cap).
- **Indexing** — offline/unattended accrual efficiency (data builds while the app drifts
  on autopilot with no input).
- **Auto-Collect I–III** — the cruiser begins **auto-firing *collection* beams** (the
  warm colour) into the forward zone to harvest as you pass — the idle harvest, visible on
  screen. It takes the **common layer in your flight corridor**; off-route and deep finds
  you steer to (manually or by routing). Tiers widen its range / what it grabs — so even a
  hands-off run keeps progressing.
- **Auto-Route** — chain autopilot between known (scanned) sites so the ship runs its own
  survey circuit.
- **Synthesis** (late) — convert accumulated understanding into the most advanced
  faculties: the **"to the extreme"** terminus — e.g. perceive the *whole* dead
  network's shape, time-lapse the ruin, translate the giants' final transmissions. A
  long, branching, deliberately expensive arc.

**Depth, honestly.** "A tech tree to the extreme" is the biggest design+balance
commitment in the game — getting an idle/active economy to *feel* good is its own
discipline. So the build plan (§13) is: **a small, tuned v1 tree that proves the loop**,
then expand each branch outward toward the late-game arcs. We design the *shape* fully
now; we flesh nodes as we climb, exactly like the roadmap's milestone philosophy.

## 9. Decipherment as the heart

This is where melancholy and "numbers go up" reconcile. Scripts start as glowing,
beautiful nonsense. Spending data buys **legibility**, and inscriptions begin rendering
**translated** (the E17 path already chooses glyph strings — legibility just swaps
glyph → meaning, progressively). What surfaces is fragmentary and elegiac: epitaphs,
warnings, the names of giants, the shape of an ending. The reward for optimising is
**understanding the grief** — the tree's whole point.

## 10. Session shape

Drift on autopilot; the map fills and data trickles; dip into the tree; *optionally* take
the stick for a targeted run to a giant / cave / pristine pocket for optional rares and to
hurry a script along; watch a script turn legible and read what the dead left; set
autopilot; drift on. Endless, calm, deepening — and just as valid never touching the stick.
A session can be five minutes of processing the buffer or an hour of expeditions.

## 11. Save / share

State = **seed + a sparse progress log** (collected-glyph set + tree state + decoded
scripts + codex notes) — the *same* `seed + sparse deltas` artifact E12/E14 already use.
So progress permalinks, shares, and slots into multiplayer as a **shared archive** with
no new persistence model.

## 12. How it maps to existing systems

| Mechanic | Built on | Genuinely new |
|---|---|---|
| Cruiser auto-scan (cool beam → map) | D3 auto-fly + E10 map + seed placement | short-lived scan beams; scan-category gating |
| Auto-collect (warm beam, unlocked) | the survey-beam, cruiser-fired | automated common-layer harvest + the rare-stays-manual guard |
| Manual/foot expeditions | E19 modes (+ foot-collision follow-up) | "what's exclusive to manual" gating |
| **Survey-beam** (collect + decaying rail + interaction verb) | E14 DDA pick + `solid` oracle + walker + E19 mode machine; emissive/bloom | lifespan/fade + 1-D attach-slide + line-sweep collect + contact-to-interact (board cruiser w/ lock-on, reach-gated) + **post-palette depth-aware vivid draw** |
| Glyph collection | E17 world-text + E14 DDA pick | collect event; script→stratum yield |
| Five data strata | E17's five scripts | the typed-currency economy |
| Codex of finds | E10 map + headless RTT thumbnails | find-set model + archive screen |
| Decipherment / translation | E17 glyph-string selection | lexicon model; translated rendering |
| Autopilot "learns to seek" | seed-pure placement + auto-fly | site-seeking routing |
| Resonance / pristine sense | E16 drone swell + pristine field | direction-sense + pocket harvest |
| Idle accrual / storage | (new) | buffer + offline-efficiency model |
| **The tech tree + codex UI** | **E17 world-text is the stated substrate for an in-engine UI** | **the big new system — a tree/codex screen** |
| Save / share | E12/E14 seed + deltas | add progress to the payload |

**The one substantial new build is the tech-tree + codex UI.** It should ride the
**E17 in-world-text path** (the roadmap already flags E17 as "the substrate for an
eventual in-engine UI") — keeping the no-DOM-UI promise and cross-platform parity, and
making even the menus on-aesthetic (glowing, palettised, dithered).

## 13. Build plan (slots in after M9)

[M9](milestones/M9-engine-game-split.md) gave the game its own `scraped-again` crate; the loop
lands in vertical slices, each a milestone brief. The block system
([`game-system.md`](game-system.md)) is the foundational architecture — but G1–G3 shipped
*before* it was settled, so it's **retrofitted at G4** and grown from there:

**G1–G3 were built before the block substrate was settled** (as direct keybind features, not
blocks). So the substrate is **retrofitted at G4**, and the old "first tech tree" (G4) is
**superseded** by it — its goals (in-engine menu, decipherment-legibility, auto-collect)
survive, reframed as the console + composed routines + comprehension unlocks (G4–G6).

1. **✅ G1 — Data, strata & the codex.** Collection (aim + `T`) into the five typed strata,
   strata on the HUD, a `J` codex; progress = seed + a sparse `pg=` log. *Proved the core
   sensation.* ([`milestones/G1-data-strata-codex.md`](milestones/G1-data-strata-codex.md).)
2. **✅ G2 — The survey-beam.** Cast → persist → fade beam: collect-along-path, 1-D ride rail,
   drop-on-expire, cruiser-board, + the **post-palette overlay** engine capability (realised in
   `bm-render`). *Proved the active verb + fixes "you get stuck."*
   ([`milestones/G2-survey-beam.md`](milestones/G2-survey-beam.md).)
3. **✅ G3 — Cruiser auto-scan + the map opportunity surface.** The cruiser reads what it drifts
   past (cool scan flick → the map fills with amber scanned-but-uncollected pins you triage).
   *Proved the idle scan→map→triage loop.*
   ([`milestones/G3-cruiser-scan-map.md`](milestones/G3-cruiser-scan-map.md).)
4. **✅ G4 — Block substrate & console (the retrofit)** ([brief](milestones/G4-block-substrate.md)).
   The **missing foundation**: re-express G1–G3's keybind actions (`collect`/`scan`/`fire-beam`/
   nav) as **visible, clickable blocks**; a minimal runtime running the **given routines**
   (`scan(shards) → on-scan → collect`, `drift`); the **console UI** (E17 text path, no typing).
   Same behaviour, now as the block interface. *Everything later composes on this. Supersedes
   the conventional "first tree."*
5. **✅ G5 — Composition editor & parameterised blocks.** The **wiring editor** (author routines)
   + the generic `match` filter + parameterised blocks (swap `scan`'s item; unlock fields) + nav
   `seek`/`circle`. *The management game — selective auto-collect + routing you build.*
6. **◑ G6 — Control, budgets & the block vocabulary (the "tree").** `decode` + the
   comprehension-gated **unlock economy** (strata → blocks) + **Decipherment legibility**. The
   tree *as the growing block vocabulary*. *(`when`/`repeat` + the free-form runtime moved to G7.)*
7. **✅ G7 — Routine runtime & free-form editor** ([brief](milestones/G7-routine-runtime.md)). The
   **composability core**: a real `Routine { trigger, body }` model run by an **interpreter**
   (the named-accessor hacks deleted; the givens are plain data), a no-typing **free-form editor**
   (create/delete/insert/remove/reorder/param), `when`/`repeat` on the interpreter, `scan(item)`.
8. **⏳ G8+ — Two agents, the expedition & the arc.** Walking branch (Descent/Hull interiors) +
   **independent simultaneous agents** (now on the real runtime) + the **hail** + cross-agent meta
   (automate an expedition); then decipherment fluency, the late **Concordance/Synthesis**,
   Resonance/pristine, co-op shared archive (N1).

## 14. Open items

- **The game's name** — **resolved: _Scraped Again_** (crate `scraped-again`). Styling
  (`Scraped Again` / `Scraped, Again` / lowercase) is a cosmetic call left open.
- **v1 tree tuning** — the economy's *feel* (sweep rate, costs, buffer caps) needs live
  iteration; design the shape now, balance on play.
- **Tone guardrails for the UI** — keep the tree/codex *quiet and archival*, not a
  flashing idle-game dashboard; it must sit inside the doom palette, not on top of it.
- **How much lore is authored vs. procedural** — the Decipherment payoff wants *some*
  resonant writing; decide per-seed procedural grammar vs. a thin authored backbone
  (revisit when G2's legibility lands).
