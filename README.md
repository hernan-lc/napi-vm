# node-vm

A sandboxed JavaScript virtual machine built from scratch in Rust with NAPI bindings for Node.js.

Execute JavaScript in an isolated environment with no access to the host system's Node.js globals, filesystem, or network.

## Features

- **Hand-written lexer & parser** — Full tokenizer and recursive descent parser
- **Tree-walking interpreter** — Executes parsed JavaScript in a custom runtime
- **Sandboxed execution** — No access to `require`, `process`, or other Node.js globals
- **Isolated VM instances** — Each `Vm()` has independent, persistent state
- **ES Module support** — `import`/`export` syntax and `import.meta`
- **Class support** — Constructors, methods, static methods, fields, and `extends`
- **Functions** — Regular, arrow, expressions, closures, and recursion
- **Control flow** — `if/else`, `while`, `for`, `for...in`, `for...of`, `switch/case`, `break`, `continue`
- **Error handling** — `try/catch/finally` with `throw`
- **Built-in objects** — `Math`, `console`, `JSON`, `Object`, `Array`, and Web API stubs

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
├── lexer.rs        # Tokenizer
├── parser.rs       # Recursive descent parser
├── interpreter.rs  # AST interpreter with Environment
├── value.rs        # Value enum
├── error.rs        # Error types
├── builtins.rs     # Built-in object definitions
└── bindings.rs     # NAPI bindings to Node.js
```

## License

No license specified.
