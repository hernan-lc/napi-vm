/**
 * Exposing host-side APIs (fetch, filesystem, child_process) to the VM.
 *
 * The VM is fully isolated by default — no `require`, no `process`, no
 * network, no filesystem.  This example shows how to selectively open
 * controlled channels via `vm.exposeFunction()` so the sandboxed code
 * can call real Node APIs.
 *
 * LIMITATION: The NAPI bridge is *synchronous*.  Exposed functions
 * receive plain values and return plain values — no Promises cross
 * the boundary.  Node's fetch() is async and returns a Promise, so it
 * cannot be exposed directly.  For HTTP, we shell out to a sync
 * subprocess (cross-platform via node child_process).
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

// ── 1. hostFetch — synchronous HTTP via Node subprocess ─────────────
// Node's fetch() is async, but the bridge is sync.  We run a tiny
// Node script that does the fetch and prints the result as JSON,
// then parse it back.  This is cross-platform (no curl dependency).

vm.exposeFunction("hostFetch", (url: string) => {
  try {
    const script = `
      fetch("${url}")
        .then(r => r.text().then(body => {
          console.log(JSON.stringify({ status: r.status, body }));
        }))
        .catch(e => {
          console.log(JSON.stringify({ error: e.message }));
          process.exit(1);
        });
    `;
    const out = execSync(`node -e "${script.replace(/"/g, '\\"')}"`, {
      encoding: "utf-8",
      timeout: 10_000,
    });
    const result = JSON.parse(out.trim());
    if (result.error) throw new Error(result.error);
    return result;
  } catch (err: any) {
    throw new Error(`hostFetch failed: ${err.message}`);
  }
});

// ── 2. hostPost — synchronous POST via Node subprocess ──────────────

vm.exposeFunction(
  "hostPost",
  (url: string, data: string, contentType?: string) => {
    try {
      const ct = contentType ?? "application/json";
      // Write data to a temp file to avoid escaping issues
      const tmpFile = `/tmp/vm-post-${Date.now()}.json`;
      writeFileSync(tmpFile, data, "utf-8");
      const script = `
        const fs = require("fs");
        const body = fs.readFileSync("${tmpFile}", "utf-8");
        fetch("${url}", {
          method: "POST",
          headers: { "Content-Type": "${ct}" },
          body
        })
          .then(r => r.text().then(t => {
            console.log(JSON.stringify({ status: r.status, body: t }));
          }))
          .catch(e => {
            console.log(JSON.stringify({ error: e.message }));
            process.exit(1);
          });
      `;
      const out = execSync(`node -e "${script.replace(/"/g, '\\"')}"`, {
        encoding: "utf-8",
        timeout: 10_000,
      });
      const result = JSON.parse(out.trim());
      if (result.error) throw new Error(result.error);
      return result;
    } catch (err: any) {
      throw new Error(`hostPost failed: ${err.message}`);
    }
  }
);

// ── 3. filesystem (read / write) ────────────────────────────────────
// Thin wrappers around Node `fs` — intentionally limited to a single
// directory so the sandbox can't wander off.

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

console.log("=== Host API Demo ===\n");

// 5a. fetch — basic GET
console.log("--- fetch (GET) ---");
const fetchResult = vm.run(`
  var res = hostFetch("https://jsonplaceholder.typicode.com/posts/1");
  JSON.stringify({ status: res.status, bodyLen: res.body.length });
`);
console.log("  ", fetchResult);

// 5b. fetch — parse JSON response
console.log("\n--- fetch (parse JSON) ---");
const parsedResult = vm.run(`
  var res = hostFetch("https://jsonplaceholder.typicode.com/posts/1");
  var data = JSON.parse(res.body);
  JSON.stringify({ title: data.title, userId: data.userId });
`);
console.log("  ", parsedResult);

// 5c. POST — create a resource
console.log("\n--- fetch (POST) ---");
const postResult = vm.run(`
  var res = hostPost(
    "https://jsonplaceholder.typicode.com/posts",
    JSON.stringify({ title: "VM Post", body: "Created from the sandbox", userId: 1 })
  );
  var data = JSON.parse(res.body);
  JSON.stringify({ status: res.status, createdId: data.id });
`);
console.log("  ", postResult);

// 5d. filesystem
console.log("\n--- filesystem ---");
vm.run(`hostWriteFile("data.json", JSON.stringify({ count: 42 }))`);
const readBack = vm.run(`hostReadFile("data.json")`);
console.log("  read back:", readBack);
const files = vm.run(`hostListDir()`);
console.log("  directory:", files);

// 5e. child_process
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

// 5g. combined: fetch + process data in the VM
console.log("\n--- combined: fetch + transform ---");
vm.run(`import { add } from "math";`);
const combined = vm.run(`
  var r = hostFetch("https://jsonplaceholder.typicode.com/posts/1");
  var body = JSON.parse(r.body);
  // add the title length to the userId
  add(body.title.length, body.userId);
`);
console.log("  title.length + userId:", combined);

// 5h. timestamp utility
console.log("\n--- timestamp ---");
const ts = vm.run(`hostNow()`);
console.log("  hostNow():", ts);

// ── Cleanup ─────────────────────────────────────────────────────────

console.log("\nDone.");
