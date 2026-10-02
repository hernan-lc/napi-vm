# Compatibility and evidence

## Inherited Linux execution observations

The retained implementation reports describe Linux x64 GNU; Node 24.19.0, Bun 1.4.2, Rust/Cargo 1.98.1. The earlier documentation reported a Rust 1.96.0 CI baseline; engine/compiler declarations are not execution evidence.

| Host | Node plugin | Bun plugin | Rust executable | Bun standalone |
|---|---|---|---|---|
| Node | Runtime-tested | Runtime-tested | Unsupported, rejected | Unsupported, rejected |
| Bun | Runtime-tested | Runtime-tested | Unsupported, rejected | Unsupported, rejected |
| Rust | Runtime-tested | Runtime-tested | Runtime-tested | Runtime-tested without external Node/Bun on PATH |

The six JS cells use one compiled TS package. The Rust-host examples exercise actual callbacks and domain errors, not mocks. State migration TS→Rust→TS and Rust→TS→Rust preserves 12, advances 15 then 17, retains logical identity and rotates sessions. These are inherited observations, not fresh validation of this checkout; see [provenance](evidence/README.md).

The npm consumer test installs all five local package tarballs outside the checkout without workspace links. Third-party build dependencies may use the configured registry/cache. Rust-only template checks are reproduced separately using the [development guide](development.md) with Node/Bun absent from PATH. One controlled Node-API v1 fixture exposes answer 42 through an explicitly declared/inventoried external-runtime dependency; this is not evidence for all addons or ABI versions.

## Current Linux reference validation

Source revision `d94803c5aa22f3310544f7e01aeceb40f8323ae4` was exercised locally on Linux x64 GNU with Node 24.19.0, Bun 1.4.2 and Rust 1.96.0. The Node/Bun process matrix, actual Rust-host interoperability and relocated native packages passed. A separate external application built from extracted `.crate` archives compiled all four generated Rust contract bindings and exercised READY, callbacks, events, snapshot, reload and awaited shutdown with an empty `PATH` and minimal environment. No Node/Bun/npm/Cargo command is available in that runtime environment.

The npm consumer installed all five packages from tarballs, compiled their declarations with `skipLibCheck: false`, generated greeter and wire-type bindings and ran templates from the installed CLI. POSIX development reload and signal shutdown were exercised; Windows console-interrupt injection remains untested by this helper.

These are local runtime observations. Cross-platform remote CI remains a separate prerequisite; see the configured jobs and the [verification commands](testing.md).

## Native platform scope

| Platform | Execution claim |
|---|---|
| Linux x64 GNU | Locally runtime-tested on the source revision above; historical benchmarks separately attributed |
| Linux arm64 / musl | No runtime evidence retained |
| macOS | No runtime evidence retained |
| Windows GNU | Earlier documentation reported compile-only checks; no runtime evidence retained |
| Browser/Wasm | Earlier documentation reported a root VM compile check; no browser runtime evidence; process hosts are not browser runtimes |

Native CI definitions exist for Linux/macOS/Windows. Their existence does not establish successful execution; no fresh remote CI result is claimed here. Compile-only results never become runtime support claims.

## Explicit limitations

- Trusted processes retain the launching account's OS access. No new sandbox or application permission model
- One active handler per endpoint; concurrent busy calls fail promptly rather than wait in a possible distributed deadlock
- No automatic retry, transaction rollback, crash persistence or atomic hot-swap
- Direct-child cleanup only; arbitrary descendants require OS-specific containment
- Author-created state-mutating tasks must use SDK tracking so quiescence can observe them
- External HTTP/MCP injection exposes connection capabilities. It does not implement a complete MCP client, registry or authentication workflow
- Native dependencies currently require the explicit external-runtime Node-API path or an explicitly managed Rust-host executable service; there is no universal dynamic-library loader
- CLI/codegen are authored ESM JavaScript with strict public declaration/consumer checks; protocol/SDK/host and TS examples are strictly compiled TypeScript
