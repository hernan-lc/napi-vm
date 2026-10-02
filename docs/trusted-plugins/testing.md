# Verification

Shared schema vectors live under fixtures/trusted-plugins/contracts. Node/Bun protocol tests and native protocol tests consume the same semantics. Unit and harness tests are distinct from process interoperability.

`plugins:test:interop` requires Node, Bun and built Rust fixtures. It executes the same compiled TS artifact under all six host/runtime combinations and Rust native under Rust, and checks TS rejection of native launch. Missing required tools fail; they are not silently skipped.

After staging those fixtures, `npm run plugins:test:rust-interop` explicitly selects
the otherwise ignored Rust integration test. It reads `fixtures.json` and supplies
`NAPI_VM_GREETER_JS_MANIFEST`, `NAPI_VM_GREETER_RUST_MANIFEST`,
`NAPI_VM_COUNTER_JS_MANIFEST`, and `NAPI_VM_COUNTER_RUST_MANIFEST` to
`cargo test -p napi-vm-plugin-conformance --test interop -- --ignored`.
The dedicated native CI runs both entrypoints. No compiled fixture means a failure.

For the Node unit runner, set `NAPI_VM_BUN` to the installed Bun executable to
include the Bun missing-dependency/no-install regression. Without that explicit
path, this one optional cross-runtime unit case is reported skipped; the required
interop matrix still fails if Bun is missing. `NAPI_VM_NODE` can select an explicit
Node executable when running the same unit suite under Bun.

## Independent consumer and package checks

Run these permanent checks after `npm ci --ignore-scripts` and `npm run plugins:build`:

```sh
npm run plugins:check-assets
npm run plugins:test:packaged
npm run plugins:test:consumers
npm run plugins:test:rust-consumer
npm run plugins:test:cargo-package
```

`plugins:check-assets` invokes the CLI asset synchronizer with `--check`; it detects drift in the Rust template's vendored source and bindings without rewriting files.

`plugins:test:packaged` verifies independently relocated plugin artifacts and rejects a modified entry before launch. Its structured outputs are written under `artifacts/trusted-plugins/`.

`plugins:test:consumers` packs all five npm packages and installs their tarballs in an external application without workspace links. It generates bindings and performs strict TypeScript consumer checks, then exercises templates created by the packed CLI. Required public build dependencies must be installed or available through the configured npm registry/cache; this entire runner is not an offline-install guarantee.

`plugins:test:rust-consumer` builds an external application against extracted protocol/SDK/host crate archives and relocated native fixtures. The consumer exercises callbacks, events, reload, snapshot and awaited shutdown with an empty runtime `PATH`, demonstrating that the native execution phase needs no Node, Bun or Cargo. Preparing and building this check still requires Node, Cargo and archive tooling. This runner also performs the same real Cargo archive verification as `plugins:test:cargo-package`, so CI does not repeat that packaging step separately.

`plugins:test:cargo-package` creates and verifies real Cargo package archives for the three public crates. Because sibling packages are unpublished, verification uses registry patches pointing only to previously extracted sibling archives. It does not prove crates.io publication or registry resolution without those patches. The `napi-vm-plugin-conformance` crate has `publish = false`: it is repository test infrastructure, not a public consumer dependency.

## CI execution scope

Native CI is configured for Linux x64, macOS arm64 and Windows x64, with smaller native runtime jobs for Linux arm64, Windows arm64 and macOS x64. Read the workflow for each job's selected checks; not every packaging check runs on every platform. These definitions are not evidence of successful remote execution. Current-source results must come from an actual local run or a verified CI run.

Existing root VM tests, native bindings, browser/Wasm, LSP and capability-based host tests remain their own jobs. See [implementation boundaries](implementation-status.md) and [evidence provenance](evidence/README.md) for scope and historical observations. Benchmark data is descriptive, never a fabricated or hardcoded pass criterion.

The packed CLI development signal smoke runs on POSIX. On Windows, `child.kill('SIGINT')` terminates the process instead of injecting a console interrupt, so this signal path is explicitly reported as not tested. Windows host shutdown and artifact invocation gates still run.

Before running root VM Node-API tests that build the isolated napi-rs fixture offline, populate its locked dependencies with `cargo fetch --locked --manifest-path tests/fixtures/node-api/napi-rs/Cargo.toml`, as the existing runtime matrix does.
