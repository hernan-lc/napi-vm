# napi-vm-plugin-sdk

Rust authoring SDK for trusted process plugins, including handler registries, call contexts, event subscriptions, cooperative cancellation and tracked resources. Importing the library does not launch a process.

Use `Registry` to register contract implementations and the SDK session API to connect a native executable to its host. Contracts and errors are re-exported from the protocol crate. Plugin executables require no Node, Bun or npm at runtime.

Plugins are ordinary OS processes with the launching user's privileges. Track asynchronous work with SDK resources before quiescing and snapshotting; unmanaged threads and external effects are not automatically reversible. Deadline expiry requests cancellation and does not prove that a side effect was undone.
