import type { ModuleDef } from "./types.ts";

export const MATH = `
export function double(x) { return x * 2; }
export function clamp(x, lo, hi) { return x < lo ? lo : (x > hi ? hi : x); }
export const PI = 3.141592653589793;
`;

export const FORMAT = `
export function upper(s) { return (s + "").toUpperCase(); }
export function label(name, value) { return name + " = " + value; }
export function pad2(n) { return n < 10 ? "0" + n : "" + n; }
`;

export const STORE = `
import { clamp } from "math";

export class Store {
  constructor(initial) {
    this.state = initial;
    this.listeners = [];
  }
  read(key) { return this.state[key]; }
  write(key, value) {
    this.state[key] = clamp(value, -1000000, 1000000);
    for (let i = 0; i < this.listeners.length; i++) this.listeners[i](key, value);
    return this;
  }
  subscribe(fn) { this.listeners.push(fn); return this; }
}

export function createStore(initial) { return new Store(initial); }
export default createStore;
`;

export const MODULES: ModuleDef[] = [
  { name: "math", source: MATH },
  { name: "format", source: FORMAT },
  { name: "store", source: STORE },
];

export const SAMPLE = `// napi-vm playground — a JS interpreter written in Rust, running in your
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

console.log(label("double(21)", math.double(21)));
console.log(upper("modular"));
bump(10);
console.log("final count:", store.read("count"));

alert("store count is " + store.read("count"));
store;
`;
