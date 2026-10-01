# Final-source performance samples

All 21 baseline/modified pairs from the complete uncontended-by-compilation trial are included. `measurement-context.json` identifies the measured code/source tree, baseline, binary hashes, identical instrumentation and machine versions. A later documentation-only commit does not change these binaries.

`comparison-summary.json` records every public, timer and Rust call-metrics row, with paired median ratios and bootstrap 95% intervals. `required-gates.json` selects the six required public gates and 21 timer cases. `process-monitor.jsonl` retains concurrent desktop activity; no compiler or Miri overlapped this accepted trial.

Earlier complete failed trials and interruption/build-invalidated trials remain archived outside this final-only evidence directory. No individual samples were omitted from this accepted 21-pair run. Remote platform checks remain blocked and the PR remains draft.
