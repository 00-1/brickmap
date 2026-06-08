//! E9 — a global **weather** state machine: a slow, seeded cycle
//! `Clear → Building → Precip → Clearing → Clear …` exposing a `0..1` **intensity** the app uses
//! to drive precipitation (rain/snow particles), and later fog/wetness/ambient-audio. Pure +
//! deterministic (seed-jittered phase durations); the app advances it by `dt` in the live loop, so
//! the static headless/golden render (which never ticks it) is untouched. Snow vs rain is the
//! caller's call (by biome coldness) — kept out of here so this stays pure.

/// The weather cycle phases.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Phase {
    #[default]
    Clear,
    Building,
    Precip,
    Clearing,
}

impl Phase {
    fn next(self) -> Phase {
        match self {
            Phase::Clear => Phase::Building,
            Phase::Building => Phase::Precip,
            Phase::Precip => Phase::Clearing,
            Phase::Clearing => Phase::Clear,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Phase::Clear => "clear",
            Phase::Building => "clouding",
            Phase::Precip => "precip",
            Phase::Clearing => "clearing",
        }
    }
}

/// The weather controller: the current phase + time-in-phase + the seed (for per-cycle jitter).
#[derive(Clone, Debug)]
pub struct Weather {
    phase: Phase,
    t: f32,
    seed: u32,
    /// Counts cycles so the seed-jitter varies each time round (deterministic in seed + count).
    cycle: u32,
}

impl Weather {
    /// Base phase durations (seconds): long dry spells, brief build/clear, a decent downpour.
    const BASE: [f32; 4] = [42.0, 9.0, 26.0, 11.0]; // Clear, Building, Precip, Clearing

    pub fn new(seed: u32) -> Self {
        Weather {
            phase: Phase::Clear,
            t: 0.0,
            seed,
            cycle: 0,
        }
    }

    fn base(phase: Phase) -> f32 {
        Self::BASE[match phase {
            Phase::Clear => 0,
            Phase::Building => 1,
            Phase::Precip => 2,
            Phase::Clearing => 3,
        }]
    }

    /// This phase's duration: the base ± up to ~40%, jittered deterministically by seed + cycle +
    /// phase, so the weather doesn't feel metronomic.
    fn duration(&self) -> f32 {
        let h = self
            .seed
            .wrapping_mul(0x9e37_79b9)
            .wrapping_add(self.cycle.wrapping_mul(0x0001_0001))
            .wrapping_add(self.phase as u32 + 1);
        let j = ((h >> 8) & 0xff) as f32 / 255.0; // 0..1
        Self::base(self.phase) * (0.6 + 0.8 * j) // 0.6×..1.4×
    }

    /// Advance the cycle by `dt`. Crosses into the next phase when the current one elapses.
    pub fn advance(&mut self, dt: f32) {
        self.t += dt.max(0.0);
        // `while` so a big dt (or a short jittered phase) can step multiple phases cleanly.
        while self.t >= self.duration() {
            self.t -= self.duration();
            self.phase = self.phase.next();
            if self.phase == Phase::Clear {
                self.cycle = self.cycle.wrapping_add(1);
            }
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Precipitation intensity, `0..1`: 0 in `Clear`, ramps up over `Building`, full in `Precip`,
    /// ramps down over `Clearing`. Smooth (no hard jumps at the boundaries).
    pub fn intensity(&self) -> f32 {
        let frac = (self.t / self.duration()).clamp(0.0, 1.0);
        match self.phase {
            Phase::Clear => 0.0,
            Phase::Building => frac,
            Phase::Precip => 1.0,
            Phase::Clearing => 1.0 - frac,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_in_order_and_intensity_is_bounded() {
        let mut w = Weather::new(1337);
        let mut seen = vec![w.phase()];
        // Step through a few minutes; collect phase transitions.
        for _ in 0..6000 {
            let before = w.phase();
            w.advance(0.1);
            let i = w.intensity();
            assert!((0.0..=1.0).contains(&i), "intensity out of range: {i}");
            if w.phase() != before {
                seen.push(w.phase());
            }
        }
        // The order is always Clear→Building→Precip→Clearing→Clear…
        for pair in seen.windows(2) {
            assert_eq!(pair[1], pair[0].next(), "out-of-order transition: {pair:?}");
        }
        // It actually went round at least once (hit every phase).
        for p in [
            Phase::Clear,
            Phase::Building,
            Phase::Precip,
            Phase::Clearing,
        ] {
            assert!(seen.contains(&p), "never reached {p:?}");
        }
    }

    #[test]
    fn clear_has_no_precip_and_precip_is_full() {
        let w = Weather::new(7);
        assert_eq!(w.phase(), Phase::Clear);
        assert_eq!(w.intensity(), 0.0); // a fresh world starts dry (so the golden frame is dry)
                                        // Drive to Precip and confirm full intensity.
        let mut w = Weather::new(7);
        let mut guard = 0;
        while w.phase() != Phase::Precip && guard < 100000 {
            w.advance(0.5);
            guard += 1;
        }
        assert_eq!(w.phase(), Phase::Precip);
        assert_eq!(w.intensity(), 1.0);
    }

    #[test]
    fn deterministic_in_seed() {
        let mut a = Weather::new(42);
        let mut b = Weather::new(42);
        for _ in 0..1000 {
            a.advance(0.123);
            b.advance(0.123);
        }
        assert_eq!(a.phase(), b.phase());
        assert_eq!(a.intensity(), b.intensity());
        // A different seed diverges (different jittered durations) within a cycle or two.
        let mut c = Weather::new(43);
        for _ in 0..1000 {
            c.advance(0.123);
        }
        // Not asserting inequality of a single sample (could coincide); just that it's well-formed.
        assert!((0.0..=1.0).contains(&c.intensity()));
    }
}
