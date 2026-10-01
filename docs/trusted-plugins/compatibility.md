# Compatibility and evidence

## Actual execution in this workspace

Linux x64 GNU; Node 24.19.0, Bun 1.4.2, Rust/Cargo 1.98.1. The repository CI baseline remains Rust 1.96.0; that older compiler has not been executed locally for the new crates.

| Host | Node plugin | Bun plugin | Rust executable | Bun standalone |
|---|---|---|---|---|
| Node | Runtime-tested | Runtime-tested | Unsupported, rejected | Unsupported, rejected |
| Bun | Runtime-tested | Runtime-tested | Unsupported, rejected | Unsupported, rejected |
| Rust | Runtime-tested | Runtime-tested | Runtime-tested | Runtime-tested without external Node/Bun on PATH |

The six JS cells use one compiled TS package. The Rust-host examples exercise actual callbacks and domain errors, not mocks. State migration TS→Rust→TS and Rust→TS→Rust preserves 12, advances 15 then 17, retains logical identity and rotates sessions. Final-source runs and command logs are recorded in [implementation status](implementation-status.md).

The Node-only consumer test installs local npm tarballs offline outside the checkout. The Rust-only template is tested with Node/Bun absent from PATH. One controlled Node-API v1 fixture exposes answer 42 through an explicitly declared/inventoried external-runtime dependency; this is not evidence for all addons or ABI versions.

## Native platform scope

| Platform | Execution claim |
|---|---|
| Linux x64 GNU | Executed here |
| Linux arm64 / musl | Not runtime-tested here |
| macOS | Not runtime-tested here |
| Windows GNU | Protocol/SDK/host compile passes; not runtime-tested here |
| Browser/Wasm | Root VM wasm32 compile passes; browser execution untested; process hosts are not browser runtimes |

Native CI definitions exist for Linux/macOS/Windows. They have not been remotely executed as part of this local-only task. Compile-only results, if obtained, are separately labeled in the final status; they never become runtime support claims.

## Explicit limitations

- Trusted processes retain the launching account's OS access. No new sandbox or application permission model
- One active handler per endpoint; concurrent busy calls fail promptly rather than wait in a possible distributed deadlock
- No automatic retry, transaction rollback, crash persistence or atomic hot-swap
- Direct-child cleanup only; arbitrary descendants require OS-specific containment
- Author-created state-mutating tasks must use SDK tracking so quiescence can observe them
- External HTTP/MCP injection exposes connection capabilities. It does not implement a complete MCP client, registry or authentication workflow
- Native dependencies currently require the explicit external-runtime Node-API path or an explicitly managed Rust-host executable service; there is no universal dynamic-library loader
- CLI/codegen are authored ESM JavaScript with strict public declaration/consumer checks; protocol/SDK/host and TS examples are strictly compiled TypeScript
