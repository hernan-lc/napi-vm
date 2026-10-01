# TypeScript runtime verification

This report covers the independent TypeScript protocol, SDK, host, and JS/TS examples. It does not certify the Rust implementation, the aggregate interoperability matrix, native platforms not executed here, or legacy VM behavior. Those are reported separately in [implementation status](implementation-status.md) and [compatibility](compatibility.md).

## Final integration update

The final Node and Bun aggregate suites each pass 113/113 with no skips:
[Node log](evidence/node-aggregate-2026-10-01.log),
[Bun log](evidence/bun-aggregate-2026-10-01.log).
These include two later real-process lifecycle regressions in addition to the
focused 43-test runs recorded below: an active handler finishes its event/result
before a draining snapshot, and a retired call-bound callback context cannot
invoke a replacement session. Top-level stable handle clients still rebind.
The Node aggregate explicitly sets `NAPI_VM_BUN` so the optional Bun
missing-dependency/no-auto-install case executes rather than skips.

## Implemented boundary

- `@napi-vm/plugin-protocol`: bounded TCP framing, strict JSON data model, schema and digest validation, request correlation, deadlines, cooperative cancellation, priority control traffic, and bounded diagnostics
- `@napi-vm/plugin-sdk`: generated-contract registration, managed startup, callbacks, typed-client integration, lifecycle hooks, tracked author tasks, bidirectional events, and a no-process harness
- `@napi-vm/plugin-host`: manifest/package validation, runtime preflight, direct-child supervision, stable handles, bounded/redacted logs, host-service registration, events, state-preserving reload and explicit force-without-state reload
- [Greeter TS](../../examples/trusted/greeter-ts), [plain JS](../../examples/trusted/greeter-js), [counter TS](../../examples/trusted/counter-ts), and [host TS](../../examples/trusted/host-ts)

The TypeScript host accepts JavaScript/TypeScript execution routes only. Production requires emitted JavaScript and a verified package inventory. Bun may run TypeScript in explicit development mode; Node always uses emitted JavaScript. Native and Bun-compiled executable launch routes are rejected by the TypeScript host, including when an executable could itself run JavaScript. Use the independent Rust host for those artifacts.

`portable-js` rejects declared native dependencies. `external-runtime` permits explicitly declared, target-matching, runtime-tested Node-API dependencies, with selected-runtime ABI inspection and injected resolved paths. It does not establish universal Node-addon compatibility. Services are explicitly injected HTTP/MCP endpoints; the TypeScript host accepts external ownership and rejects host-owned native service processes. Runtime credentials remain outside manifests/inventories and are redacted from retained/emitted child logs when known.

## Environment and commands

Executed on Linux x64 GNU, Node **24.19.0**, Bun **1.4.2 (744846f84)**, TypeScript **6.0.3**. These are execution evidence, not a claim that every earlier version covered by package engine metadata has been tested.

Strict package compilation:

```sh
node node_modules/typescript/bin/tsc -p packages/plugin-protocol/tsconfig.json
node node_modules/typescript/bin/tsc -p packages/plugin-sdk/tsconfig.json
node node_modules/typescript/bin/tsc -p packages/plugin-host/tsconfig.json
```

Final focused Node and Bun suites (set `NAPI_VM_BUN` to the actual Bun executable; this explicitly enables the optional external-runtime dependency-failure test in the Node suite):

```sh
NAPI_VM_BUN=/absolute/path/to/bun node --test \
  packages/plugin-protocol/test/*.test.mjs packages/plugin-sdk/test/*.test.mjs \
  packages/plugin-host/test/*.test.mjs examples/trusted/*/test/*.test.mjs

NAPI_VM_NODE=/absolute/path/to/node NAPI_VM_BUN=/absolute/path/to/bun \
  /absolute/path/to/bun test --timeout 30000 \
  packages/plugin-protocol/test packages/plugin-sdk/test packages/plugin-host/test \
  examples/trusted/greeter-ts/test examples/trusted/greeter-js/test examples/trusted/counter-ts/test
```

Final evidence: [Node log](evidence/ts-node-2026-10-01.log), [Bun log](evidence/ts-bun-2026-10-01.log). **Final result: Node 43/43 passed (81.80 s); Bun 43/43 passed (78.26 s), zero failures and zero skips in both suites.** These final runs include integer wire-timeout regression, session-tagged queued-event rejection and escaped-descendant pipe cleanup. A previous Bun run with the runner's default five-second timeout failed under concurrent builds; process tests now declare 30-second test deadlines. That failed run was not treated as a compatibility pass.

## Coverage mapped to the original requirements

| Category | Executed focused evidence | Boundaries / remaining verification |
| --- | --- | --- |
| W01–W05 | Every split boundary for small framed Unicode messages; one-byte fragmentation; coalesced frames; zero/oversized/truncated frames; invalid UTF-8/JSON/BOM/depth; duplicate-key last-wins; prototype-sensitive fields | The JS writer uses Node-compatible serialized socket writes; these tests do not inject every OS partial-write boundary or establish Rust parity by themselves |
| W06–W10 | Duplicate active request IDs terminate without double dispatch; unknown replies are ignored; out-of-order responses correlate; batches and malformed envelopes are rejected; wrong token followed by valid child; authorized protocol/digest mismatch before initialization | Full per-fixture cross-language results belong to the conformance report |
| D01–D06 | Undefined/accessors/classes/cycles/sparse arrays rejected; null versus absence retained; safe integers; signed-64 decimal boundaries and canonical representations; base64; astral/lone-surrogate handling; invalid output and undeclared domain errors | The generated wire-types corpus additionally exercises unsigned-64 limits, Gregorian leap-day/year bounds, nullable absence versus explicit null, numeric literals, and tagged unions; exhaustive cross-language parity is not inferred from these unit counts |
| D07–D10 | Runtime metadata rejects unsupported/recursive/unresolved schema constructs and digest drift; prototype-sensitive data is tested | Generator rejection/drift suites, generated identifier escaping and the full datetime/nullable-union corpus are owned by codegen/conformance tooling; see their separate results |
| R01–R07 | Missing/disallowed/wrong runtime preflight; explicit compiled Node path; callback and cyclic callback rejection; bounded stdout/stderr flooding; startup hang/exit and call-time exit; full-access filesystem/HTTP/environment/subprocess fixture | Bun/Node production-artifact permutations and target-specific native probes are additionally covered by the parent artifact/matrix runners; this report does not expand their support claims |
| R08–R12 | Pre-aborted requests; immediate local cancellation; timeouts retain the occupied remote slot; independent-chain calls fail promptly while busy; saturated business pending capacity leaves control responsive; missing managed environment; actual Bun missing-package failure with `--no-install` and no created lockfile | Cooperative cancellation is not rollback or guaranteed interruption. A caller timeout leaves the remote outcome unknown; no automatic retries occur |
| L01–L02, L05–L10 | Initialize/cleanup hook failure; counter snapshot/restore/reload; stable client and new session; events; quiesce failure leaves the owned old process DRAINING; failed replacement retains count `19` for explicit recovery; multiple instances; idempotent unload/shutdown; background tasks drain; split bootstrap-token redaction | Old-session wire events are ignored and queued entries are session-tagged, checked before delivery, and cleared on reload. A callback already executing can finish. A controlled descendant holding inherited pipes does not prevent direct-child shutdown. Library users must explicitly await shutdown; CLI signal handling is tested separately. Arbitrary descendants are not OS-contained |
| L03–L04 | Shared decimal-string snapshot envelope implemented and validated | TS↔Rust migration execution belongs to the root interoperability report; JS-only tests cannot prove it |
| P01–P06 | JS example builds without a TS compiler; same emitted TS entry is suitable for Node/Bun; no executable route in TS host; runtime path with spaces/Unicode exercised by temporary fixtures; controlled direct access works | Relocated inventories, tampering, clean-room consumers, standalone artifacts and native probes are owned by the packaging runner. Native executable tests cannot be claimed through the TS host |

Test sources: [protocol](../../packages/plugin-protocol/test/protocol.test.mjs), [SDK](../../packages/plugin-sdk/test/sdk.test.mjs), [host/processes](../../packages/plugin-host/test/host.test.mjs).

## Operational qualifications

- One active business handler is admitted per endpoint. Busy business calls fail promptly with `REENTRANT_CALL`, including independent chains; they do not wait in a potentially deadlocking queue
- Default frame/depth/pending/write/log/event bounds are exported by the protocol package. There is bounded reserved control capacity, not an unbounded priority queue
- Event subscriber overflow terminates that subscription and reports an error; subscriptions do not replay disconnected events. Already-running callbacks cannot be undone; queued retired-generation deliveries are counted in `handle.discardedStaleEvents` and discarded
- SDK-tracked author tasks are cancelled and drained before snapshot. Unregistered arbitrary runtime tasks cannot be discovered automatically; plugin authors must register state-mutating background work
- Reload is non-atomic. Quiesce/snapshot failure leaves DRAINING; failed restoration leaves FAILED with the validated snapshot retained for explicit recovery
- Loopback token and contract digests identify a session/interface; they do not sandbox trusted code or authenticate a publisher
- Node/Bun package operation has no VM, N-API binding, Rust compilation, Bun compiler, implicit dependency installation, or mandatory Rust-sidecar dependency
- Only Linux x64 GNU was executed in this workspace. macOS and Windows are not runtime-tested here
