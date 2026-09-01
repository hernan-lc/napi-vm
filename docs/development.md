# Development

## Commands

| Command | Description |
|---------|-------------|
| `npm run build` | Build the optimized native addon |
| `npm run build:debug` | Build the debug native addon |
| `npm test` | Run the JavaScript regression suite (requires Bun) |
| `npm run test:rust` | Run the Rust unit and integration tests |
| `npm run test:node` | Run the Node.js compatibility suite (`engines.node` range) |
| `npm run lint` | Rust formatting + Clippy, then TypeScript type-checking |
| `npm run lint:rust` | `cargo fmt --check` and `cargo clippy -D warnings` |
| `npm run lint:ts` | `tsc --noEmit` for the package and the playground |
| `npm run check:generated` | Rebuild the committed bindings and fail on drift |
| `npm run ipc:smoke` | Test IPC commands and events |
| `npm run runtime:smoke` | Test the local runtime locator/transport |
| `npm run lsp` | Start the native `napi-vm-lsp` server |
| `npm run lsp:build` | Build the standalone `napi-vm-lsp` binary |
| `npm run lsp:test` | Run the protocol-level LSP and runtime tests |
| `npm run lsp:legacy` | Start the deprecated Node stdio server (`lsp/server.cjs`) |
| `npm run lsp:legacy-smoke` | Legacy: LSP framing with an explicit temporary manifest |
| `npm run lsp:legacy-runtime-smoke` | Legacy: LSP discovery from a live `VmSession` |
| `npm run zed:build` | Build the Zed `wasm32-wasip2` extension |
| `npm run playground:build` | Build the browser WASM package |
| `npm run playground` | Start the Vite browser playground |
| `npm run bench` | Run NAPI end-to-end benchmarks |
| `npm run bench:stress` | Run hot-reload and bridge stress tests |
| `npm run bench:rust` | Run Criterion interpreter benchmarks |

The quality gate is:

```bash
npm run lint          # cargo fmt, clippy, and tsc --noEmit
npm run build
npm test              # main suite, under Bun
npm run test:node     # Node compatibility boundaries
npm run test:rust     # Rust unit and integration tests
npm run test:wasm     # the browser build, loaded under Node
```

`test:rust` is easy to forget because most of the suite is JavaScript, but the
lexer, parser, bignum, symbol index and language-analysis tests live in Rust
and cover things the JavaScript suite cannot reach. A parser change that only
those tests notice has slipped through before.

`test:wasm` builds the browser package and loads it under Node. The two
targets compile different code — generators especially — so a change that
passes everything else can still break the playground. It has.

CI enforces each of these on every pull request, plus two checks that are easy
to miss locally:

- **`generated`** rebuilds `index.js`, `index.mjs` and `index.d.ts` and fails
  if the committed copies differ. These are build outputs that are also
  committed, so nothing but a check keeps them honest — they have previously
  drifted to two different versions with an empty `index.d.ts`.
- **`node-compat`** runs `tests/node/` on every Node version in
  `engines.node`. The main suite runs under Bun, which has its own module
  loader and native-addon implementation, so it cannot verify that claim.

### Prerequisites

Rust 1.96+, Node.js 18+, and [Bun](https://bun.sh) for the main test suite.

## Sanitizers

Generators run their body on a dedicated stack via a stackful coroutine, on the
calling thread. `tests/generator_stress.rs` exercises that lifecycle, and
ThreadSanitizer is worth running after any change to it:

```bash
rustup component add rust-src --toolchain nightly
RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test -Zbuild-std \
    --release --no-default-features --test generator_stress \
    --target x86_64-unknown-linux-gnu -- --test-threads=1
```

This reports **zero** races. It is a regression gate, not an open issue: an
earlier OS-thread implementation moved `Rc`-backed state across threads under
`unsafe impl Send` and measured ~7.6 races per run (~1.0 after the threads were
joined). Replacing the thread with a coroutine removed the boundary entirely,
so there is no non-atomic refcount for two threads to race on.

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
