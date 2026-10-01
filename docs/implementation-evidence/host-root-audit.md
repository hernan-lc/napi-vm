# HostBridge guest-root audit

Review of the required-fixes checkout based on PR #8 head `71a9bb314724cf4e2ad034bf928d1883d3aec760`:

| Implementation | Retained guest state | Collector treatment |
| --- | --- | --- |
| `NodeAddonSidecar` | Object proxies, callbacks, callback graph nodes, symbols, native promises | Explicit `trace_roots`; promises enter as `Value::Promise`. JSON ingress and handle/identity maps contain no additional guest references. |
| `RustNodeApiHost` | Global environment, object prototype, type tags, native callbacks, environment handles, strong references, deferred promises, threadsafe callbacks, async resources, wraps/finalizers, externals/buffers, exceptions | Existing `trace_roots` visits these. Weak N-API references remain weak intentionally. Active callback frames contain raw handle IDs; values are traced through handle arenas. |
| `CompositeHostBridge` | Plugin and native bridges | Delegates root tracing to both. |
| `PluginHostBridge` | Opaque Rust callback closures | Built-in filesystem/path callbacks capture host policy and strings, not guest `Value` or `Env`. Custom capability closures must pin captured guest values: Rust closure captures cannot be enumerated. Documented on `RustPluginFunction`. |
| `NapiHostBridge` | N-API references, wire messages, integer pending IDs | Contains no retained guest `Value` or guest `Env`; pending wire values are reconstructed inside an owner lease. Runtime exports have explicit heap pins. |
| `WasmBridge` | `js_sys::Function` handles | Contains no retained guest `Value` or guest `Env`. |
| `QueuedCallbackBridge` (integration fixture) | Guest callback | Added `trace_roots` for its callback. |
| Scheduler `ProbeBridge`, `FloodBridge`, `BlockingBridge`, and callback-bearing local `Bridge` fixtures | Guest callbacks, promises, queued events | Added `trace_roots` for retained values. Barrier and blocking-wait-only fixtures retain only host channels and need no guest roots. |
| Scheduler benchmark `Events` | Queued `HostEvent` callbacks, receivers, arguments, promises and exceptions | Added `trace_roots` for every event variant. |
| Call benchmark `Bridge`, scheduler metrics `TimingBridge`, and evaluation fixture `PanickingBridge` | None; timing metrics retain a host clock and numeric samples | No guest roots required. |

Host closures that retain guest objects remain responsible for explicit owner-local pins. Opaque host closures do not become implicit collector roots; guest values gain no new `Send`/`Sync` implementation.
