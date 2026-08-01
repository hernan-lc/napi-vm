#!/usr/bin/env node

// Protocol-level regression test for the Node LSP adapter. It exercises the
// same stdio framing used by an editor instead of calling the service directly.
const assert = require("node:assert/strict");
const path = require("node:path");
const { spawn } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const server = spawn(process.execPath, [
  path.join(__dirname, "server.cjs"),
  "--config",
  path.join(root, "examples", "hotreload.napi-vm.json"),
], { cwd: root, stdio: ["pipe", "pipe", "pipe"] });

let input = Buffer.alloc(0);
const waiters = [];

function flush() {
  while (true) {
    const separator = input.indexOf("\r\n\r\n");
    if (separator < 0) return;
    const header = input.subarray(0, separator).toString("ascii");
    const match = header.match(/Content-Length:\s*(\d+)/i);
    if (!match) {
      input = input.subarray(separator + 4);
      continue;
    }
    const length = Number(match[1]);
    const start = separator + 4;
    if (input.length < start + length) return;
    const body = input.subarray(start, start + length).toString("utf8");
    input = input.subarray(start + length);
    const message = JSON.parse(body);
    for (let index = waiters.length - 1; index >= 0; index--) {
      if (waiters[index](message)) waiters.splice(index, 1);
    }
  }
}

server.stdout.on("data", (chunk) => {
  input = Buffer.concat([input, chunk]);
  flush();
});

let stderr = "";
server.stderr.on("data", (chunk) => { stderr += chunk.toString(); });

function send(message) {
  const body = JSON.stringify(message);
  server.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

function response(id) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(
      `Timed out waiting for response ${id}. stderr: ${stderr}`,
    )), 5000);
    waiters.push((message) => {
      if (message.id !== id) return false;
      clearTimeout(timer);
      if (message.error) reject(new Error(JSON.stringify(message.error)));
      else resolve(message.result);
      return true;
    });
  });
}

function notification(method, params) {
  send({ jsonrpc: "2.0", method, params });
}

async function main() {
  send({
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: { rootUri: `file://${root}`, capabilities: {} },
  });
  const initialize = await response(1);
  assert.equal(initialize.serverInfo.name, "napi-vm-lsp");
  assert.equal(initialize.capabilities.hoverProvider, true);

  notification("initialized", {});
  const uri = `file://${path.join(root, "examples", "lsp-smoke.js")}`;
  const source = "hostNow;\nimport { add } from \"math\";\nadd;\n";
  notification("textDocument/didOpen", {
    textDocument: { uri, languageId: "javascript", version: 1, text: source },
  });

  send({
    jsonrpc: "2.0",
    id: 2,
    method: "textDocument/completion",
    params: { textDocument: { uri }, position: { line: 0, character: 7 } },
  });
  const completion = await response(2);
  assert.ok(completion.items.some((item) => item.label === "hostNow"));
  assert.ok(completion.items.some((item) => item.label === "math"));

  send({
    jsonrpc: "2.0",
    id: 3,
    method: "textDocument/hover",
    params: { textDocument: { uri }, position: { line: 0, character: 4 } },
  });
  const hover = await response(3);
  assert.match(hover.contents.value, /hostNow/);
  assert.match(hover.contents.value, /\(\) => number/);

  send({ jsonrpc: "2.0", id: 4, method: "shutdown", params: null });
  await response(4);
  notification("exit", {});
  await new Promise((resolve, reject) => {
    server.once("exit", (code) => code === 0
      ? resolve()
      : reject(new Error(`LSP exited with ${code}. stderr: ${stderr}`)));
  });
  console.log("LSP smoke test passed");
}

main().catch((error) => {
  server.kill();
  console.error(error.stack || error);
  process.exitCode = 1;
});
