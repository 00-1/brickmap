# Scraped Again — Core game mechanics (design)

> *Scraped Again* is the **game**; **brickmap** is the **engine** it's built on. The
> title is the plain-English sense of *palimpsest* ("scraped again") — a dead,
> overwritten world you read back into legibility, where the endless re-seeded world
> is itself one more scraping. (Workspace crate `scraped`; see
> [`milestones/M9-engine-game-split.md`](milestones/M9-engine-game-split.md).)
>
> **Status: living design.** The direction below is **decided** (2026-06): a
> **melancholy-archival exploration game** built on the brickmap engine, with an
> **autopilot-as-idle data-collection loop feeding a deep tech tree of
> comprehension**. The engine/game *separation* that gives this a home is planned in
> [`milestones/M9-engine-game-split.md`](milestones/M9-engine-game-split.md) (in
> progress); this doc is the **game's** design and will keep evolving. The tree's
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
2. **Autopilot is a first-class way to play.** The passive layer must be genuinely
   rewarding and time-respecting (you can drift, tab away, return). Manual play is the
   *bursty, targeted, higher-yield* layer — never punished-for, always *additive*.
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
        │  drift the endless world ▸ ambient sweep logs common glyphs into a       │
        │  buffer (capped by Memory tech) ▸ the ship learns to seek dense/rare/    │
        │  undiscovered sites (Locomotion tech)                                    │
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
        │  ▸ collect rare data + decoding keys + read interiors autopilot can't    │
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

## 6. Collection — passive vs active

- **Ambient sweep (autopilot / passive).** Inscriptions within a *survey radius* of the
  drifting cruiser are auto-logged, trickling mostly **Records** (+ some **Schematics**)
  into a buffer. Rate and radius scale with Sensing/Memory tech. This is the idle layer:
  the cinematic auto-fly (D3) becomes a slow, self-improving harvester.
- **Targeted collection (manual / foot).** The **rare strata, decoding keys, and
  interiors** are *only* reachable by hand:
  - a **named colossus's** monument inscription (E17 `colossus_label`),
  - glyphs **inside caves** and **solid-colossus interiors** (needs foot collision /
    descent tech — ties to the E19 follow-ups),
  - **pristine-pocket** inscriptions (Rites/Signals + the choral beat),
  - **ethereal** colossi glyphs you'd otherwise drift straight through unnoticed.
- **The codex.** Every collected glyph is recorded in a growing **archive** (catalogue +
  a headless-RTT thumbnail per find). Collection thus pays out twice: **quantity** (feeds
  the tree) and **understanding** (fills the archive — the melancholy payoff).

The rule that keeps manual play essential: **autopilot sweeps the common ambient layer;
everything rare, deep, interior, or decoding-critical must be gone to.**

## 7. Pressure / failure

**None.** No death, no threat. Data is always spent *toward comprehension*, so the tree
is the sink. The only pacing lever is the **idle storage cap** (Memory tech) — a *gentle*
reason to return and process, never a punishment. If playtesting wants more pull, the old
"pilgrim" idea returns as flavour, not threat: the cruiser's drift sips a charge
replenished at **pristine pockets**, making them rhythmic waypoints rather than fail
states. Default: keep it pure.

## 8. The tech tree — faculties of comprehension

Five branches. Nodes are **illustrative** (the structure is the commitment); each branch
is meant to deepen "to the extreme" over time, terminating in a long, expensive
late-game arc. **Every node is comprehension or reach — never force.**

### 8.1 Sensing — *perceive & sweep* &nbsp;(feeds: Records, Schematics)
- **Survey Radius I–V** — widen the ambient-sweep range.
- **Sweep Rate I–V** — log faster as the world drifts past.
- **Deep Scan** — detect *buried / sub-surface* glyphs (read cave inscriptions from
  above; mark them on the map).
- **Spectral Sight** — reveal **ethereal** colossi and faint inscriptions you'd
  otherwise drift through without noticing.
- **Cartograph I–III** — map resolution; pin *uncollected* sites; show stratum hints.

### 8.2 Decipherment — *the lore spine* &nbsp;(feeds: each script's own data + Relics)
- **Legibility: Records → Schematics → Rites → Relics → Signals** — five gated unlocks;
  an unlocked script renders **translated** instead of glowing nonsense.
- **Fluency I–III** (per script) — *progressive* legibility: first fragments resolve,
  then full phrases. Watching the world become readable **is** the progression curve.
- **Cross-Reference** — auto-translate newly found glyphs of an already-known script.
- **Concordance** (late) — synthesise scattered fragments into the world's actual
  history: what the giants were, why they fell, what killed the place. The terminal
  lore. *Effect, beyond lore:* a deciphered script's **yield rises** (you extract more
  once you understand it) — a multiplier with a diegetic reason.

### 8.3 Locomotion — *reach & autopilot autonomy* &nbsp;(feeds: Schematics, Relics)
- **Drift Efficiency / Range** — go further between stops.
- **Autopilot Heuristics I–III** — *"the ship learns to seek"*: the drift stops being
  blind and routes toward dense / rare / **undiscovered** sites. The idle layer
  literally improves itself.
- **Atmospheric Ceiling** — altitude to reach high giants and overview vantage.
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
- **Automation: Auto-Log** (collect without confirming) → **Auto-Route** (chain
  autopilot between known sites).
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

Drift on autopilot; data trickles; dip into the tree; take the stick for a targeted run
to a giant / cave / pristine pocket for rare data and a decoding key; watch a script turn
legible and read what the dead left; set autopilot; drift on. Endless, calm, deepening.
A session can be five minutes of processing the buffer or an hour of expeditions.

## 11. Save / share

State = **seed + a sparse progress log** (collected-glyph set + tree state + decoded
scripts + codex notes) — the *same* `seed + sparse deltas` artifact E12/E14 already use.
So progress permalinks, shares, and slots into multiplayer as a **shared archive** with
no new persistence model.

## 12. How it maps to existing systems

| Mechanic | Built on | Genuinely new |
|---|---|---|
| Autopilot idle sweep | D3 auto-fly + E17 inscriptions + seed placement | survey-radius check + data accrual/buffer |
| Manual/foot expeditions | E19 modes (+ foot-collision follow-up) | "what's exclusive to manual" gating |
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

Once [M9](milestones/M9-engine-game-split.md) gives the game its own crate, the loop
lands in vertical slices, each a milestone brief:

1. **G1 — Collect & accrue.** Survey-radius collection (passive + a manual pick), the
   five strata as numbers on the HUD, a codex list. *Proves the core sensation.*
2. **G2 — The first tree.** A small tuned tree (a few nodes per branch) + the in-engine
   tree UI on the E17 text path; Sensing + Memory first (they make the idle layer feel
   good); Decipherment legibility for one script. *Proves the economy + the payoff.*
3. **G3 — Autopilot autonomy + interiors.** "Ship learns to seek" routing; Descent/Hull
   tech + foot-collision so caves and solid colossi become collectible interiors.
4. **G4+ — Depth.** Expand each branch toward the late-game **Concordance/Synthesis**
   arcs; tune the idle/active balance; the Resonance/pristine layer; co-op shared
   archive (with N1).

## 14. Open items

- **The game's name** — **resolved: _Scraped Again_** (crate `scraped`). Styling
  (`Scraped Again` / `Scraped, Again` / lowercase) is a cosmetic call left open.
- **v1 tree tuning** — the economy's *feel* (sweep rate, costs, buffer caps) needs live
  iteration; design the shape now, balance on play.
- **Tone guardrails for the UI** — keep the tree/codex *quiet and archival*, not a
  flashing idle-game dashboard; it must sit inside the doom palette, not on top of it.
- **How much lore is authored vs. procedural** — the Decipherment payoff wants *some*
  resonant writing; decide per-seed procedural grammar vs. a thin authored backbone
  (revisit when G2's legibility lands).
