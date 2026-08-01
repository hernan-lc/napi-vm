// napi-vm playground — a JS interpreter written in Rust, running in your
// browser via WebAssembly. Ctrl/⌘+Enter to run · Ctrl+Space to complete.
//
// Built from three registered modules:
//   math   → import * as math         (namespace import)
//   format → import { upper, label }  (named imports)
//   store  → import createStore        (default export; store imports math)
//
// Try completing: "math." or "Math." after a dot, "al" for the exposed
// alert(), or Ctrl+Space on "up" / "cr" for the imported names.

import * as math from "math";
import { upper, label } from "format";
import createStore from "store";

const store = createStore({ count: 0, limit: 5 });
store.subscribe((key, value) => console.log("changed", key, "->", value));

function bump(times) {
  for (let i = 0; i < times; i++) {
    const next = math.clamp(store.read("count") + 1, 0, store.read("limit"));
    store.write("count", next);
  }
  return store.read("count");
}
createStore.
console.log(label("double(21)", math.double(21)));
console.log(upper("modular"));
bump(10);
console.log("final count:", store.read("count"));

alert("store count is " + store.read("count"));
store;
