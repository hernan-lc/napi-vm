# PR #8 local validation records

`current-results.json` and `current-*.log` record the final-source required local commands. `current-build-metadata.json` records the code revision and SHA-256 hashes of the native addon and benchmark executables. The Node 22 and 24 runs use downloaded, version-verified official binaries, with `PATH` set so their child processes use the same Node version.

`miri-results.json` and `miri-{owner,export,shapes}.log` record the expanded Miri commands. Owner migration and export rollback compile the production `OwnerContext` and `RuntimeCell` with `--no-default-features --features napi`. They create no Node environment and invoke no live Node FFI. Leak detection remains enabled.

`before-*.log` reproduce failures against the original PR head `71a9bb314724cf4e2ad034bf928d1883d3aec760`. The export controls preserve the setup factoring but restore queued rollback or omit finalizer-failure rollback. The ordinary-function parity regression preserves AST behavior and tests receiver isolation.

`prior-head/` retains the initially failing empty-owner fixture. Superseded passing validation runs remain archived outside the final evidence directory. That fixture observed lazy initialization of ambient shape TLS (`1` versus an uninitialized snapshot of `0`). The corrected fixture initializes the ambient arena before taking counters and additionally verifies pointer identity; no assertion was removed.

The `profile-*` text files are diagnostic profiles from explicitly identified earlier revisions, not final performance acceptance measurements. Final timings and paired confidence intervals are in `../pr8-final-performance/`.

Remote OS/architecture/Node checks are blocked because repository Actions are disabled. The user explicitly directed that this setting remain disabled. Local Linux x64 checks do not substitute for that matrix.

Committed console/profile logs normalize trailing whitespace and final blank lines only so `git diff --check` passes. Their original bytes remain archived locally; test results and diagnostic content are unchanged.
