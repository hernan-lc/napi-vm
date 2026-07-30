/**
 * vm-worker.ts — Runs napi-vm inside a Worker thread.
 *
 * Receives messages with { id, code, modules? } and posts back
 * { id, result?, error? }. This keeps the main event loop free
 * while heavy VM work executes in the worker.
 */

import { parentPort, workerData } from "worker_threads";
import { Vm } from "../../index";
import { readFileSync } from "node:fs";
import { join } from "node:path";

interface VmRequest {
  id: number;
  code: string;
  modules?: Record<string, string>;
}

interface VmResponse {
  id: number;
  result?: string;
  error?: string;
}

// Bootstrap VM in the worker.
const vm = new Vm();
const modulesDir = workerData?.modulesDir as string | undefined;

// Pre-register modules if provided.
if (modulesDir) {
  const { readdirSync } = require("node:fs") as typeof import("node:fs");
  const files = readdirSync(modulesDir).filter((f: string) => f.endsWith(".js"));
  // Register utils first since other modules depend on it.
  const sorted = files.sort((a: string, b: string) => {
    if (a === "utils.js") return -1;
    if (b === "utils.js") return 1;
    return a.localeCompare(b);
  });
  for (const file of sorted) {
    const name = file.replace(/\.js$/, "");
    const code = readFileSync(join(modulesDir, file), "utf-8");
    vm.registerModule(name, code);
  }
}

// Run the same bootstrap code as the main thread to define dispatch helpers.
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
    return JSON.stringify({ ok: true, callback: name, result: result });
  }
`);

parentPort?.on("message", (msg: VmRequest) => {
  try {
    // Register inline modules if provided.
    if (msg.modules) {
      for (const [name, code] of Object.entries(msg.modules)) {
        if (vm.hasModule(name)) vm.removeModule(name);
        vm.registerModule(name, code);
      }
    }

    const result = vm.run(msg.code);
    parentPort?.postMessage({ id: msg.id, result } satisfies VmResponse);
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    parentPort?.postMessage({ id: msg.id, error } satisfies VmResponse);
  }
});

parentPort?.postMessage({ id: -1, result: "worker:ready" } satisfies VmResponse);
