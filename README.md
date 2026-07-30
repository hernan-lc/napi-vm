# napi-vm

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
- **Native inspector** *(opt-in)* — a DevTools-style foldable tree over live guest values, implemented in Rust behind the `inspector` feature (`vm.inspect` / `console.dir(obj, { inspect: true })`); shows circular structures as a non-blocking inline dump, closed by default

> **Status:** the core language, classes, a working standard library, async/`await`, generators with true mid-body suspension (`yield`/`next`/`next(val)`), module export wiring, and a full `Symbol` + iterator protocol are implemented and covered by 566 passing tests (see the [Roadmap](#roadmap--implementation-tracker) for the verified picture).

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

A VM is fully isolated by default. Use `exposeFunction` to selectively open
controlled channels to the host. Exposed values land on the global scope,
reachable as bare identifiers and via `window` / `globalThis` / `self`.

#### Sync functions

`exposeFunction` exposes a synchronous Node function. Arguments and the
return value are marshalled across the boundary; thrown errors propagate
into the VM as catchable exceptions.

```javascript
const { Vm } = require('./index.js');
const vm = new Vm();

vm.exposeFunction("add", (a, b) => a + b);
console.log(vm.run("add(1, 2);")); // "3"
```

#### Async functions

`exposeAsyncFunction` + `runAsync` bridge Node's async world into the VM.
The VM thread parks at `await` while the Node event loop resolves the real
Promise — no subprocess spawning, no serialization overhead.

```javascript
const { Vm } = require('./index.js');
const vm = new Vm();

// Expose native fetch — no subprocess needed.
vm.exposeAsyncFunction("fetch", async (url) => {
  const res = await fetch(url);
  return { status: res.status, body: await res.text() };
});

// runAsync returns a Promise; the VM can await host functions.
const result = await vm.runAsync(`
  async function main() {
    var res = await fetch("https://jsonplaceholder.typicode.com/posts/1");
    var data = JSON.parse(res.body);
    return JSON.stringify({ title: data.title, status: res.status });
  }
  main();
`);
console.log(result); // { title: "sunt aut...", status: 200 }
```

Sync and async exposed functions coexist: a `runAsync` call can invoke both
`exposeFunction` and `exposeAsyncFunction` bindings. See
[`examples/async-bridge.ts`](examples/async-bridge.ts) for a full demo.

### Hot reload

The VM exposes the primitives needed for clean hot-reload cycles without
leaking stale state. The teardown order matters:

```javascript
// 1. Detach the event bus (removes the VM-side `emit` binding).
bus.detach();

// 2. Remove every registered module.
for (const name of vm.listModules()) {
  vm.removeModule(name);
}

// 3. Remove exposed host functions.
vm.removeGlobal("hostLog");

// 4. Rebuild: new Vm, re-register modules, bus.attach(newVm), re-expose.
```

Host-side listeners registered via the event bus survive across reloads
because they live on the bus, not in the VM. The VM only ever sees a single
`emit` global that is replaced atomically on each cycle, so there is never a
duplicate-listener window. See [`examples/hotreload.ts`](examples/hotreload.ts)
for a complete working demo (run with `bun examples/hotreload.ts`).

> **Event-loop note:** `vm.run()` is synchronous — it blocks the Node event
> loop until the computation finishes. A `setTimeout(0)` scheduled before a
> heavy VM call will not fire until `vm.run()` returns. Use `vm.runAsync()`
> for non-blocking execution: the VM runs on a dedicated thread and async
> host calls are dispatched back to the event loop via a ThreadsafeFunction.

## API

| Function | Description |
|----------|-------------|
| `runCode(code)` | Execute JavaScript code and return the result |
| `new Vm()` | Create a new isolated VM instance |
| `vm.run(code)` | Execute code within a VM instance (state persists across calls) |
| `vm.setGlobal(name, value)` | Define a global from a structured Node value (reachable as `name` and `window.name`) |
| `vm.exposeFunction(name, fn)` | Expose a sync Node function to the VM as a callable global; throws propagate into the VM |
| `vm.exposeAsyncFunction(name, fn)` | Expose an async Node function; VM must `await` the call (use with `runAsync`) |
| `vm.runAsync(code)` | Execute code on a VM thread; returns a `Promise<string>` that resolves when done. Supports `await` on async host functions. **Not for high-frequency use** — each call spawns an OS thread; sustained >100 calls/sec can crash. Use `vm.run()` for event handlers. |
| `vm.callFunction(name, args)` | Call a VM-defined global function; `args` is an array, returns a live JS value |
| `vm.setLoopLimit(n)` | Cap loop iterations per execution (default 100M); exceeding it throws a catchable `RangeError` |
| `vm.getGlobal(name)` | Read a global, stringified |
| `vm.registerModule(name, code)` | Register an ES module so its exports are importable by later `run` calls |
| `vm.removeModule(name)` | Remove a registered module (returns `bool`); call before re-registering on hot-reload |
| `vm.hasModule(name)` | Check whether a module is registered |
| `vm.listModules()` | Return the names of all registered modules |
| `vm.removeGlobal(name)` | Remove a global binding, including exposed host functions (returns `bool`) |
| `vm.hasGlobal(name)` | Check whether a global binding exists |
| `vm.setImportMetaMain(bool)` | Set the value of `import.meta.main` |
| `vm.inspect(code)` | Evaluate `code` and print the result through the native inspector (non-blocking inline tree) *(requires `--features inspector`)* |
| `vm.inspectValue(value)` | Marshal a host value (or `JSON.parse(str)`) and inspect it; copies across the boundary, so no circular structures *(requires `--features inspector`)* |
| `setInspectorConfig(opts)` | Override the inspector colors/depth *(requires `--features inspector`)* |
| `debugParse(code)` | Parse code and return the AST as a string |

### Native inspector

With the `inspector` Cargo feature the crate ships a DevTools-style foldable
tree inspector implemented entirely in Rust. Build it in:

```bash
npx napi build --platform --release --features inspector
```

It is **off by default** to keep the default NAPI surface minimal (it adds
no dependencies). The inspector is a **non-blocking inline dump**: it prints
a compact tree at the current position in the console flow — never on an
alternate screen, never full-page — and returns immediately, so the event
loop is never paused and there is nothing to close. The tree prints
**closed by default** (containers show `▶` fold hints); open it level by
level with `INSPECTOR_DEPTH` or `setInspectorConfig({ depth })`. Each dump
stays in the scrollback like any other log line, so multiple inspections
accumulate as a list alongside your `console.log` output, all of it visible
after the app exits. Two entry points:

```javascript
const { Vm, setInspectorConfig } = require('./index.js');
const vm = new Vm();
vm.run('var user = { name: "ada", nested: { city: "London" } };');

// From the host: evaluate an expression and inspect the result.
vm.inspect('user');

// From guest code: console.dir with { inspect: true } uses the same tree.
vm.run('console.dir(user, { inspect: true });');

// Inspect a plain host object (or parsed JSON) directly — marshalled in.
vm.inspectValue({ pid: process.pid, flags: ['inspector'] });
vm.inspectValue(JSON.parse('{"status":200}'));
```

`vm.inspect(code)` inspects the **live guest value** (cycles render as
`[Circular *n]`, any expression works). `vm.inspectValue(value)` is the
convenience path for host data: it **copies** the value across the boundary
(`from_napi` is depth-bounded, not cycle-aware), so it cannot show circular
structures — build those inside the VM and inspect the expression instead.
To inspect a JSON string as a guest value, parenthesize it —
`vm.inspect('(' + json + ')')` — since a bare `{"a":1}` is a JS block, not an
object.

Because the inspector walks the guest `Value` directly — no NAPI marshalling —
**circular guest structures render as `[Circular *n]`** instead of being lost
at the boundary. The behavior is identical in a TTY and in a pipe (CI): the
same depth-limited dump, never blocking. See
[`examples/inspector-native.ts`](examples/inspector-native.ts) for a full demo.

Colors and depth are configurable, highest precedence last:

- **Env var:** `INSPECTOR_DEPTH` (levels to open; default `0` = fully
  closed).
- **`setInspectorConfig()`**: `{ colors, depth }` — omitted fields keep
  their value.

## Sandbox limits & crash safety

"Sandboxed" here means both **isolation** (guest code cannot see `require`,
`process`, the filesystem, or the network) and **containment**: hostile or
buggy guest code cannot kill the host process. Every known process-killing
vector is guarded in the interpreter, and each guard raises a *catchable*
guest error — the same behavior V8 provides.
[`examples/crash.ts`](examples/crash.ts) is an executable catalogue of all 14
vectors: it runs each one in a disposable subprocess, scores how it died, and
fails (exit 1) if any case crashes, hangs, or disagrees with its pinned
`expected` verdict — i.e. it doubles as a CI regression gate
(`bun examples/crash.ts`). Current matrix: all 14 contained.

| Vector | Example | Behavior | Guard |
|--------|---------|----------|-------|
| Deep recursion | `function f(){ return f(); } f();` | ✅ catchable `RangeError: Maximum call stack size exceeded` | Call-depth counter (`MAX_CALL_DEPTH = 256`) checked before each VM call frame |
| Deep parse | 100k nested parens | ✅ catchable `RangeError: Maximum parse depth exceeded` | Parser nesting cap (`MAX_PARSE_DEPTH = 256`) with a depth latch |
| Cyclic structures | `let o={}; o.self=o; o;` | ✅ prints `{self: [Circular]}` | Visited-set of `Rc` pointers + depth cap in `to_string` |
| Cyclic JSON | `JSON.stringify(o)` on a cycle | ✅ catchable `TypeError: Converting circular structure to JSON` | Visited-set in `JSON.stringify`; depth cap `MAX_JSON_DEPTH = 512` (also bounds `JSON.parse`) |
| Deep nesting teardown | build `[ [ [ 0 ] ] ]` 1M deep | ✅ runs, then tears down cleanly | `Drop for Value` is iterative (explicit work stack) — O(1) native stack at any depth |
| Memory exhaustion | `while(true) a.push(…)` / `s = s + s` | ✅ catchable `RangeError: Maximum array/string length exceeded` | Hard caps `MAX_ARRAY_LEN = 262,144`, `MAX_STRING_LEN = 16 MB` on every growth path (push, concat, repeat, join, spread…) |
| Infinite loop | `while(true){}` | ✅ catchable `RangeError: Maximum loop iterations exceeded` | Per-execution loop budget (default 100M, tunable via `vm.setLoopLimit`), consumed per iteration, refilled at each NAPI entry |
| Generator misuse | recursive `yield*`, abandoned generators | ✅ absorbed / clean | Generators run on 8 MB scoped threads; thread failure surfaces as `{done: true}`, never as a host crash |
| Runtime errors, host-fn throws | `undefinedVar.foo;` | ✅ catchable error objects | Internal errors surface to `catch (e)` as real objects with `name`/`message` |

Why the guards matter: a native stack overflow or an allocation failure is a
**signal** (`SIGSEGV`/`SIGTRAP`), not an exception — `try/catch` in the VM,
`uncaughtException` in Node, and napi-rs's `catch_unwind` all fail to
intercept it; the process simply dies. Containment therefore means checking
depth, cycles, sizes, and budgets *before* the native runtime gets into
trouble, and converting every limit into an ordinary catchable error.

### Remaining residue: isolate operationally for full untrusted-code use

The in-process guards stop every known way guest code can *kill* the host,
but no in-process VM can provide complete untrusted-code containment without
OS help. Two residues are inherent:

- **CPU time is bounded per execution, not preemptible.** The loop budget
  stops `while (true) {}` within ~100M iterations (a couple of seconds), but
  the interpreter is synchronous: while guest code runs, the Node event loop
  is blocked. For strict latency guarantees, run guest code in a worker or
  child process with a watchdog.
- **Memory is capped per allocation shape, not metered in total.** The size
  caps keep any single structure survivable, but there is no aggregate heap
  quota. For a hard ceiling, run the VM under `ulimit -v` or a cgroup memory
  limit — exactly the pattern `examples/crash.ts` uses for its OOM cases.

A crash in a disposable subprocess costs one child, not the host; that
harness is a working template for operational isolation.

## Scripts

| Command | Description |
|---------|-------------|
| `npm run build` | Build optimized native binary |
| `npm run build:debug` | Build debug binary |
| `bun test` | Run the test suite |
| `npm run bench` | End-to-end JS benchmark through the NAPI binding |
| `npm run bench:stress` | Stress tests: hot-reload cycles, listener leaks, large JSON emit, middleware pressure (`--quick` for CI) |
| `npm run bench:rust` | Criterion microbenchmarks of the interpreter pipeline |

## Benchmarks

Two complementary layers measure the VM from different vantage points:

- **`benches/vm.rs`** (Criterion, `npm run bench:rust`) — microbenchmarks that drive the lexer → parser → interpreter pipeline directly, with no NAPI overhead. A `run` group times representative workloads end to end (arithmetic loops, recursion, array/string builtins, classes, closures, JSON), and a `frontend` group isolates lexing and parsing over a large source. Results, with HTML reports, land in `target/criterion/`.
- **`bench/bench.js`** (dependency-free, `npm run bench`) — runs the same workloads through the published `runCode` binding so each iteration crosses the NAPI boundary, and compares against an equivalent native-JS baseline to show the interpreter's overhead relative to the host engine. It also asserts the VM result matches the native one, so it doubles as a correctness check.

The JS workloads in both files are kept in sync so the two layers stay comparable.

As of commit `7f69614`, after a measurement-first optimization pass (recorded
in `bench/BASELINE.md`), the interpreter runs the recursion workload at ~69x
native (down from ~196x), closures at ~83x (from ~173x), and JSON roundtrips
at ~3x (from ~10x), with call-heavy workloads roughly halved overall.

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
├── inspector/          # Native inspector tree dump (feature = "inspector", off by default)
│   ├── mod.rs          # Entry point: the non-blocking inline dump
│   ├── tree.rs         # Lazy, cycle-aware foldable tree over Value
│   └── config.rs       # Config: defaults, env vars, NAPI setter
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
- ✅ Host bridge (Node ↔ VM) — `setGlobal`/`exposeFunction`/`exposeAsyncFunction`/`callFunction`/`runAsync` marshal structured values across the NAPI boundary over a stable raw `napi_sys` ABI. Sync exposed functions are persisted `napi_ref`s invoked directly; async exposed functions dispatch via a ThreadsafeFunction — the VM thread parks at `await` while the Node event loop resolves the real Promise, then resumes the VM with the fulfilled value (or re-throws a rejection). Exposed globals live on the one global scope, which `window`/`globalThis`/`self` all alias (`Value::GlobalObject`), so `window.add(1, 2)` and bare `add(1, 2)` are the same call. Not yet covered: passing a Node function *into* the VM as a first-class value, streaming responses, and a full `EventTarget`.

## License

No license specified.
