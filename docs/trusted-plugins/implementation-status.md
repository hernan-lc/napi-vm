# Implementation boundaries and validation

The independent process-plugin packages extend the existing VM; applications select them explicitly. The TS host accepts JS/TS artifacts under a declared Node/Bun runtime. The Rust host also accepts target-matching native executables, including standalone Bun artifacts. Neither host provides a sandbox.

## Engineering invariants

- Contract version and digest match before initialization; emitted packages carry a checked inventory.
- Initialization is admitted at the successful response boundary before consuming the next event. SDK startup events wait behind the initialization reply within the shared writer budget. Failed, expired or retired sessions cannot become READY.
- One active business handler per endpoint; overlapping calls fail promptly. Cancellation is cooperative, and timeout does not establish rollback or remote completion.
- State replacement drains handlers and SDK-tracked author tasks, validates a snapshot, stops the old direct child, then starts/restores the replacement. It is non-atomic; failures retain DRAINING or a recoverable validated snapshot as documented in [migration](migration-ts-to-rust.md).
- Direct children are supervised and reaped. Arbitrary descendants require OS containment. Library callers must await shutdown.
- HTTP/MCP injection supplies connection capabilities, not a full MCP client. Native dependencies need declared supported runtime/target metadata; no universal addon compatibility is implied.

## Validation reference

Use [testing](testing.md) for commands, [compatibility](compatibility.md) for platform boundaries and [evidence provenance](evidence/README.md) for inherited observations. Historical results do not certify current source or a new CI run. The retained structured reports distinguish runtime tests, compile checks, benchmarks and known legacy failures.

The initialization-order regression is covered by `packages/plugin-protocol/test/response-order.test.mjs`, `packages/plugin-host/test/initialization-order.test.mjs` and `packages/plugin-sdk/test/initialization-order.test.mjs`; Rust regressions deliberately deliver a reply/event before polling the response waiter. These tests avoid assuming separate TCP writes will coalesce.

Template reproduction is documented in [development](development.md). Separate legacy release-mode VM tests, generated bindings, LSP and Wasm checks remain required for changes to their respective surfaces. No release, publication or remote CI outcome follows from these documents.
