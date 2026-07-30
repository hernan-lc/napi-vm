/**
 * Exposing host-side APIs (fetch, filesystem, child_process) to the VM.
 *
 * The VM is fully isolated by default — no `require`, no `process`, no
 * network, no filesystem.  This example shows how to selectively open
 * controlled channels via `vm.exposeFunction()` and
 * `vm.exposeAsyncFunction()` so the sandboxed code can call real Node APIs.
 *
 * HTTP uses the native async bridge (`exposeAsyncFunction` + `runAsync`):
 * the VM thread parks at `await` while Node's event loop resolves the real
 * fetch — no subprocess spawning, no temp files, no shell escaping.
 *
 * Run:  bun examples/host-apis.ts
 */

import { Vm } from "../index";
import {
  readFileSync,
  writeFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
} from "node:fs";
import { execSync } from "node:child_process";

const vm = new Vm();

// ── 1. hostFetch — native async HTTP (no subprocess!) ───────────────
// exposeAsyncFunction lets the VM `await` real Node Promises directly.

vm.exposeAsyncFunction("hostFetch", async (url: string) => {
  const res = await fetch(url);
  const body = await res.text();
  return { status: res.status, body };
});

// ── 2. hostPost — native async POST ─────────────────────────────────

vm.exposeAsyncFunction(
  "hostPost",
  async (url: string, data: string, contentType?: string) => {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": contentType ?? "application/json" },
      body: data,
    });
    const body = await res.text();
    return { status: res.status, body };
  }
);

// ── 3. filesystem (read / write) ────────────────────────────────────
// Thin wrappers around Node `fs` — intentionally limited to a single
// directory so the sandbox can't wander off.  These are sync APIs, so
// plain `exposeFunction` is fine.

const FS_ROOT = "/tmp/napi-vm-fs-demo";

if (!existsSync(FS_ROOT)) {
  mkdirSync(FS_ROOT, { recursive: true });
  writeFileSync(`${FS_ROOT}/greeting.txt`, "Hello from the host filesystem!");
}

vm.exposeFunction("hostReadFile", (path: string) => {
  return readFileSync(`${FS_ROOT}/${path}`, "utf-8");
});

vm.exposeFunction("hostWriteFile", (path: string, data: string) => {
  writeFileSync(`${FS_ROOT}/${path}`, data, "utf-8");
  return true;
});

vm.exposeFunction("hostListDir", () => {
  return readdirSync(FS_ROOT);
});

// ── 4. child_process (exec) ─────────────────────────────────────────
// Only whitelisted commands are forwarded — this is a *controlled*
// channel, not a raw escape hatch.

const ALLOWED_CMDS = new Set(["ls", "pwd", "date", "whoami", "uname"]);

vm.exposeFunction("hostExec", (cmd: string) => {
  if (!ALLOWED_CMDS.has(cmd)) {
    throw new Error(
      `Command not allowed: ${cmd}.  Whitelist: ${[...ALLOWED_CMDS].join(", ")}`
    );
  }
  return execSync(cmd, { encoding: "utf-8" }).trim();
});

// ── 5. utility: current timestamp ───────────────────────────────────

vm.exposeFunction("hostNow", () => Date.now());

// ── Register a tiny math module so the combined example can import ──
vm.registerModule("math", `export function add(a, b) { return a + b; }`);

// ── Run demo code inside the VM ─────────────────────────────────────

async function main() {
  console.log("=== Host API Demo ===\n");

  // 5a. fetch — basic GET (native async, no subprocess)
  console.log("--- fetch (GET) ---");
  const t0 = Date.now();
  const fetchResult = await vm.runAsync(`
    async function main() {
      var res = await hostFetch("https://jsonplaceholder.typicode.com/posts/1");
      return JSON.stringify({ status: res.status, bodyLen: res.body.length });
    }
    main();
  `);
  console.log("  ", fetchResult);
  console.log("   time:", Date.now() - t0, "ms");

  // 5b. fetch — parse JSON response
  console.log("\n--- fetch (parse JSON) ---");
  const parsedResult = await vm.runAsync(`
    async function main() {
      var res = await hostFetch("https://jsonplaceholder.typicode.com/posts/1");
      var data = JSON.parse(res.body);
      return JSON.stringify({ title: data.title, userId: data.userId });
    }
    main();
  `);
  console.log("  ", parsedResult);

  // 5c. POST — create a resource (no temp files, no shell escaping)
  console.log("\n--- fetch (POST) ---");
  const postResult = await vm.runAsync(`
    async function main() {
      var res = await hostPost(
        "https://jsonplaceholder.typicode.com/posts",
        JSON.stringify({ title: "VM Post", body: "Created from the sandbox", userId: 1 })
      );
      var data = JSON.parse(res.body);
      return JSON.stringify({ status: res.status, createdId: data.id });
    }
    main();
  `);
  console.log("  ", postResult);

  // 5d. filesystem (sync — uses vm.run directly)
  console.log("\n--- filesystem ---");
  vm.run(`hostWriteFile("data.json", JSON.stringify({ count: 42 }))`);
  const readBack = vm.run(`hostReadFile("data.json")`);
  console.log("  read back:", readBack);
  const files = vm.run(`hostListDir()`);
  console.log("  directory:", files);

  // 5e. child_process (sync)
  console.log("\n--- child_process ---");
  const whoami = vm.run(`hostExec("whoami")`);
  console.log("  whoami:", whoami);
  const uname = vm.run(`hostExec("uname")`);
  console.log("  uname:", uname);

  // 5f. blocked command
  console.log("\n--- blocked command ---");
  const blockResult = vm.run(`
    try {
      hostExec("rm -rf /");
    } catch (e) {
      e.message;
    }
  `);
  console.log("  result:", blockResult);

  // 5g. combined: async fetch + sync transform in one runAsync call
  console.log("\n--- combined: fetch + transform ---");
  vm.run(`import { add } from "math";`);
  const combined = await vm.runAsync(`
    async function main() {
      var r = await hostFetch("https://jsonplaceholder.typicode.com/posts/1");
      var body = JSON.parse(r.body);
      // add the title length to the userId (sync host fn inside async context)
      return add(body.title.length, body.userId);
    }
    main();
  `);
  console.log("  title.length + userId:", combined);

  // 5h. timestamp utility (sync)
  console.log("\n--- timestamp ---");
  const ts = vm.run(`hostNow()`);
  console.log("  hostNow():", ts);

  // 5i. error handling — async errors are catchable in the VM
  console.log("\n--- async error handling ---");
  const errResult = await vm.runAsync(`
    async function main() {
      try {
        await hostFetch("https://this-domain-does-not-exist-xyz.invalid");
      } catch (e) {
        return "caught: " + e.message;
      }
    }
    main();
  `);
  console.log("  ", errResult);

  console.log("\nDone.");
}

main().catch((err) => {
  console.error("FATAL:", err);
  process.exit(1);
});
