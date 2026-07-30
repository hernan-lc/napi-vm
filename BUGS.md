# Bug Report: napi-vm Limitations & Fixes

Found by building the `examples/callback.ts` hot-reload callback system.

---

## Bug 1: `Object.keys(window)` returns `[]` (FIXED)

**Severity:** Medium  
**File:** `src/interpreter/ops.rs:189-195`  
**Status:** Fixed

### Description

`Object.keys(window)` returns an empty array even though `window` is the global
scope with hundreds of builtins. `Object.getOwnPropertyNames(window)` works
correctly and returns all keys.

### Root Cause

`Interpreter::keys()` has no match arm for `Value::GlobalObject`. It falls
through to `_ => vec![]`.

### Reproduction

```js
const vm = new Vm();
vm.run(`
  const keys = Object.keys(window);
  console.log(keys.length); // 0 (should be 206+)
`);
```

### Fix

Added `Value::GlobalObject => self.global_keys()` to the `keys()` method in
`src/interpreter/ops.rs:193`.

---

## Bug 2: `String.prototype.charCodeAt` not implemented

**Severity:** High  
**File:** `src/builtins/string.rs`  
**Status:** Fixed

### Description

`charCodeAt()` is not in the string method dispatch table. Any code that calls
`str.charCodeAt(i)` crashes with `error: Not a function`.

This breaks the `slugify` function in the callback example and any code that
needs character encoding values.

### Reproduction

```js
const vm = new Vm();
vm.run(`"hello".charCodeAt(0)`); // error: Not a function
```

### Missing Methods (related)

| Method         | Used by slugify? | Impact |
|----------------|------------------|--------|
| `charCodeAt`   | YES              | Breaks character encoding |
| `codePointAt`  | No               | Unicode support |
| `concat`       | No               | String concatenation |
| `padStart`     | No               | Formatting |
| `padEnd`       | No               | Formatting |
| `trimStart`    | No               | Whitespace trimming |
| `trimEnd`      | No               | Whitespace trimming |
| `lastIndexOf`  | No               | Reverse search |
| `search`       | No               | Regex search |
| `match`        | No               | Regex matching |
| `replaceAll`   | No               | Global replace |
| `at`           | No               | Negative indexing |

### Proposed Fix

Add `charCodeAt` to `src/builtins/string.rs`:

```rust
fn string_char_code_at(_: &mut Interpreter, s: &Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let idx = a.first().cloned().unwrap_or(Value::Number(0.0));
    let i = match idx {
        Value::Number(n) => n as usize,
        _ => 0,
    };
    match s.chars().nth(i) {
        Some(ch) => Ok(Value::Number(ch as u32 as f64)),
        None => Ok(Value::Number(f64::NAN)),
    }
}
```

And add to the dispatch:

```rust
"charCodeAt" => string_char_code_at,
```

---

## Bug 3: `substring` is aliased to `slice` (wrong semantics)

**Severity:** Low  
**File:** `src/builtins/string.rs:15`  
**Status:** Fixed

### Description

`String.prototype.substring` is mapped to the same implementation as `slice`.
In real JS, they differ:

- `slice(-3)` → last 3 chars; `substring(-3)` → treats -3 as 0
- `substring(3, 1)` → `"lo"` (swaps args); `slice(3, 1)` → `""` (no swap)

### Reproduction

```js
const vm = new Vm();
vm.run(`
  "hello".substring(3, 1)  // returns "" (wrong, should be "el")
`);
vm.run(`
  "hello".slice(3, 1)      // returns "" (correct)
`);
```

### Proposed Fix

Implement `substring` as a separate function with proper arg swapping and
non-negative clamping.

---

## Bug 4: `replace` only replaces first match

**Severity:** Low  
**File:** `src/builtins/string.rs:134-147`  
**Status:** Fixed (`replaceAll` added)

### Description

`String.prototype.replace` uses `replacen(..., 1)` so it only replaces the
first occurrence. This is actually correct JS behavior for `replace` with a
string pattern. However, `replaceAll` is missing entirely.

### Proposed Fix

Add `replaceAll` that uses Rust's `replace()` (replaces all occurrences).

---

## Bug 5: Hot reload requires VM rebuild

**Severity:** Medium (design limitation)  
**Status:** Not a bug, inherent to architecture

### Description

The VM does not support re-registering or hot-swapping modules. When a file
changes, the only option is to create a fresh `Vm` instance and re-register
all modules. This works but discards all VM state.

### Workaround (used in callback.ts)

```ts
function reload(changedFile: string) {
  // Create entirely new VM
  const vm = createVmWithCallbacks();
  runAllCallbacks(vm);
  testDispatch(vm);
}
```

### Possible Enhancement

Add a `vm.unregisterModule(name)` method and support re-registration, so
existing global state (dispatch functions, etc.) can be rebuilt without
discarding everything.
