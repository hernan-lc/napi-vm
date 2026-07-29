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
- **Error handling** — `try/catch/finally` with `throw`, catching both `throw` and runtime errors
- **Classes & OOP** — constructors, instance methods/fields, static members, `extends`/`super`, `instanceof`
- **Standard library** — `Math`, `JSON`, `Object`, `Array`/`String`/`Number` prototype methods, and global functions (`parseInt`, `isNaN`, …)

> **Status:** the core language plus classes and a working standard library are implemented and covered by 460+ passing tests (see the [Roadmap](#roadmap--implementation-tracker) for the verified picture). Remaining gaps are advanced areas — full async/`await` semantics, generators/`yield`, and module export wiring.

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
├── lexer.rs            # Tokenizer (783 lines — see Modularization Plan)
├── parser/
│   ├── mod.rs          # Parser core (token cursor, helpers) + tests
│   ├── ast.rs          # AST node types (Expr, Statement, ...)
│   ├── stmt.rs         # Statement parsing (765 lines)
│   └── expr.rs         # Expression parsing (1083 lines)
├── interpreter/
│   ├── mod.rs          # Interpreter, eval_stmt / eval_expr, calls + tests (1471 lines)
│   ├── env.rs          # Environment, scope chain, Module
│   └── ops.rs          # Value operations (binary/unary, coercion, stringify)
├── value.rs            # Value enum
├── error.rs            # Error types
├── builtins.rs         # Built-in object definitions (1187 lines)
└── bindings.rs         # NAPI bindings to Node.js
```

## Modularization Plan

Four source files exceed ~800 lines and are candidates for splitting. The goal is to keep each file focused on a single responsibility, make the codebase navigable, and reduce merge-conflict surface. The split is designed so that each new module has a clear boundary and minimal cross-references.

### 1. `src/interpreter/mod.rs` (1471 lines) → 4 files

This is the largest file. It mixes struct definition, statement evaluation, expression evaluation, function/constructor calls, and property resolution.

| Extract to | Contains | Lines ~ |
|---|---|---|
| `interpreter/eval.rs` | `eval_stmt()` + `eval_expr()` — the two giant match dispatchers | ~450 |
| `interpreter/call.rs` | `call_this()`, `invoke_ctor()`, `ctor()`, `run_catch()`, `destructure()`, `assign_member()` | ~260 |
| `interpreter/resolve.rs` | `prop()`, `get_prop_value()`, and the prototype-chain lookup logic | ~90 |
| `interpreter/mod.rs` | `Interpreter` struct, `new()`, `run()`, helpers, tests | ~670 → ~300 |

**Key consideration:** `eval` and `call` are tightly coupled (each calls the other). They stay as sibling `impl Interpreter` blocks in separate files under the same `interpreter/` module, so cross-references are `crate::interpreter::*` or `super::` — no API change.

### 2. `src/builtins.rs` (1187 lines) → 6 files

This file defines every global, then implements native methods for Math, Array, String, Number, Object, and JSON — all in one flat namespace.

| Extract to | Contains | Lines ~ |
|---|---|---|
| `builtins/math.rs` | `math_methods()` + all `math_*` functions | ~130 |
| `builtins/array.rs` | `array_method()` + `array_is_array()` + all `array_*` prototype functions | ~240 |
| `builtins/string.rs` | `string_method()` + all `string_*` prototype functions | ~135 |
| `builtins/number.rs` | `number_method()` + `number_is_nan/finite` | ~35 |
| `builtins/object.rs` | `object_keys/values/entries/assign` | ~40 |
| `builtins/json.rs` | `json_stringify/parse` + `escape_json` | ~75 |
| `builtins/mod.rs` | `setup_builtins()`, `install_functions()`, helpers (`nf`, `arg_num`, `arr_items`, `str_this`, `join_str`) | ~530 → ~250 |

**Key consideration:** All native functions share the `NativeFn` type and the `nf()` constructor — these live in `builtins/mod.rs` and are `pub use`-d into sub-modules. The `setup_builtins()` function stays as the single orchestrator that calls into each sub-module's `install_*()` function.

### 3. `src/parser/expr.rs` (1083 lines) → 2 files

The precedence ladder (comma → assign → cond → ... → unary → postfix → primary) is well-structured but long. The bottom of the stack — `primary()` and its helpers — is the most self-contained chunk.

| Extract to | Contains | Lines ~ |
|---|---|---|
| `parser/primary.rs` | `primary()`, `new_callee()`, `try_arrow()`, `arrow_body()`, `take_quasi()` | ~320 |
| `parser/expr.rs` | Precedence ladder (comma through postfix) | ~760 → ~440 |

**Key consideration:** `postfix()` calls `primary()` — a single `super::primary::primary(&mut self)` reference. The ladder itself remains intact and readable.

### 4. `src/parser/stmt.rs` (765 lines) → 2 files

Statement parsing mixes simple one-liners (if, while, return) with large compound parsers (class, import/export).

| Extract to | Contains | Lines ~ |
|---|---|---|
| `parser/stmt.rs` | `stmt()`, `var_decl`, `pattern`, `fn_decl`, `ret`, `if_`, `while_`, `do_`, `for_`, `throw`, `try_`, `switch`, `block_or_stmt`, `params`, `ident` | ~765 → ~480 |
| `parser/compound.rs` | `class_decl()`, `export()`, `import()`, `from()`, `block_body()`, `default_guard()` | ~285 |

**Key consideration:** `class_decl` and `import`/`export` are the largest individual parsers in this file and the most likely to grow as features are added. Isolating them makes the core `stmt` dispatcher easier to follow.

### 5. `src/lexer.rs` (783 lines) → minor cleanup

The lexer is borderline. The template literal scanner (`read_template` + `lex_interp`) is ~80 lines that could move to `lexer/template.rs`, but the file is otherwise cohesive. **Recommended:** defer unless template support grows.

### Execution order

1. **builtins.rs** first — it has no internal coupling beyond shared helpers, so it's the safest to split and immediately pays dividends when implementing P2 stdlib features.
2. **interpreter/mod.rs** second — split out eval/call/resolve, then the P0 bug fixes become easier to navigate.
3. **parser/expr.rs** and **parser/stmt.rs** third — lower urgency since the parser is stable and well-tested.
4. **lexer.rs** — optional, defer.

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
- ✅ `Array.prototype` — `map`, `filter`, `reduce`, `forEach`, `find`, `some`, `every`, `push`, `pop`, `join`, `slice`, `concat`, `reverse`, `indexOf`, `includes`
- ✅ `String.prototype` — `toUpperCase`, `toLowerCase`, `slice`, `substring`, `split`, `includes`, `indexOf`, `trim`, `replace`, `charAt`, `startsWith`, `endsWith`, `repeat`
- ✅ String index access `'abc'[1]`
- ✅ `Number.prototype` — `toFixed`
- ✅ Global functions — `parseInt`, `parseFloat`, `isNaN`, `isFinite`, `Number.isNaN`, `Number.isFinite`
- 🟡 `console` — members exist as stubs (no host output yet)
- ❌ `Array.prototype.sort`, `flat`, `flatMap`, `reduceRight`
- ❌ `Date` (`now`, parsing)
- ❌ Real `Error` objects with a `message`

### P3 — Advanced

- ✅ Prototype chain — `Value::Object` carries a `proto` pointer (shared `Rc`), underpinning `instanceof` and prototype method lookup
- 🟡 `async` functions — declared/called and resolve to a fulfilled/rejected `Promise`; `await` is not yet implemented
- 🟡 Generators — `function*` parses and is callable (`typeof` → `"function"`); `yield` suspension is not yet implemented
- ❌ Module exports actually reaching importers (`register_module` currently runs a module in a throwaway interpreter and discards its exports)
- ❌ Symbols and the iterator protocol

## License

No license specified.
