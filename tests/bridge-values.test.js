import { test, expect } from "bun:test";
import { Vm } from "../index.js";

// ---------------------------------------------------------------------------
// The N-API value bridge: which values survive the round trip, in both
// directions. Each test drives a VM function through `callFunction`, which is
// the structured (non-string) boundary.
// ---------------------------------------------------------------------------

function vmWith(source) {
  const vm = new Vm();
  vm.run(source);
  return vm;
}

function produce(expression) {
  const vm = vmWith(`function __probe() { return (${expression}); }`);
  return vm.callFunction("__probe", []);
}

/// Like `produce`, but for a body that needs statements rather than one
/// expression.
function produceBody(body) {
  const vm = vmWith(`function __probe() { ${body} }`);
  return vm.callFunction("__probe", []);
}

function roundTrip(value) {
  const vm = vmWith("function __echo(x) { return x; }");
  return vm.callFunction("__echo", [value]);
}

// --- Outbound: VM values reaching the host ---------------------------------

test("a Date crosses as a Date", () => {
  const out = produce("new Date(0)");
  expect(out instanceof Date).toBe(true);
  expect(out.getTime()).toBe(0);
});

test("a BigInt crosses with full precision", () => {
  expect(produce("123456789012345678901234567890n")).toBe(123456789012345678901234567890n);
  expect(produce("-42n")).toBe(-42n);
});

test("a symbol crosses as a symbol", () => {
  const out = produce("Symbol('hi')");
  expect(typeof out).toBe("symbol");
  expect(out.description).toBe("hi");
});

test("a typed array crosses as the same view type", () => {
  const out = produce("new Uint8Array([1, 2, 3])");
  expect(out instanceof Uint8Array).toBe(true);
  expect(Array.from(out).join()).toBe("1,2,3");
});

test("an ArrayBuffer crosses with its bytes", () => {
  const out = produceBody("const b = new ArrayBuffer(4); new Uint8Array(b)[0] = 9; return b;");
  expect(out instanceof ArrayBuffer).toBe(true);
  expect(out.byteLength).toBe(4);
  expect(new Uint8Array(out)[0]).toBe(9);
});

test("a Map crosses as a Map", () => {
  const out = produce("new Map([[1, 'a'], [2, 'b']])");
  expect(out instanceof Map).toBe(true);
  expect(out.get(1)).toBe("a");
  expect(out.size).toBe(2);
});

test("a Set crosses as a Set", () => {
  const out = produce("new Set([1, 2, 2])");
  expect(out instanceof Set).toBe(true);
  expect(out.size).toBe(2);
  expect(out.has(1)).toBe(true);
});

test("a settled promise crosses as a settled promise", async () => {
  await expect(produce("Promise.resolve(42)")).resolves.toBe(42);
});

test("a rejected promise crosses as a rejection", async () => {
  const rejected = produce("Promise.reject(new Error('nope'))");
  await expect(rejected).rejects.toBeDefined();
});

test("an async function's result crosses settled", async () => {
  const vm = vmWith("async function __probe() { return await Promise.resolve(7); }");
  await expect(vm.callFunction("__probe", [])).resolves.toBe(7);
});

test("a cyclic object crosses intact", () => {
  const out = produceBody("const a = { n: 1 }; a.self = a; return a;");
  expect(out.n).toBe(1);
  expect(out.self).toBe(out);
});

test("a shared reference stays shared", () => {
  const out = produceBody("const x = { v: 1 }; return [x, x];");
  expect(out[0]).toBe(out[1]);
});

test("a proxy crosses as its target", () => {
  expect(produce("new Proxy({ a: 1 }, {})").a).toBe(1);
});

test("a regular expression crosses as its source and flags", () => {
  const out = produce("/ab+/gi");
  expect(out.source).toBe("ab+");
  expect(out.flags).toBe("gi");
});

test("internal slots do not cross", () => {
  const out = produceBody("const s = Symbol('k'); const o = { a: 1 }; o[s] = 2; return o;");
  expect(Object.keys(out).join()).toBe("a");
});

test("non-enumerable properties do not cross", () => {
  const out = produceBody(
    "const o = {}; Object.defineProperty(o, 'hidden', { value: 1 }); o.shown = 2; return o;",
  );
  expect(Object.keys(out).join()).toBe("shown");
});

// --- Inbound: host values reaching the VM ----------------------------------

test("a Date round-trips", () => {
  const out = roundTrip(new Date(5));
  expect(out instanceof Date).toBe(true);
  expect(out.getTime()).toBe(5);
});

test("a BigInt round-trips", () => {
  expect(roundTrip(42n)).toBe(42n);
  expect(roundTrip(-12345678901234567890n)).toBe(-12345678901234567890n);
});

test("a typed array round-trips", () => {
  expect(Array.from(roundTrip(new Uint8Array([7, 8]))).join()).toBe("7,8");
});

test("the VM sees a host BigInt as a bigint", () => {
  const vm = vmWith("function __kind(x) { return typeof x; }");
  expect(vm.callFunction("__kind", [1n])).toBe("bigint");
});

test("the VM computes with a host BigInt", () => {
  const vm = vmWith("function __double(x) { return (x * 2n).toString(); }");
  expect(vm.callFunction("__double", [9007199254740993n])).toBe("18014398509481986");
});

test("the VM reads a host Date", () => {
  const vm = vmWith("function __year(d) { return d.getFullYear(); }");
  expect(vm.callFunction("__year", [new Date("2024-03-01T00:00:00Z")])).toBe(2024);
});

test("the VM indexes a host typed array", () => {
  const vm = vmWith("function __sum(x) { return x[0] + x[1]; }");
  expect(vm.callFunction("__sum", [new Uint8Array([3, 4])])).toBe(7);
});

test("the VM reads a host ArrayBuffer", () => {
  const vm = vmWith("function __len(b) { return b.byteLength; }");
  expect(vm.callFunction("__len", [new ArrayBuffer(6)])).toBe(6);
});

test("a host symbol arrives as a symbol", () => {
  const vm = vmWith("function __kind(x) { return typeof x; }");
  expect(vm.callFunction("__kind", [Symbol("s")])).toBe("symbol");
});
