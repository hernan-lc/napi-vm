# Workflow build setup

All Rust jobs use `.github/actions/setup-build` after installing Rust:

- Linux GNU targets use mold 2.40.4. Its x64/arm64 release archives are checked
  against fixed SHA-256 hashes and cached. Only GNU target flags change; WASM
  and static musl targets retain their existing linkers.
- Windows MSVC targets use `lld-link`, an alias of Rust's bundled LLVM linker.
  The MSVC action still supplies the C compiler, Windows SDK and import libraries.
  Both x64 and arm64 are configured. Optimized test PDBs retain line tables.
- macOS keeps Apple's linker. GNU/mold flags do not apply to Mach-O binaries.
- Miri opts out of native linker configuration.

Cargo caches separate runner OS/architecture, the exact compiler (including
nightly commit), target flags/linkers, profile overrides, workload and manifests.
A commit-specific key saves new build artifacts; a compatible restore prefix
reuses them on later commits. This avoids the old lockfile-only caches freezing
their contents after the first build. Different workload groups avoid lint/WASM
jobs winning a cache-save race with native builds. npm's download cache is also
enabled; dependencies still install with `npm ci --ignore-scripts`.

Runtime compatibility builds one N-API addon per platform, validates generated
loaders, then distributes that exact addon to Node 22/24/26 tests. Native addon
builds drop from 18 to 6 in this workflow. Rust feature coverage and all 18 JS
matrix entries are retained. WASM has its own build/typecheck/test job, using a
prebuilt, checksum-verified wasm-pack 0.15.0 instead of `cargo install`.
Workflow NAPI builds pass `--lib --locked` to Cargo, avoiding compilation of
the unrelated LSP binary with NAPI enabled. LSP builds remain explicit and
use their existing core-only feature configuration.

Routine tests use `ci`, with debug assertions and no debug symbols. Coroutine
backend/stress checks and CI LSP builds use `ci-optimized`: release opt-level 3,
no cross-crate LTO, 16 codegen units, and line tables for crash diagnostics.
The two coroutine suites and LSP share a core-only build, avoiding NAPI code
and repeated LTO.
The LSP step runs its protocol integration tests; Cargo also builds the normal
LSP executable for those tests, with the same dependency features as the other
optimized integration suites. This avoids rebuilding it with a separate
`cargo build` dependency graph. CI LSP artifacts live in `target/ci-optimized`;
release-tag jobs still build and publish `target/release` binaries with thin LTO
and a single codegen unit.
Native addon builds also retain the release profile.

Node-API host jobs compile their tests in `ci` once, then run them with the
linked smoke fixture. Windows gets `node.lib` from that test build. macOS avoids
an unused release host build and an explicit target directory that prevented
reuse by the example. `NAPI_VM_HOST_PROFILE=ci` lets both example scripts reuse
the test profile while continuing to load a release-built napi-rs addon.

Plugin Node/Bun suites use `tools/plugins/test-js.mjs` to resolve wildcard paths
before launching either runtime. npm's Windows command shell does not expand
these patterns; explicit `./` file arguments work consistently on each OS and
avoid Bun scanning unrelated files. An empty suite is an error.

`build.rs` tracks the NAPI CLI's declaration directory and force-build variable
so restored Cargo artifacts cannot silently replace `index.d.ts` with an empty
file when declaration metadata is missing.

Release verification and platform builds run concurrently. Only the final
publishing job depends on both verification and all six build jobs, preserving
the tagged-commit gate. Superseded PR runs are cancelled; tag runs are retained.

CI's quality job runs actionlint 1.7.12 via `tools/ci/lint-workflows.sh`. Run it
locally with `RUNNER_TEMP` set to a temporary directory. Performance gains on
GitHub-hosted Windows/macOS/arm64 runners should be measured from Actions run
durations; local Linux checks do not establish speedups on those runners.
