import { test, expect } from "bun:test";
import { runCode } from "../index.js";

test("number literals", () => {
  expect(runCode("42;")).toBe("42");
  expect(runCode("3.14;")).toBe("3.14");
  expect(runCode("0;")).toBe("0");
  expect(runCode("-1;")).toBe("-1");
});

test("string literals", () => {
  expect(runCode("'hello';")).toBe("hello");
  expect(runCode('"world";')).toBe("world");
  expect(runCode("'';")).toBe("");
});

test("boolean literals", () => {
  expect(runCode("true;")).toBe("true");
  expect(runCode("false;")).toBe("false");
});

test("null literal", () => {
  expect(runCode("null;")).toBe("null");
});

test("undefined literal", () => {
  expect(runCode("undefined;")).toBe("undefined");
});

test("array literal", () => {
  expect(runCode("[1, 2, 3];")).toBe("[1, 2, 3]");
  expect(runCode("[];")).toBe("[]");
  expect(runCode("['a', 'b'];")).toBe("[a, b]");
});

test("array access", () => {
  expect(runCode("const a = [10, 20, 30]; a[0];")).toBe("10");
  expect(runCode("const a = [10, 20, 30]; a[1];")).toBe("20");
  expect(runCode("const a = [10, 20, 30]; a[2];")).toBe("30");
});

test("array out of bounds", () => {
  expect(runCode("const a = [1, 2]; a[5];")).toBe("undefined");
});

test("array length", () => {
  expect(runCode("[1, 2, 3].length;")).toBe("3");
  expect(runCode("[].length;")).toBe("0");
  expect(runCode("const a = [1, 2, 3, 4, 5]; a.length;")).toBe("5");
});

test("nested arrays", () => {
  expect(runCode("const a = [[1, 2], [3, 4]]; a[0];")).toBe("[1, 2]");
  expect(runCode("const a = [[1, 2], [3, 4]]; a[1];")).toBe("[3, 4]");
});

test("object literal", () => {
  expect(runCode("const o = {a: 1}; o.a;")).toBe("1");
  expect(runCode("const o = {name: 'Alice', age: 30}; o.name;")).toBe("Alice");
  expect(runCode("const o = {name: 'Alice', age: 30}; o.age;")).toBe("30");
});

test("object computed access", () => {
  expect(runCode("const o = {x: 10}; o['x'];")).toBe("10");
  expect(runCode("const o = {x: 10}; const k = 'x'; o[k];")).toBe("10");
});

test("object missing property", () => {
  expect(runCode("const o = {a: 1}; o.b;")).toBe("undefined");
});

test("nested objects", () => {
  expect(runCode("const o = {inner: {x: 42}}; o.inner;")).toContain("x: 42");
});

test("object with string keys", () => {
  expect(runCode("const o = {'my-key': 99}; o['my-key'];")).toBe("99");
});

test("string length", () => {
  expect(runCode("'hello'.length;")).toBe("5");
  expect(runCode("''.length;")).toBe("0");
  expect(runCode("'hello world'.length;")).toBe("11");
});

test("string concatenation", () => {
  expect(runCode("'hello' + ' ' + 'world';")).toBe("hello world");
  expect(runCode("'count: ' + 42;")).toBe("count: 42");
  expect(runCode("42 + ' items';")).toBe("42 items");
});

test("typeof operator", () => {
  expect(runCode("typeof 42;")).toBe("number");
  expect(runCode("typeof 'hello';")).toBe("string");
  expect(runCode("typeof true;")).toBe("boolean");
  expect(runCode("typeof undefined;")).toBe("undefined");
  expect(runCode("typeof null;")).toBe("object");
  expect(runCode("typeof {};")).toBe("object");
  expect(runCode("typeof [];")).toBe("object");
  expect(runCode("typeof function(){};")).toBe("function");
});

test("void operator", () => {
  expect(runCode("void 0;")).toBe("undefined");
  expect(runCode("void 42;")).toBe("undefined");
});

test("delete operator", () => {
  expect(runCode("delete 42;")).toBe("true");
});

test("type coercion in +", () => {
  expect(runCode("'5' + 3;")).toBe("53");
  expect(runCode("5 + '3';")).toBe("53");
  expect(runCode("true + 1;")).toBe("2");
});

test("type coercion in arithmetic", () => {
  expect(runCode("'5' - 3;")).toBe("2");
  expect(runCode("'5' * 2;")).toBe("10");
  expect(runCode("'10' / 2;")).toBe("5");
  expect(runCode("true - false;")).toBe("1");
  expect(runCode("1 + 1;")).toBe("2");
});

test("NaN from invalid coercion", () => {
  expect(runCode("'abc' * 1;")).toBe("0");
  expect(runCode("undefined - 1;")).toBe("NaN");
});

test("Infinity", () => {
  expect(runCode("Infinity;")).toBe("inf");
});

test("array in string context", () => {
  expect(runCode("'' + [1, 2, 3];")).toBe("1,2,3");
});

test("object in string context", () => {
  expect(runCode("'' + {};")).toBe("[object Object]");
});
