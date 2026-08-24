const crypto = require("node:crypto");
const fs = require("node:fs");
const net = require("node:net");
const path = require("node:path");

const PROTOCOL_VERSION = 1;
const MAX_FRAME_BYTES = 1024 * 1024;
const AUTH_TIMEOUT_MS = 5000;

function workspaceId(root) {
  const resolved = fs.realpathSync(root);
  return crypto.createHash("sha256").update(resolved).digest("hex").slice(0, 20);
}

function locatorPath(root) {
  return path.join(root, ".napi-vm", "runtime.json");
}

function processAlive(pid) {
  if (process.platform === "win32") return true;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

class RuntimeClient {
  constructor(root, callbacks) {
    this.root = root;
    this.callbacks = callbacks;
    this.socket = null;
    this.sessionId = null;
    this.locator = null;
    this.timer = null;
    this.stopped = false;
  }

  start() {
    this.stopped = false;
    this.timer = setInterval(() => this.discover(), 500);
    this.timer.unref?.();
    this.discover();
  }

  stop() {
    this.stopped = true;
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
    this.close();
  }

  discover() {
    if (this.stopped) return;
    let locator;
    try {
      locator = JSON.parse(fs.readFileSync(locatorPath(this.root), "utf8"));
      if (locator.protocolVersion !== PROTOCOL_VERSION) throw new Error("protocol mismatch");
      if (locator.workspaceId !== workspaceId(this.root)) throw new Error("workspace mismatch");
      if (typeof locator.authToken !== "string" || locator.authToken.length === 0 || locator.authToken.length > 256) {
        throw new Error("invalid runtime capability token");
      }
      if (!processAlive(locator.pid)) throw new Error("stale process");
    } catch {
      if (this.sessionId) {
        this.sessionId = null;
        this.locator = null;
        this.close();
        this.callbacks.onSnapshot(null);
      }
      return;
    }

    if (locator.sessionId === this.sessionId) return;
    this.close();
    this.locator = locator;
    this.sessionId = locator.sessionId;
    const socket = net.createConnection(locator.transport.address);
    this.socket = socket;
    let buffer = "";
    let authenticated = false;
    socket.setEncoding("utf8");
    socket.setTimeout(AUTH_TIMEOUT_MS, () => socket.destroy());
    socket.on("connect", () => {
      socket.write(`${JSON.stringify({ type: "auth", token: locator.authToken })}\n`);
    });
    socket.on("data", (chunk) => {
      buffer += chunk;
      if (Buffer.byteLength(buffer, "utf8") > MAX_FRAME_BYTES) {
        socket.destroy();
        return;
      }
      let newline;
      while ((newline = buffer.indexOf("\n")) >= 0) {
        const line = buffer.slice(0, newline).trim();
        buffer = buffer.slice(newline + 1);
        if (!line) continue;
        try {
          const message = JSON.parse(line);
          if (!authenticated) {
            if (message.type !== "snapshot") {
              throw new Error("runtime authentication failed");
            }
            authenticated = true;
            socket.setTimeout(0);
          }
          if (message.type === "snapshot") this.callbacks.onSnapshot(message.payload);
        } catch (error) {
          this.callbacks.onError(error);
        }
      }
    });
    socket.on("error", (error) => this.callbacks.onError(error));
    socket.on("close", () => {
      if (this.socket === socket) {
        this.socket = null;
        this.sessionId = null;
        this.locator = null;
        this.callbacks.onSnapshot(null);
      }
    });
  }

  close() {
    this.socket?.destroy();
    this.socket = null;
  }
}

module.exports = { RuntimeClient, locatorPath, workspaceId };
