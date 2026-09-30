# Event-loop scheduling and persistent async sessions

Existing `Vm`, `runAsync`, Rust drains, and the default clock retain historical
delay ordering: timers are sorted by normalized delay and run without sleeping.
Equal deadlines use insertion order. Negative and nonfinite delays become zero.
Cancellation immediately removes the job and its queue-owned GC roots. Timer IDs
wrap within JavaScript's exact integer range and skip IDs still in use.

## Rust turns and clocks

```rust
use napi_vm::{ClockMode, EventLoopOptions, Fairness, Interpreter, TurnBudget, VirtualClock};
let mut vm = Interpreter::with_builtins();
let clock = VirtualClock::default();
vm.jobs.borrow_mut().set_clock(ClockMode::Virtual(clock.clone()))?;
vm.set_event_loop_options(EventLoopOptions {
    fairness: Fairness::Alternate,
    ..EventLoopOptions::default()
})?;
clock.advance(10.0)?;
let turn = vm.poll_event_loop(TurnBudget::jobs(100))?;
```

Changing clocks with pending timers is rejected. Virtual and real-time clocks
schedule absolute `now + delay` deadlines. Virtual time advances only explicitly.
`ClockMode::RealTime(Rc::new(RealTimeClock::default()))` uses native monotonic time;
embedders can supply their own `Clock`. Injected clocks must return finite,
nonnegative, nondecreasing milliseconds. Browser real time clamps `Date.now()`
against backward movement.

`poll_event_loop` never waits. Its outcome reports actual `executed_jobs`
(including nested dispatch), `runnable`, `checkpoint_pending`, `next_deadline`,
and `Idle`, `JobBudget`, `TimeBudget`, or `Backpressure`. Deadlines use the selected
clock's millisecond origin. `run_event_loop_once(timeout)` reports true only when
jobs executed and does not wait after making progress. Native real-time waits end
at the earliest caller timeout, incoming wake, timer deadline, or execution
deadline. Cancellation also wakes an already sleeping owner. Spurious/stale wakes
recheck readiness within the remaining timeout; virtual turns never advance time.
WASM methods always poll without blocking the browser thread.

Microtasks are FIFO and recursively drained before another macrotask. Host ingress
is sampled at checkpoints. A yielded checkpoint must finish before timers or a new
evaluation; `ensure_can_evaluate()` enforces this for embedding entry points.
Every drain exit reconciles checkpoint state, including thrown callbacks and
hard-limit errors. Legacy `Vm.run` resumes an interrupted microtask checkpoint
before admitting new guest code, without refilling that checkpoint's hard budget.
A final throwing microtask clears the checkpoint; queued successors keep their
roots and checkpoint priority.
The Node and WASM adapters apply that guard automatically. Low-level Rust embedders
must call it before starting an evaluation themselves.

Soft job/time budgets yield at dispatch boundaries. They do not refill hard fuel,
loop, or job limits, including nested drains and awaits. Configure hard limits
with `set_execution_budget`, `set_fuel_budget`, `set_loop_budget`,
`set_execution_timeout`, and `set_cancellation_token`. AST and bytecode execution
cooperatively check fuel, cancellation, and deadlines. A synchronous native host
function cannot be preempted by the VM. Nested drains can exceed the outer soft
job count; the outcome counts all executed jobs, and hard limits still apply.

External-first ordering remains the default. `Fairness::Alternate` alternates an
eligible external event and timer, preserving FIFO within each class; a microtask
checkpoint follows either. This prevents a sustained external stream from starving
due timers. Default host batches are 64 and admitted external capacity is 1024.
`try_push_external_event` returns the rejected event to its producer when full.
Native ingress retains unconsumed events and re-notifies the owner. Custom bridges
with threaded ingress must implement `set_wake_notifier`; the scheduler uses
nonblocking ingress checks and a latched owner wait. `set_execution_context` is
an optional compatible trait hook for blocking host operations to receive the
execution cancellation token and remaining timeout. The Node sidecar uses it;
interrupting a transport request retires that connection to prevent stale replies. Legacy bridges
that return oversized batches retain overflow and report backpressure rather than
lose callbacks; their overflow cannot be memory-bounded without producer support.
Configure finite native producer queue sizes: existing native ABI queue size zero
continues to mean unbounded. Guest timer/microtask queues are not capacity-limited.

## Explicit Node owner session

```js
const { AsyncSession } = require('napi-vm');
const session = new AsyncSession({
  clock: 'real-time', fairness: 'alternate', commandCapacity: 64,
  maxJobsPerTurn: 1024, autoPoll: true,
});
try {
  await session.exposeFunction('answer', async () => 42, true);
  console.log(await session.run('await answer();')); // "42"
} finally {
  session.dispose();
}
```

Each session has one persistent owner thread. Guest values, interpreter state, and
coroutine stacks are constructed, used, and dropped there. Only owned wire data,
commands, cancellation tokens, and completion handles cross threads. Host JavaScript
and Node-value marshalling run on the Node environment thread through TSFNs.
Existing synchronous `Vm` APIs and the legacy per-execution `runAsync` remain available.
No new blanket unsafe `Send`/`Sync` implementation is used.

`run` evaluates and drains eligible work, returning the existing string result
format. `evaluate` schedules without draining. All state operations return Promises.
Use `autoPoll: false` with `pollEventLoop(maxJobs)` and `advanceClock(milliseconds)`
for explicit turns. `pollEventLoop` returns camel-case outcome fields.
`setExecutionLimits(fuel, timeoutMs)` applies limits to subsequent executions;
`cancel()` cancels all currently accepted commands, including ones not yet dequeued.
Queue saturation throws synchronously before accepting a command. The configured
capacity bounds outstanding command completions as well as queued commands.
Accepted completions settle unless their Node environment itself has terminated.

Notifications are latched and coalesced; events remain queued individually.
Idle sessions do not keep Node alive; pending command completions do. `dispose()`
is idempotent, cancels pending work, wakes the owner, waits for it to relinquish
Node handles, and retires references. Environment cleanup also joins the owner
before Node destroys its TSFNs, including Worker termination.

Unawaited host calls do not lock command admission. Stored result handles remain
available for a later guest await while reachable through globals, closures,
promises, or queued jobs. At command completion, the owner traces those roots and
retires abandoned receivers; Node retires their promise references at completion
or dispatch boundaries. Opaque suspended coroutine stacks conservatively retain
results. Pending-result capacity remains 1024. Await consumes a result handle
(as in the existing bridge); cancellation/disposal settle Node callbacks once.

Native real-time top-level await processes future and nested timers, preserving
microtask checkpoints and the active hard budget. A promise with no possible VM
or host progress fails clearly. Virtual and browser awaits of future timers
return a host-driven-progress error without advancing time or sleeping. Keep the
promise in a guest global, advance/poll from the host, then await it again; an
arbitrary top-level continuation is not automatically suspended and restored.

A host callback must not submit another command to its own waiting session:
that would deadlock, so the adapter rejects it explicitly. Unrelated synchronous
VMs remain usable from host callbacks. Awaiting a Node host completion currently
suspends the owner until settlement, cancellation, deadline, or shutdown; it does
not pump unrelated guest timers during that wait. A background auto-poll error
stops automatic scheduling and rejects future commands; recreate the session
for recovery. Arbitrarily blocking native host code can also delay shutdown.
These are remaining limitations, not changes to legacy defaults.

## WASM adapters

`set_clock('legacy' | 'virtual' | 'real-time')`, `advance_clock(ms)`, and
`poll_event_loop(maxJobs)` expose readiness and deadlines. An adapter can schedule
its next browser/desktop wake from `nextDeadline` and call another nonblocking turn
when `runnable` is true. A pending microtask checkpoint retains priority across
browser frames. The browser supplies the waiting mechanism; the VM never sleeps
on the browser thread.

## Verification and benchmarks

See [scheduler-benchmarks.md](scheduler-benchmarks.md) for reproduction, measured
results, tested configurations, and checks that were not run.
