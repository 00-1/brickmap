//! G8c-2b — the **automated expedition**: the cross-agent choreography a ship routine kicks via
//! `run(foot)` (game-system §7 ceiling). When the autopiloted ship reaches a site and a
//! `run(foot)` step fires, it **deploys the walker** to collect on foot, then **recalls** it and
//! cruises on — *ship flies to a site → lands → walker disembarks + collects → returns → fly on.*
//!
//! This module is the **pure phase state machine** (unit-tested); the app feeds it the walker's
//! arrival booleans + `dt` and applies the per-phase movement (walk the walker out to the site,
//! collect, walk it back, board) and the ship hold. **Feel-tuning** (the dwell time, walk speed,
//! arrival/board radii) is left to the app's consts + noted for end-of-run iteration — the
//! *systems* are here and tested.

/// The expedition phases. `Idle` = no expedition; the rest is one deploy→harvest→return cycle.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Phase {
    #[default]
    Idle,
    /// The walker is leaving the ship, heading for the site.
    Deploy,
    /// The walker is at the site, collecting (a brief dwell).
    Harvest,
    /// The walker is heading back to the ship.
    Return,
}

/// Dwell at the site while harvesting, in seconds (feel-tuning).
pub const HARVEST_TIME: f32 = 1.5;

/// The expedition controller — a phase + a dwell timer. Pure: [`Expedition::advance`] only reads
/// the two arrival booleans + `dt`; the app owns positions/collection.
#[derive(Copy, Clone, Debug, Default)]
pub struct Expedition {
    pub phase: Phase,
    pub timer: f32,
}

impl Expedition {
    pub fn active(&self) -> bool {
        self.phase != Phase::Idle
    }

    /// Kick off an expedition (the ship arrived + `run(foot)` fired). Idempotent while one runs.
    pub fn start(&mut self) {
        if self.phase == Phase::Idle {
            self.phase = Phase::Deploy;
            self.timer = 0.0;
        }
    }

    /// Advance one tick. `at_site` = the walker reached the site; `home` = it's back at the ship.
    /// Returns the (possibly new) phase; the caller collects when it transitions **into**
    /// `Harvest`.
    pub fn advance(&mut self, at_site: bool, home: bool, dt: f32) -> Phase {
        match self.phase {
            Phase::Idle => {}
            Phase::Deploy => {
                if at_site {
                    self.phase = Phase::Harvest;
                    self.timer = 0.0;
                }
            }
            Phase::Harvest => {
                self.timer += dt;
                if self.timer >= HARVEST_TIME {
                    self.phase = Phase::Return;
                }
            }
            Phase::Return => {
                if home {
                    self.phase = Phase::Idle;
                    self.timer = 0.0;
                }
            }
        }
        self.phase
    }

    /// A short HUD tag for the current phase (`None` when idle).
    pub fn label(&self) -> Option<&'static str> {
        match self.phase {
            Phase::Idle => None,
            Phase::Deploy => Some("expedition: walker out"),
            Phase::Harvest => Some("expedition: collecting"),
            Phase::Return => Some("expedition: walker returning"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_cycle_deploy_harvest_return() {
        let mut e = Expedition::default();
        assert!(!e.active());
        e.start();
        assert_eq!(e.phase, Phase::Deploy);
        // Not at the site yet → stays deploying.
        assert_eq!(e.advance(false, false, 0.1), Phase::Deploy);
        // Reached the site → harvest.
        assert_eq!(e.advance(true, false, 0.1), Phase::Harvest);
        // Dwell isn't over → still harvesting.
        assert_eq!(e.advance(true, false, HARVEST_TIME * 0.5), Phase::Harvest);
        // Dwell elapses → return.
        assert_eq!(e.advance(true, false, HARVEST_TIME), Phase::Return);
        // Not home yet → still returning.
        assert_eq!(e.advance(false, false, 0.1), Phase::Return);
        // Home → idle (cycle complete).
        assert_eq!(e.advance(false, true, 0.1), Phase::Idle);
        assert!(!e.active());
    }

    #[test]
    fn start_is_idempotent_while_running() {
        let mut e = Expedition::default();
        e.start();
        e.advance(true, false, 0.1); // now Harvest
        assert_eq!(e.phase, Phase::Harvest);
        e.start(); // must NOT reset a running expedition
        assert_eq!(e.phase, Phase::Harvest);
    }

    #[test]
    fn harvest_entry_is_detectable_once() {
        // The app collects on the Deploy→Harvest transition; verify it happens exactly once.
        let mut e = Expedition::default();
        e.start();
        let mut harvested = 0;
        let mut prev = e.phase;
        for _ in 0..10 {
            let now = e.advance(true, false, 0.1);
            if prev != Phase::Harvest && now == Phase::Harvest {
                harvested += 1;
            }
            prev = now;
        }
        assert_eq!(harvested, 1);
    }
}
