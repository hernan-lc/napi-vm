# napi-vm playground

An in-browser playground for the VM. The Rust interpreter is compiled to
**WebAssembly** and runs entirely in the page — there is no server-side VM and
no WebSocket. The Bun server (`server.ts`) only serves the static frontend and
the wasm package.

Completion, diagnostics, and document symbols are served by the shared
pure-Rust language core in [`src/lang/`](../src/lang/) — the same code a future
LSP server or native GUI (egui/eframe/slint) will call, so editor intelligence
is written once and never re-derived per frontend.

## Prerequisites

- [Bun](https://bun.sh)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (with the
  `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`)

## Build + run

```sh
# from the repo root
npm run playground:build   # compile the VM to wasm + JS glue into playground/pkg
npm run playground         # serve at http://localhost:3000
```

`playground/pkg/` is generated output (it is git-ignored) and must be rebuilt
after changing the Rust core.

## Layout

| Path                 | Purpose                                                    |
| -------------------- | ---------------------------------------------------------- |
| `server.ts`          | Static file server for `public/` and `pkg/`                |
| `public/index.html`  | Editor + console UI                                        |
| `public/app.js`      | ES module: loads the wasm `WasmVm`, wires run/complete/etc. |
| `public/style.css`   | Theme                                                      |
| `smoke.ts`           | Headless end-to-end test of the wasm API (`bun smoke.ts`)  |

## The wasm API

`WasmVm` (from `pkg/napi_vm.js`) mirrors the NAPI `VM`:

- `run(code) -> { ok, value, error, logs }` — execute; `logs` are the captured
  `console.*` lines for that run.
- `expose_function(name, fn)` — make a browser function callable from the VM
  (and offered as a completion).
- `register_module(name, source)` — register an importable module; its exports
  feed `import * as ns` completion.
- `set_loop_limit(n)` / `reset()` / `get_global(name)`.
- `complete(code, byteOffset)`, `diagnose(code)`, `symbols(code)` — the shared
  language services.
