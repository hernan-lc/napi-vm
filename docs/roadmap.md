# Roadmap and implementation tracker

The executable specification lives in `tests/`, especially
`tests/ecma-gaps.test.js`. The current native test suite covers 568 cases.

## Completed

- Arithmetic, variables, functions, closures, recursion, control flow
- Classes, inheritance, static methods, fields, `super`, `instanceof`
- Destructuring, spread, optional chaining, nullish coalescing, templates
- ES modules with named/default/namespace imports and export wiring
- Async/await, Promise helpers, generators with true suspension
- Symbols and iterator protocol, including custom iterables
- Math, JSON, Object, Array, String, Number, Date, Error, console, and web-like globals
- NAPI host bridge with structured values and synchronous/asynchronous callbacks
- Shared completion, hover, diagnostics, import-aware analysis
- Browser playground with editor, console, explorer, colors, and expandable values
- Node stdio LSP and optional Zed launcher
- Opt-in live `VmSession` metadata over local IPC
- IPC-style VM command/event example and hot-reload lifecycle
- Crash-safety guards and executable containment catalogue

## Known boundaries

- The interpreter is not a replacement for a full JavaScript engine.
- `runAsync` creates one OS thread per invocation and is not intended for high-frequency events.
- The in-process sandbox needs worker/process isolation for strict untrusted-code CPU and memory limits.
- Host functions are explicitly typed by metadata; JavaScript runtime functions do not carry TypeScript annotations.

Contributions should add or update a regression test with each language or
bridge feature, then run the quality gate from `docs/development.md`.
