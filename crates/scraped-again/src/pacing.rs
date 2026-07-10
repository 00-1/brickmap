//! G19: the pacing probe made **permanent**. Ported from `docs/probes/pacing_probe.rs` (the
//! 2026-06-16 quantitative playtest whose findings drove the G19 economy truing) — drives the
//! REAL per-frame loop headlessly (`App::headless` + `run_frame`, the D11 seam) under pure
//! autopilot (the given routines only) and measures the pacing a player would experience.
//!
//! Two layers:
//! - [`pacing_envelope_seed_1337`] — a **bounded CI regression** (one seed, ≤35 sim-min,
//!   early-out) asserting the post-truing envelope holds: first gated discovery, first
//!   comprehension, and the shard-income band. Always on.
//! - The **full multi-seed probe** (runs 1–5: income tables, the faculty ladder, the full
//!   vocabulary ladder, the expedition handshake) — measurement, not assertion; env-gated:
//!   `PACING_FULL=1 cargo test --release -p scraped-again pacing -- --nocapture`.

use super::*;
use crate::progress::{Event, Faculty, ResearchTarget, Stratum};
use crate::shards::Rarity;

const DT: f32 = 1.0 / 60.0;
const SEEDS: [u32; 6] = [1337, 7, 2024, 99999, 42, 555];

const GATED: [console::Block; 4] = [
    console::Block::Seek,
    console::Block::Circle,
    console::Block::Goto,
    console::Block::RunFoot,
];

fn frames_of_minutes(m: f32) -> usize {
    (m * 60.0 / DT) as usize
}

/// The full probe is measurement (tables on stdout), not regression — opt in with
/// `PACING_FULL=1` (use `--release`; six seeds × 60–480 sim-min are slow in debug).
fn full_probe_enabled() -> bool {
    std::env::var("PACING_FULL").is_ok_and(|v| v != "0")
}

// ---------------------------------------------------------------- income tracking

/// Per-window income tally, rebuilt from public `Progress` getters via per-frame deltas.
#[derive(Default, Clone, Copy)]
struct Income {
    /// Shard items picked up, per domain.
    items: [u64; 5],
    /// Shard yield banked (Σ rarity yields).
    shard_yield: u64,
    /// Decomposed rarity counts (max-likelihood from per-frame (items, yield) deltas).
    rarity: [u64; 3],
    /// Inscription data banked, per stratum.
    data: [u64; 5],
    /// Inscriptions collected.
    finds: u64,
}

impl Income {
    fn items_total(&self) -> u64 {
        self.items.iter().sum()
    }
    fn add(&mut self, o: &Income) {
        for (a, b) in self.items.iter_mut().zip(&o.items) {
            *a += b;
        }
        for (a, b) in self.data.iter_mut().zip(&o.data) {
            *a += b;
        }
        for (a, b) in self.rarity.iter_mut().zip(&o.rarity) {
            *a += b;
        }
        self.shard_yield += o.shard_yield;
        self.finds += o.finds;
    }
}

fn ln_fact(n: u64) -> f64 {
    (2..=n).map(|k| (k as f64).ln()).sum()
}

/// Decompose an (items, yield) delta into (common, uncommon, rare) counts by maximum
/// likelihood under the design mix 85/13/2 (yields 1/3/9). Exact when items == 1.
fn decompose(n: u64, y: u64) -> [u64; 3] {
    let mut best: Option<(f64, [u64; 3])> = None;
    for r in 0..=n {
        let rem = y as i64 - n as i64 - 8 * r as i64;
        if rem < 0 || rem % 2 != 0 {
            continue;
        }
        let u = (rem / 2) as u64;
        if r + u > n {
            continue;
        }
        let c = n - u - r;
        let score = ln_fact(n) - ln_fact(c) - ln_fact(u) - ln_fact(r)
            + c as f64 * 0.85f64.ln()
            + u as f64 * 0.13f64.ln()
            + r as f64 * 0.02f64.ln();
        if best.is_none_or(|(s, _)| score > s) {
            best = Some((score, [c, u, r]));
        }
    }
    best.map(|(_, x)| x).unwrap_or([n, 0, 0])
}

/// Snapshot of the public counters we diff each frame.
struct Meter {
    items: [u64; 5],
    bank: u64,
    data: [u64; 5],
    finds: u64,
}

impl Meter {
    fn new(app: &App) -> Meter {
        Meter {
            items: Stratum::ALL.map(|d| app.progress.shard_count(d) as u64),
            bank: app.progress.shard_bank(),
            data: Stratum::ALL.map(|d| app.progress.strata.get(d)),
            finds: app.progress.collected_count() as u64,
        }
    }
    /// Diff against the app's current counters, accumulate into `into`, update self.
    fn tick(&mut self, app: &App, into: &mut Income) {
        let mut n = 0u64;
        for (i, d) in Stratum::ALL.iter().enumerate() {
            let c = app.progress.shard_count(*d) as u64;
            into.items[i] += c - self.items[i];
            n += c - self.items[i];
            self.items[i] = c;
            let dd = app.progress.strata.get(*d);
            into.data[i] += dd - self.data[i];
            self.data[i] = dd;
        }
        let b = app.progress.shard_bank();
        if n > 0 {
            let dec = decompose(n, b - self.bank);
            for (a, d) in into.rarity.iter_mut().zip(dec) {
                *a += d;
            }
        }
        into.shard_yield += b - self.bank;
        self.bank = b;
        let f = app.progress.collected_count() as u64;
        into.finds += f - self.finds;
        self.finds = f;
    }
}

fn gated_discovered(app: &App) -> Vec<console::Block> {
    GATED
        .iter()
        .copied()
        .filter(|b| app.progress.is_discovered(*b))
        .collect()
}

// ---------------------------------------------------------------- the bounded CI envelope

/// G19 envelope regression — **bounded** (seed 1337 only; ≤35 sim-min, early-out once every
/// measured quantity is in hand; the multi-seed sweep is the env-gated probe below). One player
/// action, as in the probe's run 1: allocate the first discovered gated block.
///
/// Post-truing arithmetic the bounds encode (recomputed at G19b, 2026-07-10; measured values
/// from this test's own run in parentheses):
/// - **First gated discovery ≤ 8 sim-min** (measured 0.3 — a monument label right on the
///   opening drift path; the 1/6 bearer rate + flat monument table aim the *across-seed* spread
///   at the 2–6 min envelope, and the bound guards the tail, not the lucky floor).
/// - **First comprehension ≤ 35 sim-min** (measured 8.8, block `circle`). A Schematics-tier
///   block costs 50 (`25 << 1`) at the measured ~3.2 steady domain-yield/min ⇒ ≈16 min of fill
///   after discovery + allocation (the richer early window fills faster). A Relics-first seed
///   would blow this bound *by design* (200 + four rare pickups) — seed 1337 discovers a
///   Schematics block first (measured; re-pin if worldgen or the label tables change).
/// - **Shard income within [6, 30] yield/min** over sim-minutes 2–10 (measured 23.4 — the
///   early window outpaces the 12.7–18.0 steady band; truing changes costs, not income). Wide
///   enough to absorb erosion trims and early-window richness, tight enough to catch an economy
///   flatline (the G19a seek/collect deadlock read ~0.2 y/min) or a runaway.
///
/// CI cost: ~9 s in debug on the dev container (early-out at the 10-sim-min income window,
/// 36 k real `run_frame` ticks); a no-comprehension run would cap at ~30 s before failing.
#[test]
fn pacing_envelope_seed_1337() {
    const CAP_MIN: usize = 35;
    const INCOME_START_MIN: usize = 2;
    const INCOME_END_MIN: usize = 10;
    let mut app = App::headless(1337);
    let mut meter = Meter::new(&app);
    let mut buckets = vec![Income::default(); CAP_MIN];
    let mut t_disc: Option<(f32, console::Block)> = None;
    let mut t_compr: Option<f32> = None;
    for f in 0..frames_of_minutes(CAP_MIN as f32) {
        app.run_frame(DT);
        let minute = ((f as f32 * DT) / 60.0) as usize;
        meter.tick(&app, &mut buckets[minute.min(CAP_MIN - 1)]);
        if t_disc.is_none() {
            if let Some(b) = gated_discovered(&app).first().copied() {
                t_disc = Some((app.time, b));
                // The probe's one fair player action: allocate the first find.
                app.progress.allocate(ResearchTarget::Block(b));
            }
        }
        if let (Some((_, b)), None) = (t_disc, t_compr) {
            if app.progress.is_block_comprehended(b) {
                t_compr = Some(app.time);
            }
        }
        // Early out: discovery + comprehension seen and the income window complete.
        if t_compr.is_some() && minute >= INCOME_END_MIN {
            break;
        }
    }
    let (t0, block) = t_disc.expect("no gated discovery within 35 sim-min (envelope: ≤8)");
    assert!(
        t0 <= 8.0 * 60.0,
        "first gated discovery took {:.1} sim-min (envelope: ≤8; block {})",
        t0 / 60.0,
        block.name()
    );
    let t1 = t_compr.unwrap_or_else(|| {
        let (fill, cost) = app.progress.research_progress(ResearchTarget::Block(block));
        panic!(
            "first comprehension ({}) not within 35 sim-min (fill {fill}/{cost})",
            block.name()
        )
    });
    assert!(
        t1 <= 35.0 * 60.0,
        "first comprehension ({}) took {:.1} sim-min (envelope: ≤35)",
        block.name(),
        t1 / 60.0
    );
    let mut win = Income::default();
    for b in &buckets[INCOME_START_MIN..INCOME_END_MIN] {
        win.add(b);
    }
    let ypm = win.shard_yield as f64 / (INCOME_END_MIN - INCOME_START_MIN) as f64;
    assert!(
        (6.0..=30.0).contains(&ypm),
        "shard income {ypm:.2} yield/min outside the [6, 30] envelope \
         (measured band 12.7–18.0 at the G19 truing)"
    );
    println!(
        "envelope seed 1337: discovery {:.1} min ({}), comprehension {:.1} min, income {ypm:.1} y/min",
        t0 / 60.0,
        block.name(),
        t1 / 60.0
    );
}

// ---------------------------------------------------------------- run 1: hands-off

#[test]
fn pacing_1_handsoff_income_discovery_first_comprehension() {
    if !full_probe_enabled() {
        return; // measurement probe — opt in with PACING_FULL=1 (see module docs)
    }
    println!("\n== RUN 1: pure autopilot (given routines only), 60 sim-min per seed ==");
    println!("   one player action allowed: allocate the first discovered gated block.");
    let mut all_rates: Vec<f64> = Vec::new();
    for &seed in &SEEDS {
        let mut app = App::headless(seed);
        let mut meter = Meter::new(&app);
        let total_min = 60usize;
        let mut buckets: Vec<Income> = vec![Income::default(); total_min];
        let mut t_first_disc: Option<(f32, console::Block)> = None;
        let mut t_compr: Option<f32> = None;
        let mut disc_times: Vec<(console::Block, f32)> = Vec::new();
        let mut known: Vec<console::Block> = Vec::new();
        let frames = frames_of_minutes(total_min as f32);
        for f in 0..frames {
            app.run_frame(DT);
            let minute = ((f as f32 * DT) / 60.0) as usize;
            meter.tick(&app, &mut buckets[minute.min(total_min - 1)]);
            // discovery watch
            for b in gated_discovered(&app) {
                if !known.contains(&b) {
                    known.push(b);
                    disc_times.push((b, app.time));
                    if t_first_disc.is_none() {
                        t_first_disc = Some((app.time, b));
                        // the one fair player action: allocate it
                        app.progress.allocate(ResearchTarget::Block(b));
                    }
                }
            }
            if t_compr.is_none() {
                if let Some((_, b)) = t_first_disc {
                    if app.progress.is_block_comprehended(b) {
                        t_compr = Some(app.time);
                    }
                }
            }
        }
        // steady-state window: minutes 10..60
        let mut win = Income::default();
        for b in &buckets[10..] {
            win.add(b);
        }
        let win_min = (total_min - 10) as f64;
        let ipm = win.items_total() as f64 / win_min;
        let ypm = win.shard_yield as f64 / win_min;
        all_rates.push(ypm);
        // stability: first half vs second half of the window
        let mut h1 = Income::default();
        let mut h2 = Income::default();
        for b in &buckets[10..35] {
            h1.add(b);
        }
        for b in &buckets[35..] {
            h2.add(b);
        }
        println!("\n-- seed {seed} --");
        match t_first_disc {
            Some((t, b)) => println!("first gated discovery: {:.1} min ({})", t / 60.0, b.name()),
            None => println!("first gated discovery: NEVER in 60 min"),
        }
        for (b, t) in &disc_times {
            println!("  discovered {} @ {:.1} min", b.name(), t / 60.0);
        }
        match (t_first_disc, t_compr) {
            (Some((t0, b)), Some(t1)) => println!(
                "first comprehension ({}): {:.1} min (fill took {:.1} min; cost {})",
                b.name(),
                t1 / 60.0,
                (t1 - t0) / 60.0,
                app.progress.research_cost(ResearchTarget::Block(b)),
            ),
            (Some((_, b)), None) => {
                let (f, c) = app.progress.research_progress(ResearchTarget::Block(b));
                let (rh, rn) = app
                    .progress
                    .research_rare_progress(ResearchTarget::Block(b));
                println!(
                    "first comprehension ({}): NOT in 60 min (fill {f}/{c} · r {rh}/{rn})",
                    b.name()
                );
            }
            _ => println!("first comprehension: n/a (nothing discovered)"),
        }
        println!(
            "steady income (min 10-60): {ipm:.2} shards/min, {ypm:.2} yield/min | halves {:.2} vs {:.2} y/min",
            h1.shard_yield as f64 / 25.0,
            h2.shard_yield as f64 / 25.0
        );
        println!(
            "  rarity mix c/u/r: {}/{}/{} ({:.1}%/{:.1}%/{:.1}%)",
            win.rarity[0],
            win.rarity[1],
            win.rarity[2],
            100.0 * win.rarity[0] as f64 / win.items_total().max(1) as f64,
            100.0 * win.rarity[1] as f64 / win.items_total().max(1) as f64,
            100.0 * win.rarity[2] as f64 / win.items_total().max(1) as f64,
        );
        print!("  per-domain items/min: ");
        for (i, d) in Stratum::ALL.iter().enumerate() {
            print!("{} {:.2}  ", d.label(), win.items[i] as f64 / win_min);
        }
        println!();
        print!("  per-domain data/min (inscriptions): ");
        for (i, d) in Stratum::ALL.iter().enumerate() {
            print!("{} {:.2}  ", d.label(), win.data[i] as f64 / win_min);
        }
        println!();
        println!(
            "  finds/min {:.2}; totals @60min: known {}, collected {}, shard items {}, bank {}",
            win.finds as f64 / win_min,
            app.progress.known_count(),
            app.progress.collected_count(),
            app.progress.shard_total_count(),
            app.progress.shard_bank()
        );
        // theoretical fill table from this seed's measured domain rates (G19 costs: 25 << depth)
        println!("  theoretical fill (cost/domain-yield-rate), this seed:");
        for (i, d) in Stratum::ALL.iter().enumerate() {
            let cost = 25u64 << i;
            let dom_ypm = win.shard_yield as f64 / win_min
                * (win.items[i] as f64 / win.items_total().max(1) as f64);
            let t = if dom_ypm > 0.0 {
                cost as f64 / dom_ypm
            } else {
                f64::INFINITY
            };
            println!(
                "    {} tier (cost {cost}): {:.1} min at {:.2} y/min",
                d.label(),
                t,
                dom_ypm
            );
        }
    }
    let max = all_rates.iter().cloned().fold(f64::MIN, f64::max);
    let min = all_rates.iter().cloned().fold(f64::MAX, f64::min);
    println!(
        "\nseed spread on yield/min: min {min:.2}, max {max:.2}, ratio {:.2}x",
        max / min.max(1e-9)
    );
}

// ---------------------------------------------------------------- run 2: faculties

#[test]
fn pacing_2_faculty_ladder() {
    if !full_probe_enabled() {
        return; // measurement probe — opt in with PACING_FULL=1 (see module docs)
    }
    println!("\n== RUN 2: faculty pacing — allocate Sensing at t=0, ride the ladder (Sensing→Reach→Drive), 300 sim-min cap ==");
    for &seed in &SEEDS {
        let mut app = App::headless(seed);
        let order = [Faculty::Sensing, Faculty::Reach, Faculty::Drive];
        app.progress.allocate(ResearchTarget::Faculty(order[0]));
        let mut next = 1usize;
        let mut level_times: Vec<(String, f32)> = Vec::new();
        let mut prev = app.progress.faculty_levels();
        let frames = frames_of_minutes(300.0);
        for _ in 0..frames {
            app.run_frame(DT);
            let lv = app.progress.faculty_levels();
            if lv != prev {
                for f in Faculty::ALL {
                    if lv[f.idx()] != prev[f.idx()] {
                        level_times.push((format!("{} L{}", f.label(), lv[f.idx()]), app.time));
                    }
                }
                prev = lv;
            }
            if app.progress.active_research().is_none() && next < order.len() {
                app.progress.allocate(ResearchTarget::Faculty(order[next]));
                next += 1;
            }
            if lv == [3, 3, 3] {
                break;
            }
        }
        print!("seed {seed}: ");
        for (name, t) in &level_times {
            print!("{name} @ {:.1}m  ", t / 60.0);
        }
        if prev != [3, 3, 3] {
            print!("(cap hit — final levels {prev:?})");
        }
        println!();
    }
}

// ---------------------------------------------------------------- run 3: the full ladder

#[test]
fn pacing_3_full_vocabulary_ladder() {
    if !full_probe_enabled() {
        return; // measurement probe — opt in with PACING_FULL=1 (see module docs)
    }
    println!("\n== RUN 3: full ladder — allocate every gated block as it's discovered (then faculties), 8 sim-h cap ==");
    for &seed in &SEEDS {
        let mut app = App::headless(seed);
        let mut done: Vec<(String, f32)> = Vec::new();
        let mut disc: Vec<(console::Block, f32)> = Vec::new();
        let mut known: Vec<console::Block> = Vec::new();
        let fac_order = [Faculty::Sensing, Faculty::Reach, Faculty::Drive];
        let frames = frames_of_minutes(480.0);
        let mut all_done_at: Option<f32> = None;
        for _ in 0..frames {
            app.run_frame(DT);
            for b in gated_discovered(&app) {
                if !known.contains(&b) {
                    known.push(b);
                    disc.push((b, app.time));
                }
            }
            // completion watch via active_research clearing
            if app.progress.active_research().is_none() {
                // record any newly comprehended blocks
                for b in GATED {
                    if app.progress.is_block_comprehended(b)
                        && !done.iter().any(|(n, _)| n == b.name())
                    {
                        done.push((b.name().to_string(), app.time));
                    }
                }
                // next target: a discovered, uncomprehended block, else next unmaxed faculty
                let next_block = GATED.iter().copied().find(|b| {
                    app.progress.is_discovered(*b) && !app.progress.is_block_comprehended(*b)
                });
                if let Some(b) = next_block {
                    app.progress.allocate(ResearchTarget::Block(b));
                } else if let Some(f) = fac_order
                    .iter()
                    .copied()
                    .find(|f| app.progress.faculty_levels()[f.idx()] < progress::MAX_FACULTY_LEVEL)
                {
                    app.progress.allocate(ResearchTarget::Faculty(f));
                }
            }
            let vocab_done = GATED.iter().all(|b| app.progress.is_block_comprehended(*b));
            let fac_done = app.progress.faculty_levels() == [3, 3, 3];
            if vocab_done && fac_done && all_done_at.is_none() {
                all_done_at = Some(app.time);
                break;
            }
        }
        println!("\n-- seed {seed} --");
        for (b, t) in &disc {
            println!("  discovered {} @ {:.1} min", b.name(), t / 60.0);
        }
        for (n, t) in &done {
            println!("  comprehended {} @ {:.1} min", n, t / 60.0);
        }
        let lv = app.progress.faculty_levels();
        match all_done_at {
            Some(t) => println!(
                "  ENTIRE ladder (4 blocks + faculties 3/3/3) done @ {:.2} h",
                t / 3600.0
            ),
            None => {
                let pending: Vec<String> = GATED
                    .iter()
                    .filter(|b| !app.progress.is_block_comprehended(**b))
                    .map(|b| {
                        let t = ResearchTarget::Block(*b);
                        let (f, c) = app.progress.research_progress(t);
                        let (rh, rn) = app.progress.research_rare_progress(t);
                        format!("{} {f}/{c}·r{rh}/{rn}", b.name())
                    })
                    .collect();
                println!(
                    "  NOT done in 8 h — faculties {lv:?}, blocks pending: {pending:?}, undiscovered: {:?}",
                    GATED
                        .iter()
                        .filter(|b| !app.progress.is_discovered(**b))
                        .map(|b| b.name())
                        .collect::<Vec<_>>()
                )
            }
        }
    }
}

// ---------------------------------------------------------------- run 4/5: expedition handshake

struct ExpStats {
    expeditions: u32,
    deposits: u32,
    deposited: u64,
    drains: u32,
    drained: u64,
    max_carry: u32,
    carry_full_frames: u64,
    bank: u64,
    items: u32,
}

/// Discover + comprehend a block through the canonical progress seam (the same helper shape as
/// e2e's; local so the probe stays self-contained).
fn grant(app: &mut App, b: console::Block, d: Stratum) {
    app.progress.apply(&Event::Discover { block: b });
    app.progress.allocate(ResearchTarget::Block(b));
    let mut guard = 0;
    while !app.progress.is_block_comprehended(b) && guard < 100_000 {
        app.progress.apply(&Event::CollectShard {
            domain: d,
            rarity: Rarity::Rare,
        });
        guard += 1;
    }
    assert!(
        app.progress.is_block_comprehended(b),
        "grant({b:?}) stalled"
    );
}

fn run_expedition(seed: u32, deposit_min_pct: u32, disable_ship_autocollect: bool) -> ExpStats {
    let mut app = App::headless(seed);
    // Scripted setup (player actions): grant seek + runfoot through the canonical research seam.
    grant(&mut app, console::Block::Seek, Stratum::Schematics);
    grant(&mut app, console::Block::RunFoot, Stratum::Relics);
    app.sync_console_unlock();
    // Authored wiring (player actions):
    // ship: continuous seek (nav to the nearest known site) — overrides drift (later routine wins)
    let nav = app.console.create_routine(console::Agent::Ship);
    app.console.routines[nav].trigger = console::Trigger::Continuous;
    app.console.routines[nav].body = vec![console::Step::Do(console::Block::Seek)];
    app.console.routines[nav].enabled = true;
    // ship: on arrive → run(foot) (deploy the walker)
    let exp = app.console.create_routine(console::Agent::Ship);
    app.console.routines[exp].trigger = console::Trigger::OnArrive;
    app.console.routines[exp].body = vec![console::Step::Do(console::Block::RunFoot)];
    app.console.routines[exp].enabled = true;
    // foot: when(carry ≥ pct) → deposit
    let dep = app.console.create_routine(console::Agent::Foot);
    app.console.routines[dep].trigger = console::Trigger::When(console::Cond {
        state: console::State::Carry,
        min: deposit_min_pct,
    });
    app.console.routines[dep].body = vec![console::Step::Do(console::Block::Deposit)];
    app.console.routines[dep].enabled = true;
    if disable_ship_autocollect {
        // isolate the handshake: turn off the given ship on-scan `collect` (routine index 3)…
        app.console.routines[3].enabled = false;
        // …and give the ship an authored drain: when(cache ≥ 1) → collect
        let dr = app.console.create_routine(console::Agent::Ship);
        app.console.routines[dr].trigger = console::Trigger::When(console::Cond {
            state: console::State::Cache,
            min: 1,
        });
        app.console.routines[dr].body = vec![console::Step::Do(console::Block::Collect)];
        app.console.routines[dr].enabled = true;
    }

    let mut st = ExpStats {
        expeditions: 0,
        deposits: 0,
        deposited: 0,
        drains: 0,
        drained: 0,
        max_carry: 0,
        carry_full_frames: 0,
        bank: 0,
        items: 0,
    };
    let mut was_active = false;
    let mut prev_cache = app.progress.cache_count();
    let frames = frames_of_minutes(60.0);
    for _ in 0..frames {
        app.run_frame(DT);
        let active = app.expedition.active();
        if active && !was_active {
            st.expeditions += 1;
        }
        was_active = active;
        let cache = app.progress.cache_count();
        if cache > prev_cache {
            st.deposits += 1;
            st.deposited += (cache - prev_cache) as u64;
        } else if cache < prev_cache {
            st.drains += 1;
            st.drained += (prev_cache - cache) as u64;
        }
        prev_cache = cache;
        let carry = app.progress.carry_count();
        st.max_carry = st.max_carry.max(carry);
        if app.progress.carry_is_full() {
            st.carry_full_frames += 1;
        }
    }
    st.bank = app.progress.shard_bank();
    st.items = app.progress.shard_total_count();
    st
}

/// RUN 5: the handshake with the expedition kicked the way a player would from the console:
/// re-run `run(foot)` whenever idle. (Kept from the pre-G19a probe — comparing it against RUN 4,
/// which exercises the now-fixed automated `on-arrive` path, shows what the fixes bought.)
fn run_expedition_kicked(seed: u32, deposit_min_pct: u32) -> (ExpStats, u64, f32, u32) {
    let mut app = App::headless(seed);
    grant(&mut app, console::Block::Seek, Stratum::Schematics);
    grant(&mut app, console::Block::RunFoot, Stratum::Relics);
    app.sync_console_unlock();
    let setup_items = app.progress.shard_total_count();
    // foot: when(carry >= pct) -> deposit (authored)
    let dep = app.console.create_routine(console::Agent::Foot);
    app.console.routines[dep].trigger = console::Trigger::When(console::Cond {
        state: console::State::Carry,
        min: deposit_min_pct,
    });
    app.console.routines[dep].body = vec![console::Step::Do(console::Block::Deposit)];
    app.console.routines[dep].enabled = true;
    let mut st = ExpStats {
        expeditions: 0,
        deposits: 0,
        deposited: 0,
        drains: 0,
        drained: 0,
        max_carry: 0,
        carry_full_frames: 0,
        bank: 0,
        items: 0,
    };
    let mut was_active = false;
    let mut prev_cache = app.progress.cache_count();
    let mut active_time = 0.0f32;
    let mut idle_since = 0.0f32;
    let frames = frames_of_minutes(60.0);
    for _ in 0..frames {
        // the scripted player action: re-run `run(foot)` from the console when it's been idle a
        // few seconds (player pace — also lets the away-walker tick deposit between runs)
        if !app.expedition.active() && app.time - idle_since > 5.0 {
            app.dispatch_block(console::Block::RunFoot);
        }
        if app.expedition.active() {
            idle_since = app.time;
        }
        app.run_frame(DT);
        let active = app.expedition.active();
        if active {
            active_time += DT;
        }
        if active && !was_active {
            st.expeditions += 1;
        }
        was_active = active;
        let cache = app.progress.cache_count();
        if cache > prev_cache {
            st.deposits += 1;
            st.deposited += (cache - prev_cache) as u64;
        } else if cache < prev_cache {
            st.drains += 1;
            st.drained += (prev_cache - cache) as u64;
        }
        prev_cache = cache;
        st.max_carry = st.max_carry.max(app.progress.carry_count());
        if app.progress.carry_is_full() {
            st.carry_full_frames += 1;
        }
    }
    st.bank = app.progress.shard_bank();
    st.items = app.progress.shard_total_count();
    let stranded = app.progress.cache_count() as u64 + app.progress.carry_count() as u64;
    (st, stranded, active_time, setup_items)
}

#[test]
fn pacing_5_expedition_kicked_manually() {
    if !full_probe_enabled() {
        return; // measurement probe — opt in with PACING_FULL=1 (see module docs)
    }
    println!("\n== RUN 5: handshake with `run(foot)` re-dispatched whenever idle (player-kicked), 60 sim-min ==");
    for (label, pct) in [
        ("deposit when carrying >=1", 12u32),
        ("deposit only when FULL", 100u32),
    ] {
        println!("variant: {label}, givens ON");
        for &seed in &SEEDS[..3] {
            let (s, stranded, active_t, setup_items) = run_expedition_kicked(seed, pct);
            println!(
                "  seed {seed}: expeditions/h {}, avg cycle {:.1} s, deposits/h {} ({} sh), drains/h {} ({} sh), max carry {}/{}, carry-full {:.1} s, stranded (carry+cache) {}, items {} (setup {})",
                s.expeditions,
                if s.expeditions > 0 { active_t / s.expeditions as f32 } else { 0.0 },
                s.deposits, s.deposited, s.drains, s.drained,
                s.max_carry, progress::CARRY_CAP,
                s.carry_full_frames as f64 * DT as f64,
                stranded, s.items, setup_items
            );
        }
    }
}

#[test]
fn pacing_4_expedition_handshake() {
    if !full_probe_enabled() {
        return; // measurement probe — opt in with PACING_FULL=1 (see module docs)
    }
    println!("\n== RUN 4: authored expedition (seek + on-arrive run(foot) + when(carry)->deposit), 60 sim-min ==");
    println!("variant A: deposit whenever carrying >=1 (pct 12), ship auto-collect ON (the default loop)");
    for &seed in &SEEDS[..3] {
        let s = run_expedition(seed, 12, false);
        println!(
            "  seed {seed}: expeditions/h {}, deposits/h {} ({} shards), drains/h {} ({} shards), max carry {}/{}, carry-full {:.1} s, bank {} ({} items)",
            s.expeditions, s.deposits, s.deposited, s.drains, s.drained, s.max_carry,
            progress::CARRY_CAP, s.carry_full_frames as f64 * DT as f64, s.bank, s.items
        );
    }
    println!("variant B: deposit only when carry FULL (pct 100), ship auto-collect OFF, authored when(cache>=1)->collect drain");
    for &seed in &SEEDS[..3] {
        let s = run_expedition(seed, 100, true);
        println!(
            "  seed {seed}: expeditions/h {}, deposits/h {} ({} shards), drains/h {} ({} shards), max carry {}/{}, carry-full {:.1} s, bank {} ({} items)",
            s.expeditions, s.deposits, s.deposited, s.drains, s.drained, s.max_carry,
            progress::CARRY_CAP, s.carry_full_frames as f64 * DT as f64, s.bank, s.items
        );
    }
    // context: the faculty ladder arithmetic
    println!(
        "(faculty costs {:?}; block costs 25<<depth: SCH 50, REL 200 + rare gate 4)",
        progress::FACULTY_COSTS,
    );
}
