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

`plugins:test:packaged` verifies independent filesystem artifacts and offline npm package installation. Results are recorded in artifacts/trusted-plugins/*.json. Native matrix CI is configured separately for Linux/macOS/Windows, but a workflow definition is not a passed run.

Existing root VM tests, native bindings, browser/Wasm, LSP and capability-based host tests remain their own jobs. Check implementation-status for exact local commands/results and unverified portions. Benchmark data is descriptive, never a fabricated or hardcoded pass criterion.
