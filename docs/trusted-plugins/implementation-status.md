# Trusted process plugin implementation status

Baseline: `238cf7c9de8e68601e13c401401c466a1feddf24`, clean `main`, local branch `feat/portable-plugin-hosts`. This is a fresh implementation. The earlier interrupted task's source was not recovered, and its reported tests are not counted here.

Environment: Linux x64 GNU, Node 24.19.0, Bun 1.4.2, TypeScript 6.0.3, Rust/Cargo 1.98.1. No remote push, release or package publication has occurred.

## Scope clarification

The latest requested asymmetric matrix overrides the initial design: TS host (Node/Bun) accepts JS/TS artifacts only. Rust host accepts JS/TS plus compatible native executables. The same compiled TS package is reused across hosts. TS rejects executable routes, including standalone Bun binaries; Rust can run those without an external JS runtime.

## WP00 — Baseline and preservation
Status: COMPLETE

Implemented:
- Inspected clean checkout, root package/crate settings, CI, and legacy boundaries; no AGENTS.md found
- Existing VM source, legacy plugins, runtime, LSP and generated root bindings remain unmodified
- Removed redundant root `"."` membership because it caused Cargo to ignore nested standalone exclusions; the root remains an implicit member/default. The legacy N-API fixture again resolves its own workspace and lockfile, and its locked dependency fetch passes

Verification:
| Command | Result | Evidence |
|---|---|---|
| `tsc --noEmit` before integration | PASS | Baseline strict TypeScript check |
| `cargo metadata --no-deps --format-version 1` before integration | PASS | Root package identified before workspace changes |
| VM Wasm dependency tree after integration | PASS | No new process crates or Tokio in normal Wasm graph |

Decisions:
- Initial npm install failed because the default cache directory was unavailable; an explicit workspace-local cache fixed it
- Root legacy Rust compilation was paused to serialize memory-heavy work; final regression results are tracked under WP10

## WP01 — Independent packages and protocol
Status: COMPLETE

Implemented:
- Five new JS packages and four independent Rust crates; root Cargo default member remains the existing VM
- Separate strict TS package/example checks and declaration-consumer checks for authored ESM CLI/codegen
- Explicit manifest, descriptor and complete protocol-control schemas

Verification: New TS packages compile; all seven new Rust crates/examples compile. Rust-only template `cargo check` and `cargo test` execute with Node/Bun absent from PATH.

## WP02 — Real vertical slices
Status: COMPLETE

Verification:
- Node/Bun hosts with Node/Bun plugin runtimes: four actual process combinations, one compiled TS artifact
- Rust host with the same TS artifact under Node and Bun, plus Rust-native plugin: three additional combinations
- Extra: Rust host with standalone Bun artifact and no external Node/Bun on PATH
- Real host callbacks and declared domain errors verified; unsupported TS-native route rejected

Initial interoperability exposed request-counter and fractional-timeout mismatches. Both were corrected; wire timeout is a positive integer and counters begin at 1. A later explicit Rust integration run hit its two-second runtime-version probe deadline; the Rust bound is aligned with the JS host at five seconds, and the final explicit integration rerun passes.

## WP03 — Protocol and supervision
Status: COMPLETE

Implemented: Strict data/envelope validation, framing, request correlation, cancellation, fail-fast serial business admission, bounded priority writer/events/logs, runtime preflight, secret-redacted logs and direct-child cleanup.

Verification: Final aggregate suites pass 113/113 under Node and 113/113 under Bun, including lifecycle and explicit-null initialization regressions. Final synchronized Rust correctness suite passes 42 tests, with its explicit cross-runtime integration entrypoint selected separately. Syntax checks, formatting and strict all-targets Clippy pass across all seven new Rust packages/examples. Tests cover real-process timeout, snapshot, cleanup, exit, log, integrity and shared writer-budget bounds.

Decisions: Concurrent busy business calls fail promptly instead of entering a potentially deadlocking queue. Already-running event callbacks may finish; queued retired-session events are discarded and counted. No operation retries or rollback guarantees.

## WP04 — Contracts and typed SDKs
Status: COMPLETE

Implemented: Deterministic normalized contracts/digests, strict subset rejection, typed TS/Rust clients/handlers/errors/events, manifest-derived identity, optional/null presence, canonical wide integers/bytes/dates, checked finite Rust numbers, in-process harnesses, tracked author tasks.

Verification: Generated-file check passes. Generated Rust fixtures compile and pass numeric, nullable, escaped-literal, prototype-key and identifier-collision tests. The shared schema vector is executed by both runtimes. Canonical Rust hashing uses an RFC8785 serializer rather than an independently invented format.

A transient aggregate check overlapped generator correction and reported drift; regeneration plus the read-only check and full Bun rerun passed afterward.

## WP05 — Lifecycle, events and state migration
Status: COMPLETE

Verification: TS→Rust→TS and Rust→TS→Rust counter migrations preserve 12, advance 15 then 17; logical handles/subscriptions remain stable and sessions rotate. Failed quiescence retains DRAINING; failed replacement retains a validated snapshot for explicit recovery. SDK author tasks drain before snapshot. In-flight handler events/results remain valid during draining, and retained call-bound callback contexts cannot invoke a replacement session. Direct child exit is reaped even when an escaped descendant holds inherited pipes.

## WP06 — CLI and author workflows
Status: COMPLETE

Implemented: create/codegen/check/validate/doctor/inspect/build/pack/invoke/test/dev, TS/JS/Rust templates, explicit native-host adapter, serialized watch/rebuild/reload and signal cleanup.

Verification: Node/Bun template compilation, artifact invocation and logic tests pass; watch/edit/reload/interrupt smoke passes. Rust-only check/test passes without JS tools. Actual fresh-template offline npm installation and subsequent Bun lock migration, typechecks, builds, tests, invocation and live reload all pass. Unpublished local package names resolve to explicitly vendored package copies. The synchronized Rust template passes locked check/test/release with Node/Bun absent from PATH, plus real packaged invocation and edit/rebuild/reload/Ctrl-C through the explicit Rust adapter. Native dev stages are checksummed before launch; the affected CLI/codegen suite passes 17/17 after that final fix.

## WP07 — Relocated artifacts
Status: COMPLETE

Verification:
- JS package relocated under spaces/Unicode paths executes under Node and Bun
- Declared text assets and dynamic local imports survive relocation
- Tampered packaged bytes fail integrity checks
- Packed npm Node-only consumer installs offline without workspace links
- Rust and Bun standalone plugins and host execute outside the checkout with Node/Bun/Cargo absent from PATH
- Controlled Node-API v1 fixture is target/ABI inventoried and injected; answer 42 verified

This single native probe does not establish universal addon compatibility. Inventories distinguish built, distributed and runtime-tested facts.

## WP08 — Conformance and CI
Status: COMPLETE

Implemented: Shared wire/schema fixtures, actual runtime matrix, ignored-by-default Rust integration entrypoint requiring explicit artifact paths, native CI and unchanged separate legacy CI boundaries.

Final-source seven-route matrix, standalone Bun executable, bidirectional migration and unsupported-route rejection pass. All six relocated/package checks pass. The explicitly selected Rust integration entrypoint is also run separately. macOS/Windows native execution is not available in this local-only task; configured workflows are not execution evidence. Any cross-target checks are labeled compile-only.

## WP09 — Documentation and measurements
Status: COMPLETE WITH DOCUMENTED VALIDATION LIMITS

Implemented: Architecture, protocol, manifest, contract, development, packaging, testing, compatibility and migration guides, plus detailed TS verification report. JavaScript benchmark measures actual startup, p50/p95/p99,64B–1MiB payloads, bounded offered load,1/5/20 instances, RSS, lifecycle phases and footprint.

Completed: Equivalent counter workload under the Rust host with TS/Node, TS/Bun and Rust plugins, 10 warmups and 30 timed increments each, validated final state 40. Node/Bun echo reports include all payload/load/instance/lifecycle measurements. Numbers and debug modes are disclosed in the evidence JSON. Startup and initialization are combined; isolated restoration CPU timing is not measured. Migration correctness is separately verified.

## WP10 — Final verification and delivery
Status: COMPLETE WITH DOCUMENTED VALIDATION LIMITS

Passed: Final strict new-package lint/build and legacy TypeScript checks.

Legacy Rust: 446 tests pass across the VM units, completed integration suites, separately selected scheduler suite and documentation example. This includes all 351 VM unit tests and 27 LSP protocol tests. Three LSP-runtime tests fail at temporary Unix-socket bind with EPERM from this environment, including after the supported command escalation. No workaround or source change bypasses that restriction. Cargo stops at that failure, so subsequent suites are selected separately.

The debug N-API binding builds and generated declarations match byte-for-byte. Debug recursion crashes reproduce with the preserved binary against the exact original JS tree; native dependency versions/features are identical. This comparison does not claim a separately compiled baseline binary. The repository's release build gate passes: Node 46/46, Bun 1431/1432 with a 30-second per-test allowance. All 33 crash-safety cases pass. Bun's sole remaining failure is Unix IPC bind EPERM, also reproduced from the original baseline; the supported command escalation does not remove the restriction. At the default five-second Bun deadline, two stress tests exceeded the deadline; they pass under the stated allowance. Debug suites are not marked passed.

Final checks: Node 113/113, Bun 113/113; Rust 42 passed with two intentionally ignored tests (cross-runtime integration selected separately and passing; controlled sidecar helper exercised by its parent test). All seven required routes, additional standalone Bun execution, both migrations and six package checks pass. Generated drift, strict TypeScript, formatting and all-targets Clippy pass. The synchronized Rust-only template passes check/test/release without Node/Bun on PATH and final packaged invocation.

Root VM wasm32 compile passes, with one existing dead-code warning. Protocol/SDK/host Windows GNU compilation passes; this is compile-only. Native macOS/Windows execution, browser runtime execution and Rust 1.96 CI execution are not claimed. Four legacy Unix-socket checks remain environment-blocked in total. No implementation review bugs remain open.

Delivery consists of the full source tree, a binary-capable patch against the stated base, reproduction instructions and captured evidence. No compiled dependency tree or private environment helper is included. The patch is checked against a clean archive of the base and the ZIP is CRC-verified before delivery.

See [compatibility](compatibility.md) for explicit support boundaries. HTTP/MCP service injection exposes connection capabilities, not a full MCP client. Native libraries require a declared supported external-runtime Node-API profile or an explicit managed executable service. Library callers must await shutdown. No claim covers arbitrary descendants, external-effect rollback, or unexecuted platforms.
