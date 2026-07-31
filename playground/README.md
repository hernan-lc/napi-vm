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

| Path                    | Purpose                                                        |
| ----------------------- | -------------------------------------------------------------- |
| `server.ts`             | Static file server for `public/` and `pkg/`                    |
| `public/index.html`     | Editor + console UI; loads `/js/main.js`                       |
| `public/js/main.js`     | Entry module: DOM wiring, run/reset/keys, boot                 |
| `public/js/vm.js`       | Wasm init, VM factory, host setup, language-service wrappers   |
| `public/js/examples.js` | The editor sample + the registered demo modules (data only)    |
| `public/js/console.js`  | Console pane rendering                                         |
| `public/js/completion.js` | Autocomplete popup (candidates come from the Rust core)      |
| `public/js/diagnostics.js` | Live diagnostics indicator                                  |
| `public/style.css`      | Theme                                                          |
| `smoke.ts`              | Headless end-to-end test of the wasm API (`bun smoke.ts`)      |

The frontend is split into plain ES modules with no build step — the browser
imports them directly. `main.js` is the only module the HTML references; it
composes the rest through small factory functions (`createConsole`,
`createCompletion`, `createDiagnostics`) and a `getVm` accessor, so no module
holds a stale reference across a VM reset.

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
