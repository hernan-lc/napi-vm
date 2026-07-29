import { test, expect } from "bun:test";
import { runCode } from "../index.js";

test("if true branch", () => {
  expect(runCode("if (true) { 'yes'; }")).toBe("yes");
});

test("if false branch", () => {
  expect(runCode("if (false) { 'yes'; }")).toBe("undefined");
});

test("if/else", () => {
  expect(runCode("if (true) { 'yes'; } else { 'no'; }")).toBe("yes");
  expect(runCode("if (false) { 'yes'; } else { 'no'; }")).toBe("no");
});

test("if/else if/else chain", () => {
  const code = "function grade(s) { if (s >= 90) { return 'A'; } else if (s >= 80) { return 'B'; } else if (s >= 70) { return 'C'; } else { return 'F'; } } grade(95);";
  expect(runCode(code)).toBe("A");
  expect(runCode(code.replace("grade(95)", "grade(85)"))).toBe("B");
  expect(runCode(code.replace("grade(95)", "grade(75)"))).toBe("C");
  expect(runCode(code.replace("grade(95)", "grade(50)"))).toBe("F");
});

test("nested if", () => {
  expect(runCode("if (true) { if (true) { 'nested'; } }")).toBe("nested");
});

test("while loop", () => {
  expect(runCode("let i = 0; while (i < 5) { i++; } i;")).toBe("5");
  expect(runCode("let sum = 0; let i = 1; while (i <= 10) { sum += i; i++; } sum;")).toBe("55");
});

test("while loop with condition-based exit", () => {
  expect(runCode("let i = 0; while (i < 5) { i++; } i;")).toBe("5");
});

test("for loop basic", () => {
  expect(runCode("let sum = 0; for (let i = 0; i < 10; i++) { sum += i; } sum;")).toBe("45");
});

test("for loop with var", () => {
  expect(runCode("let sum = 0; for (var i = 0; i < 5; i++) { sum += i; } sum;")).toBe("10");
});

test("for loop with expression init", () => {
  expect(runCode("let sum = 0; let i = 0; for (i = 0; i < 5; i++) { sum += i; } sum;")).toBe("10");
});

test("for loop countdown", () => {
  expect(runCode("let r = ''; for (let i = 5; i > 0; i--) { r += i; } r;")).toBe("54321");
});

test("for loop with no init", () => {
  expect(runCode("let i = 0; let sum = 0; for (; i < 5; i++) { sum += i; } sum;")).toBe("10");
});

test("for loop with no update", () => {
  expect(runCode("let sum = 0; for (let i = 0; i < 5;) { sum += i; i++; } sum;")).toBe("10");
});

test("nested for loops", () => {
  expect(runCode("let count = 0; for (let i = 0; i < 3; i++) { for (let j = 0; j < 3; j++) { count++; } } count;")).toBe("9");
});

test("for...of array", () => {
  expect(runCode("let sum = 0; for (const x of [1, 2, 3, 4, 5]) { sum += x; } sum;")).toBe("15");
});

test("for...of builds string", () => {
  expect(runCode("let r = ''; for (const x of ['a', 'b', 'c']) { r += x; } r;")).toBe("abc");
});

test("for...in object keys", () => {
  expect(runCode("let r = ''; for (const k in {a: 1, b: 2, c: 3}) { r += k; } r;")).toBe("abc");
});

test("for...in array indices", () => {
  expect(runCode("let r = ''; for (const i in [10, 20, 30]) { r += i; } r;")).toBe("012");
});

test("switch statement", () => {
  const code = "function dayName(n) { switch (n) { case 0: return 'Sun'; case 1: return 'Mon'; case 2: return 'Tue'; default: return 'Other'; } } dayName(1);";
  expect(runCode(code)).toBe("Mon");
  expect(runCode(code.replace("dayName(1)", "dayName(0)"))).toBe("Sun");
  expect(runCode(code.replace("dayName(1)", "dayName(99)"))).toBe("Other");
});

test("switch fallthrough", () => {
  expect(runCode("let r = ''; switch (1) { case 1: r += 'one'; case 2: r += 'two'; break; case 3: r += 'three'; } r;")).toBe("onetwo");
});

test("switch with break", () => {
  expect(runCode("let r = ''; switch (2) { case 1: r += 'one'; break; case 2: r += 'two'; break; case 3: r += 'three'; break; } r;")).toBe("two");
});

test("switch default only", () => {
  expect(runCode("let r = 'none'; switch (99) { default: r = 'default'; } r;")).toBe("default");
});

test("break in switch works", () => {
  expect(runCode("let r = ''; switch (1) { case 1: r += 'one'; break; case 2: r += 'two'; } r;")).toBe("one");
});

test("for loop exits via condition", () => {
  expect(runCode("let r = ''; for (let i = 0; i < 3; i++) { r += i; } r;")).toBe("012");
});

test("ternary operator", () => {
  expect(runCode("true ? 'yes' : 'no';")).toBe("yes");
  expect(runCode("false ? 'yes' : 'no';")).toBe("no");
  expect(runCode("5 > 3 ? 'big' : 'small';")).toBe("big");
  expect(runCode("1 === 1 ? 'eq' : 'neq';")).toBe("eq");
});

test("nested ternary", () => {
  expect(runCode("const x = 5; x > 10 ? 'big' : x > 3 ? 'medium' : 'small';")).toBe("medium");
});

test("ternary with expressions", () => {
  expect(runCode("const x = 4; (x % 2 === 0) ? x * 2 : x * 3;")).toBe("8");
});

test("block statement", () => {
  expect(runCode("{ const x = 42; x; }")).toBe("42");
});

test("empty statement", () => {
  expect(runCode(";")).toBe("undefined");
});

test("multiple statements return last", () => {
  expect(runCode("1; 2; 3;")).toBe("3");
});
