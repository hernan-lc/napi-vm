# node-vm

A sandboxed JavaScript virtual machine built from scratch in Rust with NAPI bindings for Node.js.

Execute JavaScript in an isolated environment with no access to the host system's Node.js globals, filesystem, or network.

## Features

- **Hand-written lexer & parser** — Full tokenizer and recursive descent parser
- **Tree-walking interpreter** — Executes parsed JavaScript in a custom runtime
- **Sandboxed execution** — No access to `require`, `process`, or other Node.js globals
- **Isolated VM instances** — Each `Vm()` has independent, persistent state
- **ES Module syntax** — `import`/`export` (named, default, namespace) with exports wired through to importers, plus `import.meta`
- **Functions** — Regular, arrow, expressions, closures, and recursion
- **Control flow** — `if/else`, `while`, `do...while`, `for`, `for...in`, `for...of`, `switch/case`, `break`, `continue`
- **Error handling** — `try/catch/finally` with `throw`, catching both `throw` and runtime errors
- **Classes & OOP** — constructors, instance methods/fields, static members, `extends`/`super`, `instanceof`
- **Generators** — `function*`/`yield` with true suspension (infinite generators, `next(val)` sent values, `for...of`)
- **Symbols & iterators** — `Symbol()`, well-known symbols, `Symbol.for`/`keyFor`, full iterator protocol (`[Symbol.iterator]`, `for...of` over custom iterables)
- **Standard library** — `Math`, `JSON`, `Object`, `Array`/`String`/`Number` prototype methods, and global functions (`parseInt`, `isNaN`, …)

> **Status:** the core language, classes, a working standard library, async/`await`, generators with true mid-body suspension (`yield`/`next`/`next(val)`), module export wiring, and a full `Symbol` + iterator protocol are implemented and covered by 535 passing tests (see the [Roadmap](#roadmap--implementation-tracker) for the verified picture).

## Installation

```bash
npm install
npm run build
```

> Requires the Rust toolchain and Node.js. The pre-built binary is for Linux x64 GNU only — other platforms must build from source.

## Usage

```javascript
const { Vm, runCode, debugParse } = require('./index.js');

// Stateless execution
console.log(runCode("2 + 2;")); // "4"

// Stateful VM instance
const vm = new Vm();
vm.run("let x = 10;");
console.log(vm.run("x;")); // "10"

// Inspect the AST
console.log(debugParse("const x = 1;"));
```

### Host bridge

A VM is fully isolated by default. The bridge methods open a controlled,
synchronous channel between Node and the VM, marshalling real structured
values (numbers, strings, booleans, arrays, plain objects) in both directions.
Exposed values land on the global scope, reachable as bare identifiers and via
`window` / `globalThis` / `self` (all three alias the one global object).

```javascript
const vm = new Vm();

// Node -> VM: expose a structured value and a function.
vm.setGlobal("config", { retries: 3 });
vm.exposeFunction("add", (a, b) => a + b);
vm.run("add(config.retries, 4);"); // "7"
vm.run("window.add(1, 2);");       // "3"  (same function, via the global alias)

// A Node function that throws is catchable inside the VM.
vm.exposeFunction("boom", () => { throw new Error("nope"); });
vm.run("try { boom(); } catch (e) { e.message; }"); // "nope"

// VM -> Node: call a function defined inside the VM. Arguments are passed as
// a single array; the return value comes back as a live JS value.
vm.run("function point(x, y) { return { x, y }; }");
vm.callFunction("point", [5, 6]); // { x: 5, y: 6 }
```

## API

| Function | Description |
|----------|-------------|
| `runCode(code)` | Execute JavaScript code and return the result |
| `new Vm()` | Create a new isolated VM instance |
| `vm.run(code)` | Execute code within a VM instance (state persists across calls) |
| `vm.setGlobal(name, value)` | Define a global from a structured Node value (reachable as `name` and `window.name`) |
| `vm.exposeFunction(name, fn)` | Expose a Node function to the VM as a callable global; throws propagate into the VM |
| `vm.callFunction(name, args)` | Call a VM-defined global function; `args` is an array, returns a live JS value |
| `vm.getGlobal(name)` | Read a global, stringified |
| `vm.registerModule(name, code)` | Register an ES module so its exports are importable by later `run` calls |
| `vm.setImportMetaMain(bool)` | Set the value of `import.meta.main` |
| `debugParse(code)` | Parse code and return the AST as a string |

## Scripts

| Command | Description |
|---------|-------------|
| `npm run build` | Build optimized native binary |
| `npm run build:debug` | Build debug binary |
| `bun test` | Run the test suite |
| `npm run bench` | End-to-end JS benchmark through the NAPI binding |
| `npm run bench:rust` | Criterion microbenchmarks of the interpreter pipeline |

## Benchmarks

Two complementary layers measure the VM from different vantage points:

- **`benches/vm.rs`** (Criterion, `npm run bench:rust`) — microbenchmarks that drive the lexer → parser → interpreter pipeline directly, with no NAPI overhead. A `run` group times representative workloads end to end (arithmetic loops, recursion, array/string builtins, classes, closures, JSON), and a `frontend` group isolates lexing and parsing over a large source. Results, with HTML reports, land in `target/criterion/`.
- **`bench/bench.js`** (dependency-free, `npm run bench`) — runs the same workloads through the published `runCode` binding so each iteration crosses the NAPI boundary, and compares against an equivalent native-JS baseline to show the interpreter's overhead relative to the host engine. It also asserts the VM result matches the native one, so it doubles as a correctness check.

The JS workloads in both files are kept in sync so the two layers stay comparable.

## Project Structure

```
src/
├── lexer.rs            # Tokenizer
├── parser/
│   ├── mod.rs          # Parser core (token cursor, helpers) + tests
│   ├── ast.rs          # AST node types (Expr, Statement, ...)
│   ├── stmt.rs         # Statement parsing
│   ├── compound.rs     # Class, import/export parsing + shared block helpers
│   ├── expr.rs         # Expression precedence ladder (comma → postfix)
│   └── primary.rs      # Primary expressions, arrow functions, new callees
├── interpreter/
│   ├── mod.rs          # Interpreter struct, run() + tests
│   ├── eval.rs         # eval_stmt / eval_expr dispatchers
│   ├── call.rs         # Function/constructor calls, destructuring, catch
│   ├── resolve.rs      # Property resolution, prototype-chain lookup
│   ├── env.rs          # Environment, scope chain, Module
│   └── ops.rs          # Value operations (binary/unary, coercion, stringify)
├── builtins/
│   ├── mod.rs          # setup_builtins(), shared native helpers, global functions
│   ├── math.rs         # Math methods
│   ├── array.rs        # Array statics + prototype methods
│   ├── string.rs       # String prototype methods
│   ├── number.rs       # Number statics/prototype + parseInt/parseFloat
│   ├── object.rs       # Object statics
│   └── json.rs         # JSON.stringify / JSON.parse
├── value.rs            # Value enum
├── error.rs            # Error types
└── bindings.rs         # NAPI bindings to Node.js
```

## Modularization

The four files that previously exceeded ~800 lines have been split so that each module has a single responsibility (see [Project Structure](#project-structure)):

- **`interpreter/mod.rs`** (was 1471 lines) → `eval.rs` (stmt/expr dispatch), `call.rs` (calls, destructuring, catch), `resolve.rs` (property lookup), plus a slim `mod.rs` with the struct and tests. The pieces remain sibling `impl Interpreter` blocks, so there was no API change.
- **`builtins.rs`** (was 1187 lines) → one file per standard-library surface (`math`, `array`, `string`, `number`, `object`, `json`) under `builtins/`, orchestrated by `setup_builtins()` in `mod.rs`, which also hosts the shared `NativeFn` helpers.
- **`parser/expr.rs`** (was 1083 lines) → the precedence ladder stays in `expr.rs`; `primary()` and the arrow-function machinery moved to `primary.rs`.
- **`parser/stmt.rs`** (was 765 lines) → classes and `import`/`export` moved to `compound.rs`.

`lexer.rs` was deliberately left intact — it is cohesive at ~780 lines.

## Roadmap & Implementation Tracker

This tracker reflects **verified** behavior (checked against the interpreter directly, not just the parser). The executable specification lives in [`tests/ecma-gaps.test.js`](tests/ecma-gaps.test.js) — a live regression suite covering every feature below.

Legend: ✅ done · 🟡 partial · ❌ missing

### P0 — Correctness bugs

All fixed. These previously broke or hung on valid JavaScript.

| Status | Item | Notes |
|--------|------|-------|
| ✅ | `break` / `continue` inside loops | Loops catch the control-flow signal |
| ✅ | `do...while` loop | Implemented |
| ✅ | Default parameters `f(a = 10)` | Desugared to an `if (a === undefined) a = …` guard |
| ✅ | Rest parameters `f(...a)` | Bound as a real array |
| ✅ | Paren-less arrow as call arg `map(x => x)` | Backtracking arrow-parameter parsing |
| ✅ | `typeof undeclaredVar` | Returns `"undefined"` without throwing |
| ✅ | `try/catch` catches runtime errors | Catches both `throw` and runtime errors |
| ✅ | `finally` runs after `catch` | Correct body → catch → finally ordering |
| ✅ | Member assignment `o.x = 5` | Creates the property when absent |

### P1 — Core language features

**Operators:**
- ✅ Bitwise `&` `|` `^` `~` `<<` `>>` `>>>` (i32 semantics, `>>>` zero-fills)
- ✅ Exponentiation `**`
- ✅ Nullish coalescing `??`
- ✅ Optional chaining `?.` (member, computed, and call forms)
- ✅ Comma operator `(a, b, c)`

**Objects:**
- ✅ Property shorthand `{ x }`
- ✅ Method shorthand `{ f() {} }`
- ✅ Computed keys `{ [k]: v }`
- ✅ Getters / setters
- ✅ Object spread `{ ...o }`
- ✅ `this` binding in method calls (arrows inherit lexically)

**Functions:**
- ✅ Default parameters
- ✅ Rest parameters
- ✅ `arguments` object
- ✅ Spread in calls `f(...args)`
- ✅ Arrow fn single param without parens / multiple params

**Destructuring & spread:**
- ✅ Array destructuring `const [a, b] = ...`
- ✅ Object destructuring `const { a, b } = ...`
- ✅ Spread in array literal `[...a, b]`

**Template literals:**
- ✅ Basic `` `hi` `` and interpolation `` `hi ${name}` `` (lexer-level scanning preserves whitespace, supports nesting)

**Control flow:**
- ✅ Labeled `break` / `continue`
- ✅ `for...of` over strings

**Classes & OOP** — backed by a dedicated `Value::Class` with a shared prototype object:
- ✅ Constructor + `this`
- ✅ Instance methods
- ✅ Fields (desugared into the constructor)
- ✅ Static methods
- ✅ Inheritance (`extends`)
- ✅ `super` calls in derived constructors
- ✅ `instanceof` (prototype-identity walk)
- ✅ Function constructors via `new` + `this`

**Coercion correctness:**
- ✅ Boolean → number in arithmetic (`true + 1` → `2`)
- ✅ Loose equality coercion (`'5' == 5` and `0 == false` → `true`)

### P2 — Standard library

Implemented as native functions (`fn(&mut Interpreter, Value /*this*/, Vec<Value>)`), with prototype methods resolved through `prop()`.

- ✅ `Math` methods — `abs`, `floor`, `ceil`, `round`, `sqrt`, `cbrt`, `pow`, `min`, `max`, `random`, `trunc`, `sign`, `log`/`log2`/`log10`, `exp`, `sin`/`cos`/`tan`, `hypot`
- ✅ `JSON.parse` / `JSON.stringify`
- ✅ `Object` — `keys`, `values`, `entries`, `assign`
- ✅ `Array` statics — `isArray`
- ✅ `Array.prototype` — `map`, `filter`, `reduce`, `reduceRight`, `forEach`, `find`, `some`, `every`, `push`, `pop`, `shift`, `unshift`, `join`, `slice`, `splice`, `concat`, `reverse`, `sort`, `flat`, `flatMap`, `indexOf`, `includes`
- ✅ `String.prototype` — `toUpperCase`, `toLowerCase`, `slice`, `substring`, `split`, `includes`, `indexOf`, `trim`, `replace`, `charAt`, `startsWith`, `endsWith`, `repeat`
- ✅ String index access `'abc'[1]`
- ✅ `Number.prototype` — `toFixed`
- ✅ Global functions — `parseInt`, `parseFloat`, `isNaN`, `isFinite`, `Number.isNaN`, `Number.isFinite`
- ✅ `console` — `log`/`info`/`debug` write to stdout, `error`/`warn` to stderr
- ✅ `Date` (`now`, `parse` for ISO-8601, `UTC`; all times UTC)
- ✅ Real `Error` objects — `Error`, `TypeError`, `RangeError`, `SyntaxError`, `ReferenceError` are constructible classes with `name`/`message`, and thrown values survive `catch`

### P3 — Advanced

- ✅ Prototype chain — `Value::Object` carries a `proto` pointer (shared `Rc`), underpinning `instanceof` and prototype method lookup
- ✅ `async`/`await` — eager synchronous model (async bodies run immediately and settle a `Promise`); `await` unwraps fulfilled values and rethrows rejections, plus `Promise.resolve`/`reject`/`all`/`race`
- ✅ Module exports reaching importers — `export const`/`function`/`class`/`default` wire through `import { }`, `import * as`, and default imports
- ✅ Generators — `function*`/`yield` with true mid-body suspension (thread-based): infinite generators work, `next(val)` sends values into `yield`, yields inside loops/conditionals/try-finally all behave correctly, `for...of` drives generators, and generators are their own iterators (`[Symbol.iterator]() === this`)
- ✅ Symbols & iterator protocol — `Symbol(desc)`, well-known symbols (`Symbol.iterator`, `Symbol.toStringTag`, `Symbol.hasInstance`, `Symbol.asyncIterator`, …), `Symbol.for`/`Symbol.keyFor` registry, computed `[Symbol.iterator]()` methods in object literals, and `for...of` over any object implementing the iterator protocol (arrays, strings, generators, and custom iterables)
- ✅ Host bridge (Node ↔ VM) — `setGlobal`/`exposeFunction`/`callFunction` marshal structured values across the NAPI boundary over a stable raw `napi_sys` ABI (the VM stays single-threaded; exposed Node functions are persisted `napi_ref`s invoked synchronously, and thrown errors cross back as catchable exceptions). Exposed globals live on the one global scope, which `window`/`globalThis`/`self` all alias (`Value::GlobalObject`), so `window.add(1, 2)` and bare `add(1, 2)` are the same call. Not yet covered: passing a Node function *into* the VM as a first-class value, async/`postMessage`-style messaging, and a full `EventTarget`.

## License

No license specified.
