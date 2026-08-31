# Roadmap and implementation tracker

The executable specification lives in `tests/`, especially
`tests/ecma-gaps.test.js`.

## How to read this document

A feature is listed under exactly one status. The point of the taxonomy is
that a name existing in the global scope is not evidence that a feature works
— several entries below are objects with no members, which previously appeared
in this file as "completed".

| Status | Meaning |
|--------|---------|
| **Full** | Behaves as specified for the cases the test suite covers. |
| **Partial** | Works for common shapes; documented gaps remain. |
| **Stub** | The name exists; calling or constructing it does not work. |
| **Unsupported** | Not implemented. Now reported as a `SyntaxError` where it is syntax. |

Every claim below was checked against the current build.

## Full

- Arithmetic, operators, control flow, functions, closures, recursion
- **Lexical scoping**: block scope, `let`/`const`/`var` as distinct kinds,
  the temporal dead zone, `var`/function hoisting, per-iteration `let`
  bindings in `for` loops (`tests/scoping.test.js`)
- **Syntax errors**: malformed programs are rejected with a position instead
  of being partially executed; truncated input terminates rather than hanging
  (`tests/syntax-errors.test.js`)
- Destructuring, spread (via the iterator protocol), rest, optional chaining,
  nullish coalescing, template literals, tagged templates
- Generators: true suspension on a same-thread coroutine, `yield*`
  delegation, iterator closing on early `for...of` exit
  (`tests/generators-delegation.test.js`)
- Crash-safety guards: recursion, parse depth, loop budget, array/string
  caps, generator nesting (`docs/safety.md`)
- NAPI host bridge for structured values and sync/async callbacks
- Plugin capability host: manifests, filesystem permissions, `napi:fs`,
  `napi:path`, byte limits

## Partial

- **Classes** — declarations, inheritance, `super`, static methods and fields
  work. Missing: class *expressions*, private fields (`#x`), static
  initialization blocks, `async`/generator class methods (the AST carries no
  flags for them).
- **ES modules** — named, default and namespace imports work, and module
  registration is transactional. Missing: live bindings (imports copy the
  value at import time), re-export (`export … from` parses but its `source` is
  ignored), `export *`, dynamic `import()`, cyclic graphs. The namespace
  object exposes the default export as `_default`, not `"default"`.
- **Promises / async** — `async`/`await` and the common combinators work, but
  the implementation is eager and synchronous: there is no microtask queue, so
  `Promise.resolve().then(…)` runs before subsequent synchronous code
  (observably `"acb"` where JavaScript gives `"abc"`). `new Promise(executor)`
  is not constructible. Missing: pending promises, thenable assimilation,
  `Promise.allSettled`, `Promise.any`, async iterators, `for await…of`,
  async generators.
- **Object model** — property access, prototypes and getters/setters work for
  ordinary use. Missing: property descriptors (`writable`, `enumerable`,
  `configurable`), `Object.create`, `defineProperty`, `getOwnPropertyDescriptor`,
  `getPrototypeOf`/`setPrototypeOf`, `hasOwn`, `freeze`/`seal`, and `Reflect.*`.
  `delete obj.prop` evaluates to `true` without removing the property, and `in`
  checks own properties only, not the prototype chain.
- **Symbols** — `Symbol()` and the iterator protocol work, but a symbol is
  represented as its description string and strict equality has no symbol
  case, so `s === s` is `false`. Needs a unique identity (id + description)
  and a registry for `Symbol.for`.
- **N-API value bridge** — structured data crosses in both directions.
  Functions, promises, generators, symbols, `Map`/`Set`, `Date`, typed arrays
  and cyclic objects become `undefined` on the way out.
- **LSP** — synchronization, completion and hover. Document symbols are
  disabled; definition, references, rename, signature help, semantic tokens,
  formatting, code actions and inlay hints are not implemented.
- **WASM / playground** — the interpreter runs, but generators degrade to
  empty iterators: `corosensei` needs assembly stack-switching that
  `wasm32-unknown-unknown` does not provide. Reaching parity needs a
  same-thread resumable evaluator (a CPS transform of generator bodies).

## Stub

These names exist in the global scope but are empty objects — constructing or
calling them fails. They are listed so their presence is not mistaken for
support.

- `Map`, `Set`, `WeakMap`, `WeakSet`
- `RegExp`, `Proxy`, `DataView`, `Function`
- `Promise` as a constructor (`new Promise(…)`)
- `Array.from`, `Array.of` (advertised on `Array`; only `isArray` is real)
- `fetch`, `Headers`, `Request`, `URLSearchParams`, and the other web-like
  globals — shapes without implementations

The intended direction for the web-like globals is *not* ambient network or
filesystem access inside the sandbox. It is the capability-host pattern that
`napi:fs` already uses: the host grants `napi:fetch`, `napi:timers`,
`napi:crypto` and friends explicitly, and the guest gets nothing by default.

## Unsupported

Now reported as syntax errors rather than silently mis-parsed:

- Regular-expression literals (`/ab+/gi`) and BigInt literals (`123n`)
- Logical assignment: `&&=`, `||=`, `??=`
- `class` expressions, private fields, static blocks
- `import()` expressions, `for await…of`, async generators
- Typed arrays and `ArrayBuffer`

## Priority order

1. Object model: descriptors, `delete`, prototype semantics, `Reflect`
2. Symbol identity
3. ES module live bindings, re-exports, `export *`
4. Microtask queue and a real `Promise` constructor
5. `Map`, `Set`, `RegExp`, BigInt — replace the stubs
6. Modern syntax: logical assignment, class expressions, private fields
7. Same-thread resumable generators (also fixes WASM parity)
8. Richer N-API boundary values
9. LSP features

## Known boundaries

- The interpreter is not a replacement for a full JavaScript engine.
- `runAsync` creates one OS thread per invocation and is not intended for
  high-frequency events. Call `dispose()` when finished with such a VM.
- The in-process sandbox needs worker/process isolation for strict untrusted-code
  CPU and memory limits.
- Host functions are explicitly typed by metadata; JavaScript runtime functions
  do not carry TypeScript annotations.

Contributions should add or update a regression test with each language or
bridge feature, then run the quality gate from `docs/development.md`.
