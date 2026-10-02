# TypeScript verification scope

The TypeScript protocol, SDK and host have separate test suites; process interoperability and packaging require their own runners. Test counts from past runs are not current-source certification. See [testing](testing.md) for full reproduction and [evidence provenance](evidence/README.md) for inherited results.

| Surface | Sources | Behavior to verify |
|---|---|---|
| Protocol | `packages/plugin-protocol/test` | Bounded frames/writes, validation, correlation, deadlines and reply/event ordering |
| SDK | `packages/plugin-sdk/test` | Registration, tracked tasks, lifecycle, callbacks and startup-event fence |
| Host | `packages/plugin-host/test` | Manifest/inventory integrity, runtime selection, readiness, stable handles, session retirement and direct-child cleanup |
| CLI/codegen | `packages/plugin-cli/test`, `packages/plugin-codegen/test`, `tools/plugins/test` | Strict descriptors, generated drift, consumers and command workflows |
| Examples | `examples/trusted/*/test` | Business logic through SDK harnesses |

The TypeScript host accepts JS/TS execution routes only. Production uses emitted JavaScript and verified inventories. Explicit Bun development mode may run TypeScript; Node uses emitted JavaScript. Native executables and Bun standalone artifacts require the Rust host.

`portable-js` rejects declared native dependencies. `external-runtime` supports explicitly inventoried target/runtime-matching Node-API dependencies; a fixture is not universal addon evidence. Service injection accepts external HTTP/MCP connections and rejects host-owned native service processes. Known runtime secrets are redacted from child logs; plugins still hold the launching account's OS privileges.

Cooperative cancellation cannot forcibly interrupt arbitrary code or undo external effects. A timed-out handler can retain the busy endpoint slot. Author-created state-mutating background work must use SDK tracking so quiescence can observe it. Already-executing event subscribers can finish; retired queued session deliveries are discarded. Library users must await shutdown, which supervises direct children only.
