/**
 * Native object inspector — a non-blocking inline tree dump, implemented
 * entirely in Rust. Run anywhere:
 *
 *   bun examples/inspector-native.ts
 *
 * Every inspect() prints its tree and returns immediately: no session, no
 * key to press, nothing blocking the event loop.
 */
import { Vm, setInspectorConfig } from "../index";

// ── Optional: customize colors / depth before any dump ──────────────
// Omitted fields keep their defaults. Depth 0 (the default) prints the
// tree fully closed (▶ hints only); remove this call to see that.
setInspectorConfig({
  // colors: false,   // force off (default: auto — TTY + NO_COLOR)
  depth: 2,           // open containers two levels deep
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

// Dump 1: host-driven — evaluate an expression and inspect the result.
//vm.inspect("user");

// Dump 2: guest-driven — console.dir with { inspect: true }.
vm.run('console.dir(user.address, { inspect: true });');

// Dump 3: a circular guest structure. The Rust inspector walks the live
// Value, so the cycle shows up as [Circular *1] instead of being lost.
vm.run(`
  var ring = { name: "root" };
  ring.self = ring;
  ring.nested = { back: ring, list: [1, ring] };
`);
vm.inspect("ring");

// Dump 4: inspect a host object directly — no setGlobal needed. The value
// is marshalled (copied) into the VM, so this path is for plain data; cycles
// live on the guest side (dump 3).
vm.inspectValue({
  source: "host",
  pid: process.pid,
  versions: { node: process.versions.node, platform: process.platform },
  flags: ["inspector"],
});

// Dump 5: inspect parsed JSON. Wrap in parens and inspect as a guest
// expression, or JSON.parse + inspectValue — both work; the parens form keeps
// it a guest value.
const json = '{"title":"sunt aut","status":200,"tags":["a","b"]}';
vm.run(`var payload = ${json};`);
vm.inspect("payload");

// Dumps are plain console output, so this lands right after the last
// inspection and the full log stays visible after exit.
console.log("\ndone.");
await new Promise(resolve => setTimeout(resolve, 5000));
