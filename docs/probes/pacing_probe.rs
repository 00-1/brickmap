//! THROWAWAY pacing probe (not for commit) — drives the REAL per-frame loop headlessly
//! (`App::headless` + `run_frame`, exactly like e2e.rs) under pure autopilot (the given
//! routines only) and measures the pacing a player would experience. Prints tables with
//! `--nocapture`. Run: `cargo test --release -p scraped-again pacing_probe -- --nocapture`.

use super::*;
use crate::progress::{Event, Faculty, ResearchTarget, Stratum, FACULTY_COSTS};
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
        for i in 0..5 {
            self.items[i] += o.items[i];
            self.data[i] += o.data[i];
        }
        for i in 0..3 {
            self.rarity[i] += o.rarity[i];
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
        if best.map_or(true, |(s, _)| score > s) {
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
            for i in 0..3 {
                into.rarity[i] += dec[i];
            }
            into.shard_yield += b - self.bank;
        } else {
            // yield without items should not happen (drain emits items too) — but stay honest
            into.shard_yield += b - self.bank;
        }
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

// ---------------------------------------------------------------- run 1: hands-off

#[test]
fn pacing_1_handsoff_income_discovery_first_comprehension() {
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
            let d = gated_discovered(&app);
            for b in &d {
                if !known.contains(b) {
                    known.push(*b);
                    disc_times.push((*b, app.time));
                    if t_first_disc.is_none() {
                        t_first_disc = Some((app.time, *b));
                        // the one fair player action: allocate it
                        app.progress.allocate(ResearchTarget::Block(*b));
                    }
                }
            }
            if t_compr.is_none() {
                if let Some((t0, b)) = t_first_disc {
                    if app.progress.is_block_comprehended(b) {
                        t_compr = Some(app.time);
                        let _ = t0;
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
            Some((t, b)) => println!(
                "first gated discovery: {:.1} min ({})",
                t / 60.0,
                b.name()
            ),
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
                println!(
                    "first comprehension ({}): NOT in 60 min (fill {f}/{c})",
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
        // theoretical fill table from this seed's measured domain rates
        println!("  theoretical fill (cost/domain-yield-rate), this seed:");
        for (i, d) in Stratum::ALL.iter().enumerate() {
            let cost = 30 + 20 * i as u64;
            let dom_ypm = win.shard_yield as f64 / win_min * (win.items[i] as f64 / win.items_total().max(1) as f64);
            let t = if dom_ypm > 0.0 { cost as f64 / dom_ypm } else { f64::INFINITY };
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
    println!("\n== RUN 2: faculty pacing — allocate Sensing at t=0, ride the ladder (Sensing→Reach→Drive), 120 sim-min cap ==");
    for &seed in &SEEDS {
        let mut app = App::headless(seed);
        let order = [Faculty::Sensing, Faculty::Reach, Faculty::Drive];
        let mut next = 0usize;
        app.progress.allocate(ResearchTarget::Faculty(order[0]));
        next = 1;
        let mut level_times: Vec<(String, f32)> = Vec::new();
        let mut prev = app.progress.faculty_levels();
        let frames = frames_of_minutes(120.0);
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
    println!("\n== RUN 3: full ladder — allocate every gated block as it's discovered (then faculties), 5 sim-h cap ==");
    for &seed in &SEEDS {
        let mut app = App::headless(seed);
        let mut done: Vec<(String, f32)> = Vec::new();
        let mut disc: Vec<(console::Block, f32)> = Vec::new();
        let mut known: Vec<console::Block> = Vec::new();
        let fac_order = [Faculty::Sensing, Faculty::Reach, Faculty::Drive];
        let frames = frames_of_minutes(300.0);
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
                let next_block = GATED
                    .iter()
                    .copied()
                    .find(|b| app.progress.is_discovered(*b) && !app.progress.is_block_comprehended(*b));
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
            None => println!(
                "  NOT done in 5 h — faculties {lv:?}, blocks done {}/4, undiscovered: {:?}",
                GATED
                    .iter()
                    .filter(|b| app.progress.is_block_comprehended(**b))
                    .count(),
                GATED
                    .iter()
                    .filter(|b| !app.progress.is_discovered(**b))
                    .map(|b| b.name())
                    .collect::<Vec<_>>()
            ),
        }
    }
}

// ---------------------------------------------------------------- run 4: expedition / handshake

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

fn run_expedition(seed: u32, deposit_min_pct: u32, disable_ship_autocollect: bool) -> ExpStats {
    let mut app = App::headless(seed);
    // Scripted setup (player actions): grant seek + runfoot through the canonical research seam.
    for (b, d) in [
        (console::Block::Seek, Stratum::Schematics),
        (console::Block::RunFoot, Stratum::Relics),
    ] {
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
    }
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

/// RUN 5: OnArrive is unreachable from the air (see RUN 4), so measure the handshake with the
/// expedition kicked the way a player would from the console: re-run `run(foot)` whenever idle.
fn run_expedition_kicked(seed: u32, deposit_min_pct: u32) -> (ExpStats, u64, f32, u32) {
    let mut app = App::headless(seed);
    for (b, d) in [
        (console::Block::Seek, Stratum::Schematics),
        (console::Block::RunFoot, Stratum::Relics),
    ] {
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
    }
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
    println!("\n== RUN 5: handshake with `run(foot)` re-dispatched whenever idle (player-kicked), 60 sim-min ==");
    for (label, pct) in [("deposit when carrying >=1", 12u32), ("deposit only when FULL", 100u32)] {
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
    // context: faculty ladder arithmetic
    println!(
        "(faculty costs {:?}; block costs: Schematics 50, Relics 90; DECODE_COST const {} is orphaned)",
        FACULTY_COSTS,
        progress::DECODE_COST
    );
}
