# napi-vm-plugin-protocol

Wire framing, peer request correlation, protocol limits, contract hashing and schema validation for trusted process plugins. The protocol uses a four-byte big-endian payload length followed by UTF-8 JSON. Protocol version is 1.0.

This crate supplies no sandbox. Tokens authenticate a child session; they do not restrict operating system privileges. Cancellation is cooperative and does not undo external effects. Requests are not automatically retried.

Use `Contract` for validated contract definitions, `Limits` for resource bounds and `Peer` for an asynchronous connection. A Tokio runtime is required for peer operations. This crate is independent of the existing napi-vm interpreter and requires no JavaScript tooling.
