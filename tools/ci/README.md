# Workflow build setup

All Rust jobs use `.github/actions/setup-build` after installing Rust:

- Linux GNU targets use mold 2.40.4. Its x64/arm64 release archives are checked
  against fixed SHA-256 hashes and cached. Only GNU target flags change; WASM
  and static musl targets retain their existing linkers.
- Windows MSVC targets use `lld-link`, an alias of Rust's bundled LLVM linker.
  The MSVC action still supplies the C compiler, Windows SDK and import libraries.
  Both x64 and arm64 are configured. Release PDB settings remain intact.
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
