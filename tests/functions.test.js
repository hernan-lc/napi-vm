import { test, expect } from "bun:test";
import { runCode } from "../index.js";

test("function declaration and call", () => {
  expect(runCode("function add(a, b) { return a + b; } add(3, 4);")).toBe("7");
  expect(runCode("function greet(name) { return 'Hello, ' + name; } greet('World');")).toBe("Hello, World");
});

test("function with no params", () => {
  expect(runCode("function f() { return 42; } f();")).toBe("42");
});

test("function with no return", () => {
  expect(runCode("function f() { } f();")).toBe("undefined");
});

test("function with early return", () => {
  expect(runCode("function f(x) { if (x > 5) { return 'big'; } return 'small'; } f(10);")).toBe("big");
  expect(runCode("function f(x) { if (x > 5) { return 'big'; } return 'small'; } f(2);")).toBe("small");
});

test("arrow function expression body", () => {
  expect(runCode("const f = (x) => x * x; f(5);")).toBe("25");
  expect(runCode("const f = (x) => x + 1; f(3);")).toBe("4");
  expect(runCode("const double = (x) => x * 2; double(7);")).toBe("14");
});

test("arrow function block body", () => {
  expect(runCode("const f = (x) => { return x * 2; }; f(5);")).toBe("10");
});

test("arrow function no params", () => {
  expect(runCode("const f = () => 42; f();")).toBe("42");
});

test("arrow function single param with parens", () => {
  expect(runCode("const f = (x) => x * 3; f(4);")).toBe("12");
});

test("function expression", () => {
  expect(runCode("const f = function() { return 42; }; f();")).toBe("42");
  expect(runCode("const f = function add(a, b) { return a + b; }; f(3, 4);")).toBe("7");
});

test("higher-order functions", () => {
  expect(runCode("function apply(f, x) { return f(x); } apply((x) => x * 2, 5);")).toBe("10");
});

test("functions as return values", () => {
  expect(runCode("function makeAdder(x) { return (y) => x + y; } const add5 = makeAdder(5); add5(3);")).toBe("8");
  expect(runCode("function makeMul(x) { return (y) => x * y; } const mul3 = makeMul(3); mul3(7);")).toBe("21");
});

test("closures capture variables", () => {
  expect(runCode("function counter() { let n = 0; return () => ++n; } const c = counter(); c(); c(); c();")).toBe("3");
  expect(runCode("function counter() { let n = 0; return () => ++n; } const c = counter(); c(); c();")).toBe("2");
});

test("closure over loop variable", () => {
  expect(runCode("function makeFns() { let fns = []; for (let i = 0; i < 3; i++) { fns = i; } return fns; } makeFns();")).toBe("2");
});

test("recursion: factorial", () => {
  expect(runCode("function factorial(n) { return n <= 1 ? 1 : n * factorial(n-1); } factorial(5);")).toBe("120");
  expect(runCode("function factorial(n) { return n <= 1 ? 1 : n * factorial(n-1); } factorial(1);")).toBe("1");
  expect(runCode("function factorial(n) { return n <= 1 ? 1 : n * factorial(n-1); } factorial(0);")).toBe("1");
});

test("recursion: fibonacci", () => {
  expect(runCode("function fib(n) { return n <= 1 ? n : fib(n-1) + fib(n-2); } fib(10);")).toBe("55");
  expect(runCode("function fib(n) { return n <= 1 ? n : fib(n-1) + fib(n-2); } fib(0);")).toBe("0");
  expect(runCode("function fib(n) { return n <= 1 ? n : fib(n-1) + fib(n-2); } fib(1);")).toBe("1");
});

test("mutual recursion", () => {
  expect(runCode("function isEven(n) { return n === 0 ? true : isOdd(n - 1); } function isOdd(n) { return n === 0 ? false : isEven(n - 1); } isEven(4);")).toBe("true");
  expect(runCode("function isEven(n) { return n === 0 ? true : isOdd(n - 1); } function isOdd(n) { return n === 0 ? false : isEven(n - 1); } isEven(3);")).toBe("false");
});

test("nested function declarations", () => {
  expect(runCode("function outer() { function inner() { return 42; } return inner(); } outer();")).toBe("42");
});

test("function calling another function", () => {
  expect(runCode("function double(x) { return x * 2; } function quadruple(x) { return double(double(x)); } quadruple(3);")).toBe("12");
});

test("function with default-like behavior via ||", () => {
  expect(runCode("function f(x) { x = x || 10; return x; } f(5);")).toBe("5");
  expect(runCode("function f(x) { x = x || 10; return x; } f(0);")).toBe("10");
});

test("variadic-like via arguments count", () => {
  expect(runCode("function f(a, b, c) { return a + b; } f(1, 2);")).toBe("3");
});

test("missing arguments are undefined", () => {
  expect(runCode("function f(a, b) { return typeof b; } f(1);")).toBe("undefined");
});

test("extra arguments ignored", () => {
  expect(runCode("function f(a) { return a; } f(1, 2, 3);")).toBe("1");
});

test("function name in typeof", () => {
  expect(runCode("function f() {} typeof f;")).toBe("function");
  expect(runCode("const f = () => 1; typeof f;")).toBe("function");
});

test("constructor with new returns object", () => {
  expect(runCode("function Point(x, y) { return {x: x, y: y}; } const p = new Point(1, 2); typeof p;")).toBe("object");
});

test("IIFE pattern", () => {
  expect(runCode("(function() { return 42; })();")).toBe("42");
});
