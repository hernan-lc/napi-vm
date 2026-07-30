/**
 * Async host bridge demo: exposeAsyncFunction + runAsync.
 *
 * Shows how to expose Node's native async APIs (fetch, timers, fs) to the
 * VM without subprocess spawning. The VM thread parks at `await` while the
 * Node event loop resolves the real async work.
 *
 * Run:  bun examples/async-bridge.ts
 */

import { Vm } from "../index";

const vm = new Vm();

// ── 1. Expose native fetch (no subprocess!) ─────────────────────────
vm.exposeAsyncFunction("fetch", async (url: string) => {
  const res = await fetch(url);
  const body = await res.text();
  return { status: res.status, body };
});

// ── 2. Expose a delay function (timer-based async) ──────────────────
vm.exposeAsyncFunction("sleep", (ms: number) => {
  return new Promise((resolve) => setTimeout(() => resolve(ms), ms));
});

// ── 3. Expose an async function that can throw ──────────────────────
vm.exposeAsyncFunction("maybeFail", async (shouldFail: boolean) => {
  if (shouldFail) throw new Error("intentional failure");
  return "success";
});

// ── Run async code inside the VM ────────────────────────────────────

async function main() {
  console.log("=== Async Bridge Demo ===\n");

  // 3a. fetch — native async, no subprocess
  console.log("--- fetch (native async) ---");
  const t0 = Date.now();
  const fetchResult = await vm.runAsync(`
    async function main() {
      var res = await fetch("https://jsonplaceholder.typicode.com/posts/1");
      var data = JSON.parse(res.body);
      return JSON.stringify({ title: data.title, status: res.status });
    }
    main();
  `);
  console.log("  result:", fetchResult);
  console.log("  time:", Date.now() - t0, "ms (no subprocess overhead)");

  // 3b. sleep — timer-based async
  console.log("\n--- sleep (timer) ---");
  const t1 = Date.now();
  const sleepResult = await vm.runAsync(`
    async function main() {
      var ms = await sleep(100);
      return "slept " + ms + "ms";
    }
    main();
  `);
  console.log("  result:", sleepResult);
  console.log("  elapsed:", Date.now() - t1, "ms");

  // 3c. error handling — async throw crosses the boundary
  console.log("\n--- error handling ---");
  const errResult = await vm.runAsync(`
    async function main() {
      try {
        await maybeFail(true);
        return "should not reach here";
      } catch (e) {
        return "caught: " + e.message;
      }
    }
    main();
  `);
  console.log("  result:", errResult);

  // 3d. multiple sequential awaits
  console.log("\n--- sequential awaits ---");
  const seqResult = await vm.runAsync(`
    async function main() {
      var a = await sleep(10);
      var b = await sleep(20);
      return "total: " + (a + b) + "ms";
    }
    main();
  `);
  console.log("  result:", seqResult);

  // 3e. sync exposeFunction still works alongside async
  console.log("\n--- sync + async coexistence ---");
  vm.exposeFunction("addSync", (a: number, b: number) => a + b);
  const mixedResult = await vm.runAsync(`
    async function main() {
      var x = addSync(10, 20);
      var y = await sleep(5);
      return "sync=" + x + " async=" + y;
    }
    main();
  `);
  console.log("  result:", mixedResult);

  console.log("\nDone.");
}

main().catch((err) => {
  console.error("FATAL:", err);
  process.exit(1);
});
