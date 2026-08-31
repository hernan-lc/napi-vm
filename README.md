# napi-vm

A sandboxed JavaScript virtual machine written in Rust with Node.js NAPI and
WebAssembly integrations. It includes a shared language service, browser
playground, local LSP, optional Zed extension, live VM metadata, and an
IPC-style command/event bridge for deterministic tests.

## Quick start

Prerequisites: **Rust** (1.96+), **Node.js** (18+) and **[Bun](https://bun.sh)**.
Bun runs the main test suite; the library itself has no Bun dependency at
runtime.

```bash
npm install
npm run build
npm test          # main suite (requires Bun)
npm run test:node # Node compatibility suite
npm run lint      # cargo fmt + clippy, then tsc --noEmit
```

```javascript
const { Vm } = require("./index.js");

const vm = new Vm();
vm.run("let answer = 40 + 2;");
console.log(vm.run("answer;")); // 42
```

## Documentation

- [Getting started](docs/getting-started.md) — installation, VM usage, host bridge, modules, and playground
- [API reference](docs/api.md) — `Vm`, `LanguageService`, and `VmSession`
- [Editor integration](docs/editor.md) — playground, LSP, Zed, live metadata, and IPC commands
- [Plugins](docs/plugins.md) — manifests, filesystem permissions, `napi:fs` / `napi:path`, and the plugin lifecycle
- [Sandbox safety](docs/safety.md) — containment guards and operational limits
- [Development](docs/development.md) — quality gate, scripts, benchmarks, and project structure
- [Roadmap](docs/roadmap.md) — implemented features and known boundaries

## Useful examples

```bash
bun examples/plugins.ts
bun examples/hotreload.ts
NAPI_VM_SESSION=1 bun examples/hotreload.ts
npm run ipc:smoke
```

The first command runs the VM entirely in-process. The second opt-in command
publishes live metadata for the LSP through `.napi-vm/runtime.json`; the
temporary locator is ignored and removed when the session stops.

## Disposing a VM

A VM that has run `runAsync` holds a native handle for dispatching host calls,
and that handle keeps the Node process alive. Call `dispose()` when you are
finished with a VM that used `runAsync`, or the script will not exit:

```javascript
const vm = new Vm();
vm.exposeAsyncFunction("fetchRow", async (id) => db.get(id));
await vm.runAsync(`await fetchRow(1);`);
vm.dispose();
```

`dispose()` is idempotent, and safe to call while an async worker is still in
flight. VMs that only use the synchronous `run()` do not need it.

## Status

The core language and bridge are actively developed and covered by the native
regression suite. Run `npm test` to see the current verified count.

## License

MIT — see [LICENSE](LICENSE).
