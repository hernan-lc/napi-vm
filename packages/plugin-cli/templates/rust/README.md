# Trusted Rust greeter

This process has your account's OS privileges. Its vendored SDK is independent of JS tooling.

```
cargo check
cargo test
cargo build --release
napi-vm-plugin build . --format executable
napi-vm-plugin test . --artifact --host-executable /absolute/path/to/trusted-host-rust
```

The explicit CLI build stages the executable and records the actual OS/architecture/libc. Normal host loading never compiles. Generated Rust sources are checked in; changing the contract requires explicit code generation.
