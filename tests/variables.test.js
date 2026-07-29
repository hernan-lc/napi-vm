import { test, expect } from "bun:test";
import { runCode } from "../index.js";

test("const declaration", () => {
  expect(runCode("const x = 42; x;")).toBe("42");
  expect(runCode("const name = 'Alice'; name;")).toBe("Alice");
  expect(runCode("const flag = true; flag;")).toBe("true");
});

test("let declaration", () => {
  expect(runCode("let x = 1; x;")).toBe("1");
  expect(runCode("let x = 1; x = 2; x;")).toBe("2");
});

test("var declaration", () => {
  expect(runCode("var x = 1; x;")).toBe("1");
  expect(runCode("var x = 1; x = 3; x;")).toBe("3");
});

test("uninitialized declaration", () => {
  expect(runCode("let x; x;")).toBe("undefined");
  expect(runCode("var x; x;")).toBe("undefined");
});

test("multiple declarations", () => {
  expect(runCode("const a = 1; const b = 2; const c = 3; a + b + c;")).toBe("6");
});

test("reassignment", () => {
  expect(runCode("let x = 10; x = 20; x;")).toBe("20");
  expect(runCode("let x = 'a'; x = 'b'; x;")).toBe("b");
});

test("compound assignment +=", () => {
  expect(runCode("let x = 5; x += 3; x;")).toBe("8");
  expect(runCode("let s = 'hello'; s += ' world'; s;")).toBe("hello world");
});

test("compound assignment -=", () => {
  expect(runCode("let x = 10; x -= 4; x;")).toBe("6");
});

test("compound assignment *=", () => {
  expect(runCode("let x = 3; x *= 2; x;")).toBe("6");
});

test("compound assignment /=", () => {
  expect(runCode("let x = 10; x /= 2; x;")).toBe("5");
});

test("chained compound assignment", () => {
  expect(runCode("let x = 1; x += 1; x += 1; x += 1; x;")).toBe("4");
});

test("assignment expression returns value", () => {
  expect(runCode("let x = 5;")).toBe("5");
  expect(runCode("let x = 0; x = 42;")).toBe("42");
});

test("variable shadowing in functions", () => {
  expect(runCode("const x = 1; function f() { const x = 2; return x; } f();")).toBe("2");
  expect(runCode("const x = 1; function f() { const x = 2; return x; } x;")).toBe("1");
});

test("global variable accessible in functions", () => {
  expect(runCode("const x = 10; function f() { return x; } f();")).toBe("10");
});

test("assignment to global from function", () => {
  expect(runCode("let x = 1; function f() { x = 2; } f(); x;")).toBe("2");
});
