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
| `vm.removeModule(name)` | Remove a registered module |
| `vm.hasModule(name)` | Check whether a module is registered |
| `vm.listModules()` | List registered module names |
| `vm.setLoopLimit(n)` | Set the per-execution loop budget |
| `vm.setImportMetaMain(bool)` | Set `import.meta.main` |
| `debugParse(code)` | Parse source and return its AST string |

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
