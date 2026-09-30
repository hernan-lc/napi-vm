# PR #7 verification

Reviewed head: `ba1443705bb80d2e4eafeb2783e74b2cb1ad40b3` (also the remote head at the start of this task). Final implementation and regression-test revision: `b918b38`.

## Reproductions

The checked-in addon was stale and did not export `AsyncSession`. A fresh isolated build of the reviewed source reproduced all three reported failures:

- The throwing-microtask example threw `boom`, then `Vm.run("42;")` failed with `resume the pending microtask checkpoint before evaluating new guest code`.
- `background(); 42;` returned `42`, but the next command failed with `async session is awaiting Node`.
- Awaiting a 50 ms real-time timer failed with `cannot synchronously await a pending Promise: no VM or host event can settle it`.

Reproduction build: `git archive ba1443705bb80d2e4eafeb2783e74b2cb1ad40b3` into `/tmp/napi-vm-reviewed`, then `cargo build --manifest-path /tmp/napi-vm-reviewed/Cargo.toml --target-dir /tmp/napi-vm-reviewed-target --lib`; its library was copied to the platform addon filename before running the supplied examples with Node.

A controlled-clock ordering regression also fails against `4fa8b9a`, before the one-event await fix, and passes in the final implementation. It forces both timers to become due through the wait path and requires code after await to run before the second timer. That comparison used a separate target directory to avoid sharing compiled artifacts across source trees.

## Changes

- Drain guards reconcile checkpoint state on every exit. Queued jobs retain their roots. Legacy `Vm.run` finishes a pending microtask checkpoint with its old hard budget before starting another program.
- Actual Node waits control session admission independently of stored results. A quiescent root walk keeps result handles reachable through globals, closures, promises and queued jobs; opaque suspended stacks conservatively retain handles. Abandoned receivers and Node promise references retire at owner/main-thread lifecycle boundaries. Settlement remains one-shot.
- Native real-time top-level await waits for future timers and resumes after one event and its microtask checkpoint. Virtual/browser awaits report that host-driven progress is required, retain timers/promises, and never sleep or implicitly advance time.
- Latched condition-variable waits cap time by the caller timeout, timer and execution deadline. Cancellation wakes sleeping owners. Stale/spurious wakes recheck readiness. Node replies and sidecar responses use wake signals instead of response polling; sidecar EOF becomes visible before notification. Interrupted sidecar requests retire their connection to avoid stale replies.
- Timer IDs reside alongside callbacks in the ordered tree, eliminating the reverse index. Ordering, cancellation and GC roots remain covered by tests.
- Full Bun testing exposed an existing deep-array teardown stack overflow. Heap registry weak references prevented `Rc::get_mut` from enabling iterative destruction. Sole-owner arrays/objects now drain through their cells; a small-stack regression drops 20,000-deep tracked graphs.
- WASM compilation no longer references the native-only `url` dependency. Bun package/CI commands explicitly use `./tests`, excluding vendored Node-only tests that call Bun's unimplemented `node:test` `t.skip`.

## Exact local checks

Run against the final implementation on Linux x86_64, Rust 1.97.1, Node 26.10.0, Bun 1.4.0 (the installed executable identifies itself as a canary build in crash diagnostics):

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| `cargo test --all-features` | 455 passed, 1 ignored; includes Linux native-addon fixtures and 21 scheduler tests |
| `cargo test --no-default-features` | 422 passed, 1 ignored |
| `bun test ./tests --timeout 30000` | 1,477 passed with debug addon |
| `bun test ./tests` | 1,477 passed with final release addon |
| `node --test tests/node/*.test.js` | 36 passed |
| `npm run lint:ts` | Passed |
| `npm run lint:playground` | Passed |
| `cargo check --no-default-features --features wasm --target wasm32-unknown-unknown` | Passed after native-dependency guard |
| `npm run test:wasm` | WASM release build, playground types and 9 browser tests passed |
| `npx napi build --platform --release --esm --js index.mjs` then `npx napi build --platform --release --js index.js` | Passed; regenerated bindings unchanged |
| `git diff --exit-code -- index.js index.mjs index.d.ts` | No public N-API binding changes |
| `bench/run-scheduler.sh` | Original baseline versus final implementation; see measured report and raw records |

Builds, addon replacement, tests and final benchmark timing are serialized. Earlier exploratory Bun processes also collided with addon replacement and are excluded from final results. The independently reproduced reviewed-head full-suite stack overflow was investigated under GDB, which showed recursive `ArrayCell` teardown. The final scoped repository suite passes. Unscoped `bun test` includes vendored input-device tests and fails because Bun cannot implement their Node-only skip call; it is not the repository test command after this change.

GitHub Actions was disabled (`GET /repos/nglmercer/napi-vm/actions/permissions` returned `enabled: false`). It was enabled before the final push so the pull-request workflow can run. The CI workflow also accepts manual dispatch and pushes to the PR branch, allowing an explicit final-head run when a synchronize event does not schedule one. Check [the PR's current checks](https://github.com/nglmercer/napi-vm/pull/7/checks) for the final documentation descendant of the implementation revision; local checks do not establish remote CI success.

## Remaining limitations / checks not run locally

- macOS, Windows, ARM native execution, cross-platform addon ABI behavior and Node-version matrices require CI; they were not run locally.
- Awaiting a Node host completion does not pump unrelated guest timers. Arbitrarily blocking in-process native functions cannot be preempted.
- A background auto-poll hard error stops session scheduling; recreate the session. Legacy checkpoint recovery does not refill exhausted hard budgets.
- Virtual/browser future-timer await is a nonblocking error with host-driven recovery through a stored promise, not resumable arbitrary top-level code.
- Opaque coroutine stacks conservatively retain pending results until their lifecycle ends. Live stored results remain bounded by the existing 1,024-handle limit and are consumed by await as before.
- Custom threaded bridges need wake notifications for prompt scheduler wakeups and must implement the optional execution-context hook to interrupt their own blocking operations. Sidecar startup/shutdown process-management polling remains separate from guest event waits.
- Benchmark Node allocations/retained bytes and browser/other-platform idle CPU were not measured. Rust records allocation/reallocation counts for entire batches, not retained memory or bytes. Single-host samples are not production guarantees.

See [event-loop behavior](event-loop.md) and [benchmarks](scheduler-benchmarks.md).
