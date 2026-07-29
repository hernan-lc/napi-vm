import { test, expect } from "bun:test";
import { runCode, debugParse } from "../index.js";

test("tokenizes integers", () => {
  const ast = debugParse("42;");
  expect(ast).toContain("Number(42.0)");
});

test("tokenizes floats", () => {
  const ast = debugParse("3.14;");
  expect(ast).toContain("Number(3.14)");
});

test("tokenizes numeric separators", () => {
  expect(runCode("1_000_000;")).toBe("1000000");
});

test("tokenizes single-quoted strings", () => {
  const ast = debugParse("'hello';");
  expect(ast).toContain('String("hello")');
});

test("tokenizes double-quoted strings", () => {
  const ast = debugParse('"world";');
  expect(ast).toContain('String("world")');
});

test("tokenizes string escape sequences", () => {
  expect(runCode("'hello\\nworld';")).toBe("hello\nworld");
  expect(runCode("'tab\\there';")).toBe("tab\there");
  expect(runCode("'back\\\\slash';")).toBe("back\\slash");
});

test("tokenizes identifiers with $ and _", () => {
  expect(runCode("const $foo = 1; $foo;")).toBe("1");
  expect(runCode("const _bar = 2; _bar;")).toBe("2");
});

test("skips single-line comments", () => {
  expect(runCode("1; // comment\n2;")).toBe("2");
});

test("skips multi-line comments", () => {
  expect(runCode("1; /* block\ncomment */ 2;")).toBe("2");
});

test("tokenizes arithmetic operators in expressions", () => {
  const ast = debugParse("1 + 2 - 3 * 4 / 5 % 6;");
  expect(ast).toContain("Binary");
});

test("tokenizes increment/decrement in expressions", () => {
  const ast = debugParse("let i = 0; i++; ++i; i--; --i;");
  expect(ast).toContain("Unary");
});

test("tokenizes comparison operators in expressions", () => {
  const ast = debugParse("1 == 2; 1 != 2; 1 === 2; 1 !== 2; 1 < 2; 1 > 2; 1 <= 2; 1 >= 2;");
  expect(ast).toContain("Binary");
});

test("tokenizes logical operators in expressions", () => {
  const ast = debugParse("true && false; true || false; !true;");
  expect(ast).toContain("Binary");
  expect(ast).toContain("Unary");
});

test("tokenizes assignment operators in expressions", () => {
  const ast = debugParse("let x = 1; x += 2; x -= 1; x *= 3; x /= 2;");
  expect(ast).toContain("Assignment");
});

test("tokenizes arrow function syntax", () => {
  const ast = debugParse("const f = (x) => x;");
  expect(ast).toContain("ArrowFn");
});

test("tokenizes spread in array", () => {
  const ast = debugParse("const a = [...x];");
  expect(ast).toContain("Spread");
});

test("tokenizes keywords in context", () => {
  const ast = debugParse("var a = 1; let b = 2; const c = 3; function f() { return 1; } class C {}");
  expect(ast).toContain("VarDecl");
  expect(ast).toContain("FnDecl");
  expect(ast).toContain("ClassDecl");
});

test("handles empty input", () => {
  expect(runCode("")).toBe("undefined");
});

test("handles whitespace-only input", () => {
  expect(runCode("   \n\t  ")).toBe("undefined");
});
