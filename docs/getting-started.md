# Getting started

## Requirements

- Node.js 16 or newer
- Rust toolchain
- Bun is useful for the JavaScript and playground smoke tests

## Installation

```bash
npm install
npm run build
```

The native package includes the Linux x64 GNU binary. Other platforms must
build the addon from source.

## Basic usage

```javascript
const { Vm, runCode, debugParse } = require("./index.js");

console.log(runCode("2 + 2;"));

const vm = new Vm();
vm.run("let x = 10;");
console.log(vm.run("x;"));
console.log(debugParse("const x = 1;"));
```

`Vm` instances keep their state between calls and are isolated from one
another. Guest code cannot access Node's `require`, filesystem, network, or
process globals unless the host explicitly exposes a controlled function.

## Host bridge

### Synchronous functions

```javascript
const { Vm } = require("./index.js");
const vm = new Vm();

vm.exposeFunction("add", (a, b) => a + b);
console.log(vm.run("add(1, 2);"));
```

Arguments, return values, and thrown errors are marshalled across the NAPI
boundary. Exposed functions are available as globals and through
`window`, `globalThis`, and `self`.

### Asynchronous functions

Use `exposeAsyncFunction` with `runAsync` when guest code awaits a host Promise:

```javascript
const vm = new Vm();

vm.exposeAsyncFunction("fetchJson", async (url) => {
  const response = await fetch(url);
  return response.json();
});

const result = await vm.runAsync(`
  async function main() {
    const data = await fetchJson("https://example.test/data.json");
    return data;
  }
  main();
`);
```

`runAsync(source)` returns `Promise<string>`. It runs the interpreter on a
dedicated thread and dispatches async host callbacks to Node through a
ThreadsafeFunction. Do not call `run` and `runAsync` concurrently on the same
VM. Each `runAsync` call creates an OS thread, so high-frequency handlers
should use synchronous `run()` or a worker pool instead.

### Modules

```javascript
vm.registerModule("math", `
  export function double(value) { return value * 2; }
`);

console.log(vm.run(`
  import { double } from "math";
  double(21);
`));
```

Use `removeModule` before re-registering a changed module during hot-reload.

## Browser playground

```bash
npm run playground:build
npm run playground
```

The playground provides syntax highlighting, autocomplete, diagnostics, hover
information, expandable console values, UTF-8 file editing, and an editable
file explorer. Registered module sources are analyzed by the same Rust
language service used by the Node LSP.

For browser host-function metadata, use the helper from
`playground/public/src/vm.ts`:

```typescript
exposeFunction(vm, "alert", (message: unknown) => {
  console.log(String(message));
}, {
  params: [{ name: "message", type: "string" }],
  returns: "void",
  documentation: "Writes a message to the playground console.",
});
```

JavaScript functions do not retain TypeScript annotations at runtime, so rich
hover and completion metadata must be supplied explicitly.
