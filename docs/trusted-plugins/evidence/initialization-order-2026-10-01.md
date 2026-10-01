# Initialization response/event ordering regression

The TypeScript host previously settled the initialize request and consumed the
next event in the same socket read before the awaiting startup continuation could
transition the handle to READY. A deterministic replay delivered these two valid
frames through one socket `data` callback:

1. The matching `system.initialize` response with `result: null`
2. A negotiated `example.counter.changed` event for the same session, sequence 1

Before the fix, the real host/compiled plugin reproduction printed:

```json
{"status":"READY","sequence":"0","statuses":["STARTING","FAILED","READY"]}
```

The event was rejected as premature and the connection was torn down. The startup
continuation then incorrectly overwrote FAILED with READY and returned success.
The Rust reader had the analogous ordering risk: it sent the response through a
oneshot channel and processed the next frame without waiting for the load task.

The hosts now validate and admit initialization synchronously at the successful
response boundary, before the reader consumes another frame. Both retain a final
liveness/generation check before returning load success. Invalid results, earlier
events, retired/failed sessions, timed-out or cancelled calls, and a dropped Rust
response waiter cannot admit initialization.

A second real SDK reproduction started a tracked task during initialization:

```js
initialize(_, context) {
  context.spawnTask(async () => {
    await Promise.resolve();
    await context.emit(COUNTER, 'changed', { count: '1' });
  });
}
```

The task could observe SDK readiness before the successful initialize response
had been queued. Before the sender fix, the task's emit succeeded, but host load
failed with `NOT_READY: Premature business event` and status history
`STARTING → FAILED → FAILED`.

Both SDKs now hold outbound event frames behind the initialize response. The
response is enqueued before the fence opens, so the priority writer puts it on
the wire first. SDK readiness is already set when the host can read the reply;
an immediate host invocation remains valid. Normal host-service RPC remains live
while events are held. Held frames use the same advertised outbound byte budget,
overflow is observable, and initialization failure or disconnect drops them
through normal session cleanup. Tracked event tasks do not wait on the fence.

Repeatable focused commands:

```sh
node --test packages/plugin-protocol/test/response-order.test.mjs packages/plugin-host/test/initialization-order.test.mjs packages/plugin-sdk/test/initialization-order.test.mjs
bun test --timeout 30000 packages/plugin-protocol/test/response-order.test.mjs packages/plugin-host/test/initialization-order.test.mjs packages/plugin-sdk/test/initialization-order.test.mjs
cargo test -p napi-vm-plugin-protocol -p napi-vm-plugin-sdk -p napi-vm-plugin-host
cargo clippy -p napi-vm-plugin-protocol -p napi-vm-plugin-sdk -p napi-vm-plugin-host --all-targets -- -D warnings
```

The Rust ordering regression deliberately does not poll the waiting request
between delivering the reply and event. The TypeScript regression uses one
synchronous data callback instead of assuming that two actual TCP writes will
coalesce. Separate tests cover failures and late results.

Final focused verification: Node 14/14 and Bun 14/14 passed. Rust protocol, SDK,
and host tests passed 32/32; one existing controlled sidecar entry point remains
ignored because its owning lifecycle test launches it explicitly. Formatting and
Clippy were checked for all three native packages. The adjacent log files retain
the exact command output; the broader implementation matrix is recorded separately.
