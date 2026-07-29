# node-vm

A sandboxed JavaScript virtual machine built from scratch in Rust with NAPI bindings for Node.js.

Execute JavaScript in an isolated environment with no access to the host system's Node.js globals, filesystem, or network.

## Features

- **Hand-written lexer & parser** — Full tokenizer and recursive descent parser
- **Tree-walking interpreter** — Executes parsed JavaScript in a custom runtime
- **Sandboxed execution** — No access to `require`, `process`, or other Node.js globals
- **Isolated VM instances** — Each `Vm()` has independent, persistent state
- **ES Module syntax** — `import`/`export` parsing and `import.meta` (module export wiring is incomplete — see roadmap)
- **Functions** — Regular, arrow, expressions, closures, and recursion
- **Control flow** — `if/else`, `while`, `do...while`, `for`, `for...in`, `for...of`, `switch/case`, `break`, `continue`
- **Error handling** — `try/catch/finally` with `throw`
- **Built-in objects** — `Math` constants and a wide set of global/Web API *stubs* (most built-in functions are not yet implemented — see roadmap)

> **Status:** the core language (variables, functions, control flow, operators, basic data types) is solid and covered by 380+ passing tests. Higher-level features — classes, prototype methods, and most built-ins — are parsed but not yet functional. See the [Roadmap](#roadmap--implementation-tracker) for the full, verified picture.

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

## API

| Function | Description |
|----------|-------------|
| `runCode(code)` | Execute JavaScript code and return the result |
| `new Vm()` | Create a new isolated VM instance |
| `vm.run(code)` | Execute code within a VM instance (state persists across calls) |
| `debugParse(code)` | Parse code and return the AST as a string |

## Scripts

| Command | Description |
|---------|-------------|
| `npm run build` | Build optimized native binary |
| `npm run build:debug` | Build debug binary |
| `bun test` | Run the test suite |

## Project Structure

```
src/
├── lexer.rs            # Tokenizer
├── parser/
│   ├── mod.rs          # Parser core (token cursor, helpers) + tests
│   ├── ast.rs          # AST node types (Expr, Statement, ...)
│   ├── stmt.rs         # Statement parsing
│   └── expr.rs         # Expression parsing (precedence ladder)
├── interpreter/
│   ├── mod.rs          # Interpreter, eval_stmt / eval_expr, calls + tests
│   ├── env.rs          # Environment, scope chain, Module
│   └── ops.rs          # Value operations (binary/unary, coercion, stringify)
├── value.rs            # Value enum
├── error.rs            # Error types
├── builtins.rs         # Built-in object definitions
└── bindings.rs         # NAPI bindings to Node.js
```

## Roadmap & Implementation Tracker

This tracker reflects **verified** behavior (checked against the interpreter directly, not just the parser). The executable specification lives in [`tests/ecma-gaps.test.js`](tests/ecma-gaps.test.js): each missing feature is a `test.skip` there — when a feature lands, un-skip its test and it should pass unchanged.

Legend: ✅ done · 🟡 partial · ❌ missing · 🐛 bug (breaks or hangs on valid JS)

### P0 — Correctness bugs

These break or hang on valid JavaScript and should be fixed before adding new features. Infinite loops are the worst failure mode because they hang the host process.

| Status | Item | Notes |
|--------|------|-------|
| ✅ | `break` inside loops | Was surfacing as an error; loops now catch the control-flow signal |
| ✅ | `continue` inside loops | Same fix as `break` |
| ✅ | `do...while` loop | Implemented |
| 🐛 | Default parameters `f(a = 10)` | **Parser infinite loop** — `params()` never consumes `=` |
| 🐛 | Rest parameters `f(...a)` | **Parser infinite loop** (same code path) |
| 🐛 | Paren-less arrow as call arg `map(x => x)` | **Parser infinite loop** in call-argument parsing |
| 🐛 | `typeof undeclaredVar` | Throws instead of returning `"undefined"` |
| 🐛 | `try/catch` catches runtime errors | Only catches `throw`; a runtime error (e.g. undefined var) escapes |
| 🐛 | `finally` runs after `catch` | `finally` is skipped when a `catch` clause handles the error |
| 🐛 | Member assignment `o.x = 5` | "Invalid assignment target" — only identifier targets supported |

### P1 — Core language features

**Operators** (bitwise/`**`/`??` are parsed but the interpreter rejects them as "Unknown op"):
- ❌ Bitwise `&` `|` `^` `~` `<<` `>>` `>>>`
- ❌ Exponentiation `**`
- ❌ Nullish coalescing `??`
- ❌ Optional chaining `?.`
- ❌ Comma operator `(a, b, c)`

**Objects:**
- ❌ Property shorthand `{ x }`
- ❌ Method shorthand `{ f() {} }`
- ❌ Computed keys `{ [k]: v }`
- ❌ Getters / setters
- ❌ Object spread `{ ...o }`
- ❌ `this` binding in object methods

**Functions:**
- ❌ Default parameters *(after the P0 hang fix)*
- ❌ Rest parameters
- ❌ `arguments` object
- ❌ Spread in calls `f(...args)`
- ❌ Arrow fn single param without parens / multiple params

**Destructuring & spread:**
- ❌ Array destructuring `const [a, b] = ...`
- ❌ Object destructuring `const { a, b } = ...`
- ❌ Spread in array literal `[...a, b]`

**Template literals** (the lexer has no backtick support):
- ❌ Basic `` `hi` `` and interpolation `` `hi ${name}` ``

**Control flow:**
- ❌ Labeled `break` / `continue`
- ❌ `for...of` over strings

**Classes & OOP** — parsed but **entirely non-functional** at runtime (`new Foo()` → "Not a constructor"):
- ❌ Constructor + `this`
- ❌ Instance methods
- ❌ Fields
- ❌ Static methods
- ❌ Inheritance (`extends`)
- ❌ `super`
- ❌ `instanceof` (always returns `false`)
- ❌ Function constructors via `new` + `this`

**Coercion correctness:**
- ❌ Boolean → number in arithmetic (`true + 1` gives `"true1"`, should be `2`)
- ❌ Loose equality coercion (`'5' == 5` and `0 == false` should be `true`)

### P2 — Standard library

Almost every built-in is currently a stub (`Value::Undefined`), so calling any of these throws "Not a function". Only `Math` constants (`PI`, `E`, …) are real.

- ❌ `Math` methods — `abs`, `floor`, `ceil`, `round`, `sqrt`, `pow`, `min`, `max`, `random`
- ❌ `console` — `log`, `error`, `warn`, `info`, `debug` (real output)
- ❌ `JSON.parse` / `JSON.stringify`
- ❌ `Object` — `keys`, `values`, `entries`, `assign`
- ❌ `Array` statics — `isArray`, `from`, `of`
- ❌ `Array.prototype` — `map`, `filter`, `reduce`, `forEach`, `find`, `push`, `pop`, `join`, `slice`, `concat`, `indexOf`, `includes`, `sort`
- ❌ `String.prototype` — `toUpperCase`, `toLowerCase`, `slice`, `split`, `includes`, `indexOf`, `trim`, `replace`, `charAt`
- ❌ String index access `'abc'[1]`
- ❌ `Number.prototype` — `toFixed`, `toString`
- ❌ Global functions — `parseInt`, `parseFloat`, `isNaN`, `isFinite`, `Number.isNaN`
- ❌ `Date` (`now`, parsing)
- ❌ Real `Error` objects with a `message`

### P3 — Advanced (not started)

- ❌ Prototype chain / `__proto__` — underpins `instanceof` and all prototype method lookup
- ❌ Promises and `async`/`await`
- ❌ Generators and `yield`
- ❌ Module exports actually reaching importers (`register_module` currently runs a module in a throwaway interpreter and discards its exports)
- ❌ Symbols and the iterator protocol

### Recommended implementation order

1. **Fix the P0 parser hangs first.** Infinite loops hang the host Node process, so they are more dangerous than ordinary errors. Make `params()` and call-argument parsing tolerant of `=`, `...`, and `=>`.
2. **Introduce a prototype/property model.** `Value::Object` is a flat property list with no prototype pointer. Adding one is the single highest-leverage change: it unblocks classes, `instanceof`, `this`-based method lookup, and every `String`/`Array`/`Number` prototype method in one stroke.
3. **Make native functions context-aware.** `NativeFunction` is currently `fn(Vec<Value>) -> Result<Value, VmErr>` — it can see neither the interpreter nor `this`. A richer callable signature (e.g. `fn(&mut Interpreter, Value /*this*/, Vec<Value>)`) is needed before `console.log`, `JSON`, and prototype methods can be implemented faithfully.
4. **Then the standard library falls out naturally** on top of (2) and (3).

## License

No license specified.
