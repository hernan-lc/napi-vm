# Roadmap and implementation tracker

The executable specification lives in `tests/`. Each area below names the
suite that covers it, so a claim here can be checked without reading the
implementation.

## How to read this document

A feature is listed under exactly one status. The point of the taxonomy is
that a name existing in the global scope is not evidence that a feature works
— entries have appeared here as "completed" while being objects with no
members.

| Status | Meaning |
|--------|---------|
| **Full** | Behaves as specified for the cases the test suite covers. |
| **Partial** | Works for common shapes; documented gaps remain. |
| **Unsupported** | Not implemented. Reported as an error rather than silently mis-executed. |

Every claim below was checked against the current build.

## Full

### Language

- Arithmetic, operators, control flow (including labelled `break` and
  `continue`), functions, closures, recursion
- **Lexical scoping**: block scope, `let`/`const`/`var` as distinct kinds,
  the temporal dead zone, `var`/function hoisting, per-iteration `let`
  bindings in `for` loops (`tests/scoping.test.js`)
- **Syntax errors**: malformed programs are rejected with a position instead
  of being partially executed; truncated input terminates rather than hanging
  (`tests/syntax-errors.test.js`)
- **Destructuring** in all three positions — declarations, assignments
  (`[a, b] = [b, a]`, `({ x } = o)`, property targets) and parameters,
  with defaults, nesting and rest elements (`tests/destructuring.test.js`)
- Spread (via the iterator protocol), rest, optional chaining, nullish
  coalescing, template literals
- **Tagged templates**: the tag receives the cooked chunks, a `raw` companion
  array, and the interpolated values (`tests/builtin-constructors.test.js`)
- **Logical assignment**: `&&=`, `||=`, `??=`, with short-circuit evaluation
  (`tests/logical-assignment.test.js`)
- Numeric literals: decimal, hexadecimal, binary, octal, separators, and the
  `BigInt` suffix
- **Generators**: true suspension on a same-thread coroutine, `yield*`
  delegation, `throw()`/`return()`, iterator closing on early `for...of` exit
  (`tests/generators-delegation.test.js`, `tests/array-methods.test.js`)
- **Classes**: declarations *and expressions*, inheritance, `super(...)` and
  `super.method()`, static methods and fields, static initialization blocks,
  private fields and methods (`#x`), and `async`/generator methods
  (`tests/class-features.test.js`)
- **Object model**: property descriptors, `Object.create`, `defineProperty`,
  the `getOwnPropertyDescriptor(s)` pair, `getPrototypeOf`/`setPrototypeOf`,
  `hasOwn`, `fromEntries`, `is`, the integrity levels (`freeze`, `seal`,
  `preventExtensions`) and `Reflect`. `delete` removes the property, `in`
  walks the prototype chain, and `===` compares reference identity
  (`tests/object-model.test.js`)
- **Symbols**: a unique identity, `Symbol.for`'s registry, the well-known
  symbols, and symbol-keyed properties that stay out of `Object.keys` and
  `JSON.stringify` (`tests/symbols.test.js`)

### Asynchrony

- **Promises**: a real `Promise` constructor, pending promises, thenable
  assimilation, and `resolve`/`reject`/`all`/`allSettled`/`any`/`race`
- **A microtask queue**: reactions run as microtasks, so
  `Promise.resolve().then(f); g()` runs `g` first
- **Async functions** that genuinely suspend at `await`, including async
  arrows and async methods
- `for await…of`, async generators, and `Symbol.asyncIterator`
- `queueMicrotask`, and `setTimeout`/`setInterval`/`clearTimeout` on a
  clock-free timer queue: callbacks run after every microtask, ordered against
  each other by delay (`tests/promises.test.js`)

### Standard library

- `Map`, `Set`, `WeakMap`, `WeakSet`, keyed by SameValueZero
  (`tests/collections.test.js`)
- **Regular expressions**: literals and the constructor, `exec`/`test` with a
  stateful `lastIndex`, and `match`/`matchAll`/`search`/`replace`/
  `replaceAll`/`split` on strings. Character classes, greedy and lazy
  quantifiers, bounded repetition, alternation, capturing/non-capturing/named
  groups, backreferences, anchors, lookahead and lookbehind, and the
  `g`/`i`/`m`/`s`/`y`/`u` flags (`tests/regexp.test.js`)
- **BigInt**: arbitrary-precision integers with the arithmetic, bitwise and
  shift operators, `asIntN`/`asUintN`, and the type separation the language
  requires — mixed arithmetic is a `TypeError` (`tests/bigint.test.js`)
- **Typed arrays**: `ArrayBuffer`, all eleven views, and `DataView`
  (`tests/typed-arrays.test.js`)
- **`Date`** instances, with UTC accessors and ISO/JSON serialization
  (`tests/dates.test.js`)
- **`Proxy`** with the `get`, `set`, `has`, `deleteProperty`, `ownKeys`,
  `apply` and `construct` traps, and the `Function` constructor
  (`tests/proxy.test.js`)
- `TextEncoder`, `TextDecoder`, `URLSearchParams` and `structuredClone` —
  the web globals that are pure computation (`tests/web-globals.test.js`)
- The built-in namespaces are callable: `String(x)`, `Number(x)`,
  `Boolean(x)`, `Object(x)`, `Array(n)`, plus `Array.from`/`of` and
  `String.raw` (`tests/builtin-constructors.test.js`)

### Modules

- Named, default and namespace imports and exports, with **live bindings**:
  an imported name tracks the exporting module
- Re-exports (`export … from`), `export *`, `export * as ns`
- `import(specifier)`, resolving to a namespace object
- Cyclic graphs, through `defineModule` (which records a module without
  evaluating it, so the bodies run on first import)
- Namespace objects expose the default export as `"default"`
  (`tests/modules-linking.test.js`)

### Host integration

- Crash-safety guards: recursion, parse depth, loop budget, array/string
  caps, generator nesting, regular-expression backtracking, and the job queue
  (`docs/safety.md`)
- **N-API value bridge**: structured data, `Date`, `BigInt`, symbols,
  `ArrayBuffer` and typed arrays, `Map`/`Set`, settled promises, and cyclic or
  shared references, in both directions (`tests/bridge-values.test.js`)
- Plugin capability host: manifests, filesystem permissions, `napi:fs`,
  `napi:path`, byte limits
- **LSP**: synchronization, completion, hover, document symbols, definition,
  references, document highlight, rename, signature help, inlay hints and
  semantic tokens (`tests/lsp_protocol.rs`)

## Partial

- **Generators on `wasm32`** — the browser target has no stack switching, so a
  body cannot be suspended. It runs once to completion on the first `next()`
  and its yields are buffered for the remaining calls to drain. Values,
  `for…of`, spread, `Array.from` and `yield*` all work
  (`tests/wasm/browser-build.test.mjs`), but the difference is observable: the
  body's side effects happen at the first `next()` rather than interleaved
  with the consumer, `next(v)` cannot send a value in, abandoning a `for…of`
  early does not stop a body that has already run, and an unbounded generator
  hits a cap and raises a catchable `RangeError`. Real suspension needs a
  resumable evaluator (a CPS transform of generator bodies).
- **Module scope** — module bodies share one global scope, so a top-level
  binding in one module is visible to another. Imports are still linked
  properly, and re-importing a name already bound to the same cell is a no-op
  rather than a redeclaration, but two modules that declare the same top-level
  name will collide.
- **N-API boundary** — functions and generators still cross as `undefined`.
  Handing a VM function to the host needs a host callable that re-enters the
  interpreter, which the marshaller has no handle on; `Vm.exposeFunction`
  remains the way to cross in that direction.
- **`Error` objects** — `name` and `message` work, and the built-in error
  constructors produce the right names, but there is no `stack` property and
  no user-defined `Error` subclassing.
- **LSP** — document formatting and code actions are not implemented.
  Formatting would need a printer that reconstructs source from the AST, which
  does not retain comments; a formatter that deletes them is worse than none.

## Unsupported

Reported as errors rather than silently mis-executed:

- The web-like globals that reach outside the sandbox — `fetch`, `Headers`,
  `Request`, `Response`, `WebSocket`, `crypto`, `localStorage` and friends —
  remain inert shapes. This is deliberate. The intended direction is the
  capability-host pattern that `napi:fs` already uses: the host grants
  `napi:fetch`, `napi:crypto` and friends explicitly, and the guest gets
  nothing by default.
- `Intl`, `Object.groupBy`, and the other recent library additions not listed
  above.
- `with`, which is not in the grammar at all. (Labelled `break` and `continue`
  *are* supported, including on a labelled block.)

## Priority order

1. Per-module scope, so two modules can declare the same top-level name
2. Functions and generators across the N-API boundary
3. `Error.stack`, and user-defined `Error` subclassing
4. The capability-host modules: `napi:fetch`, `napi:crypto`, `napi:timers`
5. A resumable evaluator, for true generator suspension on `wasm32`
6. LSP formatting and code actions

## Known boundaries

- The interpreter is not a replacement for a full JavaScript engine.
- `runAsync` creates one OS thread per invocation and is not intended for
  high-frequency events. Call `dispose()` when finished with such a VM.
- The in-process sandbox needs worker/process isolation for strict untrusted-code
  CPU and memory limits.
- Host functions are explicitly typed by metadata; JavaScript runtime functions
  do not carry TypeScript annotations.
- String and array indices count Unicode scalar values, not UTF-16 code units,
  so text outside the Basic Multilingual Plane is indexed differently from a
  real engine.

Contributions should add or update a regression test with each language or
bridge feature, then run the quality gate from `docs/development.md`.
