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
            .map(|p| structures::colossus_label(seed, p))
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
    app.collect_index(idx, false);
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
// 7c) G18 — the uncertainty layer end to end: provisional → confirmed via BOTH paths (a second
//     sighting; the first use after comprehension), the worn-yield reduction, an ⟦erased⟧ collect
//     logging without yield, the codex's live Leiden re-render, and the `pg=` v8 round-trip —
//     all through the real collect/dispatch/frame seams (no direct kicks of the fixed machinery).
// ----------------------------------------------------------------------------------------------

/// Teleport the (manually-helmed) ship onto an inscription and collect it through the real
/// streaming + collect seam (`update_inscriptions` → `collectible` → `collect_index`).
/// G21: a **ship** collect — rung 0 whatever's been researched (the on-foot variant below).
fn collect_inscription(app: &mut App, m: &structures::Inscription) {
    app.camera.position = m.pos;
    app.update_inscriptions();
    let id = progress::find_id(m.cell, m.script, &m.text);
    let idx = app
        .collectible
        .iter()
        .position(|c| c.find_id == id)
        .expect("the inscription streamed in as a collectible at our position");
    app.collect_index(idx, false);
}

/// Does the codex overlay currently show an underdot row (a line of only dots — the Leiden
/// provisional mark beneath a glyph cluster)?
fn codex_has_underdots(app: &App) -> bool {
    app.codex_text()
        .lines()
        .any(|l| !l.trim().is_empty() && l.trim().chars().all(|c| c == '.'))
}

#[test]
fn uncertainty_attestation_condition_and_erasure() {
    use crate::progress::Attestation;
    let seed = 1337u32;
    let mut app = App::headless(seed);
    app.auto_fly = false; // scripted helm — hold each teleport position

    // The deterministic inscription field near the origin (the same one streaming sees).
    let g = ground_fn(seed);
    let marks = structures::inscriptions_near(seed, Vec3::ZERO, 2500.0, g);

    // --- Path (a): a SECOND SIGHTING confirms. Pick a gated block (≠ seek, kept for path b)
    //     with two distinct name-bearing inscriptions. ---
    let bearers_of = |name: &str| -> Vec<&structures::Inscription> {
        marks
            .iter()
            .filter(|m| {
                m.name
                    .is_some_and(|b| b.name() == name && b.required().is_some())
            })
            .collect()
    };
    let a_block = console::Block::ALL
        .iter()
        .copied()
        .find(|b| b.required().is_some() && b.name() != "seek" && bearers_of(b.name()).len() >= 2)
        .expect("a gated block (≠ seek) has ≥2 bearer inscriptions within 2500 units");
    let ab = bearers_of(a_block.name());
    assert!(!app.progress.is_discovered(a_block));
    collect_inscription(&mut app, ab[0]);
    assert_eq!(
        app.progress.attestation(a_block),
        Some(Attestation::Provisional),
        "the first sighting is a hypothesis"
    );
    assert!(
        codex_has_underdots(&app),
        "the codex underdots the provisional reading"
    );
    collect_inscription(&mut app, ab[1]);
    assert_eq!(
        app.progress.attestation(a_block),
        Some(Attestation::Confirmed),
        "a second sighting (different inscription, same name) confirms"
    );
    assert!(
        !codex_has_underdots(&app),
        "the codex re-renders the confirmed reading clean (live state, no snapshots)"
    );

    // --- Path (b): BEHAVIORAL confirmation. Discover `seek`, research it while provisional
    //     (Decision 1 — no gate), and let its first real execution attest it. ---
    let b_block = console::Block::Seek;
    let bb = bearers_of(b_block.name());
    assert!(!bb.is_empty(), "a seek bearer exists within 2500 units");
    collect_inscription(&mut app, bb[0]);
    assert_eq!(
        app.progress.attestation(b_block),
        Some(Attestation::Provisional)
    );
    // No mechanical gate: the provisional block allocates + researches through the normal seam.
    // (Inlined rather than via `comprehend`, whose extra `Discover` would count as a second
    // sighting and confirm through path (a) — here the *use* must do it.)
    assert!(
        app.progress.allocate(ResearchTarget::Block(b_block)),
        "a provisional reading researches freely (Decision 1 — no softlock)"
    );
    let domain = b_block.required().unwrap();
    let mut guard = 0;
    while !app.progress.is_block_comprehended(b_block) && guard < 100_000 {
        app.progress.apply(&Event::CollectShard {
            domain,
            rarity: Rarity::Rare,
        });
        guard += 1;
    }
    assert!(app.progress.is_block_comprehended(b_block));
    app.sync_console_unlock();
    assert_eq!(
        app.progress.attestation(b_block),
        Some(Attestation::Provisional),
        "comprehension alone does NOT confirm — the machine hasn't answered yet"
    );
    // The gentle lit-goal nudge: comprehended-but-unconfirmed suggests one use.
    let goal = app
        .console
        .lit_goal(&app.progress)
        .expect("the attestation nudge lights the goal");
    assert!(
        goal.contains(&b_block.glyphs(seed)) && goal.contains("use once"),
        "nudge names the reading by its glyphs: {goal}"
    );
    // First execution — the given nav routine steers by `seek`; the real frame loop answers.
    app.console.routines[0].body = vec![console::Step::Do(b_block)];
    app.run_frame(DT);
    assert_eq!(
        app.progress.attestation(b_block),
        Some(Attestation::Confirmed),
        "the first execution after comprehension confirms (world-as-oracle)"
    );

    // --- Worn: a lacuna-bearing name inscription still discovers, and yields only its
    //     surviving glyphs (reduced proportionally vs the intact name). ---
    let worn = marks
        .iter()
        .find(|m| matches!(m.condition, structures::Condition::Worn(_)) && m.name.is_some())
        .expect("a worn name-bearer within 2500 units");
    let surviving = progress::glyph_count(&worn.text);
    let full = progress::glyph_count(&structures::name_text(seed, worn.name.unwrap()));
    assert!(surviving < full, "the worn text lost glyph positions");
    let before = app.progress.strata.total();
    let discovered_before = app.progress.is_discovered(worn.name.unwrap());
    collect_inscription(&mut app, worn);
    assert_eq!(
        app.progress.strata.total() - before,
        progress::yield_amount(worn.script, surviving),
        "worn yield pays only the surviving glyphs"
    );
    assert!(
        app.progress.is_discovered(worn.name.unwrap()) || discovered_before,
        "a worn name-bearer still discovers (Decision 3)"
    );
    assert!(
        app.codex_text().contains("[..]"),
        "the codex renders a [..] lacuna per lost glyph"
    );

    // --- Erased: collecting the gouge yields NOTHING but logs the erasure event (the G20
    //     sensing-ladder hook), rendered ⟦——⟧ in the codex. ---
    let erased = marks
        .iter()
        .find(|m| m.condition == structures::Condition::Erased)
        .expect("an erased inscription within 2500 units");
    assert!(erased.name.is_none(), "an erasure discovers nothing");
    let (before_strata, before_bank) = (app.progress.strata.total(), app.progress.shard_bank());
    let before_codex = app.progress.collected_count();
    collect_inscription(&mut app, erased);
    assert_eq!(
        app.progress.strata.total(),
        before_strata,
        "an erased collect banks no data"
    );
    assert_eq!(app.progress.shard_bank(), before_bank);
    assert_eq!(
        app.progress.collected_count(),
        before_codex + 1,
        "…but the erasure event is logged in the codex"
    );
    assert!(
        app.codex_text()
            .contains("\u{27E6}\u{2014}\u{2014}\u{27E7}"),
        "the codex renders the erasure as ⟦——⟧"
    );

    // --- The attestation + erasure log survive the `pg=` v8 share round-trip. ---
    let restored = progress::Progress::decode(&app.share_string());
    assert_eq!(restored, app.progress, "pg= v8 round-trips the new state");
    assert_eq!(restored.attestation(a_block), Some(Attestation::Confirmed));
    // And the loop keeps running cleanly with the new state live.
    drive(&mut app, 120);
}

// ----------------------------------------------------------------------------------------------
// 7d) G20 — formulaic frames as cribs, end to end through the real collect seam: a worn frame
//     instance pays only its survivors while the frame is unknown; three INTACT sightings teach
//     it (codex gains the structural skeleton entry); a worn instance whose survivors uniquely
//     match then collects RESTORED — Leiden-bracketed `[abc]` in the codex, FULL yield — and the
//     crib state rides `pg=` v10.
// ----------------------------------------------------------------------------------------------

#[test]
fn frames_crib_three_sightings_teach_and_worn_matches_restore_full() {
    use structures::Condition;
    // Find a world whose origin field holds the full scenario: ≥3 intact frame instances and
    // ≥2 worn ones of which ≥1 is restorable (lacunae only in skeleton positions).
    let mut found: Option<(u32, Vec<structures::Inscription>)> = None;
    'seeds: for seed in [1337u32, 7, 42, 2024, 99999, 555] {
        let g = ground_fn(seed);
        let marks = structures::inscriptions_near(seed, Vec3::ZERO, 3000.0, g);
        let known = structures::world_frames(seed);
        let intact = marks
            .iter()
            .filter(|m| m.frame.is_some() && m.condition == Condition::Intact)
            .count();
        let worn: Vec<&structures::Inscription> = marks
            .iter()
            .filter(|m| m.frame.is_some() && matches!(m.condition, Condition::Worn(_)))
            .collect();
        let restorable = worn
            .iter()
            .filter(|m| structures::restore_worn(&m.text, m.script, &known).is_some())
            .count();
        if intact >= 3 && worn.len() >= 2 && restorable >= 1 {
            found = Some((seed, marks));
            break 'seeds;
        }
    }
    let (seed, marks) = found.expect("some seed holds 3 intact + 2 worn (1 restorable) frames");
    let known = structures::world_frames(seed);
    let frame_id = known[0].id;
    let intact: Vec<&structures::Inscription> = marks
        .iter()
        .filter(|m| m.frame.is_some() && m.condition == Condition::Intact)
        .collect();
    let restorable = marks
        .iter()
        .find(|m| {
            m.frame.is_some()
                && matches!(m.condition, Condition::Worn(_))
                && structures::restore_worn(&m.text, m.script, &known).is_some()
        })
        .unwrap();
    let worn_pre = marks
        .iter()
        .find(|m| {
            m.frame.is_some()
                && matches!(m.condition, Condition::Worn(_))
                && m.cell != restorable.cell
        })
        .unwrap();

    let mut app = App::headless(seed);
    app.auto_fly = false; // scripted helm

    // --- Unknown frame: a worn instance pays only its SURVIVING glyphs and logs lacunae. ---
    let before = app.progress.strata.total();
    collect_inscription(&mut app, worn_pre);
    assert_eq!(
        app.progress.strata.total() - before,
        progress::yield_amount(worn_pre.script, progress::glyph_count(&worn_pre.text)),
        "pre-known worn pays survivors only"
    );
    assert!(
        app.codex_text().contains("[..]"),
        "pre-known worn renders plain lacunae"
    );
    assert!(!app.codex_text().contains("FRAMES"), "nothing known yet");

    // --- Three INTACT sightings teach the frame (the worn collect above counted nothing). ---
    for (i, m) in intact.iter().take(3).enumerate() {
        assert!(!app.progress.frame_known(frame_id), "not known before 3");
        assert_eq!(app.progress.frame_sightings(frame_id), i as u8);
        collect_inscription(&mut app, m);
    }
    assert!(
        app.progress.frame_known(frame_id),
        "three intact sightings crack the frame"
    );
    let codex = app.codex_text();
    assert!(
        codex.contains("FRAMES — 1 known") && codex.contains("__"),
        "the codex records the skeleton with the slot marked: {codex}"
    );

    // --- A worn unique match now collects RESTORED: Leiden brackets + FULL yield. ---
    let full = progress::glyph_count(&structures::transliterate(
        // G22: the world spells the cell's stratum's surface form of the frame.
        &crate::lexicon::surface_frame(
            seed,
            restorable.cell,
            progress::stratum_of(restorable.script),
        ),
        restorable.script,
    ));
    assert!(
        progress::glyph_count(&restorable.text) < full,
        "the restorable instance really lost glyphs"
    );
    let before = app.progress.strata.total();
    collect_inscription(&mut app, restorable);
    assert_eq!(
        app.progress.strata.total() - before,
        progress::yield_amount(restorable.script, full),
        "a restored collect pays FULL, unreduced (Decision 2)"
    );
    let entry = app.progress.codex.last().unwrap();
    assert!(
        entry.text.contains('[') && entry.text.contains(']'),
        "the codex stores the Leiden-bracketed restoration: {:?}",
        entry.text
    );
    assert!(
        !entry.text.contains(crate::text::MARK_LACUNA),
        "no lacunae remain in a restored entry"
    );
    assert!(
        app.codex_text().contains('['),
        "the codex renders the restoration brackets (distinct from [..] lacunae)"
    );

    // --- The crib rides `pg=` v10. ---
    let restored = progress::Progress::decode(&app.share_string());
    assert_eq!(restored, app.progress, "pg= v10 round-trips the frame crib");
    assert!(restored.frame_known(frame_id));
    // And the loop keeps running cleanly with the new state live.
    drive(&mut app, 120);
}

// ----------------------------------------------------------------------------------------------
// 7e) G21 — the sensing ladder, rung 1: the first worn collect DISCOVERS close reading (the
//     frustration funnel), the console offers it as a research target, the canonical seam
//     researches it (Rites domain fill), and then an ON-FOOT collect of a worn inscription
//     recovers FULLY (bracketed text, full yield) while the ship's collect of an equal worn
//     cell stays rung 0 (reduced) — the differential, through the real collect seams.
// ----------------------------------------------------------------------------------------------

/// The walker collects the nearest site to `pos` through the real foot seam (`foot_collect_act`
/// → `collect_nearest_to` → `collect_index(_, true)` — the same path the expedition harvest and
/// the away-walker's routines drive).
fn foot_collect_at(app: &mut App, pos: Vec3) {
    app.camera.position = pos; // stream the cell in
    app.update_inscriptions();
    app.foot_collect_act(pos, None);
}

#[test]
fn sensing_rung1_close_reading_recovers_worn_on_foot_only() {
    use crate::progress::Sense;
    let seed = 1337u32;
    let g = ground_fn(seed);
    let marks = structures::inscriptions_near(seed, Vec3::ZERO, 2500.0, g);
    // Two deterministic worn inscriptions: one for the ship (rung 0), one for the walker.
    let worn: Vec<&structures::Inscription> = marks
        .iter()
        .filter(|m| matches!(m.condition, structures::Condition::Worn(_)))
        .take(2)
        .collect();
    assert_eq!(worn.len(), 2, "the origin field holds ≥2 worn inscriptions");

    let mut app = App::headless(seed);
    app.auto_fly = false; // scripted helm

    // --- The frustration funnel: the FIRST worn collect (a ship collect — reduced yield)
    //     discovers close reading; the console then lists the remedy as a research target. ---
    assert!(!app.progress.is_sense_discovered(Sense::CloseReading));
    let survivors = progress::glyph_count(&worn[0].text);
    let full0 = progress::glyph_count(worn[0].pristine.as_deref().unwrap());
    assert!(survivors < full0, "the worn cell really lost glyphs");
    let before = app.progress.strata.total();
    collect_inscription(&mut app, worn[0]);
    assert_eq!(
        app.progress.strata.total() - before,
        progress::yield_amount(worn[0].script, survivors),
        "the ship's worn collect pays survivors only (rung 0)"
    );
    assert!(
        app.progress.is_sense_discovered(Sense::CloseReading),
        "the first worn collect discovers close reading (the frustration teaches)"
    );
    app.sync_console_unlock();
    let home = app.console.render();
    let glyphs = progress::ResearchTarget::Sense(Sense::CloseReading).glyphs(seed);
    assert!(
        home.contains(&glyphs) && home.contains("(locked: research)"),
        "the console offers the remedy as a research target"
    );

    // --- Research it through the canonical seam (the console's allocate + Rites domain fill). ---
    assert!(app
        .progress
        .allocate(progress::ResearchTarget::Sense(Sense::CloseReading)));
    let mut guard = 0;
    while !app.progress.is_sense_comprehended(Sense::CloseReading) && guard < 100_000 {
        app.progress.apply(&Event::CollectShard {
            domain: Stratum::Rites,
            rarity: Rarity::Common,
        });
        guard += 1;
    }
    assert!(app.progress.is_sense_comprehended(Sense::CloseReading));

    // --- The walker, in reach, now recovers the second worn cell FULLY through the real foot
    //     seam — full text (Leiden-bracketed in the codex, no lacunae), full unreduced yield. ---
    let full1 = progress::glyph_count(worn[1].pristine.as_deref().unwrap());
    assert!(progress::glyph_count(&worn[1].text) < full1);
    let before = app
        .progress
        .strata
        .get(progress::stratum_of(worn[1].script));
    foot_collect_at(&mut app, worn[1].pos);
    let id1 = progress::find_id(worn[1].cell, worn[1].script, &worn[1].text);
    assert!(app.progress.has(id1), "the walker collected the worn cell");
    assert_eq!(
        app.progress
            .strata
            .get(progress::stratum_of(worn[1].script))
            - before,
        progress::yield_amount(worn[1].script, full1),
        "close reading pays FULL, unreduced yield on foot"
    );
    let entry = app
        .progress
        .codex
        .iter()
        .find(|e| e.find_id == id1)
        .unwrap();
    assert!(
        !entry.text.contains(crate::text::MARK_LACUNA),
        "no lacunae remain in the recovered entry"
    );
    assert!(
        entry.text.contains('[') && entry.text.contains(']'),
        "the codex stores the Leiden-bracketed recovery: {:?}",
        entry.text
    );

    // --- The sensing state + discovery ride `pg=` v11. ---
    let restored = progress::Progress::decode(&app.share_string());
    assert_eq!(restored, app.progress, "pg= v11 round-trips the ladder");
    assert!(restored.is_sense_comprehended(Sense::CloseReading));
    drive(&mut app, 120); // and the loop keeps running cleanly
}

// ----------------------------------------------------------------------------------------------
// 7f) G21 — the sensing ladder, rung 2: an erased log DISCOVERS deep sensing; researching it
//     demands Signals fill + the 8-rare gate; the logged gouge then becomes a DESTINATION —
//     returning on foot reveals its hidden content (deep-weighted Relics/Signals data or a deep
//     name), the codex's ⟦——⟧ resolves to recovered glyphs, and the ship's rung-0 revisit
//     obtains nothing — all through the real streaming/collect seams.
// ----------------------------------------------------------------------------------------------

#[test]
fn sensing_rung2_deep_sensing_reveals_the_logged_gouge_on_foot() {
    use crate::progress::Sense;
    let seed = 1337u32;
    let g = ground_fn(seed);
    let marks = structures::inscriptions_near(seed, Vec3::ZERO, 2500.0, g);
    let erased = marks
        .iter()
        .find(|m| m.condition == structures::Condition::Erased)
        .expect("an erased inscription within 2500 units");
    let id = progress::find_id(erased.cell, erased.script, &erased.text);

    let mut app = App::headless(seed);
    app.auto_fly = false; // scripted helm

    // --- The frustration funnel: the ship logs the gouge (no yield) → deep sensing discovered.
    assert!(!app.progress.is_sense_discovered(Sense::DeepSensing));
    collect_inscription(&mut app, erased);
    assert_eq!(
        app.progress.strata.total(),
        0,
        "a rung-0 erased collect banks nothing"
    );
    assert!(
        app.progress.erased_unresolved(id),
        "the gouge is logged, unresolved"
    );
    assert!(
        app.progress.is_sense_discovered(Sense::DeepSensing),
        "the first erased log discovers deep sensing"
    );
    assert!(app
        .codex_text()
        .contains("\u{27E6}\u{2014}\u{2014}\u{27E7}"));

    // --- The ship CANNOT return to it: once logged, a rung-0 run never re-lists the site. ---
    app.update_inscriptions();
    assert!(
        !app.collectible.iter().any(|c| c.find_id == id),
        "without deep sensing the logged gouge is spent, not a destination"
    );

    // --- Research deep sensing through the canonical seam: Signals fill + the 8-rare gate. ---
    assert!(app
        .progress
        .allocate(progress::ResearchTarget::Sense(Sense::DeepSensing)));
    for _ in 0..800 {
        app.progress.apply(&Event::CollectShard {
            domain: Stratum::Signals,
            rarity: Rarity::Common,
        });
    }
    assert!(
        !app.progress.is_sense_comprehended(Sense::DeepSensing),
        "the overfilled bar without 8 Signals rares must hold (the rare gate)"
    );
    for _ in 0..8 {
        app.progress.apply(&Event::CollectShard {
            domain: Stratum::Signals,
            rarity: Rarity::Rare,
        });
    }
    assert!(app.progress.is_sense_comprehended(Sense::DeepSensing));

    // --- The logged-erasures list is now a destination list: the site re-streams as
    //     collectible, and the WALKER's revisit reveals the hidden content. (Fly away first —
    //     the collectible cache rebuilds on cell-set change, as it does on any real journey.) ---
    app.camera.position = erased.pos + Vec3::new(2000.0, 0.0, 0.0);
    app.update_inscriptions();
    let (hidden, hscript, hname) = structures::hidden_text(seed, erased.cell);
    let before = app.progress.strata.get(progress::stratum_of(hscript));
    foot_collect_at(&mut app, erased.pos);
    assert!(
        !app.progress.erased_unresolved(id),
        "the on-foot revisit resolved the gouge"
    );
    assert_eq!(
        app.progress.strata.get(progress::stratum_of(hscript)) - before,
        progress::yield_amount(hscript, progress::glyph_count(&hidden)),
        "the reveal banks the hidden content (deep-weighted data)"
    );
    let entry = app.progress.codex.iter().find(|e| e.find_id == id).unwrap();
    assert!(
        structures::is_revealed_text(&entry.text),
        "the codex entry resolved in place: {:?}",
        entry.text
    );
    assert_eq!(structures::strip_reveal(&entry.text), hidden);
    // The codex renders the RESOLVED gouge: ⟦glyphs⟧, and this entry's ⟦——⟧ is gone.
    let codex = app.codex_text();
    assert!(
        !codex.contains("\u{27E6}\u{2014}\u{2014}\u{27E7}"),
        "the only logged erasure has resolved"
    );
    assert!(
        codex.contains('\u{27E6}'),
        "the event brackets remain, holding glyphs"
    );
    // If the gouge hid a NAME, revealing it discovered the block (the censored vocabulary).
    if let Some(b) = hname {
        assert!(app.progress.is_discovered(b), "a revealed name discovers");
    }

    // --- Everything rides pg= v11 and the loop keeps running. ---
    let restored = progress::Progress::decode(&app.share_string());
    assert_eq!(restored, app.progress);
    assert!(!restored.erased_unresolved(id));
    drive(&mut app, 120);
}

// ----------------------------------------------------------------------------------------------
// 7g) G21 — palimpsests: the doubled-baseline tell is findable at rung 0, but the under-text
//     only yields to deep sensing ON FOOT — collect banks BOTH layers, the codex stacks them —
//     while any rung-0/ship collect takes the surface alone (the under-layer is spent with it:
//     one more thing the drifting ship can never obtain).
// ----------------------------------------------------------------------------------------------

/// Grant deep sensing through the canonical seams: log a real erased site (the discovery
/// funnel), then allocate + fill Signals with its 8 rares.
fn grant_deep_sensing(app: &mut App, erased: &structures::Inscription) {
    use crate::progress::Sense;
    collect_inscription(app, erased);
    assert!(app.progress.is_sense_discovered(Sense::DeepSensing));
    assert!(app
        .progress
        .allocate(progress::ResearchTarget::Sense(Sense::DeepSensing)));
    let mut guard = 0;
    while !app.progress.is_sense_comprehended(Sense::DeepSensing) && guard < 100_000 {
        app.progress.apply(&Event::CollectShard {
            domain: Stratum::Signals,
            rarity: Rarity::Rare,
        });
        guard += 1;
    }
    assert!(app.progress.is_sense_comprehended(Sense::DeepSensing));
}

#[test]
fn sensing_palimpsests_yield_both_layers_to_deep_sensing_on_foot_only() {
    let seed = 1337u32;
    let g = ground_fn(seed);
    let marks = structures::inscriptions_near(seed, Vec3::ZERO, 3000.0, g);
    let palimpsests: Vec<&structures::Inscription> =
        marks.iter().filter(|m| m.under.is_some()).collect();
    assert!(
        palimpsests.len() >= 2,
        "the origin field holds ≥2 palimpsests (~1/60), got {}",
        palimpsests.len()
    );
    let erased = marks
        .iter()
        .find(|m| m.condition == structures::Condition::Erased)
        .expect("an erased site for the deep-sensing grant");

    let mut app = App::headless(seed);
    app.auto_fly = false;

    // --- Rung 0: the tell is visible (the billboard's text carries it), but a ship collect
    //     takes the SURFACE only — one codex entry, no under-layer yield. ---
    let p0 = palimpsests[0];
    assert!(
        p0.text.ends_with(crate::text::MARK_BASELINE),
        "the tell shows at rung 0"
    );
    let (_, u_script) = p0.under.clone().unwrap();
    let before_under = app.progress.strata.get(progress::stratum_of(u_script));
    collect_inscription(&mut app, p0);
    let id0 = progress::find_id(p0.cell, p0.script, &p0.text);
    assert_eq!(
        app.progress
            .codex
            .iter()
            .filter(|e| e.find_id == id0)
            .count(),
        1,
        "rung 0 logs the surface alone"
    );
    // (The under stratum may coincide with the surface stratum only if scripts map together —
    // they can't here: the surface is never erased and the under is Runic/Galactic; compare
    // the under stratum's balance against exactly the surface's contribution.)
    let surface_contrib = if progress::stratum_of(p0.script) == progress::stratum_of(u_script) {
        progress::yield_amount(p0.script, progress::glyph_count(&p0.text))
    } else {
        0
    };
    assert_eq!(
        app.progress.strata.get(progress::stratum_of(u_script)) - before_under,
        surface_contrib,
        "the under-text pays nothing at rung 0"
    );

    // --- Grant deep sensing (canonical seams), then the WALKER collects another palimpsest:
    //     BOTH layers bank, the codex stacks them (surface, then the └-led under-layer). ---
    grant_deep_sensing(&mut app, erased);
    let p1 = palimpsests[1];
    let (u1_text, u1_script) = p1.under.clone().unwrap();
    let id1 = progress::find_id(p1.cell, p1.script, &p1.text);
    let before_under = app.progress.strata.get(progress::stratum_of(u1_script));
    foot_collect_at(&mut app, p1.pos);
    assert!(app.progress.has(id1), "the walker collected the palimpsest");
    let layers: Vec<&progress::CodexEntry> = app
        .progress
        .codex
        .iter()
        .filter(|e| e.find_id == id1)
        .collect();
    assert_eq!(
        layers.len(),
        2,
        "both layers logged, stacked under one find"
    );
    assert_eq!(
        layers[1].text, u1_text,
        "the under-layer is the composed under-text"
    );
    assert_eq!(layers[1].script, u1_script);
    let under_gain = app.progress.strata.get(progress::stratum_of(u1_script)) - before_under;
    let expect_under = progress::yield_amount(u1_script, progress::glyph_count(&u1_text));
    assert!(
        under_gain >= expect_under,
        "the under-layer banked its own yield ({under_gain} ≥ {expect_under})"
    );
    assert!(
        app.codex_text().contains('└'),
        "the codex renders the stacked under-layer"
    );

    // --- And everything rides the share string. ---
    let restored = progress::Progress::decode(&app.share_string());
    assert_eq!(restored, app.progress);
    drive(&mut app, 120);
}

// ----------------------------------------------------------------------------------------------
// 7h) G21 — the expedition-rationality assert (the pacing analysis's "expeditions are
//     income-negative" finding, answered): an AUTHORED close-reading expedition — the real
//     run(foot) Deploy→Harvest→Return machine, walker carrying the instrument — obtains from a
//     worn site what the ship provably CANNOT at any rate: the recovered-full worn yield and
//     the lacuna-free recovered text. Asserts the DIFFERENTIAL on the same site, not activity.
//     (The erased-reveal and palimpsest-under differentials are asserted in 7f/7g — three
//     on-foot-only data classes in total.)
// ----------------------------------------------------------------------------------------------

#[test]
fn expedition_rationality_close_reading_earns_what_the_ship_cannot() {
    use crate::progress::Sense;
    let seed = 1337u32;
    let g = ground_fn(seed);
    let marks = structures::inscriptions_near(seed, Vec3::ZERO, 2500.0, g);
    let worn: Vec<&structures::Inscription> = marks
        .iter()
        .filter(|m| matches!(m.condition, structures::Condition::Worn(_)))
        .take(2)
        .collect();
    let (teacher, site) = (worn[0], worn[1]); // one to teach the need, one to compare on
    let stratum = progress::stratum_of(site.script);
    let survivors = progress::glyph_count(&site.text);
    let full = progress::glyph_count(site.pristine.as_deref().unwrap());
    assert!(survivors < full);

    // --- (a) The ship's ceiling on this site: rung 0, survivors only, lacunae in the codex —
    //     and no research can raise it (the instrument rides the walker, not the hull). ---
    let mut ship = App::headless(seed);
    ship.auto_fly = false;
    // Give the SHIP run every advantage: comprehend close reading first (via the teacher site's
    // frustration event + the canonical research seam) — it must still collect at rung 0.
    let comprehend_close_reading = |app: &mut App| {
        assert!(app.progress.is_sense_discovered(Sense::CloseReading));
        assert!(app
            .progress
            .allocate(progress::ResearchTarget::Sense(Sense::CloseReading)));
        let mut guard = 0;
        while !app.progress.is_sense_comprehended(Sense::CloseReading) && guard < 100_000 {
            app.progress.apply(&Event::CollectShard {
                domain: Stratum::Rites,
                rarity: Rarity::Common,
            });
            guard += 1;
        }
        assert!(app.progress.is_sense_comprehended(Sense::CloseReading));
    };
    collect_inscription(&mut ship, teacher); // the frustration event (discovers the faculty)
    comprehend_close_reading(&mut ship);
    let before = ship.progress.strata.get(stratum);
    collect_inscription(&mut ship, site);
    let ship_gain = ship.progress.strata.get(stratum) - before;
    assert_eq!(
        ship_gain,
        progress::yield_amount(site.script, survivors),
        "the ship pays survivors only, even holding the faculty (on-foot only — Decision 2)"
    );
    let sid = progress::find_id(site.cell, site.script, &site.text);
    let ship_entry = ship
        .progress
        .codex
        .iter()
        .find(|e| e.find_id == sid)
        .unwrap();
    assert!(
        ship_entry.text.contains(crate::text::MARK_LACUNA),
        "the ship's data-class keeps its lacunae — recovered-full text is unobtainable"
    );

    // --- (b) The authored close-reading expedition, on the same site, same faculty: the REAL
    //     run(foot) cycle (deploy → walk → harvest on foot → return), no scripted collects. ---
    let mut exp = App::headless(seed);
    exp.auto_fly = false;
    collect_inscription(&mut exp, teacher);
    comprehend_close_reading(&mut exp);
    // Keep the ship's own on-scan collect quiet so the harvest is observably the walker's.
    for r in exp.console.routines.iter_mut() {
        if matches!(r.trigger, console::Trigger::OnScan) {
            r.enabled = false;
        }
    }
    // Park the piloted ship over the worn site at cruise height and author the deploy.
    let gy = worldgen::height(site.pos.x.floor() as i32, site.pos.z.floor() as i32, seed) as f32;
    exp.camera.position = Vec3::new(site.pos.x + 6.0, gy + CRUISE_HEIGHT, site.pos.z);
    exp.update_inscriptions();
    let before = exp.progress.strata.get(stratum);
    exp.start_expedition();
    assert!(exp.expedition.active(), "run(foot) deployed the walker");
    let mut completed = false;
    for _ in 0..6000 {
        exp.run_frame(DT);
        assert_finite(&exp, "close-reading expedition");
        if !exp.expedition.active() {
            completed = true;
            break;
        }
    }
    assert!(completed, "the expedition cycle completed");
    let exp_gain = exp.progress.strata.get(stratum) - before;
    assert_eq!(
        exp_gain,
        progress::yield_amount(site.script, full),
        "the walker's harvest recovered FULL yield on the same site"
    );
    assert!(
        exp_gain > ship_gain,
        "the expedition out-earns the ship on the site itself ({exp_gain} > {ship_gain}) — \
         and its recovered text is a data-class the ship cannot produce at all"
    );
    let exp_entry = exp
        .progress
        .codex
        .iter()
        .find(|e| e.find_id == sid)
        .unwrap();
    assert!(
        !exp_entry.text.contains(crate::text::MARK_LACUNA) && exp_entry.text.contains('['),
        "the walker's codex entry is the recovered-full (bracketed) text: {:?}",
        exp_entry.text
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
