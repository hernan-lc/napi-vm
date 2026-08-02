#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawn } = require("node:child_process");
const { Vm } = require("../index.js");
const { VmSession } = require("../runtime/session.cjs");

const root = fs.mkdtempSync(path.join(os.tmpdir(), "napi-vm-lsp-"));
const server = spawn(process.execPath, [path.join(__dirname, "server.cjs")], {
  cwd: path.resolve(__dirname, ".."),
  stdio: ["pipe", "pipe", "pipe"],
});
const session = new VmSession({ workspace: root, vm: new Vm(), sessionId: "lsp-smoke" });
let input = Buffer.alloc(0);
let stderr = "";
const messages = [];
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
    const message = JSON.parse(input.subarray(start, start + length).toString("utf8"));
    input = input.subarray(start + length);
    messages.push(message);
    for (let index = waiters.length - 1; index >= 0; index--) {
      if (waiters[index](message)) waiters.splice(index, 1);
    }
  }
}

server.stdout.on("data", (chunk) => {
  input = Buffer.concat([input, chunk]);
  flush();
});
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

async function waitForRuntimeCompletion(uri) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    const id = 100 + attempt;
    send({
      jsonrpc: "2.0",
      id,
      method: "textDocument/completion",
      params: { textDocument: { uri }, position: { line: 0, character: 7 } },
    });
    const result = await response(id);
    if (result.items.some((item) => item.label === "hostNow")) return result;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("LSP never received the live VmSession snapshot");
}

async function waitForEventCompletion(uri, character) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    const id = 200 + attempt;
    send({
      jsonrpc: "2.0",
      id,
      method: "textDocument/completion",
      params: { textDocument: { uri }, position: { line: 0, character } },
    });
    const result = await response(id);
    if (result.items.some((item) => item.label === "nickname")) return result;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("LSP never received the live event JSON shape");
}

async function main() {
  try {
    session.start();
    session.attach(session.vm, {
      modules: [{ name: "math", source: "export function add(a, b) { return a + b; }" }],
    });
    session.exposeFunction("hostNow", () => 1, {
      params: [],
      returns: "number",
      documentation: "Returns a live runtime value.",
    });

    send({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: { rootUri: `file://${root}`, capabilities: {} },
    });
    const initialize = await response(1);
    assert.equal(initialize.serverInfo.name, "napi-vm-lsp");
    notification("initialized", {});

    const uri = `file://${path.join(root, "main.js")}`;
    notification("textDocument/didOpen", {
      textDocument: { uri, languageId: "javascript", version: 1, text: "hostNow;\n" },
    });
    const completion = await waitForRuntimeCompletion(uri);
    assert.ok(completion.items.some((item) => item.label === "math"));

    const eventSource = "function handleChat(event) { event.data.";
    const eventUri = `file://${path.join(root, "chat.js")}`;
    session.observeHandler("handleChat", {
      platform: "tiktok",
      data: { nickname: "Ada", comment: "hello" },
    });
    notification("textDocument/didOpen", {
      textDocument: {
        uri: eventUri,
        languageId: "javascript",
        version: 1,
        text: eventSource,
      },
    });
    const eventCompletion = await waitForEventCompletion(eventUri, eventSource.length);
    assert.ok(eventCompletion.items.some((item) => item.label === "comment"));

    send({
      jsonrpc: "2.0",
      id: 500,
      method: "textDocument/hover",
      params: {
        textDocument: { uri: eventUri },
        position: { line: 0, character: eventSource.indexOf("event") + 2 },
      },
    });
    const eventHover = await response(500);
    assert.match(eventHover.contents.value, /Last value/);
    assert.match(eventHover.contents.value, /Ada/);

    send({
      jsonrpc: "2.0",
      id: 300,
      method: "textDocument/hover",
      params: { textDocument: { uri }, position: { line: 0, character: 4 } },
    });
    const hover = await response(300);
    assert.match(hover.contents.value, /hostNow/);
    assert.match(hover.contents.value, /number/);

    send({ jsonrpc: "2.0", id: 400, method: "shutdown", params: null });
    await response(400);
    notification("exit", {});
    await new Promise((resolve, reject) => {
      server.once("exit", (code) => code === 0
        ? resolve()
        : reject(new Error(`LSP exited with ${code}. stderr: ${stderr}`)));
    });
    console.log("Live runtime LSP smoke test passed");
  } finally {
    session.stop();
    if (!server.killed) server.kill();
    fs.rmSync(root, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error.stack || error);
  process.exitCode = 1;
});
