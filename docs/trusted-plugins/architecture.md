# Architecture

The protocol, SDK and host packages/crates are independent of the root VM. Cargo retains `default-members=["."]`; Wasm and existing native VM behavior are not redirected to these process crates. JavaScript packages are new ESM entrypoints; legacy CommonJS entrypoints stay intact.

A host preflights a manifest, package integrity, runtime and contract identity; allocates logical instance/session identities; binds a private listener; launches the direct child with bootstrap environment variables; drains stdout/stderr immediately; authenticates hello; initializes; then admits business calls. The connection token identifies the child/session and is not a sandbox boundary.

Initialization configuration accepts supported JSON values, including explicit `null`; only an absent option defaults to `{}`. Host application context must be a JSON object because the host augments it with declared capabilities and services. Invalid context is rejected before launching the child.

One active business handler per endpoint is a deliberate reliability rule. Busy endpoints reject rather than queue a possible independent-call deadlock. Transport responses and control processing run independently, allowing plugin-to-host callbacks. Explicit call contexts propagate active endpoint chains and remaining deadlines. No operation is retried automatically.

Tracked SDK background tasks must quiesce before snapshot. Plugin authors remain responsible for registering state-mutating background work; arbitrary unmanaged threads/tasks cannot be made safe by the host. Direct child cleanup is explicit. Neither dropping a Rust handle nor a timeout proves a process or external effect was undone.

## Services and native dependencies

Native dependencies have explicit artifact paths, target/ABI/runtime metadata; portable-js rejects them. HTTP/MCP service declarations receive host-supplied connection settings at initialization, with credentials confined to runtime configuration. This supplies a connection capability, not an implicit complete MCP protocol client. Applications select their client implementation and protocol version.

The Rust host additionally exposes an explicit managed-service adapter with executable checksum/target preflight and bounded HTTP health readiness. The TS host never silently launches native service executables. Sidecar output is drained and discarded to avoid retaining secret-bearing logs. Managed services terminate/reap their direct child; arbitrary descendants need OS containment if required.
