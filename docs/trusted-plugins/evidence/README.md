# Evidence provenance and reproduction

The six original JSON reports in this directory are inherited observations from the `feat/portable-plugin-hosts` implementation commit `d15888f` (base `238cf7c9de8e68601e13c401401c466a1feddf24`). Their original documentation dates the captures to 2026-10-01 on Linux x64 GNU with Node 24.19.0, Bun 1.4.2 and Rust/Cargo 1.98.1. The reports do not all contain a capture timestamp or source commit; artifact hashes, when present, identify tested bytes rather than proving a current checkout. These files have not been regenerated during documentation cleanup.

| Report | Reproduction | Meaning |
|---|---|---|
| `interop-results.json` | `npm run plugins:test:interop` | Actual host/runtime routes, callbacks, events and state migration observations |
| `package-results.json` | `npm run plugins:test:packaged` and `npm run plugins:test:consumers` | Relocation, inventory and independent consumer checks |
| `benchmark-node.json`, `benchmark-bun.json` | [Performance commands](../performance.md) | Single-machine descriptive measurements, including workload and build-mode limitations |
| `benchmark-rust-host.json` | `npm run plugins:bench:rust` | Counter workload observations; debug executable timings do not predict optimized performance |
| `legacy-validation-report.json` | Existing root release build/tests | Historical regression diagnosis, including failed/blocked tests and no independently compiled baseline native binary |

Build with `npm ci --ignore-scripts` and `npm run plugins:build` first. Required process runners fail if Node/Bun/native fixtures are missing. Fresh outputs are written under `artifacts/trusted-plugins/`; do not overwrite committed observations without recording the source revision, environment, commands and failures. See [testing](../testing.md) and [performance](../performance.md).

Transient command logs and dated work diaries are intentionally excluded. These retained reports are not fresh validation of later changes, proof of remote CI success, or runtime evidence for macOS/Windows/other architectures. Remote CI status must be verified independently.

`compatibility-summary.json` records fresh local reference checks on the source revision it names. Its skips, build preconditions and publication limits are explicit; it is separate from the six historical reports above.
