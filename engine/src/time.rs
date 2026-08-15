use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::sync::OnceLock;
use std::time::Instant as StdInstant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(u64);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(u64);

pub(crate) struct Clock {
    start: Instant,
    previous: Instant,
    current: Instant,
    fixed_delta: Duration,
    accumulator: Duration,
    /// Simulated time. Advances by clamped deltas, so it falls behind
    /// `current - start` by whatever the clamp discarded.
    elapsed: Duration,
}

pub(crate) struct FixedSteps<'a> {
    clock: &'a mut Clock,
}

pub struct TimeStep {
    elapsed: Duration,
    delta_time: Duration,
    accumulator: Duration,
}

impl Instant {
    pub fn now() -> Self {
        // `StdInstant` is opaque, so the epoch is the first reading taken.
        // On Windows this is QPC underneath, already scaled to nanoseconds.
        static EPOCH: OnceLock<StdInstant> = OnceLock::new();
        let epoch = EPOCH.get_or_init(StdInstant::now);
        Self(epoch.elapsed().as_nanos() as u64)
    }

    #[inline]
    pub fn elapsed(self) -> Duration {
        Self::now() - self
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    #[inline]
    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Duration> for Instant {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs.0
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Duration(self.0.saturating_sub(rhs.0))
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    #[inline]
    fn sub(self, rhs: Duration) -> Self::Output {
        Instant(self.0.saturating_sub(rhs.0))
    }
}

impl Duration {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    #[inline]
    pub const fn from_micros(micros: u64) -> Self {
        Self(micros * 1_000)
    }

    #[inline]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis * 1_000_000)
    }

    #[inline]
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs * 1_000_000_000)
    }

    /// Period of a rate: `from_hz(60)` is the step of a 60 Hz simulation.
    #[inline]
    pub const fn from_hz(hz: u32) -> Self {
        Self(1_000_000_000 / hz as u64)
    }

    #[inline]
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn as_secs_f32(self) -> f32 {
        self.0 as f32 / 1_000_000_000.0
    }

    #[inline]
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / 1_000_000_000.0
    }
}

impl Add<Duration> for Duration {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Duration> for Duration {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs.0
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign<Duration> for Duration {
    #[inline]
    fn sub_assign(&mut self, rhs: Duration) {
        self.0 = self.0.saturating_sub(rhs.0)
    }
}

impl Clock {
    const MAX_FRAME_DELTA: Duration = Duration::from_millis(250);

    pub fn new(fixed_delta: Duration) -> Self {
        assert_ne!(
            fixed_delta,
            Duration::ZERO,
            "fixed_delta must be different from 0"
        );
        let current = Instant::now();
        Self {
            start: current,
            previous: current,
            current,
            fixed_delta,
            accumulator: Duration::ZERO,
            elapsed: Duration::ZERO,
        }
    }

    pub fn advance(&mut self) {
        self.previous = self.current;
        self.current = Instant::now();

        let delta = self.delta_time();
        self.accumulator += delta;
        self.elapsed += delta;
    }

    #[inline]
    pub fn fixed_steps(&mut self) -> FixedSteps<'_> {
        FixedSteps { clock: self }
    }

    pub fn frame(&self) -> TimeStep {
        TimeStep {
            elapsed: self.elapsed,
            delta_time: self.delta_time(),
            accumulator: self.accumulator,
        }
    }

    #[inline]
    pub fn uptime(&self) -> Duration {
        self.current - self.start
    }

    /// How far the next fixed step already is, in `0.0..1.0`. What rendering
    /// interpolates the previous and current simulation states with.
    #[inline]
    pub fn alpha(&self) -> f32 {
        self.accumulator.0 as f32 / self.fixed_delta.0 as f32
    }

    #[inline]
    fn delta_time(&self) -> Duration {
        Self::MAX_FRAME_DELTA.min(self.current - self.previous)
    }
}

impl Iterator for FixedSteps<'_> {
    type Item = TimeStep;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let clock = &mut *self.clock;
        if clock.accumulator < clock.fixed_delta {
            return None;
        }

        clock.accumulator -= clock.fixed_delta;
        Some(TimeStep {
            // Simulated time at the end of this step: everything accumulated
            // so far, minus what is still pending.
            elapsed: clock.elapsed - clock.accumulator,
            delta_time: clock.fixed_delta,
            accumulator: Duration::ZERO,
        })
    }
}

impl TimeStep {
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[inline]
    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }

    /// Time already accumulated towards the next fixed step. Zero inside a
    /// fixed step, since the step consumed it.
    #[inline]
    pub fn accumulator(&self) -> Duration {
        self.accumulator
    }
}
