import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// `&&=`, `||=` and `??=`. These short-circuit: the right side is evaluated,
// and the write performed, only when the current value calls for it.
// ---------------------------------------------------------------------------

test("||= assigns to a falsy binding", () => {
  expect(runCode("let a = 0; a ||= 5; a;")).toBe("5");
});

test("||= leaves a truthy binding alone", () => {
  expect(runCode("let a = 1; a ||= 5; a;")).toBe("1");
});

test("&&= assigns to a truthy binding", () => {
  expect(runCode("let a = 1; a &&= 5; a;")).toBe("5");
});

test("&&= leaves a falsy binding alone", () => {
  expect(runCode("let a = 0; a &&= 5; a;")).toBe("0");
});

test("??= assigns to null", () => {
  expect(runCode("let a = null; a ??= 5; a;")).toBe("5");
});

test("??= leaves a defined falsy value alone", () => {
  expect(runCode("let a = 0; a ??= 5; a;")).toBe("0");
});

test("??= assigns to undefined", () => {
  expect(runCode("let a; a ??= 1; a ??= 2; a;")).toBe("1");
});

test("a member target works", () => {
  expect(runCode("const o = { x: 0 }; o.x ||= 7; o.x;")).toBe("7");
});

test("the right side is not evaluated when it is not needed", () => {
  expect(runCode("let n = 0; const o = { x: 1 }; o.x ||= (n = 1); n;")).toBe("0");
});

test("the right side runs when it is needed", () => {
  expect(runCode("let n = 0; const o = { x: 0 }; o.x ||= (n = 1); n;")).toBe("1");
});

test("the expression evaluates to the resulting value", () => {
  expect(runCode("let a = 0; a ||= 5;")).toBe("5");
  expect(runCode("let a = 3; a ||= 5;")).toBe("3");
});

test("a computed member target", () => {
  expect(runCode("const o = {}; const k = 'z'; o[k] ??= 4; o.z;")).toBe("4");
});
