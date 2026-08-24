const crypto = require("node:crypto");
const fs = require("node:fs");
const net = require("node:net");
const os = require("node:os");
const path = require("node:path");
const { Vm } = require("../index.js");

const PROTOCOL_VERSION = 1;
const RUNTIME_DIR = ".napi-vm";
const RUNTIME_FILE = "runtime.json";
const MAX_SHAPE_DEPTH = 8;
const MAX_SHAPE_PROPERTIES = 256;
const MAX_REGISTERED_GLOBALS = 256;
const MAX_SHAPE_PARAMETERS = 64;
const MAX_METADATA_NAME = 128;
const MAX_DOCUMENTATION_BYTES = 16 * 1024;
const MAX_SCHEMA_NODES = 4096;
const MAX_LAST_VALUE_DEPTH = 6;
const MAX_LAST_VALUE_PROPERTIES = 64;
const MAX_LAST_VALUE_STRING = 512;
const LAST_VALUE_PUBLISH_DELAY = 150;
const MAX_FRAME_BYTES = 1024 * 1024;
const MAX_CLIENTS = 16;
const MAX_MODULE_SOURCE_BYTES = 256 * 1024;
const AUTH_TIMEOUT_MS = 5000;

function tokenMatches(left, right) {
  if (typeof left !== "string" || typeof right !== "string") return false;
  const a = Buffer.from(left, "utf8");
  const b = Buffer.from(right, "utf8");
  return a.length === b.length && crypto.timingSafeEqual(a, b);
}

function workspaceId(workspace) {
  const resolved = fs.realpathSync(workspace);
  return crypto.createHash("sha256").update(resolved).digest("hex").slice(0, 20);
}

function runtimePath(workspace) {
  return path.join(workspace, RUNTIME_DIR, RUNTIME_FILE);
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function dataObject() {
  return Object.create(null);
}

function metadataError(message) {
  throw new Error(`Invalid language-service metadata: ${message}`);
}

function validateMetadataName(name, kind) {
  if (typeof name !== "string" || name.length === 0 || name.length > MAX_METADATA_NAME || /[\u0000-\u001f\u007f]/u.test(name)) {
    metadataError(`invalid ${kind} name`);
  }
}

function validateDocumentation(documentation, kind) {
  if (documentation !== undefined && documentation !== null && typeof documentation !== "string") {
    metadataError(`${kind} documentation must be a string`);
  }
  if (typeof documentation === "string" && Buffer.byteLength(documentation, "utf8") > MAX_DOCUMENTATION_BYTES) {
    metadataError(`${kind} documentation exceeds the maximum length`);
  }
}

function validateLegacyShape(value, depth, state) {
  if (typeof value !== "string") metadataError("shape must be an object or supported type string");
  const trimmed = value.trim();
  if (trimmed.endsWith("[]")) return validateLegacyShape(trimmed.slice(0, -2), depth + 1, state);
  if (trimmed.startsWith("Promise<") && trimmed.endsWith(">")) {
    return validateLegacyShape(trimmed.slice(8, -1), depth + 1, state);
  }
  if (!["unknown", "any", "void", "undefined", "null", "boolean", "number", "string", "object", "function"].includes(trimmed)) {
    metadataError(`unsupported legacy shape string: ${trimmed}`);
  }
}

function validateShape(shape, depth = 0, state = { nodes: 0 }) {
  if (depth > MAX_SHAPE_DEPTH) metadataError(`shape exceeds maximum depth of ${MAX_SHAPE_DEPTH}`);
  state.nodes += 1;
  if (state.nodes > MAX_SCHEMA_NODES) metadataError(`shape exceeds maximum node count of ${MAX_SCHEMA_NODES}`);
  if (typeof shape === "string") return validateLegacyShape(shape, depth, state);
  if (!shape || typeof shape !== "object" || Array.isArray(shape)) metadataError("shape must be an object");

  const kinds = ["unknown", "any", "void", "undefined", "null", "boolean", "number", "string", "array", "promise", "object", "function"];
  if (typeof shape.kind !== "string" || !kinds.includes(shape.kind)) metadataError(`unsupported shape kind: ${String(shape.kind)}`);
  validateDocumentation(shape.documentation, "shape");

  if (shape.kind === "object") {
    if (shape.properties !== undefined && (!shape.properties || typeof shape.properties !== "object" || Array.isArray(shape.properties))) {
      metadataError("object properties must be an object");
    }
    const properties = Object.keys(shape.properties || {});
    if (properties.length > MAX_SHAPE_PROPERTIES) metadataError(`object has too many properties (maximum ${MAX_SHAPE_PROPERTIES})`);
    for (const name of properties) {
      validateMetadataName(name, "property");
      validateShape(shape.properties[name], depth + 1, state);
    }
    if (shape.params !== undefined || shape.returns !== undefined || shape.items !== undefined || shape.value !== undefined) {
      metadataError("object shape contains invalid fields");
    }
    return;
  }

  if (shape.kind === "function") {
    if (shape.params !== undefined && !Array.isArray(shape.params)) metadataError("function params must be an array");
    const params = shape.params || [];
    if (params.length > MAX_SHAPE_PARAMETERS) metadataError(`function has too many parameters (maximum ${MAX_SHAPE_PARAMETERS})`);
    for (const parameter of params) {
      if (!parameter || typeof parameter !== "object" || Array.isArray(parameter)) metadataError("function parameter must be an object");
      validateMetadataName(parameter.name, "parameter");
      const parameterShape = parameter.type ?? parameter.shape ?? parameter.typeName ?? { kind: "unknown" };
      validateShape(parameterShape, depth + 1, state);
    }
    if (shape.returns !== undefined) validateShape(shape.returns, depth + 1, state);
    if (shape.items !== undefined || shape.value !== undefined || shape.properties !== undefined) metadataError("function shape contains invalid fields");
    if (shape.async !== undefined && typeof shape.async !== "boolean") metadataError("function async flag must be boolean");
    return;
  }

  if (shape.kind === "array" || shape.kind === "promise") {
    if (shape.items !== undefined && shape.value !== undefined) metadataError(`${shape.kind} shape has duplicate item fields`);
    const itemShape = shape.items ?? shape.value;
    if (itemShape !== undefined) validateShape(itemShape, depth + 1, state);
    if (shape.properties !== undefined || shape.params !== undefined || shape.returns !== undefined) metadataError(`${shape.kind} shape contains invalid fields`);
  }
}

function validateGlobal(name, shape, documentation) {
  validateMetadataName(name, "global");
  validateDocumentation(documentation, "global");
  const state = { nodes: 0 };
  validateShape(shape, 0, state);
  return state.nodes;
}

function inferJsonShape(value, depth = 0) {
  if (depth >= MAX_SHAPE_DEPTH) return { kind: "unknown" };
  if (value === null) return { kind: "null" };
  if (Array.isArray(value)) {
    let items = { kind: "unknown" };
    for (const item of value.slice(0, MAX_SHAPE_PROPERTIES)) {
      items = mergeShapes(items, inferJsonShape(item, depth + 1));
    }
    return { kind: "array", items };
  }
  switch (typeof value) {
    case "string": return { kind: "string" };
    case "number": return { kind: "number" };
    case "boolean": return { kind: "boolean" };
    case "undefined": return { kind: "undefined" };
    case "object": {
      const properties = dataObject();
      for (const name of Object.keys(value).sort().slice(0, MAX_SHAPE_PROPERTIES)) {
        properties[name] = inferJsonShape(value[name], depth + 1);
      }
      return { kind: "object", properties };
    }
    default: return { kind: "unknown" };
  }
}

function mergeShapes(left, right) {
  if (!left || left.kind === "unknown") return clone(right);
  if (!right || right.kind === "unknown") return clone(left);
  if (left.kind !== right.kind) return { kind: "unknown" };

  if (left.kind === "object") {
    const properties = dataObject();
    const names = new Set([
      ...Object.keys(left.properties || {}),
      ...Object.keys(right.properties || {}),
    ]);
    for (const name of [...names].sort().slice(0, MAX_SHAPE_PROPERTIES)) {
      if (left.properties?.[name] && right.properties?.[name]) {
        properties[name] = mergeShapes(left.properties[name], right.properties[name]);
      } else {
        properties[name] = { kind: "unknown" };
      }
    }
    return { kind: "object", properties };
  }
  if (left.kind === "array") {
    return { kind: "array", items: mergeShapes(left.items, right.items) };
  }
  return clone(left);
}

function snapshotJsonValue(value, depth = 0) {
  if (depth >= MAX_LAST_VALUE_DEPTH) return "[depth truncated]";
  if (typeof value === "string") {
    return value.length > MAX_LAST_VALUE_STRING
      ? `${value.slice(0, MAX_LAST_VALUE_STRING)}…`
      : value;
  }
  if (Array.isArray(value)) {
    const items = value.slice(0, MAX_LAST_VALUE_PROPERTIES)
      .map((item) => snapshotJsonValue(item, depth + 1));
    if (value.length > items.length) items.push(`[${value.length - items.length} more items]`);
    return items;
  }
  if (value && typeof value === "object") {
    const result = dataObject();
    const names = Object.keys(value).sort();
    for (const name of names.slice(0, MAX_LAST_VALUE_PROPERTIES)) {
      result[name] = snapshotJsonValue(value[name], depth + 1);
    }
    if (names.length > MAX_LAST_VALUE_PROPERTIES) {
      result.__truncated = `${names.length - MAX_LAST_VALUE_PROPERTIES} more properties`;
    }
    return result;
  }
  return value;
}

/**
 * Owns the live VM metadata channel used by the LSP. The runtime.json file is
 * only a locator; module sources and host metadata are sent over the local
 * socket/named pipe and are never committed to the project.
 */
class VmSession {
  constructor(options = {}) {
    this.workspace = fs.realpathSync(options.workspace || process.cwd());
    this.sessionId = options.sessionId || crypto.randomUUID();
    this.id = workspaceId(this.workspace);
    this.runtimeFile = runtimePath(this.workspace);
    this.authToken = crypto.randomBytes(32).toString("hex");
    this.vm = options.vm || null;
    this.server = null;
    this.address = null;
    this.clients = new Set();
    this.authenticatedClients = new Set();
    this.hostFunctions = new Map();
    this.globals = new Map();
    this.globalSchemaNodes = new Map();
    this.totalGlobalSchemaNodes = 0;
    this.modules = new Map();
    this.handlerShapes = new Map();
    this.lastValues = new Map();
    this.lastValueTimers = new Map();
    this.generation = 0;
    this.startedAt = new Date().toISOString();
    this.stopped = false;

    process.once("exit", () => this.cleanup());
  }

  attach(vm, options = {}) {
    this.vm = vm;
    this.hostFunctions.clear();
    this.globals.clear();
    this.globalSchemaNodes.clear();
    this.totalGlobalSchemaNodes = 0;
    this.modules.clear();
    this.handlerShapes.clear();
    this.lastValues.clear();
    this.clearLastValueTimers();
    for (const module of options.modules || []) {
      this.modules.set(module.name, {
        name: module.name,
        source: module.source,
      });
    }
    this.publish("replace");
    return vm;
  }

  detach() {
    this.vm = null;
    this.hostFunctions.clear();
    this.globals.clear();
    this.globalSchemaNodes.clear();
    this.totalGlobalSchemaNodes = 0;
    this.modules.clear();
    this.handlerShapes.clear();
    this.lastValues.clear();
    this.clearLastValueTimers();
    this.publish("replace");
  }

  exposeFunction(name, fn, info = {}) {
    const functionShape = {
      kind: "function",
      params: info.params || [],
      returns: info.returns || "unknown",
      async: Boolean(info.async),
      documentation: info.documentation,
    };
    validateGlobal(name, functionShape, info.documentation);
    this.requireVm().exposeFunction(name, fn);
    if (info.languageService === false || info.public === false) return;
    this.hostFunctions.set(name, {
      name,
      params: info.params || [],
      returns: info.returns || "unknown",
      documentation: info.documentation,
      async: Boolean(info.async),
    });
    this.publish("function");
  }

  exposeAsyncFunction(name, fn, info = {}) {
    const functionShape = {
      kind: "function",
      params: info.params || [],
      returns: info.returns || "unknown",
      async: true,
      documentation: info.documentation,
    };
    validateGlobal(name, functionShape, info.documentation);
    this.requireVm().exposeAsyncFunction(name, fn);
    if (info.languageService === false || info.public === false) return;
    this.hostFunctions.set(name, {
      name,
      params: info.params || [],
      returns: info.returns || "unknown",
      documentation: info.documentation,
      async: true,
    });
    this.publish("function");
  }

  removeGlobal(name) {
    const removed = this.requireVm().removeGlobal(name);
    const hostMetadataRemoved = this.hostFunctions.delete(name);
    const globalMetadataRemoved = this.globals.delete(name);
    if (globalMetadataRemoved) {
      this.totalGlobalSchemaNodes -= this.globalSchemaNodes.get(name) || 0;
      this.globalSchemaNodes.delete(name);
    }
    const metadataRemoved = hostMetadataRemoved || globalMetadataRemoved;
    if (removed || metadataRemoved) {
      this.publish("function-remove");
    }
    return removed || metadataRemoved;
  }

  registerGlobal(name, shape, options = {}) {
    if (!options || typeof options !== "object" || Array.isArray(options)) options = {};
    const nodeCount = validateGlobal(name, shape, options.documentation ?? shape?.documentation);
    if (!this.globals.has(name) && this.globals.size >= MAX_REGISTERED_GLOBALS) {
      metadataError(`too many global declarations (maximum ${MAX_REGISTERED_GLOBALS})`);
    }
    const previousNodes = this.globalSchemaNodes.get(name) || 0;
    if (this.totalGlobalSchemaNodes - previousNodes + nodeCount > MAX_SCHEMA_NODES) {
      metadataError(`global metadata exceeds the maximum node count of ${MAX_SCHEMA_NODES}`);
    }
    let storedShape;
    try {
      storedShape = clone(shape);
    } catch (error) {
      metadataError(`shape is not JSON-serializable: ${error.message || String(error)}`);
    }
    this.globals.set(name, {
      name,
      shape: storedShape,
      ...(options.documentation !== undefined ? { documentation: options.documentation } : {}),
    });
    this.globalSchemaNodes.set(name, nodeCount);
    this.totalGlobalSchemaNodes = this.totalGlobalSchemaNodes - previousNodes + nodeCount;
    this.publish("global");
  }

  registerModule(name, source) {
    if (Buffer.byteLength(source, "utf8") > MAX_MODULE_SOURCE_BYTES) {
      throw new Error("runtime module source exceeds the maximum size");
    }
    this.requireVm().registerModule(name, source);
    this.modules.set(name, { name, source });
    this.publish("module");
  }

  removeModule(name) {
    const removed = this.requireVm().removeModule(name);
    if (removed) {
      this.modules.delete(name);
      this.publish("module-remove");
    }
    return removed;
  }

  /**
   * Save the observed JSON shape and a bounded snapshot of the latest value
   * delivered to a VM event handler. Shapes are merged so fields from
   * different payload variants remain available to editor completion.
   */
  observeHandler(name, value) {
    if (!name) return false;
    const next = inferJsonShape(value);
    const previous = this.handlerShapes.get(name);
    const merged = previous ? mergeShapes(previous, next) : next;
    const shapeChanged = JSON.stringify(previous) !== JSON.stringify(merged);
    const lastValue = snapshotJsonValue(value);
    const previousValue = this.lastValues.get(name);
    const valueChanged = JSON.stringify(previousValue) !== JSON.stringify(lastValue);
    this.handlerShapes.set(name, merged);
    this.lastValues.set(name, lastValue);

    if (shapeChanged) {
      this.publish("shape");
    } else if (valueChanged && this.server && !this.lastValueTimers.has(name)) {
      const timer = setTimeout(() => {
        this.lastValueTimers.delete(name);
        this.publish("value");
      }, LAST_VALUE_PUBLISH_DELAY);
      this.lastValueTimers.set(name, timer);
    }
    return shapeChanged || valueChanged;
  }

  run(source) {
    return this.requireVm().run(source);
  }

  runAsync(source) {
    return this.requireVm().runAsync(source);
  }

  start() {
    if (this.server) return this;
    this.stopped = false;
    const endpoint = process.platform === "win32"
      ? `\\\\.\\pipe\\napi-vm-${this.id}-${process.pid}-${this.sessionId}`
      : path.join(os.tmpdir(), `napi-vm-${this.id}-${process.pid}-${this.sessionId}.sock`);
    this.address = endpoint;
    this.server = net.createServer((socket) => this.accept(socket));
    this.server.listen(endpoint, () => {
      if (process.platform !== "win32") {
        try { fs.chmodSync(endpoint, 0o600); } catch {}
      }
      this.writeLocator();
    });
    return this;
  }

  stop() {
    if (this.stopped) return;
    this.stopped = true;
    for (const socket of this.clients) socket.destroy();
    this.clients.clear();
    this.authenticatedClients.clear();
    this.server?.close();
    this.server = null;
    this.clearLastValueTimers();
    this.cleanup();
  }

  snapshot() {
    return {
      protocolVersion: PROTOCOL_VERSION,
      sessionId: this.sessionId,
      workspaceId: this.id,
      pid: process.pid,
      generation: this.generation,
      startedAt: this.startedAt,
      functions: [...this.hostFunctions.values()].map(clone),
      globals: [...this.globals.values()].map(clone),
      modules: [...this.modules.values()].map(clone),
      handlers: [...this.handlerShapes.entries()].map(([name, shape]) => ({
        name,
        shape: clone(shape),
        lastValue: clone(this.lastValues.get(name)),
      })),
    };
  }

  requireVm() {
    if (!this.vm) throw new Error("VmSession has no attached VM");
    return this.vm;
  }

  clearLastValueTimers() {
    for (const timer of this.lastValueTimers.values()) clearTimeout(timer);
    this.lastValueTimers.clear();
  }

  accept(socket) {
    if (this.clients.size >= MAX_CLIENTS) {
      socket.destroy();
      return;
    }
    this.clients.add(socket);
    let authenticated = false;
    let buffer = "";
    socket.setEncoding("utf8");
    // Use an absolute deadline rather than socket inactivity. Otherwise an
    // unauthenticated peer can trickle bytes often enough to occupy a client
    // slot indefinitely.
    const authTimer = setTimeout(() => socket.destroy(), AUTH_TIMEOUT_MS);
    authTimer.unref?.();
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
            if (message.type !== "auth" || !tokenMatches(message.token, this.authToken)) {
              socket.destroy();
              return;
            }
            authenticated = true;
            this.authenticatedClients.add(socket);
            clearTimeout(authTimer);
            this.send(socket, "snapshot");
            continue;
          }
          this.handleMessage(socket, message);
        } catch (error) {
          this.sendError(socket, {
            type: "error",
            message: error.message || String(error),
          });
        }
      }
    });
    const removeClient = () => {
      clearTimeout(authTimer);
      this.clients.delete(socket);
      this.authenticatedClients.delete(socket);
    };
    socket.on("close", removeClient);
    socket.on("error", removeClient);
  }

  handleMessage(socket, message) {
    if (message.type === "ping") this.send(socket, "pong");
    if (message.type === "snapshot") this.send(socket, "snapshot");
  }

  publish(reason) {
    if (!this.server || !this.server.listening) return;
    this.generation += 1;
    this.writeLocator();
    for (const socket of this.authenticatedClients) this.send(socket, "snapshot", reason);
  }

  send(socket, type, reason) {
    const encoded = JSON.stringify({
      type,
      reason,
      payload: this.snapshot(),
    });
    if (Buffer.byteLength(encoded, "utf8") > MAX_FRAME_BYTES) {
      socket.destroy();
      return false;
    }
    socket.write(`${encoded}\n`);
    return true;
  }

  sendError(socket, payload) {
    const encoded = JSON.stringify(payload);
    if (Buffer.byteLength(encoded, "utf8") > MAX_FRAME_BYTES) {
      socket.destroy();
      return;
    }
    socket.write(`${encoded}\n`);
  }

  writeLocator() {
    const directory = path.dirname(this.runtimeFile);
    fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
    if (process.platform !== "win32") {
      try { fs.chmodSync(directory, 0o700); } catch {}
    }
    const locator = {
      protocolVersion: PROTOCOL_VERSION,
      workspaceId: this.id,
      sessionId: this.sessionId,
      pid: process.pid,
      authToken: this.authToken,
      startedAt: this.startedAt,
      generation: this.generation,
      transport: {
        kind: process.platform === "win32" ? "named-pipe" : "unix",
        address: this.address,
      },
    };
    const temporary = `${this.runtimeFile}.${process.pid}.${this.sessionId}.tmp`;
    fs.writeFileSync(temporary, `${JSON.stringify(locator, null, 2)}\n`, {
      encoding: "utf8",
      mode: 0o600,
    });
    if (process.platform !== "win32") {
      try { fs.chmodSync(temporary, 0o600); } catch {}
    }
    fs.renameSync(temporary, this.runtimeFile);
  }

  cleanup() {
    try {
      const current = JSON.parse(fs.readFileSync(this.runtimeFile, "utf8"));
      if (current.sessionId === this.sessionId) fs.unlinkSync(this.runtimeFile);
    } catch {}
    if (process.platform !== "win32" && this.address) {
      try { fs.unlinkSync(this.address); } catch {}
    }
  }
}

module.exports = {
  PROTOCOL_VERSION,
  RUNTIME_DIR,
  RUNTIME_FILE,
  VmSession,
  runtimePath,
};
