# Editor integration

## Shared language service

`LanguageService` is the frontend-independent Rust implementation for
completion, hover, diagnostics, import/export analysis, and host-function
metadata:

```javascript
const { LanguageService } = require("./index.js");

const service = new LanguageService();
service.registerHostFunction(
  "alert",
  [{ name: "message", typeName: "string" }],
  "void",
  "Displays a message.",
);
service.open("file:///main.js", 'alert("hello");');
console.log(service.hover("file:///main.js", 3));
```

The browser playground, Node LSP, and future native desktop editors all call
this same service instead of implementing separate inference engines.

## LSP

The stdio adapter is `lsp/server.cjs`. Static metadata is opt-in:

```bash
node lsp/server.cjs --config ./path/to/manifest.json
```

The server does not create or load `.napi-vm.json` by default. A manifest is
only used when the embedding editor explicitly passes `--config`.

## Live VM metadata

`VmSession` is an optional local runtime bridge. It writes only a temporary
`.napi-vm/runtime.json` locator and sends the actual metadata through a local
Unix socket or Windows named pipe. The locator is ignored by Git, regenerated
for every process/machine, and removed when the session stops.

The hot-reload example enables this bridge explicitly:

```bash
NAPI_VM_SESSION=1 bun examples/hotreload.ts
```

Without `NAPI_VM_SESSION=1`, the example runs entirely in-process and creates
no runtime locator. The LSP reconnects when the session restarts and rebuilds
its analysis context when functions or modules change.

## Zed

The optional `zed-extension/` launcher starts the Node LSP with Zed's bundled
Node runtime and preserves normal JavaScript Tree-sitter highlighting.

Build it with:

```bash
npm run zed:build
```

Install the `zed-extension/` directory using Zed's local development-extension
flow. Start a configured `VmSession` in the workspace to provide live host
functions, module metadata, observed JSON event shapes, and the latest bounded
event value. When a host calls `session.observeHandler("handleChat", event)`,
the editor can complete nested properties such as `event.data.nickname` and
show the latest payload when hovering the handler parameter, without executing
the guest script. No manifest copy is required.

## IPC-style VM commands

`examples/lib/vm-ipc.ts` provides a small in-process command/event API:

```typescript
const ipc = new VmIpc();

ipc.handle("system.ping", (payload) => ({ ok: true, payload }), {
  params: [{ name: "payload", typeName: "unknown" }],
  returns: "object",
  documentation: "Round-trip test command.",
});

ipc.on("test:response", (payload) => console.log(payload));
ipc.attach(vm, session);

vm.run('ipc.send("test:response", ipc.invoke("system.ping", { ok: true }));');
```

Use `handleAsync` with `ipc.invokeAsync` and `vm.runAsync` for asynchronous
commands. Commands and host listeners survive VM replacement; the VM-side
wrapper is recreated on every reload.

Run the focused test with:

```bash
npm run ipc:smoke
```

MCP is not required for this local editor connection. An MCP adapter can be
built later on top of the same runtime snapshot if AI tooling needs access.
