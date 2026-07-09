//! D11 — **end-to-end play harness**. These tests drive the *real* per-frame game loop
//! ([`App::run_frame`], the same method the live window calls) on a headless [`App::headless`]
//! instance (`state: None`, no GPU/window/audio). Everything G9–G16 is unit-tested in isolation;
//! what was never exercised is the **orchestration over time** — autopilot → autoscan →
//! on-scan/continuous auto-collect → shard-intake → research-fill → comprehend → legibility,
//! the two-agent expedition, streaming-driven `collectible`/map state, and the persistence
//! round-trip. This module asserts that integration *progresses* and holds its invariants, plus a
//! bounded seeded soak that fuzzes the loop (no panic / NaN / overflow / unbounded growth).
//!
//! Faithfulness (brief Decision 3): we never re-implement the frame loop — every tick is a real
//! `run_frame` call. Where a milestone needs a *bounded, deterministic* amount of economy input
//! (filling a research bar), we feed shards through the canonical [`progress::Event::CollectShard`]
//! seam — the exact event the loop's auto-collect emits — rather than gambling on world shard luck
//! along an autopilot wander (which would flake in CI). That the live loop *does* drive that seam
//! is asserted independently in [`real_loop_plays_and_progresses`].
//!
//! CI: the CPU-only tests below run under `cargo test --all` (no GPU). The render-robustness sweep
//! needs a (software) Vulkan adapter and is `#[ignore]` — run locally / opt-in.

use super::*;
use crate::progress::{Event, Faculty, ResearchTarget, Stratum};
use crate::shards::Rarity;

/// A fixed simulation step — frame-rate-independent + deterministic (the live path derives this
/// from the frame clock; the harness pins it so a seed alone fully determines a run).
const DT: f32 = 1.0 / 60.0;

/// Fail fast on any non-finite sim float — the soak/integration invariant "no NaN/inf ever".
fn assert_finite(app: &App, ctx: &str) {
    assert!(app.camera.position.is_finite(), "{ctx}: camera NaN/inf");
    assert!(app.cruiser_pos.is_finite(), "{ctx}: cruiser NaN/inf");
    assert!(app.walker_pos.is_finite(), "{ctx}: walker NaN/inf");
    assert!(
        app.time.is_finite() && app.auto_fly_angle.is_finite() && app.auto_fly_t.is_finite(),
        "{ctx}: clock/heading NaN/inf"
    );
    assert!(
        app.wobble.is_finite() && app.color_steps.is_finite(),
        "{ctx}: aesthetic dials NaN/inf"
    );
}

/// Drive `ticks` real frames, asserting finiteness every step.
fn drive(app: &mut App, ticks: usize) {
    for _ in 0..ticks {
        app.run_frame(DT);
        assert_finite(app, "drive");
    }
}

/// Ground-height closure for the seed (the same query worldgen/streaming use).
fn ground_fn(seed: u32) -> impl Fn(f32, f32) -> f32 {
    move |x: f32, z: f32| worldgen::height(x.floor() as i32, z.floor() as i32, seed) as f32
}

/// Locate the nearest colossus to the origin whose monument names a **gated** block (so its
/// research is non-trivial) — deterministic from the seed, the same placements the streamer sees.
fn nearest_gated_name_bearer(seed: u32) -> (console::Block, Stratum, Vec3) {
    let g = ground_fn(seed);
    let mut found: Vec<structures::Inscription> =
        structures::colossi_near(seed, Vec3::ZERO, 3000.0, g)
            .iter()
            .map(structures::colossus_label)
            .filter(|insc| insc.name.is_some_and(|b| b.required().is_some()))
            .collect();
    found.sort_by(|a, b| a.pos.length_squared().total_cmp(&b.pos.length_squared()));
    let insc = found
        .into_iter()
        .next()
        .expect("a gated name-bearing colossus exists near the origin");
    let block = insc.name.unwrap();
    (block, block.required().unwrap(), insc.pos)
}

// ----------------------------------------------------------------------------------------------
// 1) The real loop actually *plays* — the headline "does it run end to end?" integration test.
// ----------------------------------------------------------------------------------------------

#[test]
fn real_loop_plays_and_progresses() {
    // A few seeds, so we don't lean on one lucky world. The default console is the hands-off
    // opening loop (drift + scan sites + scan shards + on-scan collect), so a bare run should
    // wander, scan, and bank without any scripted input.
    for &seed in &[1337u32, 7, 2024, 99999] {
        let mut app = App::headless(seed);
        let start = app.camera.position;
        let start_time = app.time;

        drive(&mut app, 2000);

        // Time advanced by ~ticks·DT (the clock is driven inside the real frame).
        assert!(
            (app.time - start_time - 2000.0 * DT).abs() < 1.0,
            "seed {seed}: sim clock didn't advance as expected ({})",
            app.time
        );
        // The autopilot wandered the ship to new terrain (drift nav is live).
        assert!(
            (app.camera.position - start).length() > 50.0,
            "seed {seed}: ship barely moved under autopilot ({:?} → {:?})",
            start,
            app.camera.position
        );
        // The scan→known-site wiring fired (survey routine + autoscan + map opportunity surface).
        assert!(
            app.progress.known_count() > 0,
            "seed {seed}: no sites became known over 2000 ticks of autopilot+autoscan"
        );
        // The on-scan/continuous auto-collect → economy wiring fired: *something* was banked
        // (inscriptions collected and/or shards swept). This is the per-frame integration the
        // unit tests can't see, and the proof the live loop drives `CollectShard`/`Collect`.
        let banked = app.progress.collected_count() as u64
            + app.progress.shard_total_count() as u64
            + app.progress.strata.total();
        assert!(
            banked > 0,
            "seed {seed}: the hands-off loop banked nothing over 2000 ticks"
        );
    }
}

// ----------------------------------------------------------------------------------------------
// 2) A scripted progression walks every milestone through the real seams.
// ----------------------------------------------------------------------------------------------

#[test]
fn scripted_progression_discover_research_author_expedition_roundtrip() {
    let seed = 1337u32;
    let mut app = App::headless(seed);

    // --- Discover: fly to a gated name-bearing colossus and collect it (the real collect →
    //     discover wiring: stream inscriptions → `collectible` → `collect_index` → `Discover`). ---
    let (block, domain, bearer_pos) = nearest_gated_name_bearer(seed);
    assert!(
        !app.progress.is_discovered(block),
        "block starts undiscovered"
    );
    app.auto_fly = false; // take the helm (scripted) so we hold position at the monument
    app.camera.position = bearer_pos;
    app.update_inscriptions(); // stream the monument into range as a collectible
    let idx = app
        .collectible
        .iter()
        .position(|c| c.name == Some(block))
        .expect("the name-bearer streamed in as a collectible at our position");
    app.collect_index(idx);
    app.sync_console_unlock();
    assert!(
        app.progress.is_discovered(block),
        "collecting the name-bearer discovered its block"
    );

    // --- Research: the manual console path allocates the discovered-but-locked block. ---
    app.dispatch_block(block);
    assert_eq!(
        app.progress.active_research(),
        Some(ResearchTarget::Block(block)),
        "running a locked block allocates it as the research target"
    );
    assert!(!app.progress.is_block_comprehended(block));
    let script = progress::script_for(domain);
    assert!(!app.progress.is_legible(script), "stratum starts illegible");

    // --- Fill: feed the block's own-domain shards through the canonical CollectShard seam (the
    //     same event the loop's auto-collect emits) until research completes. Bounded + exact. ---
    let mut guard = 0;
    while !app.progress.is_block_comprehended(block) && guard < 100_000 {
        app.progress.apply(&Event::CollectShard {
            domain,
            rarity: Rarity::Rare,
        });
        guard += 1;
    }
    assert!(
        app.progress.is_block_comprehended(block),
        "domain-matched shards filled the research → comprehended"
    );
    assert_eq!(
        app.progress.active_research(),
        None,
        "completing research clears the active target"
    );
    assert!(
        app.progress.is_legible(script),
        "comprehending a block folds in its stratum's legibility"
    );

    // --- Author + run a routine: a continuous Collect routine must emit a Collect act through the
    //     same interpreter `run_frame` ticks. ---
    let rid = app.console.create_routine(console::Agent::Ship);
    app.console.routines[rid].trigger = console::Trigger::Continuous;
    app.console.routines[rid].body = vec![console::Step::Do(console::Block::Collect)];
    app.console.routines[rid].enabled = true;
    app.sync_console_unlock();
    let tick = app.console.tick(console::Agent::Ship, 0, 0, 0, 0, false);
    assert!(
        tick.acts
            .iter()
            .any(|a| a.block == console::Block::Collect && a.routine == rid),
        "the authored continuous routine emits a Collect act"
    );
    // And the loop runs with it live, without panic / NaN.
    drive(&mut app, 200);

    // --- Expedition cycle: with a known site, deploy the walker and drive the phase machine
    //     (Deploy → Harvest → Return → Idle) through the real loop. ---
    app.auto_fly = false;
    drive(&mut app, 300); // let inscriptions stream so `seek_target` has a site
    if app.seek_target().is_some() {
        app.start_expedition();
        assert!(app.expedition.active(), "expedition deployed");
        let mut seen_harvest = false;
        let mut completed = false;
        for _ in 0..6000 {
            app.run_frame(DT);
            assert_finite(&app, "expedition");
            if app.expedition.phase == expedition::Phase::Harvest {
                seen_harvest = true;
            }
            if !app.expedition.active() {
                completed = true;
                break;
            }
        }
        assert!(
            seen_harvest && completed,
            "the expedition advanced through Harvest and returned (phase machine driven by the loop)"
        );
    }

    // --- State round-trip: the full share string restores the meaningful state. ---
    let s = app.share_string();
    let restored_progress = progress::Progress::decode(&s);
    assert_eq!(
        restored_progress, app.progress,
        "progress round-trips through the share string"
    );
    let mut restored_console = console::Console::default();
    restored_console.restore(&s);
    assert_eq!(
        restored_console.encode(),
        app.console.encode(),
        "authored routines round-trip through the share string"
    );
}

// ----------------------------------------------------------------------------------------------
// 2b) G17 — the expedition **handshake** end to end: the walker fills its carry (capped), deposits
//     into the world cache, the ship drains the cache home, and the value banks + credits research
//     through the canonical events. Driven through the real App dispatch helpers.
// ----------------------------------------------------------------------------------------------

#[test]
fn handshake_carry_deposit_ship_drain_banks_value() {
    use crate::progress::CARRY_CAP;
    let mut app = App::headless(1337);

    // The walker collects until its carry caps — the honest per-agent scarcity (foot `collect`
    // routes shards into carry via the real dispatch helper; value is NOT banked yet).
    for _ in 0..CARRY_CAP {
        app.progress.carry_shard(Stratum::Relics, Rarity::Common);
    }
    assert!(app.progress.carry_is_full());
    assert_eq!(
        app.progress.shard_bank(),
        0,
        "carry is in transit, not banked"
    );

    // `deposit` (the real App dispatch effect) moves the carry into the cache + places its marker.
    let moved = app.deposit_carry();
    assert_eq!(moved, CARRY_CAP);
    assert_eq!(app.progress.cache_count(), CARRY_CAP);
    assert_eq!(app.progress.carry_count(), 0);
    assert!(
        app.cache_pos.is_some(),
        "the deposit placed a world cache marker"
    );
    assert!(
        !app.cache_marker_splats().is_empty(),
        "a non-empty cache renders a (budgeted) marker"
    );
    assert!(
        app.cache_marker_splats().len() <= super::CACHE_MARKER_CAP,
        "the marker stays within its splat budget"
    );

    // The ship lands near the cache and `collect`s — the drain banks the haul home (Decision 2:
    // value lands on ship pickup) via canonical CollectShard events.
    app.cruiser_pos = app.cache_pos.unwrap();
    let before = app.progress.shard_bank();
    let (items, yields) = app.drain_cache_if_near();
    assert_eq!(items, CARRY_CAP, "the ship drained the whole cache");
    assert!(
        yields > 0 && app.progress.shard_bank() > before,
        "value banked on pickup"
    );
    assert_eq!(app.progress.shard_count(Stratum::Relics), CARRY_CAP);
    assert_eq!(app.progress.cache_count(), 0);
    assert!(
        app.cache_pos.is_none(),
        "an emptied cache clears its marker"
    );
    assert!(
        app.cache_marker_splats().is_empty(),
        "no marker once the cache is empty"
    );
}

#[test]
fn handshake_funds_research_and_loop_runs_with_a_cache() {
    use crate::progress::{Faculty, ResearchTarget};
    let mut app = App::headless(7);
    // A faculty under research draws from any domain — the drained cache should fill it.
    app.progress
        .allocate(ResearchTarget::Faculty(Faculty::Drive));
    for _ in 0..crate::progress::CARRY_CAP {
        app.progress.carry_shard(Stratum::Signals, Rarity::Rare);
    }
    app.deposit_carry();
    app.cruiser_pos = app.cache_pos.unwrap();
    app.drain_cache_if_near();
    let (filled, _cost) = app
        .progress
        .research_progress(ResearchTarget::Faculty(Faculty::Drive));
    assert!(
        filled > 0 || app.progress.faculty_levels()[Faculty::Drive.idx()] > 0,
        "the drained cache fed the active research (canonical credit path)"
    );
    // The live loop ticks cleanly with a (now-empty) cache + handshake wiring in place.
    drive(&mut app, 200);
}

// ----------------------------------------------------------------------------------------------
// 3) Determinism — same seed + same scripted inputs → identical state (the E12 promise at the
//    economy/world level, not just voxels).
// ----------------------------------------------------------------------------------------------

#[test]
fn determinism_same_seed_same_inputs_same_state() {
    let run = |seed: u32| -> (String, String, Vec3, f32) {
        let mut app = App::headless(seed);
        drive(&mut app, 1500);
        (
            app.progress.encode(),
            app.console.encode(),
            app.camera.position,
            app.time,
        )
    };
    let a = run(2024);
    let b = run(2024);
    assert_eq!(a.0, b.0, "progress diverged for the same seed+inputs");
    assert_eq!(a.1, b.1, "console diverged for the same seed+inputs");
    assert_eq!(a.2, b.2, "camera diverged (bit-exact) for the same seed");
    assert_eq!(a.3, b.3, "clock diverged for the same seed");
    // A different seed yields a different world/run (sanity — not a fixed fiction).
    let c = run(2025);
    assert!(
        a.2 != c.2 || a.0 != c.0,
        "different seeds produced identical runs"
    );
}

// ----------------------------------------------------------------------------------------------
// 4) Persistence fidelity — round-trip equality + graceful handling of malformed/old payloads.
// ----------------------------------------------------------------------------------------------

#[test]
fn persistence_round_trip_and_malformed_is_graceful() {
    let seed = 7u32;
    let mut app = App::headless(seed);
    // Play forward + author + research so there's real state to carry.
    drive(&mut app, 800);
    let (block, domain, _) = nearest_gated_name_bearer(seed);
    app.progress.apply(&Event::Discover { block });
    app.progress.allocate(ResearchTarget::Block(block));
    for _ in 0..30 {
        app.progress.apply(&Event::CollectShard {
            domain,
            rarity: Rarity::Common,
        });
    }
    app.console.create_routine(console::Agent::Foot);

    let s = app.share_string();
    assert_eq!(progress::Progress::decode(&s), app.progress);
    let mut c2 = console::Console::default();
    c2.restore(&s);
    assert_eq!(c2.encode(), app.console.encode());

    // Malformed payloads must never panic — they fall back to defaults (lenient decode).
    for bad in [
        "",
        "#",
        "garbage",
        "pg=zzzz&co=!!!!",
        "pg=00",
        "s=notanumber&pg=&co=",
        "&&&=&pg=ffff",
    ] {
        let _ = share::ShareState::decode(bad, app.current_share());
        let p = progress::Progress::decode(bad);
        assert_eq!(p, progress::Progress::default(), "malformed pg= → default");
        let mut c = console::Console::default();
        c.restore(bad); // must not panic
    }
}

// ----------------------------------------------------------------------------------------------
// 5) Bounded seeded soak/fuzz — many ticks under random *valid* edits; no panic / NaN / overflow /
//    unbounded growth. Heavy run is env-gated (E2E_SOAK_TICKS); CI runs the bounded default.
// ----------------------------------------------------------------------------------------------

/// A tiny deterministic LCG so the fuzz is seeded + reproducible (no rand crate).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

#[test]
fn soak_random_valid_edits_holds_invariants() {
    let ticks: usize = std::env::var("E2E_SOAK_TICKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let mut app = App::headless(424242);
    let mut rng = Lcg(0x5151_2323);
    let starters = [
        console::Block::Collect,
        console::Block::Drift,
        console::Block::FireBeam,
        console::Block::Scan(console::ScanItem::Sites),
        console::Block::Scan(console::ScanItem::Shards),
    ];

    for i in 0..ticks {
        // Occasionally inject a random *valid* player edit (the kinds a UI can produce).
        match rng.below(60) {
            0 => {
                // Allocate a random faculty as research (a valid no-typing action).
                let f = Faculty::ALL[rng.below(3) as usize];
                app.spend_action(f);
            }
            1 => {
                // Author + enable a random single-block continuous routine (a starter, always
                // comprehended → never an illegal step).
                let rid = app.console.create_routine(console::Agent::Ship);
                let b = starters[rng.below(starters.len() as u64) as usize];
                app.console.routines[rid].trigger = console::Trigger::Continuous;
                app.console.routines[rid].body = vec![console::Step::Do(b)];
                app.console.routines[rid].enabled = true;
            }
            2 => {
                // Toggle a random existing routine.
                if !app.console.routines.is_empty() {
                    let k = rng.below(app.console.routines.len() as u64) as usize;
                    app.console.toggle_routine(k);
                }
            }
            _ => {}
        }

        app.run_frame(DT);
        assert_finite(&app, "soak");

        // Bounded growth — no per-frame leak in any of the streamed/derived vectors.
        assert!(
            app.collectible.len() < 100_000,
            "collectible unbounded @{i}"
        );
        assert!(app.flicks.len() < 100_000, "scan flicks unbounded @{i}");
        assert!(app.shards.len() < 100_000, "shards unbounded @{i}");
        assert!(
            app.console.routines.len() < 100_000,
            "routines unbounded @{i}"
        );
        assert!(app.progress.codex.len() < 1_000_000, "codex unbounded @{i}");

        // Honesty (G11): a routine the interpreter reports as Running must actually have fired
        // this tick — no "running" while silently stuck (cross-checks state vs the executing step).
        for r in &app.console.routines {
            if matches!(r.stats.state, console::RState::Running) {
                assert!(
                    r.stats.executing_step.is_some(),
                    "@{i}: routine '{}' claims Running with no executing step",
                    r.name
                );
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------
// 6) The u64 economy survives a huge intake — no overflow / panic, the bank keeps a faithful tally.
// ----------------------------------------------------------------------------------------------

#[test]
fn economy_survives_large_shard_intake() {
    let mut app = App::headless(11);
    app.progress
        .allocate(ResearchTarget::Faculty(Faculty::Drive));
    // A torrent of rare shards: bank grows monotonically, the faculty caps, nothing overflows.
    for _ in 0..500_000 {
        app.progress.apply(&Event::CollectShard {
            domain: Stratum::Records,
            rarity: Rarity::Rare,
        });
    }
    assert_eq!(
        app.progress.shard_bank(),
        500_000 * Rarity::Rare.yield_amount(),
        "the bank is the faithful lifetime tally (no overflow/wrap)"
    );
    assert_eq!(
        app.progress.faculty_levels()[Faculty::Drive.idx()],
        progress::MAX_FACULTY_LEVEL,
        "the faculty research caps cleanly under a huge intake"
    );
    // The loop still ticks cleanly with a maxed-out economy.
    drive(&mut app, 100);
}

// ----------------------------------------------------------------------------------------------
// 7) Regression asserts for confirmed bugs from the parallel hunt (the D11 contract: encode each
//    fixed bug here). BUG1/BUG2 also have unit guards in share/shards/structures/screenshot.
// ----------------------------------------------------------------------------------------------

/// BUG1 (adversarial hunt, 2026-06-11): an extreme-but-finite camera coordinate saturates the
/// `(x / CELL) as i32` cast to `i32::MAX`, where the streamers' `cell ± reach` used to overflow.
/// At the *integration* level: a share link clamps to `±POS_BOUND` (the trust boundary), **and**
/// driving the real loop at such a coordinate is panic-free (the `*_near` saturating bounds).
#[test]
fn bug1_extreme_coords_clamp_and_dont_crash_the_loop() {
    // The trust boundary clamps a crafted extreme share coordinate.
    let st =
        share::ShareState::decode("#v=1&s=1&z=300000000000", default_share()).expect("decodes");
    assert!(st.pos[2].abs() <= share::POS_BOUND, "extreme z clamped");

    // Defense-in-depth: even an unclamped extreme camera position survives a real frame (the
    // streamers/autoscan/inscription+shard grids all saturate their cell bounds).
    let mut app = App::headless(1);
    app.auto_fly = false; // hold the (extreme) position we set, don't let autopilot reset it
    for v in [3.0e11_f32, -3.0e11, f32::MAX / 4.0] {
        app.camera.position = Vec3::new(v, 40.0, v);
        app.run_frame(DT); // must not panic/overflow
    }
}

/// A default `ShareState` for BUG1's decode test (mirrors the live default view's non-position
/// fields; only the position matters here).
fn default_share() -> share::ShareState {
    share::ShareState {
        seed: 0,
        pos: [0.0; 3],
        yaw: 0.0,
        pitch: 0.0,
        wobble: 85.0,
        color_steps: 4.0,
        toggles: 0,
    }
}

// ----------------------------------------------------------------------------------------------
// 7b) G19a regression asserts — the nav/expedition wiring the first quantitative playtest proved
//     dead (docs/pacing-analysis.md, 2026-06-16). Every scenario drives the REAL frame loop; none
//     kicks the fixed machinery directly (the gap that let the old expedition scenario pass while
//     `arrived_at` could never fire in flight).
// ----------------------------------------------------------------------------------------------

/// Discover + comprehend a gated block through the canonical progress seam (Discover → allocate →
/// domain-shard fill) so authored routines using it are legal vocabulary — no test backdoor.
fn comprehend(app: &mut App, block: console::Block) {
    app.progress.apply(&Event::Discover { block });
    let Some(domain) = block.required() else {
        return; // a starter — always comprehended
    };
    assert!(
        app.progress.allocate(ResearchTarget::Block(block)),
        "allocate({block:?}) as the research target"
    );
    let mut guard = 0;
    while !app.progress.is_block_comprehended(block) && guard < 100_000 {
        app.progress.apply(&Event::CollectShard {
            domain,
            rarity: Rarity::Rare,
        });
        guard += 1;
    }
    assert!(
        app.progress.is_block_comprehended(block),
        "comprehend({block:?}) stalled"
    );
    app.sync_console_unlock();
}

/// G19a fix 1 + 3: an **expedition from flight through the real `on-arrive` path**. The ship flies
/// a `seek` nav at cruise altitude; an authored `on-arrive → run(foot)` routine must deploy the
/// walker when `arrived_at` fires (horizontal distance — pre-fix the ~22 u vertical gap alone kept
/// the 3-D distance above the radius and this NEVER happened: measured 0 automated
/// expeditions/hour on every seed). The expedition must then harvest, return, **auto-deposit** the
/// walker's carry into the site cache on Return→Idle, and the handshake must bank the value.
#[test]
fn expedition_from_flight_fires_through_the_real_on_arrive() {
    let seed = 1337u32;
    let mut app = App::headless(seed);
    // Author the advertised loop through the canonical seams: comprehend `seek` + `run(foot)`,
    // steer the given nav routine to seek, and wire `on-arrive → run(foot)`.
    comprehend(&mut app, console::Block::Seek);
    comprehend(&mut app, console::Block::RunFoot);
    app.console.routines[0].body = vec![console::Step::Do(console::Block::Seek)];
    // Isolate the on-arrive path: the given on-scan collect would otherwise harvest the target
    // from the air (reach 45 > arrive 20) before the ship ever counts as arrived.
    for r in app.console.routines.iter_mut() {
        if matches!(r.trigger, console::Trigger::OnScan) {
            r.enabled = false;
        }
    }
    let rid = app.console.create_routine(console::Agent::Ship);
    app.console.routines[rid].trigger = console::Trigger::OnArrive;
    app.console.routines[rid].body = vec![console::Step::Do(console::Block::RunFoot)];
    app.console.routines[rid].enabled = true;
    app.sync_console_unlock();
    let bank_before = app.progress.shard_bank();

    // Fly the loop for real: streaming fills the site field, seek closes on the nearest site at
    // cruise height, arrival fires the routine, and the phase machine runs a full cycle.
    let mut deployed = false;
    let mut seen_harvest = false;
    let mut injected_return_carry = false;
    let mut completed = false;
    for _ in 0..30_000 {
        app.run_frame(DT);
        assert_finite(&app, "on-arrive expedition");
        if app.expedition.active() {
            deployed = true;
        }
        if app.expedition.phase == expedition::Phase::Harvest {
            seen_harvest = true;
        }
        if app.expedition.phase == expedition::Phase::Return && !injected_return_carry {
            // Guarantee the walker returns *carrying* value (harvest shard luck varies by site)
            // so the Return→Idle auto-deposit is deterministically observable.
            app.progress.carry_shard(Stratum::Records, Rarity::Common);
            injected_return_carry = true;
        }
        if deployed && !app.expedition.active() {
            completed = true;
            break; // assert at the transition frame, before any later tick can drain the cache
        }
    }
    assert!(
        deployed,
        "seek → arrive → run(foot) deployed an expedition from cruise altitude \
         (pre-G19a `arrived_at` never fired in flight)"
    );
    assert!(
        app.console.routines[rid].stats.fires > 0,
        "the on-arrive routine itself fired (the real trigger path, not a direct kick)"
    );
    assert!(seen_harvest, "the expedition reached Harvest");
    assert!(completed, "the expedition returned and went Idle");
    assert!(injected_return_carry, "the Return phase was observed");
    // Fix 3: Return→Idle auto-deposited the carry into the site cache (no foot routine ticked).
    assert_eq!(
        app.progress.carry_count(),
        0,
        "auto-deposit emptied the walker's carry on Return→Idle"
    );
    assert!(
        app.progress.cache_count() > 0,
        "the carry landed in the site cache"
    );
    // The handshake banks the value: the ship, holding over the site, drains the cache home.
    let (items, _) = app.drain_cache_if_near();
    assert!(items > 0, "the holding ship drained the site cache");
    assert!(
        app.progress.shard_bank() > bank_before,
        "the handshake banked value"
    );
}

/// G19a fix 2: the **seek + on-scan collect loop makes progress on a fully-known site field**.
/// Pre-fix, on-scan only fired when a site became *newly* known, so once everything in the cone
/// was already known no collect ever ran again — the ship orbited one uncollected site forever
/// (measured: ~+14 yield/h vs ~950 under drift). The scan pulse now re-hits known-but-uncollected
/// sites, so seek → scan-hit → collect → move-on keeps cycling.
#[test]
fn seek_collect_loop_progresses_on_a_known_site_field() {
    fn mark_all_known(app: &mut App) {
        let ids: Vec<u64> = app.collectible.iter().map(|c| c.find_id).collect();
        for id in ids {
            app.progress.scan(id);
        }
    }
    let seed = 1337u32;
    let mut app = App::headless(seed);
    comprehend(&mut app, console::Block::Seek);
    app.console.routines[0].body = vec![console::Step::Do(console::Block::Seek)];
    app.sync_console_unlock();
    drive(&mut app, 300); // stream the opening site field in
    assert!(
        !app.collectible.is_empty(),
        "sites streamed in near the start"
    );

    // The deadlock precondition: every site the run ever sees is *already known* — mark the field
    // scanned up front and re-mark each frame (streaming keeps adding sites), so no "fresh" scan
    // hit can ever fire. Only the re-hit path can collect here.
    let before = app.progress.collected_count();
    let mut collected = 0;
    for _ in 0..20_000 {
        mark_all_known(&mut app);
        app.run_frame(DT);
        assert_finite(&app, "seek/collect known field");
        collected = app.progress.collected_count() - before;
        if collected >= 2 {
            break; // ≥2 proves the move-on (arrive → collect → next site), not a one-off
        }
    }
    assert!(
        collected >= 2,
        "seek + on-scan collect progressed on a fully-known field (collected {collected} \
         sites; pre-G19a this deadlocked at 0)"
    );
}

/// G19a fix 3 in isolation: a directly-kicked expedition (the G17-style scripted helm) whose
/// walker carries shards **auto-deposits into the site cache on Return→Idle** — foot routines
/// don't tick between back-to-back expeditions, so `when(carry) → deposit` alone can starve.
#[test]
fn expedition_return_auto_deposits_the_carry() {
    let seed = 7u32;
    let mut app = App::headless(seed);
    drive(&mut app, 300); // stream sites in
    let target = app
        .seek_target()
        .expect("a site streamed in near the start");
    // Park the piloted ship just off the site at cruise height and kick the expedition.
    app.auto_fly = false;
    let g = worldgen::height(target.x.floor() as i32, target.z.floor() as i32, seed) as f32;
    app.camera.position = Vec3::new(target.x + 6.0, g + CRUISE_HEIGHT, target.z);
    app.start_expedition();
    assert!(app.expedition.active(), "expedition deployed");
    app.progress.carry_shard(Stratum::Relics, Rarity::Common);
    // Keep the ship's own collect quiet so the cache is observably the auto-deposit's work.
    for r in app.console.routines.iter_mut() {
        if matches!(r.trigger, console::Trigger::OnScan) {
            r.enabled = false;
        }
    }
    let mut completed = false;
    for _ in 0..6000 {
        app.run_frame(DT);
        assert_finite(&app, "auto-deposit expedition");
        if !app.expedition.active() {
            completed = true;
            break;
        }
    }
    assert!(completed, "the expedition completed its cycle");
    assert_eq!(
        app.progress.carry_count(),
        0,
        "Return→Idle auto-deposited the walker's carry"
    );
    assert!(
        app.progress.cache_count() > 0,
        "the deposited carry sits in the site cache"
    );
}

// ----------------------------------------------------------------------------------------------
// 8) Render-robustness sweep — several vantages + after state changes render without panic/NaN.
//    Needs a (software) Vulkan adapter, so it's `#[ignore]` (local / opt-in, not CI).
// ----------------------------------------------------------------------------------------------

#[test]
#[ignore = "needs a Vulkan adapter (llvmpipe); run locally: cargo test -- --ignored"]
fn render_robustness_sweep() {
    let dir = std::env::temp_dir();
    let shots: &[(&str, Option<Vec3>, Option<Vec3>)] = &[
        ("e2e-default.png", None, None),
        (
            "e2e-low.png",
            Some(Vec3::new(0.0, 8.0, 0.0)),
            Some(Vec3::new(40.0, 4.0, 10.0)),
        ),
        (
            "e2e-high.png",
            Some(Vec3::new(0.0, 220.0, 0.0)),
            Some(Vec3::new(60.0, 0.0, 60.0)),
        ),
    ];
    for (name, eye, target) in shots {
        let path = dir.join(name);
        crate::headless::capture_view(
            320,
            240,
            path.to_str().unwrap(),
            *eye,
            *target,
            None,
            true,
            1,
            false,
        );
        let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        assert!(bytes > 0, "{name}: render produced an empty file");
    }
}
