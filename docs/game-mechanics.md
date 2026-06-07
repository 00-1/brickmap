# brickmap — Core game mechanics (planning / proposal)

> **Status: proposal, not committed direction.** This doc opens a deliberate
> question the project has so far avoided: *should brickmap become a game, and if
> so, what is its core loop?* Nothing here changes the roadmap or the engine yet.
> It exists to give the human something concrete to push on.
>
> **Honest tension up front.** [`design.md`](design.md) §3 lists **gameplay,
> physics, entities, AI, persistence** as explicit *non-goals* — brickmap is a
> *rendering engine, not a game*. Adopting any core loop means **revisiting §3 on
> purpose**, not drifting past it. That's a call for the human, not a thing to
> assume. This doc assumes "yes, let's explore it" because that's what was asked;
> it does not assume "yes, let's commit."

## 1. Why this is even on the table

The engine was never meant to be a game, but the *world* has quietly grown a cast
of nouns that are begging for verbs. We already have, live and on-aesthetic:

- **Fallen colossi** (E18) — enormous seed-placed giants strewn across the world,
  some **ethereal** (drift through the point-cloud), some **solid voxel** bodies you
  can **land on and walk**. Ancient mechanical tube-relics + toppled human figures.
- **Inscriptions & monuments** (E17) — abstract writing scattered on the ground and
  **labelling each colossus**, in **five writing systems** (Latin, Greek, Hiragana,
  Standard Galactic, Runic). Glowing, lo-fi, recolour/crumble with the palette.
- **Pristine pockets** — rare, special places with their own **choral pad** (relief
  from the doom drone everywhere else) and a unique map icon. The world's "sacred sites."
- **A per-seed doom dirge** (E16) — a Sleep/*Dopesmoker* drone unique to each world;
  intensity already reacts to flight (speed + altitude) and **warps harder as you
  near a giant** (structure-approach wobble — proximity to a colossus visibly distorts
  reality).
- **Two ways to move** (E19) — pilot a **cruiser** (fly, autopilot or manual), **land**
  it, **step out and walk** the terrain on foot (caves, up to giants), walk back, fly on.
- **An explored map** (E10) that fills in as you travel, with a you-are-here dot, the
  parked cruiser, found inscriptions, and pristine icons.
- **Shareable seeds** (E12) — the whole world is a pure function of a seed; any place
  is a permalink.
- **Biomes** that crossfade as you fly (palette, audio, density, lighting all shift).
- **Dynamic voxels** (E5/E11), **voxel editing** (E14), **particles** (E2) — matter
  that can move, break, and be changed, routed through a clean event seam.

That is, almost verbatim, the **setting and toolkit of an atmospheric exploration
game** — a lonely traveller crossing a dead, doom-laden world of fallen giants and
ancient writing. What's missing is **a reason to go, a thing you do when you arrive,
and a sense of having gotten somewhere.** That's the "core mechanic."

## 2. The design pillars any mechanic must honour

These fall out of the existing identity; a mechanic that breaks one is the wrong
mechanic.

1. **Stay cheap and on-brand.** Weak-hardware-first (design §4) and "expose the tech"
   (§11) don't pause for gameplay. The best mechanics **reuse systems we already have**
   (map, text, splat path, event seam) and add ~no render cost.
2. **Honour the mood.** The world is **dark, grimy, lonely, doom-laden**, with rare
   pristine relief. Mechanics should feel **archaeological and elegiac**, not arcade.
   No score popups, no health bars fighting the palette. Closer to *Shadow of the
   Colossus* / *NaissanceE* / *Journey* than to a shooter or a survival-crafter.
3. **Don't drag the engine into the scope it refused.** Heavy entity AI, pathfinding,
   combat, inventory tetris, networked authority — these are exactly the §3 non-goals.
   A good core loop here is **goal-light and computation-light**: the *world* is the
   content, the player's *attention* is the resource.
4. **Determinism-friendly.** The world is seed-pure (the multiplayer "determinism
   dividend", roadmap N1). Mechanics should keep state as **seed + a sparse event/
   progress log**, never require syncing or storing the world itself.
5. **Single-player first, multiplayer-shaped.** Whatever the loop is, it should survive
   "now do it with a friend in the same seed" without a rewrite (presence + shared
   discoveries layer cleanly onto N1).

## 3. The core question, stated plainly

Everything downstream forks on **two axes**:

- **Goal axis — what pulls the player forward?**
  *Pure wander* (no goals, the world is the point) ↔ *directed* (objectives, a thing
  to find/finish) ↔ *open systemic* (emergent goals from systems like survival/sim).
- **Pressure axis — what can go wrong?**
  *No-fail / contemplative* ↔ *soft pressure* (resources, dread, decay) ↔ *hard fail*
  (death, loss).

The mood and the engine constraints push **hard toward the low-pressure, discovery-
directed quadrant** — but that's the decision to confirm, so §4 lays out three honest
candidates across the space.

## 4. Three candidate directions

### A. The Cartographer / Decipherer *(recommended)*

**Fantasy.** You are the last surveyor of a dead world. You travel between **fallen
giants**, read the **inscriptions** that name and mourn them, and slowly **decipher**
a lost script — turning glowing nonsense into meaning. Discovery *is* the reward.

**Core loop.**
1. **Spot** a landmark (a giant on the horizon, a glow, a map icon, a swell in the
   drone). 2. **Travel** there by cruiser. 3. **Land & walk** it — get close to the
   body, find its **monument inscription** and nearby **ground inscriptions**.
4. **Record / decode** — the find enters a **codex**; collecting glyphs fills in a
   **cipher** so more of the world's writing becomes legible. 5. **A thread opens** —
   a decoded inscription **points you** to the next site (a bearing, a named giant, a
   pristine pocket). 6. Repeat; the dead world slowly **tells you what happened to it.**

**Why it fits.** Almost zero new tech: it's the **map + text + colossi + audio we
already render**, plus a **codex screen** (reuse the map overlay + text path) and a
**glyph-progress model** (pure data, seed-keyed). The five writing systems stop being
decoration and become the **content spine**. The doom drone is the world's grief; the
**pristine pockets are the payoff beats** (legibility/relief). No entities, no combat,
no physics beyond what E19 already does.

**Pros.** Cheapest to build; most distinctive; deepest fit with the existing fiction;
trivially seed-deterministic; multiplayer = shared codex. **Cons.** Needs *authored or
generated meaning* behind the glyphs (a content pipeline — see §6); risk of feeling
thin if decoding is just "collect N glyphs." **Fail/pressure:** none by default
(contemplative); optional soft pressure in §5.

### B. The Pilgrim *(directed, light pressure)*

**Fantasy.** A pilgrimage across the dead world toward something — the next pristine
pocket, the largest giant, a vanishing point. Movement and **endurance** are the game.

**Core loop.** Travel → manage a thin resource (the cruiser's **drift/charge**, refilled
only at **pristine pockets**) → reach waypoints → the world reacts (drone, warp, palette)
as you approach significance. Stakes: run dry far from a pocket and you're stranded
(soft-fail: walk, or a slow recovery), so **route-planning on the map** matters.

**Why it fits.** Adds a **single resource + the map as a planning surface** — both cheap.
Turns the existing biome/pristine structure into a **navigation economy**. **Pros.**
Gives forward pull + mild stakes without entities; pristine pockets gain mechanical
weight. **Cons.** A resource meter risks fighting the contemplative mood and the
"no HUD clutter" aesthetic; the fun is thinner than A's decoding. Easy to **layer on top
of A** rather than instead of it (A provides the *why*, B the *pacing pressure*).

### C. The Caretaker / Terraformer *(open systemic)*

**Fantasy.** You don't just witness the dead world — you **act on it**. Use **editing
(E14)** and **dynamic voxels (E11)** to clear, restore, grow, or reshape sites; **grow
moss/crystals** on a dead giant; divert **water/sand**; light the dark.

**Core loop.** Find a degraded site → apply systemic tools (CA growth, fluids, edits) →
the world responds and (optionally) records the change as **seed + edit deltas** → share
it. Emergent, sandbox-leaning.

**Why it fits the engine** (it's the §12 "dynamic voxels are the headline" bet cashed in)
**but fights the mood** (active fixing vs. elegiac witnessing) and is the **most scope-
heavy** (real sim systems, balance, UI). **Pros.** Maximum use of the sim/edit tech;
strong creative/shareable angle. **Cons.** Biggest build; least defined "win"; risks
becoming "Minecraft creative," which the project explicitly does **not** want to look or
feel like. Best treated as a **later mode**, not the core.

### Recommendation

**Lead with A (Cartographer/Decipherer) as the core**, keep **B (Pilgrim pressure) as an
optional layer** to tune pacing once A exists, and **defer C** to a creative mode. A is
the cheapest, the most on-mood, and the one that turns content we've *already built but
under-used* (five scripts, labelled giants, the map, pristine pockets, the reactive
drone) into an actual loop.

## 5. The recommended loop, fleshed out

A "no-fail, discovery-directed explorer." The verbs and feedback, mapped to systems
that already exist.

### Player verbs
- **Travel** — cruiser (autopilot for sightseeing, manual to choose a heading). *(E19,
  done.)*
- **Approach / land / walk** — the on-foot mode for getting *to* a body or inscription.
  *(E19, done; wants voxel collision on foot to enter giants/caves properly — already a
  listed E19 follow-up.)*
- **Look / read** — face an inscription; proximity makes it legible / records it. *(E17
  text path, done; needs a "you are reading this" trigger + dwell.)*
- **Record** — the act that banks a discovery into the **codex** (a found giant, a found
  inscription, a decoded glyph). New, but small: a data model + a screen.
- **Orient** — open the **map/codex** to see what's found, what's decoded, and the
  **next thread's bearing**. *(E10 map, done; extend with markers + a codex tab.)*

### What pulls you forward (the "thread")
A decoded inscription resolves to a small payload: a **bearing + distance** to another
seed-placed site, or a **named giant**, or "a pristine pocket lies <direction>." Because
placement is seed-deterministic (`structures::colossi_near`, `inscriptions_near`,
pristine field), **the thread can be computed, not authored** — pick the next site by a
seeded function of the current one. The map draws the lead; the drone/warp swells as you
near it. This is the engine's determinism turned into a quest chain for free.

### Progression (the sense of "getting somewhere")
- **The codex fills** — a growing illustrated list of giants/inscriptions found (reuses
  the headless RTT to capture a thumbnail per find — we already render to texture).
- **The cipher fills** — each script starts opaque; finding "Rosetta" inscriptions (a
  known landmark giant whose name you're told) **unlocks glyph→meaning mappings**, so
  later writing renders *translated*. Watching the world become legible **is** the
  progression curve. Pure data; no new render tech (we already choose glyph strings).
- **Pristine pockets** are the **payoff beats** — reaching one is a milestone (choral
  relief, a map pin, maybe the clearest inscriptions).

### Stakes / failure
**None by default** — contemplative. If playtesting finds it aimless, add **B's soft
pressure** (cruiser charge refilled at pristine pockets) as a *toggle*, the same way the
engine ships look-features as toggles (D6). Never a hard death; the mood is grief, not
threat.

### Session shape
A session = drift the world, chase 1–3 threads, bank a handful of discoveries, reach a
pristine beat, stop whenever. **Save state = seed + a sparse progress log** (found-set +
decoded-glyph-set + codex notes) — the exact same `seed + sparse deltas` artifact E12/E14
already use, so it shares, permalinks, and slots into multiplayer (shared codex) with no
new persistence model.

## 6. The one real new dependency: *meaning*

The honest risk in A is that decoding is hollow if the glyphs decode to nothing. Options,
cheapest first:

- **Procedural-poetic.** Inscriptions compose from a **seeded grammar** over a small
  word-bank ("here lies / the warden of / the drowned light / who slept"). Decoding
  reveals these. Zero authoring, infinite worlds, on-mood (fragmentary, elegiac). **Risk:**
  can read as Mad-Libs if the grammar is thin — invest in the grammar, not the tech.
- **Authored layer over procedural.** A handful of **hand-written key inscriptions** at
  landmark giants (the "Rosetta" stones + a loose backstory), with procedural filler
  between. More resonant; small authoring cost; ties specific seeds to specific stories
  (or a "canonical seed").
- **Pure cipher, no semantics.** Decoding is just legibility (glyph→letter), content is
  the procedural grammar above. Simplest; the "story" is whatever the grammar emits.

**Recommendation:** start **procedural-poetic with a real grammar**, leave room to drop
**authored key inscriptions** on top. Decide the fiction's tone separately (see §8).

## 7. How it maps to existing systems (cheap-build table)

| Mechanic | Built on | New work | Cost |
|---|---|---|---|
| Travel / land / walk | E19 movement modes | foot voxel-collision (already queued) | low |
| Read an inscription | E17 world text + DDA pick (E14) | dwell/face trigger; legibility state | low |
| Codex of finds | E10 map overlay + headless RTT thumbnails | a find-set data model + a screen | medium |
| Cipher / translation | E17 glyph-string selection | glyph→meaning model; render translated | low |
| The "thread" / next lead | seed-pure placement fns | a seeded next-site picker + map marker | low |
| Inscription meaning | E17 strings | a **seeded grammar + word-bank** | medium |
| Pristine payoff beats | existing pristine field + choral pad | mark as objectives; a "reached" event | low |
| Progress save/share | E12/E14 `seed + sparse deltas` | add found-set/decoded-set to the payload | low |
| (Optional) pilgrim pressure | biome/pristine field + map | one resource + refill-at-pristine | medium |

Nothing here needs a new render path. The heaviest items are **data models + a codex
UI** and **the grammar** — i.e. design and content, which is appropriate for a "planning,
not coding" phase.

## 8. Open questions for the human (the real forks)

1. **Do we actually want brickmap to become a game at all** — i.e. amend design §3 — or
   keep it an engine and treat this as a *separate experience built on top*? (Either is
   fine; it changes how loudly the roadmap talks about it.)
2. **Which quadrant** (§3): I'm recommending **discovery-directed, no-fail** (direction A).
   Veto or steer?
3. **Meaning** (§6): procedural-poetic grammar, authored key inscriptions, or pure cipher?
   And **what's the fiction's tone** — wordless and abstract, or an actual lost-civilisation
   story you can piece together?
4. **Pressure** (§5): contemplative-only, or do you want the optional pilgrim resource
   layer designed in from the start (even if toggle-off by default)?
5. **Scope of "core"**: is the first playable milestone "**reach a giant, read it, bank it
   in a codex, get pointed at the next one**" — or smaller/larger?

## 9. Suggested next planning steps (once a direction is picked)

- Turn the chosen direction into a **milestone brief** in `docs/milestones/` (the roadmap
  template), slotted as a new **`✨ G1 — Core loop`** rung (kept out of the linear engine
  ladder, like the D-series).
- Spec the **codex + cipher + progress** data model against the **E12/E14 event/delta
  seam** so save/share/multiplayer come free.
- Draft the **inscription grammar** as its own small design note (it's the content engine).
- Decide whether any of this lands behind a **mode/toggle** so the pure-engine "fly and
  look" experience stays intact (it should — the engine demo is still the engine demo).

---

*This is a fork in the project's identity, deliberately surfaced rather than assumed.
The recommendation is the lowest-risk, most on-mood path — but the call is the human's.*
