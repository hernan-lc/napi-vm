import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// Standard-library methods and semantics that were missing under features the
// roadmap already listed as working.
// ---------------------------------------------------------------------------

// --- Property access on nullish values --------------------------------------

test("reading a property of null is a TypeError", () => {
  expect(() => runCode("null.x;")).toThrow("Cannot read properties of null");
  expect(() => runCode("undefined.x;")).toThrow("Cannot read properties of undefined");
});

test("a nested miss reports where it failed", () => {
  expect(() => runCode("({}).a.b;")).toThrow("TypeError");
});

test("optional chaining still short-circuits", () => {
  expect(runCode("const o = null; String(o?.a);")).toBe("undefined");
  expect(runCode("let u; String(u?.a?.b);")).toBe("undefined");
});

// --- super ------------------------------------------------------------------

test("super.method calls the superclass implementation", () => {
  expect(
    runCode("class A { m() { return 1; } } class B extends A { m() { return super.m() + 1; } } new B().m();"),
  ).toBe("2");
});

test("super works from a differently named method", () => {
  expect(
    runCode("class A { m() { return 1; } } class B extends A { n() { return super.m(); } } new B().n();"),
  ).toBe("1");
});

test("super reaches a superclass getter", () => {
  expect(
    runCode("class A { get v() { return 2; } } class B extends A { get v() { return super.v * 2; } } new B().v;"),
  ).toBe("4");
});

test("super() still runs the superclass constructor", () => {
  expect(
    runCode("class A { constructor() { this.x = 1; } } class B extends A { constructor() { super(); this.y = 2; } } const b = new B(); b.x + b.y;"),
  ).toBe("3");
});

// --- Destructuring defaults and rest ----------------------------------------

test("an object pattern supplies a default", () => {
  expect(runCode("const { a = 3 } = {}; a;")).toBe("3");
  expect(runCode("const { a = 3 } = { a: 1 }; a;")).toBe("1");
});

test("a renamed property takes a default", () => {
  expect(runCode("const { a: b = 3 } = {}; b;")).toBe("3");
});

test("an object rest pattern collects what is left", () => {
  expect(runCode("const { a, ...rest } = { a: 1, b: 2, c: 3 }; JSON.stringify(rest);")).toBe(
    '{"b":2,"c":3}',
  );
});

test("defaults nest", () => {
  expect(runCode("const { a: { b = 7 } = {} } = {}; b;")).toBe("7");
});

test("a destructured parameter takes defaults", () => {
  expect(runCode("function f({ a = 1, b = 2 } = {}) { return a + b; } f();")).toBe("3");
});

// --- Computed method names --------------------------------------------------

test("a computed method name evaluates its key", () => {
  expect(runCode("const k = 'm'; const o = { [k]() { return 1; } }; o.m();")).toBe("1");
  expect(runCode("const k = 'm'; const o = { [k]() { return 1; } }; String(o.k);")).toBe(
    "undefined",
  );
});

test("a literal computed method name still works", () => {
  expect(runCode("({ ['lit']() { return 2; } }).lit();")).toBe("2");
});

// --- Array.prototype.splice -------------------------------------------------

test("splice returns what it removed", () => {
  expect(runCode("[1, 2, 3].splice(1, 1).join();")).toBe("2");
});

test("splice removes in place", () => {
  expect(runCode("const a = [1, 2, 3]; a.splice(1, 1); a.join();")).toBe("1,3");
});

test("splice inserts", () => {
  expect(runCode("const a = [1, 4]; a.splice(1, 0, 2, 3); a.join();")).toBe("1,2,3,4");
});

test("splice without a count removes the tail", () => {
  expect(runCode("const a = [1, 2, 3]; a.splice(1); a.join();")).toBe("1");
});

test("a negative start counts from the end", () => {
  expect(runCode("const a = [1, 2, 3]; a.splice(-1, 1); a.join();")).toBe("1,2");
});

// --- String methods ---------------------------------------------------------

test("padStart and padEnd", () => {
  expect(runCode("'5'.padStart(3, '0');")).toBe("005");
  expect(runCode("'5'.padEnd(3, '-');")).toBe("5--");
  expect(runCode("'abc'.padStart(2, '0');")).toBe("abc");
});

test("trimStart and trimEnd", () => {
  expect(runCode("'  a '.trimStart() + '|';")).toBe("a |");
  expect(runCode("'  a '.trimEnd() + '|';")).toBe("  a|");
});

test("at, lastIndexOf, concat and localeCompare", () => {
  expect(runCode("'abc'.at(-1);")).toBe("c");
  expect(runCode("String('abc'.at(9));")).toBe("undefined");
  expect(runCode("'abcb'.lastIndexOf('b');")).toBe("3");
  expect(runCode("'a'.concat('b', 'c');")).toBe("abc");
  expect(runCode("'a'.localeCompare('b');")).toBe("-1");
});

test("codePointAt", () => {
  expect(runCode("'A'.codePointAt(0);")).toBe("65");
});

// --- Number methods ---------------------------------------------------------

test("toString takes a radix", () => {
  expect(runCode("(255).toString(16);")).toBe("ff");
  expect(runCode("(10).toString(2);")).toBe("1010");
  expect(runCode("(255).toString();")).toBe("255");
});

test("an out-of-range radix throws", () => {
  expect(() => runCode("(255).toString(99);")).toThrow("RangeError");
});

test("toPrecision reports significant figures", () => {
  expect(runCode("(3.14159).toPrecision(3);")).toBe("3.14");
  expect(runCode("(0.000123).toPrecision(2);")).toBe("0.00012");
});

// --- JSON.stringify indentation ---------------------------------------------

test("a numeric indent formats the output", () => {
  expect(runCode("JSON.stringify({ a: 1 }, null, 2);")).toBe('{\n  "a": 1\n}');
});

test("nested values indent progressively", () => {
  expect(runCode("JSON.stringify({ a: [1, 2] }, null, 2);")).toBe(
    '{\n  "a": [\n    1,\n    2\n  ]\n}',
  );
});

test("empty collections stay on one line", () => {
  expect(runCode("JSON.stringify({}, null, 2);")).toBe("{}");
  expect(runCode("JSON.stringify([], null, 2);")).toBe("[]");
});

test("a string indent is used verbatim", () => {
  expect(runCode("JSON.stringify({ a: 1 }, null, '\\t');")).toBe('{\n\t"a": 1\n}');
});

test("no indent keeps the compact form", () => {
  expect(runCode("JSON.stringify({ a: 1 });")).toBe('{"a":1}');
});

test("a string containing braces is not re-indented", () => {
  expect(runCode("JSON.stringify({ a: '{x}' }, null, 2);")).toBe('{\n  "a": "{x}"\n}');
});
