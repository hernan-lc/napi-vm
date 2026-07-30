/**
 * Native interactive object inspector — implemented entirely in Rust.
 * run anywhere:
 *
 *   bun examples/inspector-native.ts
 */
import { Vm, setInspectorConfig } from "../index";

// ── Optional: customize colors / close key before any session ───────
// Omitted fields keep their defaults. The key override must be one char.
setInspectorConfig({
  // colors: false,      // force off (default: auto — TTY + NO_COLOR)
  // keyQuit: "x",       // close on `x` instead of `q`
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

// Session 1: host-driven — evaluate an expression and inspect the result.
//vm.inspect("user");

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

// Each closed session leaves a compact tree listing in the scrollback, so
// this line lands right after the last inspection and the full log — every
// console.log and every inspection — stays visible after exit.
console.log("\ndone.");
