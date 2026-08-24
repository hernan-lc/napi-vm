# API reference

## VM

| Function | Description |
|----------|-------------|
| `runCode(code)` | Execute code statelessly and return the stringified result |
| `new Vm()` | Create an isolated, stateful VM |
| `vm.run(code)` | Execute synchronously while preserving VM state |
| `vm.runAsync(code)` | Execute asynchronously and return `Promise<string>` |
| `vm.callFunction(name, args)` | Call a VM-defined function and return a live value |
| `vm.setGlobal(name, value)` | Add a structured value to the guest global scope |
| `vm.getGlobal(name)` | Read a global as a string |
| `vm.hasGlobal(name)` | Check whether a global exists |
| `vm.removeGlobal(name)` | Remove a global, including an exposed host function |
| `vm.exposeFunction(name, fn)` | Expose a synchronous host callback |
| `vm.exposeAsyncFunction(name, fn)` | Expose an asynchronous host callback |
| `vm.registerModule(name, code)` | Register an importable ES module |
| `vm.registerHostModule(name, exports, opts?)` | Register a module whose exports are host functions |
| `vm.removeModule(name)` | Remove a registered module |
| `vm.hasModule(name)` | Check whether a module is registered |
| `vm.listModules()` | List registered module names |
| `vm.setLoopLimit(n)` | Set the per-execution loop budget |
| `vm.setImportMetaMain(bool)` | Set `import.meta.main` |
| `debugParse(code)` | Parse source and return its AST string |

### registerHostModule

The generic form of `exposeFunction` + `registerModule`: the core bridges each
export and generates the wrapper module, and returns the global names it
created so the host can remove them alongside the module.

```javascript
const globals = vm.registerHostModule(
  "napi:fs",
  { readText: restrictedReadText, writeText: restrictedWriteText },
  { async: [] },              // export names the guest may `await`
);

vm.run(`import { readText } from "napi:fs"; readText("./config.json");`);

vm.removeModule("napi:fs");   // also revokes the module's bridge globals
```

Revocation is tracked per module: `removeModule` revokes the globals the
module created, and re-registering with fewer exports revokes the ones that
disappeared — a dropped export cannot stay callable through its old global.
Module names are encoded injectively, so `a:b` and `a/b` never share a
namespace. A failed registration leaves the previous one untouched.

Export names must be plain identifiers and every value must be a function. The
core stays generic on purpose: permission checks, path resolution and policy
belong to the host functions themselves — see
[Plugins](plugins.md) for a full capability host built this way.

`runAsync` should be reserved for genuinely long-running or asynchronous
work. It spawns one OS thread per call and must not run concurrently with
another operation on the same VM.

## LanguageService

| Function | Description |
|----------|-------------|
| `new LanguageService()` | Create an analysis service |
| `open(uri, source)` | Add a document |
| `update(uri, source)` | Replace a document |
| `close(uri)` | Remove a document |
| `registerModule(name, source)` | Add module exports to analysis context |
| `registerHostFunction(name, params, returns, docs, async)` | Add typed host metadata |
| `complete(uri, offset)` | Return completion items |
| `hover(uri, offset)` | Return hover information |
| `diagnostics(uri)` | Return diagnostics |

## VmSession

`runtime/session.cjs` exposes:

- `new VmSession({ workspace, vm, sessionId })`
- `start()` / `stop()`
- `attach(vm, { modules })` / `detach()`
- `exposeFunction` / `exposeAsyncFunction`
- `registerModule` / `removeModule`
- `observeHandler(name, value)` — publish a JSON-derived property shape and a
  bounded snapshot of the latest value for the handler's first parameter. The
  live LSP uses the shape for nested completion and shows the latest value in
  parameter hover information.
- `snapshot()`

The session is opt-in. Constructing or starting no session means no runtime
locator is created.
