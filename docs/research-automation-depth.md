# Research — automation/idle game depth (for the block substrate)

> 2026-06-11 research pass (web). Question: what makes automation / programming-lite / idle
> games deep and interesting **long-term**, distilled into actionable principles for the
> *Scraped Again* block substrate ([`game-system.md`](game-system.md)). Feeds
> [`game-depth.md`](game-depth.md), which turns these into milestones. Sibling of
> [`research-points-splatting.md`](research-points-splatting.md).

## 1. The core loop: solve → automate → scale → re-solve (Factorio)

**P1. Automation must create the next problem, not end play.** Factorio stays fun because
automating X immediately surfaces a bottleneck in Y; the satisfaction is understanding the
system. → *Every routine should change world/economy state so a new visible scarcity
appears: an automated scanner floods the decode queue; an automated walker drains carry
capacity. Block effects chain so automation relocates attention, never removes it.*

**P2. Depth comes from friction between subsystems, not complexity inside one.** Factorio's
circuits are deep because they mediate between belts/bots/trains with different
throughput/latency characters. → *Give ship and walker asymmetric resource regimes (ship =
scan bandwidth/altitude windows; walker = reach/carry) so routines negotiate across the
seam. The seam is where the interesting routines live.*

**P3. No one-true-build: incommensurable architectures.** Main-bus vs city-block persists
because each wins on a different axis. → *Routine shapes must trade off: few monolithic
routines (cheap, fragile) vs many small event-driven ones (resilient, costs slots/priority
budget). Never let one shape strictly dominate.*

**P4. Optional depth layers.** Factorio's circuit network is skippable; experts build
computers with it. → *match/when/priority machinery stays entirely optional for the core;
baseline blocks work naively. Protects the melancholy-exploration audience from the systems
audience's game, and vice versa.*

## 2. Optimization as endgame (Zachtronics)

**P5. Multiple scoring axes in tension → a Pareto frontier, no perfect answer.** Opus
Magnum's cost/cycles/area means optimization never terminates. → *Score routines on ~3
genuinely conflicting axes: yield/hour, energy-or-time per unit, block count (elegance).
This single decision buys years of self-directed endgame.*

**P6. Histograms beat leaderboards.** Opus Magnum shows your score on a distribution —
every player gets a meaningful target. → *Offline-friendly: show routine metrics against
authored bands ("crude / sound / cunning / uncanny"). Non-punitive ambient comparison.*

**P7. Author conditions, not solutions.** Zach Barth: tools + end condition + empty space;
content is distinct if the old solution can't be copy-pasted. → *World situations (a tide
cycle, scan interference regions) as conditions; the distinctness test: does last region's
routine still serve?*

**P8. Aesthetics is a hidden score.** Opus Magnum's GIF export made elegance shareable. →
*An exportable trace of a routine executing (blocks lighting, agents moving) is our GIF —
fits "expose the tech" exactly.*

## 3. Teaching automation to non-programmers (Autonauts, Desynced, Bitburner, Screeps)

**P9. Record-to-program.** Autonauts' programming-by-doing is the strongest non-programmer
on-ramp. → *Every manual action is already a block press: "make this a routine" can
retroactively capture the last N presses as a draft routine. The manual game IS the
tutorial.*

**P10. Tedium killers: clicks-per-intent, naming, copy/paste pain.** Node canvases die at
scale (Desynced's late-game complaint). → *(a) Linear step-lists, never a 2D graph
(controller/phone demands it — already our G7 shape); (b) blocks carry world-learned names
(G9 discovery doubles as documentation); (c) routines are first-class values: duplicate,
template, call-as-a-step (`run(routine)` exists — extend to same-agent subroutines).*

**P11. Gate vocabulary, not capability tiers (Bitburner).** Each unlocked function reopens
every old problem. → *On unlock, surface where it would help: "3 of your routines could use
`when(tide)`." Old routines become refactoring invitations — the idle 'go back stronger'
without prestige resets.*

**P12. Always-on persistence is magnetic but must be cheap to enter (Screeps).** Tight
early CPU quotas wall newcomers. → *Interpreter budgets (steps/tick, routine slots) are
great late pressure, poison early. Start generous; introduce scarcity as a discovered world
property, not a menu number.*

## 4. Idle economy pacing (Pecorella GDC, Cookie Clicker, Universal Paperclips)

**P13. Exponential costs vs polynomial production creates walls — design what breaks them.**
Prestige is the usual release valve. → *Our wall-breaker is a* vocabulary *event (a new
block that restructures routines), never a +x% multiplier — right tone for a melancholy
sandbox. Pace G9 discoveries against the cost curve like prestige tiers.*

**P14. Never let old units go dead (the lemonade-stand problem).** → *`scan`/`collect` stay
load-bearing forever as primitives inside late routines — composition, not replacement.
"This block is called by 12 routines" is a quiet pride metric.*

**P15. An active layer on the idle base (golden cookies).** → *Transient world events (a
passing signal, an aurora window) that routines catch partially and a present player catches
fully. Presence is a bonus, never a tax.*

**P16. Act structure: the interface paradigm shifts at scale boundaries (Paperclips).** →
*Built-in act break: act 1 walker/manual, act 2 routines+ship, act 3 orchestrating both
while mostly witnessing. The player automating themselves into a witness IS the melancholy
theme, made mechanical.*

**P17. Keep one arbitrary goal always lit (Lantz).** → *Always exactly one suggested
next desire visible (a half-decoded name, a threshold at 87%) — never a quest log. Idle
economies feel dead when nothing on screen is almost-finished.*

## 5. Two-agent orchestration (Factorio trains, Stationeers IC10, Nimbatus)

**P18. Agents follow simple local rules; the player designs the protocol.** Train signals
are simple; the network is yours; failures are dramatic, legible, your fault. → *Ship and
walker coordinate only via shared world state (a cache the walker fills, a window the ship
opens) — the player programs both sides of a handshake. Failed handoffs are visible little
tragedies, not error messages.*

**P19. Hard per-agent limits force elegance and identity (IC10's 128 lines).** → *Different
routine capacities + block compatibility per agent make the agents characters and "which
agent owns this job" a real decision (we have ship/foot/shared tags — add capacity
asymmetry).*

**P20. Legibility at scale: telemetry, alerts, answering "why".** Desynced's late-game
criticism is losing track of what units are doing. → *Per-routine telemetry is
non-negotiable: yield/hour, blocked-vs-running, last trigger, live step highlight, and a
one-tap "why is this idle?" ("waiting: cargo full"). The single highest-leverage UI feature
in the genre.*

## 6. Anti-patterns (how these games stop being fun)

- **Micromanagement walls** — scaling = repeated clicks. *Antidote: templates + subroutine
  calls + apply-to-matching edits (P10).*
- **Solved metas.** *Antidote: Pareto axes (P5) + conditions that invalidate copy-paste (P7).*
- **Upgrade treadmills with no decisions** (hedonic adaptation). *Antidote: progression as
  new verbs/structures, never +% (P13).*
- **Automation that obsoletes play without opening new play.** *Antidote: every closed loop
  surfaces a new scarcity (P1).*
- **Illegible automation** — it runs but the player stopped understanding, so stopped
  caring. *Antidote: P20 telemetry; short named routines.*
- **Tutorial deserts.** *Antidote: record-to-program (P9); a block's decoded description IS
  its man-page (we have this — keep it terse-cryptic but real).*

## Top 10 recommendations (ranked by leverage for Scraped Again)

1. **Record-to-program** — "make this a routine" captures recent manual presses as a draft.
2. **Three conflicting routine metrics** (yield/hr · cost-per-unit · block count) with
   authored histogram bands — the sandbox endgame.
3. **Per-routine telemetry + one-tap "why is it stuck?"** — decides whether the management
   layer feels alive.
4. **Every closed loop opens a new scarcity** — audit each automatable action for its
   downstream bottleneck.
5. **Linear step-lists + subroutine calls, never a node canvas** (G7 is already right —
   add same-agent `run(routine)` + duplicate/template).
6. **Vocabulary unlocks as the prestige curve** — pace discoveries against cost walls; on
   unlock, point at the routines it would improve.
7. **Agents coordinate via world state only** — handshakes (cache/window/beacon), both
   sides player-authored; failed handoffs visible.
8. **Asymmetric agent constraints** — per-agent routine capacity + block compatibility.
9. **Composition over obsolescence** — primitives never retire; show call-counts.
10. **Act structure via role shift** — operator → author → overseer; the interface itself
    shifts; witnessing is the point.

## Sources

- pcgamer.com/perfectly-solving-opus-magnums-puzzles-is-impossible-but-thats-ok ·
  stephensopinions.com/2020/08/17/opus-magnum-2017 · zlbb.faendir.com/help
- gamedeveloper.com/design/postmortem-zachtronics-industries-i-spacechem-i- ·
  gdcvault.com/browse/gdc-19/play/1025715
- gamedeveloper.com/design/the-math-of-idle-games-part-i/-ii/-iii · Pecorella GDC slides
  (media.gdcvault.com/gdceurope2016/presentations/Pecorella_Anthony_Quest%20for%20Progress.pdf)
- eludamos.org/index.php/eludamos/article/view/vol10no1-7 (Universal Paperclips paper) ·
  ycombinator.com/blog/frank-lantz-director-of-nyus-game-center · fastcompany.com/90996377
- kalebnek.medium.com/cookie-clicker-analysis-bf3787aa96d7
- autonauts.fandom.com/wiki/Programming · cubed3.com/games/reviews/pc/autonauts-2
- Desynced Steam discussions/reviews · fingerguns.net Desynced EA review ·
  wiki.desyncedgame.com/Behavior_Programming
- docs.screeps.com/cpu-limit.html · screeps.com/forum/topic/2673 · nodal.gg/game/bitburner-1812820
- wiki.factorio.com/Logistic_network · wiki.factorio.com/Tutorial:Train_signals ·
  alt-f4.blog/ALTF4-64 · mason-larobina.github.io/factorio/2020/05/23 ·
  forums.factorio.com/viewtopic.php?t=37024
- stationeers-wiki.com/IC10 · indiegamewebsite.com/2020/05/14/nimbatus review ·
  saveorquit.com/2020/05/15 Nimbatus review
- danmackinlay.name/notebook/patchers · dev.to node-language design pitfalls ·
  medium.com hedonic-adaptation-in-games · ericguan.substack.com idle-game-design-principles
