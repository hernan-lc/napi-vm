# Scheduler benchmark results

Measured on 2026-09-30 in the configured cloud environment: Linux x86_64,
5 available CPUs, Rust 1.96.0, Node 24.19.0, release builds. Baseline:
`0fa987d8860d620cd1008a84f2a17c9b67c495cd`; implementation:
`41ab789` (all implementation and benchmark commits, before documentation).

## Reproduce

```sh
# On this configured environment, first source /workspace/.cloud-setup/activate.sh.
bench/run-scheduler.sh
```

The script archives the baseline into an isolated directory, copies identical
comparison workloads into both builds, compiles both before timing, and runs them
sequentially. Set `SCHEDULER_BASELINE_DIR` and `SCHEDULER_OUTPUT_DIR` to override
cache/output locations. The current script copies Linux x64 GNU addon filenames;
adapt those names to run on another platform. Rust allocator/queue comparisons
use one warmup and 25 samples. Node comparisons use 20 warmups and 500 sequential
calls. These are single runs in a shared cloud environment, not confidence intervals
or production guarantees. p99 for 25 batches is the slowest observed batch.

[Raw JSONL and machine metadata](benchmarks/scheduler-2026-09-30/machine.txt) are
checked in alongside this report. JSON includes all percentiles, allocations,
poll counts, queue bounds and unavailable metrics as null.

## Isolated queues and end-to-end interpreter

Throughput counts timer registrations/drains, cancellation operations, promise
reactions, or the mixed workload's 4,000 dispatched jobs per sample. Batch latency
includes queue setup, guest execution, drain and destruction; it is not individual
callback latency. Allocations count Rust allocation/reallocation calls across the
whole sample, not bytes or retained memory.

| Workload | Baseline ops/s | Modified ops/s | Ratio | Allocations/sample baseline → modified |
| --- | ---: | ---: | ---: | ---: |
| timers/100 | 11,104,547 | 3,932,295 | 0.35× | 6.0 → 24.0 |
| cancel/100 | 4,639,338 | 4,106,621 | 0.89× | 13.0 → 30.0 |
| timers/1000 | 1,040,044 | 2,528,958 | 2.43× | 9.0 → 177.0 |
| cancel/1000 | 523,253 | 2,739,803 | 5.24× | 19.0 → 188.0 |
| timers/10000 | 88,648 | 1,933,407 | 21.81× | 13.0 → 1,680.0 |
| cancel/10000 | 52,295 | 2,323,690 | 44.43× | 27.0 → 1,693.0 |
| vm/timers | 410,941 | 519,531 | 1.26× | 19,515.4 → 18,678.4 |
| vm/cancel | 785,401 | 679,702 | 0.87× | 19,514.9 → 19,513.0 |
| vm/promises | 515,658 | 518,803 | 1.01× | 24,520.2 → 24,521.2 |
| vm/mixed | 1,040,298 | 1,140,466 | 1.10× | 34,520.8 → 34,693.8 |

| Workload | Baseline batch p50 / p95 / p99 (µs) | Modified batch p50 / p95 / p99 (µs) |
| --- | ---: | ---: |
| timers/100 | 8.8 / 9.9 / 12.2 | 25.2 / 27.3 / 30.0 |
| cancel/100 | 17.9 / 45.5 / 69.9 | 23.6 / 26.2 / 42.0 |
| timers/1000 | 938.5 / 1,043.9 / 1,047.6 | 380.5 / 440.7 / 675.6 |
| cancel/1000 | 1,883.2 / 1,994.6 / 2,260.6 | 355.3 / 438.0 / 441.0 |
| timers/10000 | 111,835.0 / 119,871.1 / 121,151.9 | 4,774.9 / 7,283.8 / 9,016.8 |
| cancel/10000 | 190,151.2 / 195,966.8 / 196,524.8 | 4,275.6 / 4,579.9 / 4,696.5 |
| vm/timers | 2,419.0 / 2,704.5 / 2,745.0 | 1,880.0 / 2,203.0 / 2,311.7 |
| vm/cancel | 1,248.4 / 1,365.1 / 1,550.0 | 1,456.8 / 1,632.1 / 1,759.8 |
| vm/promises | 1,925.9 / 2,232.0 / 2,288.1 | 1,880.4 / 2,226.5 / 2,374.3 |
| vm/mixed | 3,776.0 / 4,132.9 / 4,271.2 | 3,334.5 / 4,707.9 / 5,730.9 |

The ordered structure improves large timer/cancellation batches and has a higher
allocation cost. The 100-item queues regress; a tree and two cancellation indices
cost more than a short vector scan. Promise/fuel/checkpoint bookkeeping also has
costs, shown in the table rather than assumed away. Cancellation-churn throughput
in the interpreter includes scheduling and immediately cancelling one timer at a
time, whereas the isolated cancellation workload creates a whole batch first.

The mixed workload makes 4,001 baseline versus 2,001 modified host polls/sample. Checkpoint polling removes per-microtask host sampling.

## Repeated trivial asynchronous calls

| Mode | Calls/s | p50 / p95 / p99 (ms) | CPU ms for 500 calls | Idle CPU ms / 250 ms | Wake notifications |
| --- | ---: | ---: | ---: | ---: | ---: |
| baseline Vm.runAsync | 21,732 | 0.0370 / 0.0925 / 0.1660 | 15.192 | 0.327 | unmeasured |
| modified Vm.runAsync | 22,185 | 0.0354 / 0.0933 / 0.1787 | 14.747 | 0.367 | unmeasured |
| modified AsyncSession.run | 37,121 | 0.0238 / 0.0378 / 0.0719 | 11.751 | 0.312 | 500 |

The explicit persistent session delivers 1.71× baseline throughput in this workload. All calls are sequential, so outstanding command depth is 1. Node allocations were not instrumented.

## New real-time and wake APIs

These metrics have no equivalent absolute-deadline/wake instrumentation in the
legacy baseline, so they are observations of the new APIs, not claimed speedups.

- 2,500 real-time timer callbacks: 18,789 jobs/s;
  caller-deadline lateness p50 / p95 / p99:
  39.95 / 72.64 / 323.55 µs.
  Measured queue peak: 100.
- 10,000 ingress events through capacity 32:
  1,727,424 events/s; delivery latency p50 / p95 / p99:
  8.69 / 9.97 / 26.45 µs;
  314 coalesced wake notifications. No events lost.
  Idle process CPU during one 250 ms wait: 18 µs.
- Measured queue peak `vm/timers`: 1000.
- Measured queue peak `vm/cancel`: 1.
- Measured queue peak `vm/promises`: 1.

The common queue benchmark reports conservative depth bounds; measured peaks are
separate instrumentation using the modified VM. Timer lateness measures dispatch
relative to the caller's deadline and includes insertion overhead. Wake counts
count latched transitions, not OS context switches. Idle CPU uses process CPU
accounting, not host-wide utilization. Baseline timer lateness, baseline wake
counts, Node allocations, memory bytes, and per-callback latency for legacy timer
batches were not measured.

## Verification

Passed on this environment:

- `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
- `cargo test --locked --release --all-features`: 449 passed, 1 ignored.
- All-feature debug library and scheduler tests: 368 + 16 passed.
- Node tests: 29 passed, including persistent ownership, Node-thread marshalling,
  bounded admission, cancellation before dequeue, pending promise GC rooting,
  same-session reentrancy rejection, idle lifetime, shutdown, and Worker teardown.
- Bun suite: 1,470 passed; TypeScript and playground TypeScript checks passed.
- WASM release build and adapter tests: 9 passed in the Node harness.
- Native addon loading/TSFN tests in the all-feature Rust suite, runtime smoke,
  and IPC smoke passed.
- CJS/ESM/TypeScript bindings regenerated and matched checked-in files.

Scheduler tests cover equal deadlines, nested timers, normalized invalid delays,
ID wrap without collision, callback/root release, job/fuel boundaries, checkpoint
resumption, real/virtual clocks, host fairness and retained overflow, actual progress,
cooperative long-callback interruption and nested hard-budget accounting. Wake
race tests retain all 10,000 events; a Worker teardown regression originally exposed
late TSFN release and now passes after joining the owner during cleanup.

Not run: the ignored real rdev-node/X-display test (requires a built external addon
and isolated display), real browser UI/manual timing checks, other native platforms
and architectures, Miri/sanitizers, heap-byte profiling, or long-duration soak tests.
See [event-loop.md](event-loop.md) for migration and remaining scheduling/ownership
limitations. No claim is made that the old unbounded producer ABI or synchronous
native host code now has strict memory/preemption bounds.
