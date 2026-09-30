//! Opt-in scheduling policy and clocks. All deadline values are milliseconds.
use crate::error::VmErr;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Injectable monotonic millisecond clock. Values must be finite, nonnegative,
/// and nondecreasing. A clock is local to the interpreter owner thread.
pub trait Clock {
    fn now_ms(&self) -> f64;
}

#[derive(Clone, Default)]
pub struct VirtualClock(Rc<Cell<f64>>);
impl VirtualClock {
    pub fn advance(&self, milliseconds: f64) -> Result<(), VmErr> {
        let next = self.now_ms() + milliseconds;
        if !milliseconds.is_finite() || milliseconds < 0.0 || !next.is_finite() {
            return Err(VmErr::Msg(
                "virtual clock advance must be finite and nonnegative".into(),
            ));
        }
        self.0.set(next);
        Ok(())
    }
}
impl Clock for VirtualClock {
    fn now_ms(&self) -> f64 {
        self.0.get()
    }
}

pub struct RealTimeClock {
    #[cfg(not(target_arch = "wasm32"))]
    origin: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    origin: f64,
    #[cfg(target_arch = "wasm32")]
    last: Cell<f64>,
}
impl Default for RealTimeClock {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            origin: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            origin: js_sys::Date::now(),
            #[cfg(target_arch = "wasm32")]
            last: Cell::new(0.0),
        }
    }
}
impl Clock for RealTimeClock {
    fn now_ms(&self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.origin.elapsed().as_secs_f64() * 1000.0
        }
        #[cfg(target_arch = "wasm32")]
        {
            let now = (js_sys::Date::now() - self.origin).max(self.last.get());
            self.last.set(now);
            now
        }
    }
}

#[derive(Clone, Default)]
pub enum ClockMode {
    /// Historical delay ordering: every timer is eligible, no waiting.
    #[default]
    Legacy,
    Virtual(VirtualClock),
    RealTime(Rc<dyn Clock>),
}
impl ClockMode {
    pub(crate) fn now_ms(&self) -> f64 {
        match self {
            Self::Legacy => 0.,
            Self::Virtual(c) => c.now_ms(),
            Self::RealTime(c) => c.now_ms(),
        }
    }
    pub(crate) fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy)
    }
    pub(crate) fn is_real_time(&self) -> bool {
        matches!(self, Self::RealTime(_))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fairness {
    #[default]
    ExternalFirst,
    /// Alternate eligible external events and timers, starting with external.
    Alternate,
}
#[derive(Clone, Copy, Debug)]
pub struct EventLoopOptions {
    pub fairness: Fairness,
    pub host_batch_size: usize,
    pub external_capacity: usize,
}
impl Default for EventLoopOptions {
    fn default() -> Self {
        Self {
            fairness: Fairness::ExternalFirst,
            host_batch_size: 64,
            external_capacity: 1024,
        }
    }
}
/// Soft scheduler budget; a yield never refills hard guest limits.
#[derive(Clone, Copy, Debug)]
pub struct TurnBudget {
    pub max_jobs: usize,
    pub max_duration: Option<Duration>,
}
impl TurnBudget {
    pub fn jobs(max_jobs: usize) -> Self {
        Self {
            max_jobs,
            max_duration: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YieldReason {
    Idle,
    JobBudget,
    TimeBudget,
    Backpressure,
}
#[derive(Clone, Copy, Debug)]
pub struct TurnOutcome {
    pub executed_jobs: usize,
    pub runnable: bool,
    pub yield_reason: YieldReason,
    /// Absolute milliseconds in the configured clock; legacy reports delays.
    pub next_deadline: Option<f64>,
    pub checkpoint_pending: bool,
}
/// A thread-safe cancellation signal. Cancels active guest execution and waits;
/// never transfers guest values between threads.
#[derive(Clone, Default)]
pub struct CancellationToken(
    Arc<AtomicBool>,
    Arc<std::sync::Mutex<Vec<std::sync::Weak<crate::host::WakeSignal>>>>,
);
impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
        for wake in self
            .1
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter_map(std::sync::Weak::upgrade)
        {
            wake.fire();
        }
    }
    pub(crate) fn register_wake(&self, wake: &Arc<crate::host::WakeSignal>) {
        let mut wakes = self.1.lock().unwrap_or_else(|e| e.into_inner());
        wakes.retain(|w| w.strong_count() > 0);
        if !wakes.iter().any(|w| w.ptr_eq(&Arc::downgrade(wake))) {
            wakes.push(Arc::downgrade(wake));
        }
        if self.is_cancelled() {
            wake.fire();
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub(super) struct ExecutionState {
    pub wake: Arc<crate::host::WakeSignal>,
    pub fuel: Cell<u64>,
    pub loops: Cell<u64>,
    pub jobs: Cell<usize>,
    pub cancellation: std::cell::RefCell<CancellationToken>,
    pub deadline: Cell<Option<f64>>,
    pub clock: RealTimeClock,
    pub drain_depth: Cell<usize>,
    pub active: Cell<bool>,
}
impl ExecutionState {
    pub fn new() -> Self {
        Self {
            wake: Arc::new(crate::host::WakeSignal::default()),
            fuel: Cell::new(super::DEFAULT_FUEL_BUDGET),
            loops: Cell::new(super::DEFAULT_LOOP_BUDGET),
            jobs: Cell::new(super::jobs::MAX_JOBS_PER_DRAIN),
            cancellation: std::cell::RefCell::new(CancellationToken::default()),
            deadline: Cell::new(None),
            clock: RealTimeClock::default(),
            drain_depth: Cell::new(0),
            active: Cell::new(true),
        }
    }
    pub fn check(&self) -> Result<(), VmErr> {
        if self.cancellation.borrow().is_cancelled() {
            return Err(VmErr::Msg("Error: Guest execution cancelled".into()));
        }
        if self
            .deadline
            .get()
            .is_some_and(|d| self.clock.now_ms() >= d)
        {
            return Err(VmErr::Msg(
                "RangeError: Guest execution deadline exceeded".into(),
            ));
        }
        Ok(())
    }
}
