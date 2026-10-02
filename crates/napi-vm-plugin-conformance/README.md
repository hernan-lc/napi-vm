# napi-vm-plugin-conformance

Repository-only cross-language trusted plugin conformance fixtures and lifecycle integration tests. This crate is intentionally `publish = false`: its tests and fixture executable depend on repository contracts and JavaScript examples and are not a public downstream API.

Run `cargo test -p napi-vm-plugin-conformance` from the repository after building the declared JavaScript fixtures. Native lifecycle tests exercise executable loading, callbacks, events, reload, shutdown and direct-child reaping. Runtime tests require the runtimes declared by their fixtures; compiling a fixture alone is not runtime validation.
