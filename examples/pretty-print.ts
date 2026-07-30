/**
 * Pretty-printing nested objects from inside the VM, browser-style.
 *
 * By default `console.log(obj)` in the VM prints the opaque `[object Object]`.
 * This demo shows two ways to get a clean, expanded, browser-like tree:
 *
 *   A. `console.dir(...)`  — a native builtin. Reuses the VM's own pretty
 *      formatter (`bindings::to_string_pretty`), so it runs fully inside the
 *      sandbox, needs no host, and is cycle- and depth-safe. Prints an
 *      indented, multi-line tree (scalar-only arrays stay on one line).
 *      Output is type-colored in a TTY (keys cyan, strings green, numbers
 *      blue, booleans yellow, null/undefined dimmed); pass an options object
 *      to override: `console.dir(obj, { colors: true })` forces ANSI codes
 *      even into a pipe, `{ colors: false }` suppresses them. `NO_COLOR` and
 *      `FORCE_COLOR` environment variables are honored too.
 *
 *   B. `pretty(...)`       — a host function exposed via `exposeFunction`
 *      that marshals the value to Node and prints it with `util.inspect`
 *      (the same formatter the browser/Node consoles use). Richest output
 *      (colors, types), but values cross the NAPI boundary.
 *
 * Run:  bun examples/pretty-print.ts
 */
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { Vm } = require("../index");
import { inspect } from "node:util";

const vm = new Vm();

// ── Option B: host-side printer via util.inspect ────────────────────
// Values are marshalled to Node, then pretty-printed with the real
// console formatter. `depth: null` expands everything; `colors` enables
// syntax highlighting in a TTY.
vm.exposeFunction("pretty", (...args: unknown[]) => {
  const out = args
    .map((a) => inspect(a, { depth: null, colors: true, compact: false }))
    .join(" ");
  console.log(out);
});

// A sample nested structure to print.
const sample = `
  var user = {
    name: "ada",
    age: 36,
    active: true,
    tags: ["math", "engines"],
    address: { city: "London", zip: "NW1" },
    scores: [10, [20, [30]]]
  };
`;

console.log("=== Pretty-print Demo ===\n");

// ── Baseline: what plain console.log does today ─────────────────────
console.log("--- console.log (opaque) ---");
vm.run(sample + "console.log(user);");

// ── Option A: native console.dir builtin ────────────────────────────
console.log("\n--- console.dir (native builtin) ---");
vm.run(sample + "console.dir(user);");

// ── Option A again: circular reference stays safe ───────────────────
console.log("\n--- console.dir (circular) ---");
vm.run(`
  var o = { name: "root" };
  o.self = o;
  console.dir(o);
`);

// ── Option A with forced ANSI colors (visible even when piped) ──────
console.log("\n--- console.dir (colors forced) ---");
vm.run(sample + "console.dir(user, { colors: true });");

// ── Option B: host-side util.inspect ────────────────────────────────
console.log("\n--- pretty() via util.inspect (host) ---");
vm.run(sample + "pretty(user);");

console.log("\nDone.");
