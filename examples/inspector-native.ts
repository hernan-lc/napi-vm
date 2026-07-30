/**
 * Native interactive object inspector — implemented entirely in Rust.
 *
 * This is the Rust successor to `examples/inspector.ts`. Where that file
 * rendered values *marshalled across the NAPI boundary* (so circular guest
 * structures were lost), this one walks the live guest `Value` inside the
 * crate, so cycles render as `[Circular *n]` and nothing is copied out.
 *
 * Requires the `inspector` Cargo feature, which implies `mouse` — so a single
 * flag builds the full keyboard + mouse inspector and its NAPI surface:
 *
 *   npx napi build --platform --release --features inspector
 *
 * Two entry points:
 *
 *   A. `vm.inspect("expression")` — evaluate an expression on the host and
 *      open the inspector on the result.
 *
 *   B. `console.dir(obj, { inspect: true })` — the same inspector, reachable
 *      from inside guest code. Without `{ inspect: true }`, `console.dir`
 *      keeps its static pretty-print.
 *
 *   C. `vm.inspectValue(hostObj)` — marshal a plain Node value (or the result
 *      of `JSON.parse(str)`) and inspect it. The value is copied across the
 *      boundary, so unlike (A) this path cannot show circular structures.
 *
 *   To inspect a JSON string as a guest value, wrap it in parens:
 *   `vm.inspect("(" + json + ")")` — `{"a":1}` alone is a JS block, not an
 *   object, so it must be parenthesized (or assigned to a variable first).
 *
 * Controls (defaults, vi-style):
 *   ↑/↓ or j/k      move          →/space/enter/l   expand
 *   ←/h             collapse / go to parent         e/c   expand/collapse all
 *   mouse click     focus / toggle row              scroll wheel   move
 *   q / esc / ctrl-c  quit
 *
 * Shortcuts and colors are configurable — env vars (`INSPECTOR_KEY_QUIT=x`,
 * …) or `setInspectorConfig({ keyQuit: "x", colors: false, … })`.
 *
 * In a non-TTY environment (pipes, CI) both entry points fall back to a
 * static, cycle-safe pretty dump and never block, so this file is safe to run
 * anywhere:
 *
 *   bun examples/inspector-native.ts
 */
import { Vm, setInspectorConfig } from "../index";

// ── Optional: customize the keymap / colors before any session ──────
// Omitted fields keep their defaults. Letter overrides must be one char.
setInspectorConfig({
  // colors: false,      // force off (default: auto — TTY + NO_COLOR)
  // keyQuit: "x",       // quit on `x` instead of `q`
  // keyExpandAll: "E",
});

const vm = new Vm();

vm.run(`
  var user = {
    name: "ada",
    age: 36,
    active: true,
    tags: ["math", "engines"],
    address: { city: "London", zip: "NW1" },
    scores: [10, [20, [30]]],
    metadata: null,
    nickname: undefined,
    id: 42
  };
`);

console.log("=== Native Inspector Demo ===");
console.log("(in a TTY: arrow keys / mouse to navigate, q to quit each session)\n");

// Session 1: host-driven — evaluate an expression and inspect the result.
vm.inspect("user");

// Session 2: guest-driven — console.dir with { inspect: true }.
vm.run('console.dir(user.address, { inspect: true });');

// Session 3: a circular guest structure. The Rust inspector walks the live
// Value, so the cycle shows up as [Circular *1] instead of being lost.
vm.run(`
  var ring = { name: "root" };
  ring.self = ring;
  ring.nested = { back: ring, list: [1, ring] };
`);
vm.inspect("ring");

// Session 4: inspect a host object directly — no setGlobal needed. The value
// is marshalled (copied) into the VM, so this path is for plain data; cycles
// live on the guest side (session 3).
vm.inspectValue({
  source: "host",
  pid: process.pid,
  versions: { node: process.versions.node, platform: process.platform },
  flags: ["inspector", "mouse"],
});

// Session 5: inspect parsed JSON. Wrap in parens and inspect as a guest
// expression, or JSON.parse + inspectValue — both work; the parens form keeps
// it a guest value.
const json = '{"title":"sunt aut","status":200,"tags":["a","b"]}';
vm.run(`var payload = ${json};`);
vm.inspect("payload");

console.log("\ndone.");
