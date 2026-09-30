//! The job queues: microtasks (promise reactions, `queueMicrotask`) and
//! macrotasks (`setTimeout`).
//!
//! The queue is shared, not owned: generator and async bodies run on their own
//! `Interpreter` (a separate stack), and a promise settled inside one must
//! schedule reactions the outer loop will run. Handing every interpreter an
//! `Rc` to the same queue is what keeps a single event loop across them.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

use crate::value::{PromiseInner, PromiseState, Reaction, Value};

/// Hard cap on how many jobs one drain will run.
///
/// A promise chain can schedule work forever (`function tick() {
/// Promise.resolve().then(tick); }`), which would hang the host inside a
/// single `run()` call. The cap turns that into a catchable `RangeError`, the
/// same treatment loops and recursion get.
pub const MAX_JOBS_PER_DRAIN: usize = 1_000_000;

/// A unit of deferred work.
pub enum Job {
    /// A promise reaction: run `reaction`'s handler for a promise that settled
    /// to `state` with `value`, then settle the derived promise.
    Reaction {
        state: PromiseState,
        value: Value,
        reaction: Reaction,
    },
    /// Invoke a thenable's `then` method in a PromiseResolveThenableJob, after
    /// the current JavaScript stack has finished.
    PromiseResolveThenable {
        target: Rc<RefCell<PromiseInner>>,
        thenable: Value,
        then: Value,
        resolution_guard: Value,
    },
    /// A plain callback: `queueMicrotask(fn)`, or a timer callback.
    Callback { callback: Value, args: Vec<Value> },
    /// Callback queued by a host runtime after an external event. Unlike
    /// synchronous host calls, this runs at an event-loop checkpoint.
    HostCallback { callback: crate::host::HostCallback },
    /// Settlement of a host promise received from the external event queue.
    HostPromiseSettled {
        promise: Rc<RefCell<PromiseInner>>,
        state: PromiseState,
        value: Value,
    },
    /// An exception reported asynchronously by a host runtime. It is offered
    /// to the guest process `uncaughtException` event before escaping to Rust.
    HostUncaughtException { exception: Value },
    /// Timeout for a pending `Atomics.waitAsync` registration. The waiter is
    /// looked up by its shared-memory address and registration id so an
    /// earlier `Atomics.notify` makes this timeout a no-op.
    AtomicsWaitTimeout { key: (usize, usize), waiter_id: u64 },
}

impl Job {
    /// Values this queued job keeps alive, for the cycle collector.
    pub(crate) fn trace_values(&self, out: &mut Vec<Value>) {
        match self {
            Job::Reaction {
                value, reaction, ..
            } => out.extend([
                value.clone(),
                reaction.on_fulfilled.clone(),
                reaction.on_rejected.clone(),
                Value::Promise(reaction.derived.clone()),
            ]),
            Job::PromiseResolveThenable {
                target,
                thenable,
                then,
                resolution_guard,
            } => out.extend([
                Value::Promise(target.clone()),
                thenable.clone(),
                then.clone(),
                resolution_guard.clone(),
            ]),
            Job::Callback { callback, args } => {
                out.push(callback.clone());
                out.extend(args.iter().cloned());
            }
            Job::HostCallback { callback } => {
                out.extend([callback.callback.clone(), callback.this_value.clone()]);
                out.extend(callback.args.iter().cloned());
            }
            Job::HostPromiseSettled { promise, value, .. } => {
                out.extend([Value::Promise(promise.clone()), value.clone()]);
            }
            Job::HostUncaughtException { exception } => out.push(exception.clone()),
            Job::AtomicsWaitTimeout { .. } => {}
        }
    }
}

struct AtomicsWaiter {
    id: u64,
    promise: Rc<RefCell<PromiseInner>>,
}

#[derive(Default)]
pub struct JobQueue {
    microtasks: VecDeque<Job>,
    external_events: VecDeque<Job>,
    /// Timer callbacks, ordered by delay then by insertion. There is no real
    /// clock here: a timer runs after every microtask has, which preserves the
    /// ordering guarantees guest code depends on without a wall clock.
    // Nonnegative finite f64 bit patterns have the same order as their values.
    timers: BTreeMap<(u64, u128), Job>,
    timer_ids: HashMap<u64, (u64, u128)>,
    timer_keys: HashMap<(u64, u128), u64>,
    next_timer_id: u64,
    next_sequence: u128,
    clock: super::scheduler::ClockMode,
    pub(crate) checkpoint_pending: bool,
    pub(crate) prefer_timer: bool,
    atomics_waiters: HashMap<(usize, usize), VecDeque<AtomicsWaiter>>,
    next_atomics_waiter_id: u64,
}

impl JobQueue {
    /// Every value the queued jobs and waiters keep alive, for the cycle
    /// collector's root set.
    pub(crate) fn trace_roots(&self, out: &mut Vec<Value>) {
        for job in self.microtasks.iter().chain(self.external_events.iter()) {
            job.trace_values(out);
        }
        for job in self.timers.values() {
            job.trace_values(out);
        }
        for waiters in self.atomics_waiters.values() {
            for waiter in waiters {
                out.push(Value::Promise(waiter.promise.clone()));
            }
        }
    }

    pub fn push_microtask(&mut self, job: Job) {
        self.microtasks.push_back(job);
    }

    pub fn take_microtask(&mut self) -> Option<Job> {
        self.microtasks.pop_front()
    }

    pub fn push_external_event(&mut self, job: Job) {
        self.external_events.push_back(job);
    }

    pub fn take_external_event(&mut self) -> Option<Job> {
        self.external_events.pop_front()
    }

    /// Schedule a timer callback, returning the id `clearTimeout` cancels.
    pub fn push_timer(&mut self, delay: f64, callback: Value, args: Vec<Value>) -> u64 {
        self.push_timer_job(delay, Job::Callback { callback, args })
    }

    /// Schedule an internal event on the same timer queue used by guest
    /// timers. Returns a timer sequence id, which is deliberately not exposed
    /// to guest code for runtime-owned jobs.
    pub fn push_timer_job(&mut self, delay: f64, job: Job) -> u64 {
        // IDs are exactly representable in guest JS and never alias live timers,
        // even after wraparound. The independent sequence preserves FIFO ties.
        const MAX_ID: u64 = (1 << 53) - 1;
        loop {
            self.next_timer_id = if self.next_timer_id >= MAX_ID {
                1
            } else {
                self.next_timer_id + 1
            };
            if !self.timer_ids.contains_key(&self.next_timer_id) {
                break;
            }
        }
        let id = self.next_timer_id;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .expect("timer sequence exhausted");
        let delay = normalize_delay(delay);
        let deadline = (self.clock.now_ms() + delay).min(f64::MAX);
        let key = (deadline.to_bits(), self.next_sequence);
        self.timers.insert(key, job);
        self.timer_ids.insert(id, key);
        self.timer_keys.insert(key, id);
        id
    }

    /// Register a promise waiting on the shared memory word at `key`.
    pub fn register_atomics_waiter(
        &mut self,
        key: (usize, usize),
        promise: Rc<RefCell<PromiseInner>>,
    ) -> u64 {
        self.next_atomics_waiter_id = self.next_atomics_waiter_id.wrapping_add(1).max(1);
        let id = self.next_atomics_waiter_id;
        self.atomics_waiters
            .entry(key)
            .or_default()
            .push_back(AtomicsWaiter { id, promise });
        id
    }

    /// Remove a registered waiter, typically when its timeout fires.
    pub fn remove_atomics_waiter(
        &mut self,
        key: (usize, usize),
        waiter_id: u64,
    ) -> Option<Rc<RefCell<PromiseInner>>> {
        let waiters = self.atomics_waiters.get_mut(&key)?;
        let index = waiters.iter().position(|waiter| waiter.id == waiter_id)?;
        let waiter = waiters.remove(index)?;
        if waiters.is_empty() {
            self.atomics_waiters.remove(&key);
        }
        Some(waiter.promise)
    }

    /// Take up to `count` pending waiters in FIFO order. Settled entries are
    /// discarded so a timeout cannot make a later notify report a false hit.
    pub fn take_atomics_waiters(
        &mut self,
        key: (usize, usize),
        count: usize,
    ) -> Vec<Rc<RefCell<PromiseInner>>> {
        let Some(waiters) = self.atomics_waiters.get_mut(&key) else {
            return Vec::new();
        };
        let mut selected = Vec::new();
        let mut retained = VecDeque::new();
        while let Some(waiter) = waiters.pop_front() {
            if waiter.promise.borrow().state != PromiseState::Pending {
                continue;
            }
            if selected.len() < count {
                selected.push(waiter.promise);
            } else {
                retained.push_back(waiter);
            }
        }
        *waiters = retained;
        if waiters.is_empty() {
            self.atomics_waiters.remove(&key);
        }
        selected
    }

    pub fn cancel_timer(&mut self, id: u64) {
        if let Some(key) = self.timer_ids.remove(&id) {
            self.timer_keys.remove(&key);
            self.timers.remove(&key);
        }
    }

    /// Remove the smallest deadline, breaking ties by scheduling order.
    pub fn take_timer(&mut self) -> Option<Job> {
        if !self.has_due_timer() {
            return None;
        }
        let (key, job) = self.timers.pop_first()?;
        if let Some(id) = self.timer_keys.remove(&key) {
            self.timer_ids.remove(&id);
        }
        Some(job)
    }

    pub fn has_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }

    pub fn set_clock(
        &mut self,
        clock: super::scheduler::ClockMode,
    ) -> Result<(), crate::error::VmErr> {
        if !self.timers.is_empty() {
            return Err(crate::error::VmErr::Msg(
                "cannot change clocks with pending timers".into(),
            ));
        }
        self.clock = clock;
        Ok(())
    }
    pub fn next_deadline(&self) -> Option<f64> {
        self.timers
            .first_key_value()
            .map(|(key, _)| f64::from_bits(key.0))
    }
    pub fn has_due_timer(&self) -> bool {
        self.next_deadline()
            .is_some_and(|deadline| self.clock.is_legacy() || deadline <= self.clock.now_ms())
    }
    pub fn has_external_events(&self) -> bool {
        !self.external_events.is_empty()
    }
    pub fn external_len(&self) -> usize {
        self.external_events.len()
    }
    pub fn is_runnable(&self) -> bool {
        self.has_microtasks() || self.has_external_events() || self.has_due_timer()
    }
    pub fn try_push_external_event(&mut self, job: Job, capacity: usize) -> Result<(), Job> {
        if self.external_len() >= capacity {
            Err(job)
        } else {
            self.push_external_event(job);
            Ok(())
        }
    }
    pub(crate) fn timer_wait(&self) -> Option<std::time::Duration> {
        if !self.clock.is_real_time() {
            return None;
        }
        self.next_deadline().map(|d| {
            std::time::Duration::try_from_secs_f64(((d - self.clock.now_ms()).max(0.0)) / 1000.0)
                .unwrap_or(std::time::Duration::MAX)
        })
    }
    pub fn is_empty(&self) -> bool {
        self.microtasks.is_empty() && self.external_events.is_empty() && self.timers.is_empty()
    }
}

/// Shared handle to the queue.
pub type Jobs = Rc<RefCell<JobQueue>>;

/// Settle `promise`, moving every registration it accumulated onto the
/// microtask queue. A promise that has already settled is left alone — the
/// specification's "resolve once" rule, and what makes a `resolve`/`reject`
/// pair handed to an executor safe to call twice.
pub fn settle(jobs: &Jobs, promise: &Rc<RefCell<PromiseInner>>, state: PromiseState, value: Value) {
    let reactions = {
        let mut inner = promise.borrow_mut();
        if inner.state != PromiseState::Pending {
            return;
        }
        inner.state = state;
        inner.resolution_locked = true;
        inner.external_pending = false;
        inner.value = value.clone();
        std::mem::take(&mut inner.reactions)
    };
    let mut queue = jobs.borrow_mut();
    for reaction in reactions {
        queue.push_microtask(Job::Reaction {
            state,
            value: value.clone(),
            reaction,
        });
    }
}

/// Complete a timed `Atomics.waitAsync` registration. If a notify already
/// removed it, the timeout is stale and does nothing.
pub fn settle_atomics_wait_timeout(jobs: &Jobs, key: (usize, usize), waiter_id: u64) {
    let promise = jobs.borrow_mut().remove_atomics_waiter(key, waiter_id);
    if let Some(promise) = promise {
        settle(
            jobs,
            &promise,
            PromiseState::Fulfilled,
            Value::String("timed-out".into()),
        );
    }
}

fn normalize_delay(delay: f64) -> f64 {
    if delay.is_finite() && delay > 0.0 {
        delay
    } else {
        0.0
    }
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;
    fn job(n: f64) -> Job {
        Job::Callback {
            callback: Value::Number(n),
            args: vec![],
        }
    }
    fn take(q: &mut JobQueue) -> f64 {
        match q.take_timer().unwrap() {
            Job::Callback {
                callback: Value::Number(n),
                ..
            } => n,
            _ => panic!(),
        }
    }
    #[test]
    fn timers_normalize_and_keep_fifo_ties() {
        let mut q = JobQueue::default();
        for (n, d) in [4.0, 0.0, f64::NAN, -1.0, f64::INFINITY, -0.0, 4.0]
            .into_iter()
            .enumerate()
        {
            q.push_timer_job(d, job(n as f64));
        }
        assert_eq!(
            (0..7).map(|_| take(&mut q)).collect::<Vec<_>>(),
            vec![1., 2., 3., 4., 5., 0., 6.]
        );
        assert!(q.timer_ids.is_empty());
        assert!(q.timer_keys.is_empty());
    }
    #[test]
    fn timer_ids_wrap_without_collisions_or_reordering() {
        let mut q = JobQueue::default();
        assert_eq!(q.push_timer_job(1., job(1.)), 1);
        q.next_timer_id = (1 << 53) - 1;
        assert_eq!(q.push_timer_job(1., job(2.)), 2);
        assert_eq!(take(&mut q), 1.);
        assert_eq!(take(&mut q), 2.);
    }
    #[test]
    fn cancellation_releases_roots_immediately() {
        let mut q = JobQueue::default();
        let object = Value::object(vec![]);
        let id = q.push_timer(1., object.clone(), vec![object.clone()]);
        let mut roots = vec![];
        q.trace_roots(&mut roots);
        assert_eq!(roots.len(), 2);
        roots.clear();
        q.cancel_timer(id);
        q.cancel_timer(id);
        q.trace_roots(&mut roots);
        assert!(roots.is_empty());
        assert!(q.is_empty());
        assert!(q.timer_ids.is_empty());
    }
}
