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

test("tokenizes all operators", () => {
  const ast = debugParse("+ - * / % ++ --");
  expect(ast).toContain("Plus");
  expect(ast).toContain("Minus");
  expect(ast).toContain("Star");
  expect(ast).toContain("Slash");
  expect(ast).toContain("Percent");
  expect(ast).toContain("PlusPlus");
  expect(ast).toContain("MinusMinus");
});

test("tokenizes comparison operators", () => {
  const ast = debugParse("== != === !== < > <= >=");
  expect(ast).toContain("EqualEqual");
  expect(ast).toContain("NotEqual");
  expect(ast).toContain("EqualEqualEqual");
  expect(ast).toContain("NotEqualEqual");
});

test("tokenizes logical operators", () => {
  const ast = debugParse("&& || !");
  expect(ast).toContain("And");
  expect(ast).toContain("Or");
  expect(ast).toContain("Not");
});

test("tokenizes assignment operators", () => {
  const ast = debugParse("= += -= *= /=");
  expect(ast).toContain("Equal");
  expect(ast).toContain("PlusEqual");
  expect(ast).toContain("MinusEqual");
  expect(ast).toContain("StarEqual");
  expect(ast).toContain("SlashEqual");
});

test("tokenizes arrow function syntax", () => {
  const ast = debugParse("=>");
  expect(ast).toContain("Arrow");
});

test("tokenizes spread operator", () => {
  const ast = debugParse("...");
  expect(ast).toContain("DotDotDot");
});

test("tokenizes keywords", () => {
  const ast = debugParse("var let const function return if else for while do switch case default break continue class extends new this super import export from as async await try catch finally throw typeof instanceof in of true false null undefined delete void static get set constructor");
  expect(ast).toContain("KwVar");
  expect(ast).toContain("KwLet");
  expect(ast).toContain("KwConst");
  expect(ast).toContain("KwFunction");
  expect(ast).toContain("KwClass");
});

test("handles empty input", () => {
  expect(runCode("")).toBe("undefined");
});

test("handles whitespace-only input", () => {
  expect(runCode("   \n\t  ")).toBe("undefined");
});
