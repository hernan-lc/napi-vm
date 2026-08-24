#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const net = require("node:net");
const os = require("node:os");
const path = require("node:path");
const { Vm } = require("../index.js");
const { VmSession, runtimePath } = require("./session.cjs");

function connect(address, authToken) {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection(address);
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error("Timed out connecting to VmSession"));
    }, 3000);
    socket.once("connect", () => {
      clearTimeout(timer);
      socket.setEncoding("utf8");
      socket.write(`${JSON.stringify({ type: "auth", token: authToken })}\n`);
      resolve(socket);
    });
    socket.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
  });
}

function nextMessage(socket) {
  return new Promise((resolve, reject) => {
    let buffer = "";
    const onData = (chunk) => {
      buffer += chunk;
      const newline = buffer.indexOf("\n");
      if (newline < 0) return;
      const line = buffer.slice(0, newline).trim();
      socket.off("data", onData);
      if (!line) return nextMessage(socket).then(resolve, reject);
      try {
        resolve(JSON.parse(line));
      } catch (error) {
        reject(error);
      }
    };
    socket.on("data", onData);
    socket.once("error", reject);
  });
}

function assertUnauthorized(address) {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection(address);
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error("unauthorized runtime client was not closed"));
    }, 3000);
    socket.setEncoding("utf8");
    socket.on("connect", () => {
      socket.write(`${JSON.stringify({ type: "auth", token: "wrong-token" })}\n`);
    });
    socket.on("data", () => {
      clearTimeout(timer);
      socket.destroy();
      reject(new Error("unauthorized runtime client received data"));
    });
    socket.on("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    socket.on("close", () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

async function readLocatorWhenReady(workspace) {
  const file = runtimePath(workspace);
  for (let attempt = 0; attempt < 50; attempt += 1) {
    try {
      return JSON.parse(fs.readFileSync(file, "utf8"));
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
  }
  throw new Error("VmSession did not publish its locator");
}

async function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "napi-vm-runtime-"));
  const vm = new Vm();
  const session = new VmSession({ workspace, vm, sessionId: "smoke" });
  let socket;

  try {
    session.start();
    session.attach(vm, {
      modules: [{
        name: "math",
        source: "export function add(a, b) { return a + b; }",
      }],
    });
    session.exposeFunction("hostNow", () => 1, {
      params: [],
      returns: "number",
      documentation: "Returns a test timestamp.",
    });

    const locator = await readLocatorWhenReady(workspace);
    await assertUnauthorized(locator.transport.address);
    socket = await connect(locator.transport.address, locator.authToken);
    const first = await nextMessage(socket);
    assert.equal(first.type, "snapshot");
    assert.equal(first.payload.functions[0].name, "hostNow");
    assert.equal(first.payload.modules[0].name, "math");

    session.exposeFunction("hostJson", (value) => JSON.stringify(value), {
      params: [{ name: "value", type: "unknown" }],
      returns: "string",
    });
    const second = await nextMessage(socket);
    assert.equal(second.payload.generation > first.payload.generation, true);
    assert.ok(second.payload.functions.some((item) => item.name === "hostJson"));

    session.observeHandler("handleChat", {
      platform: "tiktok",
      data: { nickname: "Ada", comment: "hello" },
    });
    const third = await nextMessage(socket);
    assert.equal(third.payload.handlers[0].name, "handleChat");
    assert.equal(third.payload.handlers[0].shape.properties.data.properties.nickname.kind, "string");
    assert.equal(third.payload.handlers[0].lastValue.data.nickname, "Ada");

    socket.destroy();
    session.stop();
    assert.equal(fs.existsSync(runtimePath(workspace)), false);
    console.log("VmSession runtime smoke test passed");
  } finally {
    socket?.destroy();
    session.stop();
    fs.rmSync(workspace, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error.stack || error);
  process.exitCode = 1;
});
