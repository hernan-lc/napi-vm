/**
 * napi-vm IPC + hot-reload example.
 *
 * Run with:
 *   bun examples/hotreload.ts
 *
 * The VM calls host commands through ipc.invoke(), sends events through
 * ipc.send(), and imports the modules in callbacks/modules/. The host-side
 * command and event registrations survive every VM replacement.
 */

import { Vm } from "../index";
import { join } from "node:path";
import { HotReloader } from "./lib/hot-reload";
import { VmIpc } from "./lib/vm-ipc";
import { VmSession } from "../runtime/session.cjs";

const MODULES_DIR = join(import.meta.dir, "callbacks", "modules");
const WORKSPACE = join(import.meta.dir, "..");
const VM_SETUP_SOURCE = `
    import { greet, farewell, announce } from "greet";
    import { add, multiply, factorial, fib, clampValue } from "math";
    import { capitalize, reverse, repeat, slugify, wordCount } from "transform";
    import { heavyFib, whileLoop, nestedLoop, deepRecursion } from "blocking";

    function runModuleCommands() {
      return {
        greet: greet("Ada"),
        farewell: farewell("Ada"),
        announce: announce("hello", "playground"),
        sum: add(20, 22),
        product: multiply(6, 7),
        factorial: factorial(5),
        fib: fib(10),
        clamped: clampValue(99, 0, 10),
        title: capitalize("napi-vm"),
        reversed: reverse("IPC"),
        repeated: repeat(".", 3),
        slug: slugify("Hello VM IPC"),
        words: wordCount("commands cross the VM boundary"),
        loop: whileLoop(5),
        nested: nestedLoop(3),
        recursion: deepRecursion(5),
        heavy: heavyFib(8)
      };
    }

    function runIpcTest() {
      var response = ipc.invoke("system.ping", { origin: "vm", data: 42 });
      ipc.send("test:response", response);
      ipc.send("test:modules", runModuleCommands());
      return response;
    }

    function runAsyncIpcTest() {
      return ipc.invokeAsync("system.asyncPing", { origin: "vm", data: 7 });
    }
  `;

console.log("=== napi-vm IPC System ===\n");

const ipc = new VmIpc();
ipc.handle("system.ping", (payload) => ({
  ok: true,
  received: payload,
  at: Date.now(),
}), {
  params: [{ name: "payload", typeName: "unknown" }],
  returns: "object",
  documentation: "Round-trip command used by the IPC smoke test.",
});
ipc.handle("system.json", (payload) => JSON.stringify(payload), {
  params: [{ name: "payload", typeName: "unknown" }],
  returns: "string",
  documentation: "Serializes a payload in the Node host.",
});
ipc.handleAsync("system.asyncPing", async (payload) => ({
  ok: true,
  received: payload,
}), {
  params: [{ name: "payload", typeName: "unknown" }],
  returns: "object",
  documentation: "Asynchronous IPC command used with invokeAsync.",
});

console.log("Commands:", ipc.listCommands().join(", "));

// The runtime/LSP session is opt-in. Running this example normally keeps the
// VM entirely in-process and does not create .napi-vm/runtime.json.
const runtimeSession = process.env.NAPI_VM_SESSION === "1"
  ? new VmSession({ workspace: WORKSPACE })
  : undefined;
console.log(runtimeSession
  ? "Live LSP runtime session: enabled"
  : "Live LSP runtime session: disabled (set NAPI_VM_SESSION=1 to enable)");

const reloader = new HotReloader({
  modulesDir: MODULES_DIR,
  ...(runtimeSession ? { runtime: runtimeSession } : {}),
  onBeforeLoad: (vm, session) => {
    ipc.attach(vm, session);
  },
  onReload: (vm) => {
    vm.run(VM_SETUP_SOURCE);
  },
});

const vm = reloader.start();
reloader.watch();

// Host listeners are outside the VM and therefore survive hot-reload.
const unsubscribeResponse = ipc.on("test:response", (payload) => {
  console.log("  [event] test:response", payload);
});
const unsubscribeModules = ipc.on("test:modules", (payload) => {
  console.log("  [event] test:modules", payload);
});

console.log("  [command] runIpcTest =>", vm.run("runIpcTest();"));
console.log("  [command] system.json =>", vm.run(
  'ipc.invoke("system.json", { ok: true, source: "vm" });',
));
console.log("  [command] available =>", vm.run("ipc.commands();"));

// runAsync is required when the VM awaits an asynchronous host command.
vm.runAsync("async function main() { return await runAsyncIpcTest(); } main();")
  .then((result) => console.log("  [async command] system.asyncPing =>", result))
  .catch((error) => console.error("  [async command] error =>", error));

console.log("\nEdit a module in examples/callbacks/modules/ to trigger hot-reload.");
console.log("Press Ctrl+C to stop.\n");

let stopping = false;
const stdin = process.stdin;
const onStdinData = (chunk: string | Buffer) => {
  if (chunk.toString().includes("\u0003")) shutdown("Ctrl+C");
};
const shutdown = (signal: string) => {
  if (stopping) return;
  stopping = true;
  console.log(`\n[shutdown] ${signal} received`);
  try {
    if (stdin.isTTY && stdin.setRawMode) {
      stdin.setRawMode(false);
      stdin.off("data", onStdinData);
      stdin.pause();
    }
    unsubscribeResponse();
    unsubscribeModules();
    ipc.detach();
    reloader.stop();
  } catch (error) {
    console.error(`[shutdown] cleanup error: ${error instanceof Error ? error.message : error}`);
  } finally {
    console.log("[shutdown] complete");
    // A pending runAsync worker may intentionally keep its runtime alive. The
    // example is interactive, so Ctrl+C must terminate even if that worker is
    // waiting on a host promise or a module is doing CPU work.
    process.exit(0);
  }
};

process.once("SIGINT", () => shutdown("SIGINT"));
process.once("SIGTERM", () => shutdown("SIGTERM"));

// Bun/Windows terminals can deliver Ctrl+C as a raw byte instead of raising
// SIGINT when the process is launched through an editor task or pipe. Handling
// both paths makes this example reliably stoppable in PowerShell, cmd, and
// integrated terminals while restoring the terminal mode during cleanup.
if (stdin.isTTY && stdin.setRawMode) {
  stdin.setEncoding("utf8");
  stdin.setRawMode(true);
  stdin.resume();
  stdin.on("data", onStdinData);
}
