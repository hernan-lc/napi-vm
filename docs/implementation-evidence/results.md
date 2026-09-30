# Measured results and remaining release gates

The twelve implementation workstreams are present, with VM.run/runAsync compatibility retained. Local correctness gates pass. **Performance acceptance and remote release gates are not complete.** Do not treat this patch as release approved.

The reviewed baseline is `cf9d240a99747e29d94b970c30483d14ad831e9f`. The table uses seven interleaved process pairs on this Linux x64 machine. Times are medians in microseconds per operation (per scheduled timer for queue workloads). Change and 95% confidence intervals use the median of paired ratios and 10,000 paired bootstrap resamples. Ratios of the two displayed medians can differ from the paired estimator. Intervals spanning zero are inconclusive. These intervals describe this small local sample, not all supported systems.

| Workload | Baseline µs | Modified µs | Paired change | 95% interval |
| --- | ---: | ---: | ---: | --- |
| call_metrics / global-loop | 39.621 | 34.262 | -17.3% | -19.1% to -12.7% |
| call_metrics / zero | 0.549 | 0.541 | -7.5% | -10.8% to -0.4% |
| call_metrics / two | 0.840 | 0.746 | -12.9% | -14.8% to -4.9% |
| call_metrics / method | 0.885 | 0.781 | -10.4% | -14.2% to -7.8% |
| call_metrics / recursive | 7.389 | 5.992 | -19.3% | -24.7% to -14.7% |
| timer_queue_matrix / drain / 1 timers | 0.113 | 0.062 | -47.1% | -48.5% to -38.3% |
| timer_queue_matrix / drain / 10 timers | 0.124 | 0.062 | -49.9% | -51.6% to -45.9% |
| timer_queue_matrix / drain / 32 timers | 0.140 | 0.092 | -34.2% | -40.1% to -33.9% |
| timer_queue_matrix / drain / 64 timers | 0.148 | 0.125 | -13.9% | -19.0% to -4.3% |
| timer_queue_matrix / drain / 100 timers | 0.166 | 0.162 | -5.9% | -30.2% to +1.7% |
| timer_queue_matrix / drain / 1000 timers | 0.214 | 0.225 | +9.2% | -4.6% to +9.6% |
| timer_queue_matrix / drain / 10000 timers | 0.313 | 0.334 | +3.2% | -7.4% to +17.1% |
| public / runCode / tiny | 150.006 | 201.022 | +35.0% | +28.1% to +36.5% |
| public / VM.run / tiny | 3.195 | 3.145 | -1.9% | -3.7% to -1.5% |
| public / VM.runAsync / tiny | 29.414 | 18.447 | -36.2% | -58.7% to -6.0% |
| public / AsyncSession.run / tiny | 18.488 | 18.187 | -1.1% | -34.2% to -0.5% |
| public / AsyncSession.evaluate / tiny | 18.067 | 18.057 | +0.3% | -1.7% to +1.1% |
| public / runCode / arithmetic | 400.583 | 527.254 | +34.5% | +28.6% to +39.1% |
| public / VM.run / arithmetic | 218.719 | 306.772 | +40.4% | +39.0% to +42.4% |
| public / VM.runAsync / arithmetic | 293.692 | 344.960 | +19.7% | +12.2% to +23.2% |
| public / AsyncSession.run / arithmetic | 255.735 | 346.262 | +36.4% | +33.5% to +40.3% |
| public / AsyncSession.evaluate / arithmetic | 254.022 | 342.255 | +35.1% | +31.5% to +38.6% |

Prepared bytecode calls reduce measured allocations per execution: zero-argument calls 13 to 6, two-argument calls 15 to 6, and the global-loop workload 415 to 6. These figures exclude preparation and are not allocation counts for public JS entrypoints. Borrowing global names and property operands improves the prepared tier; switching public evaluation from the old AST route to bytecode still exposes a global-loop dispatch regression.

The final existing Criterion property canaries report monomorphic access 882.11 to 710.17 µs, megamorphic access 1.6616 to 1.3352 ms, and shape churn 796.19 to 730.16 µs. These are one interleaved baseline/modified pair with 30 Criterion measurements each; see the raw confidence intervals. Four interleaved tiny-call pairs also improve. Criterion's printed “change” compares its own previous saved run, so it must not be read as the baseline/modified comparison.

The compiler RSS regression fails the original revision (4,040 to 75,396 KiB after 20,000 warmed synthesized compilations) and passes the implementation's 4 MiB growth limit. Node lifecycle tests cover 20,000 persistent cyclic workloads, exported-function finalization, alternating synchronous/asynchronous state, and owner teardown.

## Release blockers

- Cold `runCode` is slower, and public global arithmetic loops regress across execution modes. Fresh calls now pay isolated owner setup, preparation, and teardown collection; public global loops also expose bytecode dispatch costs. The prepared bytecode improvement does not satisfy the separate public API performance gate. Further optimization and fresh comparisons are required before accepting regressions beyond the specification's 3–5% threshold.
- Large timer workloads have wide intervals: depth 1,000 and 10,000 do not establish a within-5% result. The selected threshold is 128, with bounded empty storage reuse; repeat longer measurements on stable machines before release. Do not infer a speedup at those depths.
- The new platform/Node CI matrix and focused Miri workflow are configured but have no remote results from this task. Local success covers Linux x64 only. At measurement closeout the configured CLI GitHub token was invalid, so no PR stack was published and no remote CI was dispatched. Subsequent PR publication uses the authenticated connector; remote results must still be reviewed.

Raw final samples are at the root of this directory. Earlier timing rounds are retained in `pre-timer-reuse/` and `before-global-name-borrowing/`; they do not represent the final implementation. `closeout-results.json` records the latest eleven passing local commands. `closeout-release-all-features.log` records the additional final full release Rust test run.

The final `cargo test --release --all-features` completed successfully: 479 passing tests across its library, integration, and documentation suites.
