# Development

## Commands

| Command | Description |
|---------|-------------|
| `npm run build` | Build the optimized native addon |
| `npm run build:debug` | Build the debug native addon |
| `npm test` | Run the JavaScript regression suite |
| `npm run ipc:smoke` | Test IPC commands and events |
| `npm run runtime:smoke` | Test the local runtime locator/transport |
| `npm run lsp:smoke` | Test LSP framing with an explicit temporary manifest |
| `npm run lsp:runtime-smoke` | Test LSP discovery from a live `VmSession` |
| `npm run lsp` | Start the Node stdio LSP server |
| `npm run zed:build` | Build the Zed `wasm32-wasip2` extension |
| `npm run playground:build` | Build the browser WASM package |
| `npm run playground` | Start the Vite browser playground |
| `npm run bench` | Run NAPI end-to-end benchmarks |
| `npm run bench:stress` | Run hot-reload and bridge stress tests |
| `npm run bench:rust` | Run Criterion interpreter benchmarks |

The quality gate is:

```bash
npm run lint:rust
npm run build
npm test
```

## Benchmarks

`benches/vm.rs` measures the lexer/parser/interpreter without NAPI overhead.
`bench/bench.js` measures the published binding and compares it with native
JavaScript. Both layers assert matching results; Criterion reports are written
under `target/criterion/`.

## Project structure

```text
src/
├── lexer.rs, parser/       tokenizer and recursive-descent parser
├── interpreter/            evaluation, calls, scope, resolution, operators
├── builtins/               Math, Array, String, Number, Object, JSON
├── lang/                   completion, hover, diagnostics, symbols
├── bindings/               Node NAPI bridge and LanguageService
├── value.rs                runtime values
└── error.rs                error types

runtime/                    live VM metadata session
lsp/                        stdio server and runtime client
playground/                 browser editor and WASM integration
examples/                   host bridge, IPC, hot-reload, and safety demos
zed-extension/              local Zed launcher
tests/                      JavaScript regression suite
```

The implementation is intentionally modular: parser expression/statement
families, interpreter operations, builtins, language analysis, NAPI bindings,
runtime sessions, and LSP transport each have separate responsibilities.
