/**
 * Node-VM Callback System — hot-reload + event bus demo.
 *
 * Build (TS → JS):  bun build examples/callback.ts --outdir examples/dist
 * Run:              bun examples/dist/callback.js
 * Or run directly:  bun examples/callback.ts
 *
 * What this demonstrates:
 *   1. Hot-reload with clean teardown (removeModule / removeGlobal) — no
 *      stale modules or leaked host functions across reloads.
 *   2. VmEventBus on/off pattern — host listeners survive reloads; the
 *      VM-side `emit` binding is replaced atomically, so there is never
 *      a duplicate-listener window.
 *   3. Event-loop blocking — the VM is synchronous, so a long computation
 *      inside `vm.run()` blocks the Node event loop. The demo schedules
 *      a `setTimeout` tick before a heavy VM call and shows that the tick
 *      fires only *after* the VM returns.
 */

import { Vm } from "../index";
import { join } from "node:path";
import { HotReloader } from "./lib/hot-reload";
import { VmEventBus } from "./lib/vm-event-bus";

const MODULES_DIR = join(import.meta.dir, "callbacks", "modules");

// ── Blocked-pattern validation (same rules as before) ────────────────

const BLOCKED_PATTERNS = [
  /\bwhile\s*\(\s*true\s*\)/,
  /\bfor\s*\(\s*;\s*;\s*\)/,
  /\beval\s*\(/,
  /\bFunction\s*\(/,
  /\bsetTimeout\s*\(/,
  /\bsetInterval\s*\(/,
  /\bimportScripts\s*\(/,
  /\bprocess\s*\./,
  /\brequire\s*\(/,
  /\b__dirname\b/,
  /\b__filename\b/,
];

function validateCode(source: string, _name: string): string[] {
  const errors: string[] = [];
  const lines = source.split("\n");
  for (let i = 0; i < BLOCKED_PATTERNS.length; i++) {
    for (let j = 0; j < lines.length; j++) {
      if (BLOCKED_PATTERNS[i].test(lines[j])) {
        errors.push(`Line ${j + 1}: blocked pattern '${BLOCKED_PATTERNS[i].source}'`);
      }
    }
  }
  if (source.length === 0) errors.push("Module source is empty");
  if (!source.includes("export")) errors.push("Module must export at least one symbol");
  return errors;
}

// ── Bootstrap: wire host functions + dispatch into a fresh VM ────────

function bootstrap(vm: Vm, bus: VmEventBus): void {
  // Expose host helpers (removeGlobal first to avoid duplicates on reload).
  for (const name of ["hostLog", "hostNow", "hostJson"]) {
    if (vm.hasGlobal(name)) vm.removeGlobal(name);
  }
  vm.exposeFunction("hostLog", (...args: unknown[]) => console.log("[host]", ...args));
  vm.exposeFunction("hostNow", () => Date.now());
  vm.exposeFunction("hostJson", (v: unknown) => JSON.stringify(v));

  // The dispatch + emit bootstrap code.
  vm.run(`
    import { greet, farewell, announce } from "greet";
    import { add, multiply, factorial, fib, clampValue } from "math";
    import { capitalize, reverse, repeat, slugify, wordCount } from "transform";

    var callbacks = {
      greet:      function(a)    { return greet(a); },
      farewell:   function(a)    { return farewell(a); },
      announce:   function(a, b) { return announce(a, b); },
      add:        function(a, b) { return add(a, b); },
      multiply:   function(a, b) { return multiply(a, b); },
      factorial:  function(a)    { return factorial(a); },
      fib:        function(a)    { return fib(a); },
      clampValue: function(a, b, c) { return clampValue(a, b, c); },
      capitalize: function(a)    { return capitalize(a); },
      reverse:    function(a)    { return reverse(a); },
      repeat:     function(a, b) { return repeat(a, b); },
      slugify:    function(a)    { return slugify(a); },
      wordCount:  function(a)    { return wordCount(a); }
    };

    function dispatch(name, args) {
      var fn = callbacks[name];
      if (!fn) throw new Error("Unknown callback: " + name);
      return fn(...args);
    }

    function dispatchToJson(name, args) {
      var result = dispatch(name, args);
      emit("callback", name, result);
      return JSON.stringify({ ok: true, callback: name, result: result });
    }

    /** Busy-work used by the event-loop blocking demo. */
    function heavyFib(n) {
      if (n <= 1) return n;
      return heavyFib(n - 1) + heavyFib(n - 2);
    }
  `);
}

// ── Demo runners ─────────────────────────────────────────────────────

const ALL_CALLS = [
  { name: "greet", args: ["Alice"] },
  { name: "farewell", args: ["Bob"] },
  { name: "announce", args: ["Server is starting", "system"] },
  { name: "add", args: [10, 20] },
  { name: "multiply", args: [6, 7] },
  { name: "factorial", args: [5] },
  { name: "fib", args: [10] },
  { name: "clampValue", args: [150, 0, 100] },
  { name: "capitalize", args: ["hello world"] },
  { name: "reverse", args: ["abcdef"] },
  { name: "repeat", args: ["ha", 3] },
  { name: "slugify", args: ["Hello World!  --  Foo Bar"] },
  { name: "wordCount", args: ["  the quick  brown fox  "] },
];

function runAllCallbacks(vm: Vm): void {
  console.log("--- Running All Callbacks ---\n");
  for (const call of ALL_CALLS) {
    const argsStr = JSON.stringify(call.args);
    const result = vm.run(`dispatchToJson("${call.name}", ${argsStr})`);
    console.log(`  ${call.name}(${call.args.join(", ")}) => ${result}`);
  }
  console.log("");
}

function printRegistry(reloader: HotReloader): void {
  console.log("--- Callback Registry ---\n");
  for (const [name, entry] of reloader.registry) {
    const status = entry.status === "active" ? "+" : "x";
    const err = entry.error ? ` (${entry.error})` : "";
    console.log(`  [${status}] ${name} => ${entry.file}${err}`);
  }
  console.log("");
}

// ── Event-loop blocking demo ─────────────────────────────────────────
// The VM interpreter is synchronous: vm.run() blocks the Node event loop
// until the computation finishes. This demo schedules a setTimeout tick
// *before* a heavy VM call, then shows the tick only fires after the VM
// returns — proving the VM work starves the event loop.

function demoEventLoopBlocking(vm: Vm): void {
  console.log("--- Event-Loop Blocking Demo ---\n");

  let tickFired = false;
  const t0 = Date.now();

  // Schedule a macrotask *before* the heavy VM call.
  setTimeout(() => {
    tickFired = true;
    console.log(`  [event-loop] setTimeout tick fired at +${Date.now() - t0}ms`);
  }, 0);

  console.log("  [main] setTimeout(0) scheduled, starting heavy VM work...");

  // heavyFib(32) is ~O(2^32) tree-recursive calls in the interpreter —
  // enough to block for a visible duration.
  const result = vm.run("heavyFib(32)");
  const elapsed = Date.now() - t0;

  console.log(`  [main] heavyFib(32) = ${result}  (took ${elapsed}ms)`);
  console.log(`  [main] tickFired after VM returned? ${tickFired}`);
  console.log(
    tickFired
      ? "  => tick fired during VM work (unexpected in a sync VM)"
      : "  => tick was STARVED until vm.run() returned — the VM blocks the event loop\n"
  );

  // Let the tick actually fire now that we yield.
  setTimeout(() => {
    console.log("  [event-loop] tick confirmed after yield — event loop resumed\n");
  }, 10);
}

// ── Listener dedup demo ──────────────────────────────────────────────
// Shows that bus.on/off prevents duplicate listeners across hot reloads.

function demoListenerDedup(bus: VmEventBus): void {
  console.log("--- Listener Dedup Demo ---\n");

  let callCount = 0;
  const counter = () => { callCount++; };

  // Simulate two "reload cycles" on a dedicated event — each time we
  // off() then on(), so the count never exceeds 1.
  for (let cycle = 1; cycle <= 2; cycle++) {
    bus.off("dedup-test", counter); // remove first (idempotent)
    bus.on("dedup-test", counter);  // then re-add
    console.log(`  cycle ${cycle}: listenerCount("dedup-test") = ${bus.listenerCount("dedup-test")}`);
  }

  console.log(`  => always exactly 1 listener, never duplicated\n`);
  bus.off("dedup-test"); // clean up
}

// ── Main ─────────────────────────────────────────────────────────────

console.log("=== Node-VM Callback System ===\n");

const reloader = new HotReloader({
  modulesDir: MODULES_DIR,
  validate: validateCode,
  onReload: (vm, bus) => bootstrap(vm, bus),
});

const vm = reloader.start();
const bus = reloader.bus;

// Register a persistent host-side listener (survives reloads).
const unsub = bus.on("callback", (name, result) => {
  // Quiet by default — uncomment to trace every dispatch:
  // console.log(`  [bus] ${name} =>`, result);
});

runAllCallbacks(vm);
printRegistry(reloader);
demoListenerDedup(bus);
demoEventLoopBlocking(vm);

// ── Hot-reload watcher ───────────────────────────────────────────────

console.log("--- Hot Reload Watcher ---\n");
console.log(`Watching: ${MODULES_DIR}\n`);

reloader.watch();

// Re-run the demo suite after each reload.
const origOnReload = reloader["opts"].onReload;
reloader["opts"].onReload = (newVm: Vm, newBus: VmEventBus) => {
  origOnReload(newVm, newBus);
  runAllCallbacks(newVm);
  printRegistry(reloader);
  console.log("Waiting for changes...\n");
};

console.log("Hot reload active. Edit a module file to see changes.");
console.log("Press Ctrl+C to stop.\n");

// Clean shutdown.
process.on("SIGINT", () => {
  unsub();
  reloader.stop();
  console.log("\nStopped.");
  process.exit(0);
});
