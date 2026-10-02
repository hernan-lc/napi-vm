# TS ↔ Rust state migration

Use the Rust host for cross-language replacement. Its stable logical handle retains instance identity while each process gets a new session. TS hosts intentionally cannot replace a TS plugin with a Rust native executable.

The counter contract is the same generated interface and digest in both implementations. State is `{stateVersion:1,contract:"example.counter.state",data:{count:"12"}}`; decimal-string representation avoids loss across languages.

Replacement is explicitly non-atomic: preflight replacement; enter DRAINING; quiesce business handlers and registered author background tasks; validate snapshot; fully stop/reap old process; start new session; restore; then READY. Old replies/events never become new-session traffic.

Quiesce/snapshot failure leaves the owned old instance DRAINING. Do not snapshot concurrently, resume automatically, or lose state silently. The caller may retry, unload, or explicitly force replacement without state. Failure after old process stops leaves FAILED with the validated snapshot retained for an explicit recovery attempt. No automatic rollback or replay is promised. Snapshot restore cannot undo external HTTP/file effects.
