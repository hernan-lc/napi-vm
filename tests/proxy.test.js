import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// `Proxy` and the `Function` constructor.
//
// A handler with no trap for an operation is transparent: the operation falls
// through to the target.
// ---------------------------------------------------------------------------

test("an empty handler is transparent", () => {
  expect(runCode("new Proxy({ a: 1 }, {}).a;")).toBe("1");
  expect(runCode("const p = new Proxy({ a: 1 }, {}); 'a' in p;")).toBe("true");
  expect(runCode("Object.keys(new Proxy({ a: 1, b: 2 }, {})).join();")).toBe("a,b");
});

test("the get trap intercepts reads", () => {
  expect(runCode("new Proxy({}, { get: (t, k) => k + '!' }).x;")).toBe("x!");
});

test("the get trap receives the target and the key", () => {
  expect(
    runCode("new Proxy({ a: 7 }, { get: (t, k) => t[k] * 2 }).a;"),
  ).toBe("14");
});

test("the set trap intercepts writes", () => {
  expect(
    runCode("const p = new Proxy({}, { set: (t, k, v) => { t[k] = v * 2; return true; } }); p.a = 5; p.a;"),
  ).toBe("10");
});

test("the has trap answers `in`", () => {
  expect(runCode("'z' in new Proxy({}, { has: () => true });")).toBe("true");
  expect(runCode("'a' in new Proxy({ a: 1 }, { has: () => false });")).toBe("false");
});

test("the deleteProperty trap intercepts delete", () => {
  expect(
    runCode("const p = new Proxy({ a: 1 }, { deleteProperty: (t, k) => { delete t[k]; return true; } }); delete p.a; JSON.stringify(p);"),
  ).toBe("{}");
});

test("the ownKeys trap answers Object.keys", () => {
  expect(runCode("Object.keys(new Proxy({ a: 1 }, { ownKeys: () => ['x'] })).join();")).toBe("x");
});

test("the apply trap intercepts calls", () => {
  expect(
    runCode("const f = new Proxy((a) => a + 1, { apply: (t, self, args) => t(...args) * 10 }); f(1);"),
  ).toBe("20");
});

test("a proxied function without an apply trap still calls", () => {
  expect(runCode("new Proxy((a) => a + 1, {})(1);")).toBe("2");
});

test("the construct trap intercepts new", () => {
  expect(
    runCode("class A { constructor(x) { this.x = x; } } const P = new Proxy(A, { construct: (t, args) => ({ x: args[0] * 2 }) }); new P(3).x;"),
  ).toBe("6");
});

test("a proxied class without a construct trap still constructs", () => {
  expect(
    runCode("class A { constructor(x) { this.x = x; } } new (new Proxy(A, {}))(3).x;"),
  ).toBe("3");
});

test("typeof reports what the proxy wraps", () => {
  expect(runCode("typeof new Proxy({}, {});")).toBe("object");
  expect(runCode("typeof new Proxy(function () {}, {});")).toBe("function");
});

test("a non-object target is rejected", () => {
  expect(() => runCode("new Proxy(1, {});")).toThrow();
});

test("a non-object handler is rejected", () => {
  expect(() => runCode("new Proxy({}, 1);")).toThrow();
});

test("a proxy serializes as its target", () => {
  expect(runCode("JSON.stringify(new Proxy({ a: 1 }, {}));")).toBe('{"a":1}');
});

// --- The Function constructor ----------------------------------------------

test("Function compiles a body", () => {
  expect(runCode("new Function('a', 'b', 'return a + b;')(1, 2);")).toBe("3");
});

test("Function works without new", () => {
  expect(runCode("Function('return 42;')();")).toBe("42");
});

test("one argument may list several parameters", () => {
  expect(runCode("new Function('a, b', 'return a * b;')(3, 4);")).toBe("12");
});

test("a body with no parameters", () => {
  expect(runCode("new Function('return 1;')();")).toBe("1");
});

test("a syntax error in the body throws", () => {
  expect(() => runCode("new Function('return (;');")).toThrow();
});
