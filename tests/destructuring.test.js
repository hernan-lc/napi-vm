import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// Destructuring in all three positions: declarations, assignments, and
// parameters.
// ---------------------------------------------------------------------------

// --- Declarations -----------------------------------------------------------

test("array and object declarations bind", () => {
  expect(runCode("const [a, b] = [1, 2]; a + b;")).toBe("3");
  expect(runCode("const { p } = { p: 1 }; p;")).toBe("1");
});

test("declarations take defaults", () => {
  expect(runCode("const { a = 3 } = {}; a;")).toBe("3");
  expect(runCode("const [x = 5] = []; x;")).toBe("5");
});

test("declarations nest", () => {
  expect(runCode("const { a: { b } } = { a: { b: 4 } }; b;")).toBe("4");
});

test("a rest element collects the tail", () => {
  expect(runCode("const [a, ...rest] = [1, 2, 3]; a + ':' + rest.join();")).toBe("1:2,3");
  expect(runCode("const { a, ...rest } = { a: 1, b: 2 }; JSON.stringify(rest);")).toBe('{"b":2}');
});

// --- Assignments ------------------------------------------------------------

test("array destructuring assignment swaps", () => {
  expect(runCode("let a = 1, b = 2; [a, b] = [b, a]; a + ',' + b;")).toBe("2,1");
});

test("object destructuring assignment needs parentheses", () => {
  expect(runCode("let x, y; ({ x, y } = { x: 1, y: 2 }); x + y;")).toBe("3");
});

test("an assignment reaches an outer binding", () => {
  expect(
    runCode("let a = 1, b = 2; function f() { [a, b] = [9, 8]; } f(); a + ',' + b;"),
  ).toBe("9,8");
});

test("a property can be a target", () => {
  expect(runCode("const o = {}; [o.p] = [5]; o.p;")).toBe("5");
  expect(runCode("const o = { a: {} }; ({ b: o.a.c } = { b: 3 }); o.a.c;")).toBe("3");
});

test("an assignment takes defaults", () => {
  expect(runCode("let a = 0; [a = 7] = []; a;")).toBe("7");
});

test("a missing property assigns undefined", () => {
  expect(runCode("let a, b; ({ a, b } = { a: 1 }); String(b);")).toBe("undefined");
});

test("a rest element works in an assignment", () => {
  expect(runCode("let a, b; [a, ...b] = [1, 2, 3]; a + ':' + b.join();")).toBe("1:2,3");
});

// --- Parameters -------------------------------------------------------------

test("an object parameter destructures", () => {
  expect(runCode("function f({ a, b }) { return a + b; } f({ a: 1, b: 2 });")).toBe("3");
});

test("an array parameter destructures", () => {
  expect(runCode("function f([a, b]) { return a + b; } f([1, 2]);")).toBe("3");
});

test("a destructured parameter takes a whole-parameter default", () => {
  expect(runCode("function f({ a = 5 } = {}) { return a; } f();")).toBe("5");
});

test("several destructured parameters", () => {
  expect(runCode("function f({ a }, { b }) { return a + b; } f({ a: 1 }, { b: 2 });")).toBe("3");
});

test("a destructured parameter mixes with a plain one", () => {
  expect(runCode("function f(x, { y }) { return x + y; } f(1, { y: 2 });")).toBe("3");
});

test("destructured parameters nest", () => {
  expect(runCode("function f({ a: { b } }) { return b; } f({ a: { b: 4 } });")).toBe("4");
});

test("an arrow takes a destructured parameter", () => {
  expect(runCode("const g = ({ a }) => a; g({ a: 7 });")).toBe("7");
  expect(runCode("const g = ([a, b]) => a + b; g([1, 2]);")).toBe("3");
  expect(runCode("const g = ({ a } = { a: 9 }) => a; g();")).toBe("9");
});
