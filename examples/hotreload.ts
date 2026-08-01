/**
 * napi-vm Callback System — hot-reload + event bus demo.
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
import { VmWorkerPool } from "./lib/vm-worker-pool";
import { VmSession } from "../runtime/session.cjs";

const MODULES_DIR = join(import.meta.dir, "callbacks", "modules");

// ── Bootstrap: wire host functions + dispatch into a fresh VM ────────

function bootstrap(vm: Vm, bus: VmEventBus, session?: VmSession): void {
  const expose = session
    ? (name: string, fn: (...args: unknown[]) => unknown, info: any) =>
      session.exposeFunction(name, fn, info)
    : (name: string, fn: (...args: unknown[]) => unknown, _info: any) =>
      vm.exposeFunction(name, fn);

  // Expose host helpers (removeGlobal first to avoid duplicates on reload).
  for (const name of ["hostLog", "hostNow", "hostJson"]) {
    if (vm.hasGlobal(name)) session ? session.removeGlobal(name) : vm.removeGlobal(name);
  }
  expose("hostLog", (...args: unknown[]) => console.log("[host]", ...args), {
    params: [{ name: "value", typeName: "unknown" }],
    returns: "void",
    documentation: "Writes a value to the Node host console.",
  });
  expose("hostNow", () => Date.now(), {
    params: [],
    returns: "number",
    documentation: "Returns the current timestamp.",
  });
  expose("hostJson", (v: unknown) => JSON.stringify(v), {
    params: [{ name: "value", typeName: "unknown" }],
    returns: "string",
    documentation: "Serializes a value using the Node host.",
  });

  // The dispatch + emit bootstrap code.
  vm.run(`
    import { greet, farewell, announce } from "greet";
    import { add, multiply, factorial, fib, clampValue } from "math";
    import { capitalize, reverse, repeat, slugify, wordCount } from "transform";
    import { heavyFib, whileLoop, nestedLoop, deepRecursion } from "blocking";

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
      wordCount:  function(a)    { return wordCount(a); },
      heavyFib:   function(a)    { return heavyFib(a); },
      whileLoop:  function(a)    { return whileLoop(a); },
      nestedLoop: function(a)    { return nestedLoop(a); },
      deepRecursion: function(a) { return deepRecursion(a); }
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
  `);
}


console.log("=== napi-vm Callback System ===\n");

const reloader = new HotReloader({
  modulesDir: MODULES_DIR,
  runtime: new VmSession({ workspace: join(import.meta.dir, "..") }),
  onReload: (vm, bus, session) => bootstrap(vm, bus, session),
});

const vm = reloader.start();
const bus = reloader.bus;

// Register a persistent host-side listener (survives reloads).
const unsub = bus.on("callback", (name, result) => {
  // Quiet by default — uncomment to trace every dispatch:
  // console.log(`  [bus] ${name} =>`, result);
});


console.log("Press Ctrl+C to stop.\n");
// Clean shutdown.
process.on("SIGINT", () => {
  unsub();
  reloader.stop();
  console.log("\nStopped.");
  process.exit(0);
});
