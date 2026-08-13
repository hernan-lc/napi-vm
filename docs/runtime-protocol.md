# napi-vm Runtime Protocol v1

The Runtime Protocol allows a running application to stream live metadata
(host functions, registered modules, observed event shapes) to the
`napi-vm-lsp` language server so the editor can provide rich completions
and hover information without executing guest scripts.

---

## Overview

```
application (Node, Electron, Tauri, …)
    │
    ├─ writes  <workspace>/.napi-vm/runtime.json   (locator)
    │
    └─ listens on:
        • Unix domain socket  (Linux / macOS)
        • Windows named pipe  (Windows)
```

```
napi-vm-lsp
    │
    ├─ polls   <workspace>/.napi-vm/runtime.json
    │
    └─ connects to the socket/pipe and reads newline-delimited JSON
```

The LSP is always the **client**. The application is the **server**.

---

## Discovery

### Locator file

Path: `<workspace>/.napi-vm/runtime.json`

The application writes this file when its session starts and removes it (or
lets it become stale) when the session ends. The LSP polls this file every
500 ms.

### Locator schema

```json
{
  "protocolVersion": 1,
  "workspaceId": "a3f1b9c4e82d67f0ab12",
  "sessionId": "550e8400-e29b-41d4-a716-446655440000",
  "pid": 12345,
  "transport": {
    "kind": "unix",
    "address": "/tmp/napi-vm-a3f1b9c4.sock"
  }
}
```

| Field             | Type     | Description                                             |
|-------------------|----------|---------------------------------------------------------|
| `protocolVersion` | `number` | Must be `1`. Mismatches are rejected.                   |
| `workspaceId`     | `string` | SHA-256 of the realpath of the workspace root, first 20 hex chars. |
| `sessionId`       | `string` | Unique opaque ID regenerated for every process start.   |
| `pid`             | `number` | OS process ID of the application. Used to detect stale sessions. |
| `transport.kind`  | `string` | `"unix"` or `"named-pipe"`.                             |
| `transport.address` | `string` | Socket path or named-pipe path.                       |

### Workspace ID computation

The workspace ID is computed identically by both the application and the LSP
to prevent cross-workspace socket hijacking:

```
sha256(realpath(workspaceRoot)).hex().slice(0, 20)
```

JavaScript (Node / Electron):

```js
const crypto = require("node:crypto");
const fs = require("node:fs");

function workspaceId(root) {
  return crypto
    .createHash("sha256")
    .update(fs.realpathSync(root))
    .digest("hex")
    .slice(0, 20);
}
```

Rust (napi-vm):

```rust
use sha2::{Digest, Sha256};

fn workspace_id(root: &Path) -> std::io::Result<String> {
    let resolved = std::fs::canonicalize(root)?;
    let digest = Sha256::digest(resolved.to_string_lossy().as_bytes());
    Ok(hex_prefix(&digest, 20))
}
```

---

## Transport

### Linux / macOS — Unix domain socket

```json
{ "kind": "unix", "address": "/tmp/napi-vm-<workspaceId>.sock" }
```

The application creates a Unix domain socket and accepts exactly one
connection (from the LSP). The LSP connects, and messages flow server→client
as newline-delimited JSON until the socket is closed.

### Windows — Named pipe

```json
{ "kind": "named-pipe", "address": "\\\\.\\pipe\\napi-vm-<workspaceId>" }
```

Same semantics, different transport. The application creates the named pipe;
the LSP opens it for reading.

---

## Encoding

All messages are **UTF-8 newline-delimited JSON** (`\n` terminated).
Each line is one complete JSON object. Empty lines are ignored.

---

## Messages (application → LSP)

### `snapshot`

Delivers a complete view of the current runtime state. The LSP replaces its
entire analysis context with the payload; it does not merge with previous
snapshots.

```json
{
  "type": "snapshot",
  "payload": {
    "functions": [ ... ],
    "modules":   [ ... ],
    "handlers":  [ ... ]
  }
}
```

The application should send a `snapshot` immediately after the LSP connects
and whenever the state changes significantly (e.g., after `exposeFunction`,
`registerModule`, or `observeHandler`).

#### `payload.functions` — host-exposed functions

Array of host functions callable from guest scripts.

```json
[
  {
    "name": "alert",
    "params": [
      { "name": "message", "typeName": "string" }
    ],
    "returns": "void",
    "documentation": "Displays a message in the host UI.",
    "async": false
  }
]
```

| Field           | Type      | Description                                             |
|-----------------|-----------|---------------------------------------------------------|
| `name`          | `string`  | Function name as it appears in the guest script.        |
| `params`        | `array`   | Positional parameter list (may be empty).               |
| `params[].name` | `string`  | Parameter name.                                         |
| `params[].typeName` | `string` | TypeScript-style type annotation.                   |
| `returns`       | `string`  | Return type annotation (default `"unknown"`).           |
| `documentation` | `string?` | Optional Markdown documentation string.                 |
| `async`         | `boolean` | Whether the function returns a `Promise`.               |

#### `payload.modules` — registered modules

Array of modules importable from guest scripts.

```json
[
  {
    "name": "utils",
    "source": "export function format(v) { return String(v); }"
  }
]
```

| Field    | Type     | Description                                                  |
|----------|----------|--------------------------------------------------------------|
| `name`   | `string` | Module specifier used in `import … from "name"`.             |
| `source` | `string` | Full source text of the module.                              |

#### `payload.handlers` — runtime event shapes

Array of observed shapes for VM event handler functions. The LSP uses these
to type the first parameter of matching handler functions in guest scripts.

```json
[
  {
    "name": "handleChat",
    "shape": {
      "kind": "object",
      "properties": {
        "platform": { "kind": "string" },
        "data": {
          "kind": "object",
          "properties": {
            "nickname": { "kind": "string" },
            "comment":  { "kind": "string" }
          }
        }
      }
    },
    "lastValue": {
      "platform": "tiktok",
      "data": { "nickname": "Ada", "comment": "hello" }
    }
  }
]
```

| Field       | Type      | Description                                                  |
|-------------|-----------|--------------------------------------------------------------|
| `name`      | `string`  | Handler function name in the guest script.                   |
| `shape`     | `object`  | JSON type schema (see [Shape schema](#shape-schema)).        |
| `lastValue` | `any?`    | Last observed value. Shown in hover to aid debugging.        |

---

## Shape schema

Shapes are recursive JSON objects used to describe the structure of runtime
values.

### Primitive kinds

| `kind`    | Description         |
|-----------|---------------------|
| `string`  | String value        |
| `number`  | Number value        |
| `boolean` | Boolean value       |
| `null`    | Null value          |
| `unknown` | Unknown/untyped     |

### Object kind

```json
{
  "kind": "object",
  "properties": {
    "<key>": { /* nested shape */ }
  }
}
```

### Array kind

```json
{
  "kind": "array",
  "items": { /* element shape */ }
}
```

---

## Session lifecycle

```
application starts
    │
    ├─ bind socket / pipe
    ├─ write .napi-vm/runtime.json
    │
    │   (LSP polls, detects new session, connects)
    │
    ├─ accept connection
    ├─ send snapshot
    │
    │   (functions / modules / handlers change)
    ├─ send updated snapshot
    │
application stops (or crashes)
    │
    ├─ close socket / pipe
    └─ delete / leave stale .napi-vm/runtime.json
        (LSP detects stale PID and clears its context)
```

---

## Security considerations

- The `workspaceId` field prevents one project's locator from being used by
  another project's LSP instance.
- Socket and pipe paths are scoped to the session and regenerated on restart.
- The protocol carries only metadata (types, names, shapes). No guest script
  bytecode or host secrets are transmitted.
- Only the local LSP process reads from the socket. No network listeners are
  opened.

---

## Implementing a server

Any language that can write the locator file and listen on a Unix socket or
Windows named pipe can implement this protocol.

### Node / Electron (`VmSession`)

The included `VmSession` class in `index.js` implements the full server side
for Node.js and Electron applications:

```js
import { VM, VmSession } from "napi-vm";

const vm = new VM();
const session = new VmSession(vm, process.cwd());
session.start();

vm.exposeFunction("alert", (msg) => alert(msg));
session.flush(); // send updated snapshot immediately
```

### Rust / Tauri (future)

A pure-Rust `VmSession` implementation for Tauri and native desktop
applications can implement the same protocol without any Node dependency.
The socket and locator formats are identical.

---

## Version history

| Version | Notes                     |
|---------|---------------------------|
| `1`     | Initial protocol version. |
