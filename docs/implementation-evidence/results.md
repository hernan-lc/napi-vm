# PR #8 implementation evidence

Local correctness, ownership, Miri, and measured performance gates passed for code head `dc612735aaa2eb10dd33201d32d24c6ba677a72a`. PR #8 remains draft: the remote OS/architecture/Node matrix is blocked because repository Actions are disabled, and the user explicitly directed that they remain disabled. No remote pass or merge approval is claimed.

Final-source measurements were taken at 2026-10-01T02:12:46Z against pinned main `cf9d240a99747e29d94b970c30483d14ad831e9f` on Linux x64, AMD BC-250, v26.10.0. Documentation-only commits after this code head do not change the measured source or binaries. SHA-256 hashes are recorded in [measurement context](pr8-final-performance/measurement-context.json) and [build metadata](pr8-final-validation/current-build-metadata.json).

## Correctness and ownership

| Finding | Fix and regression |
| --- | --- |
| Sidecar guest roots | `NodeAddonSidecar::trace_roots` visits proxies, callbacks, graph nodes, symbols and promises. The collection regression drops guest-visible references, collects, and successfully uses retained objects/callbacks and settles the promise. [HostBridge audit](host-root-audit.md) covers every implementation and callback-bearing fixture. |
| Owner-affine pins | `RootPin` carries `PhantomData<Rc<()>>`; static assertions reject `Send` and `Sync`. Pins must be created and dropped in the same owner context. |
| Arrow super receiver | Arrows resolve lexical `this`; ordinary functions use the call-frame receiver. AST/bytecode regressions cover constructor super, super methods, nested arrows and ordinary nested-function receiver isolation, preserving AST behavior. |
| Pending checkpoints | Resume the existing checkpoint with its original hard budget, check admissibility, then parse/compile new source. Valid, syntax-error, exhausted-budget and error-precedence regressions pass. |
| Failed export setup | Function creation and finalizer-registration failures synchronously release the slot through the runtime gate. Only the actual finalizer queues a release. Fault injection proves no slot, pin or stale reusable ID remains. |
| Owner migration | Production `OwnerContext` and `RuntimeCell` compile under Miri with `napi`, without live Node FFI. Tests cover thread A/B leasing, panic/nested TLS restoration, cross-thread destruction, repeated sync/async handoffs, and isolated roots/shapes/symbols/collection prototypes. |
| Hot-path scans | `PreparedProgram::tier()` is O(1). Exact-source parse-cache hits append generation records in O(1), with bounded periodic cleanup and unchanged byte/entry limits. |

The [before-fix logs](pr8-final-validation/README.md) reproduce the original callback-root, arrow-super, checkpoint, pin-trait, creation rollback and finalizer rollback failures. No parity, execution-budget or ownership assertion was removed.

## Local validation

Every requested command passed:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --release --all-features
cargo test --no-default-features
npm run build:all
npm run check:generated
npm run lint
npm run test:node
npm test
npm run test:wasm
git diff --check
cargo test --release --test compiler_memory -- --test-threads=1
cargo test --release --test gc_public_lifecycle
cargo test --release --test evaluation_modes
```

The JavaScript suite preserves all 1,487 assertions: 1,432 Bun, 46 Node and 9 Wasm tests, with dedicated runners for each environment. Explicit Linux x64 Node 22.23.3 and 24.21.0 runs also passed; the default Node 26 run passed. [Complete command records](pr8-final-validation/current-results.json) link to raw logs.

Expanded Miri passed with leak detection enabled:

```text
cargo +nightly miri test --no-default-features --features napi --lib owner_migration       # 4 passed
cargo +nightly miri test --no-default-features --features napi --lib failed_export_setup  # 1 passed
cargo +nightly miri test --no-default-features --lib shape::tests                         # 6 passed
```

[Miri records](pr8-final-validation/miri-results.json) and [machine/source context](pr8-final-validation/machine-context.json) retain versions, features and raw output.

## Final-source performance

The existing interleaved harness ran all 21 alternating baseline/modified pairs. Instrumentation sources are identical in both checkouts. Every pair is included. The reported change is the median of paired p50 ratios; confidence intervals use 10,000 paired bootstrap resamples with a fixed seed. Absolute times are medians and their quotient can differ from the paired estimate. Compiler/Miri activity was monitored; none overlapped the accepted trial. Other desktop activity is retained in the process log, so these are local observations, not a universal performance guarantee.

| Required public gate | Baseline µs | Modified µs | Paired change | 95% interval |
| --- | ---: | ---: | ---: | --- |
| runCode/tiny | 235.088 | 233.692 | +0.95% | [+0.14%, +2.45%] |
| runCode/arithmetic | 581.607 | 561.474 | -3.97% | [-6.28%, -2.39%] |
| VM.run/arithmetic | 323.917 | 316.054 | -2.32% | [-3.14%, -1.82%] |
| VM.runAsync/arithmetic | 384.159 | 333.847 | -12.95% | [-15.79%, -10.78%] |
| AsyncSession.run/arithmetic | 338.717 | 334.881 | -0.84% | [-1.80%, -0.22%] |
| AsyncSession.evaluate/arithmetic | 338.636 | 332.519 | -1.32% | [-2.01%, -0.77%] |

| Timer depth | Drain change | Cancel change | Partial-cancel change |
| ---: | ---: | ---: | ---: |
| 1 | -44.98% | -56.47% | -56.89% |
| 10 | -52.48% | -65.33% | -57.33% |
| 32 | -45.02% | -55.61% | -48.22% |
| 64 | -29.28% | -42.08% | -36.02% |
| 100 | -15.85% | -31.41% | -25.78% |
| 1000 | -15.79% | -10.78% | -13.35% |
| 10000 | -12.25% | -9.83% | -13.76% |

All 38 public workloads and all 21 timer cases have paired median regressions within 5%. The largest public estimate is callFunction/callFunction: +3.48%; the largest timer estimate is cancel/10000: -9.83%. No estimate above 5% is approved.

5 intervals extend above 5%; passing point estimates do not prove a strict 5% upper bound. Full intervals and allocation counts are in the [comparison summary](pr8-final-performance/comparison-summary.json); [required gates](pr8-final-performance/required-gates.json) and all raw samples are alongside it.

Profiles preceded optimization: fresh-owner setup/collection, global dispatch, fuel accounting, prepared-code copies, parser-cache access and lease swaps were investigated. Changes reduce scalar teardown/cloning, share only guard-free feedback-disabled verified code, reuse bounded empty owner registries only after full collection, avoid property-name formatting, borrow existing frame operands and use keyed runtime-map hashing. Fuel costs, instruction checkpoints, full collection, exact source equality and AST/bytecode semantics remain intact. No unsafe `Send`/`Sync` boundary was expanded. Diagnostic profiles are explicitly labeled by earlier code head in [profile context](pr8-final-validation/profile-context.txt); they are not acceptance timings.

## Remote gate

Linux, macOS, Windows, x64, ARM64 and Node 22/24/26 remote jobs have **not run**. [Remote gate record](pr8-final-validation/remote-gate.json) confirms Actions are disabled. The workflow contains the full matrix and expanded Miri commands, but local Linux checks cannot satisfy the remote gate. PR #8 stays draft until that gate is satisfied.
