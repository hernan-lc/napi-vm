import { test, expect } from "bun:test";
import { Vm, runCode } from "../index.js";

test("Math.PI", () => {
  expect(runCode("Math.PI;")).toBe("3.141592653589793");
});

test("Math.E", () => {
  expect(runCode("Math.E;")).toBe("2.718281828459045");
});

test("Math constants", () => {
  expect(runCode("Math.LN2;")).toContain("0.693");
  expect(runCode("Math.LN10;")).toContain("2.302");
  expect(runCode("Math.LOG2E;")).toContain("1.442");
  expect(runCode("Math.LOG10E;")).toContain("0.434");
  expect(runCode("Math.SQRT1_2;")).toContain("0.707");
  expect(runCode("Math.SQRT2;")).toContain("1.414");
});

test("console exists as object", () => {
  expect(runCode("typeof console;")).toBe("object");
});

test("console has log member", () => {
  const vm = new Vm();
  expect(vm.run("typeof console.log;")).toBe("undefined");
});

test("JSON exists", () => {
  expect(runCode("typeof JSON;")).toBe("object");
});

test("JSON has parse and stringify", () => {
  const vm = new Vm();
  expect(vm.run("'parse' in JSON;")).toBe("true");
  expect(vm.run("'stringify' in JSON;")).toBe("true");
});

test("Object exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'keys' in Object;")).toBe("true");
  expect(vm.run("'values' in Object;")).toBe("true");
  expect(vm.run("'entries' in Object;")).toBe("true");
  expect(vm.run("'assign' in Object;")).toBe("true");
});

test("Array exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'isArray' in Array;")).toBe("true");
  expect(vm.run("'from' in Array;")).toBe("true");
  expect(vm.run("'of' in Array;")).toBe("true");
});

test("Promise exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'resolve' in Promise;")).toBe("true");
  expect(vm.run("'reject' in Promise;")).toBe("true");
  expect(vm.run("'all' in Promise;")).toBe("true");
  expect(vm.run("'race' in Promise;")).toBe("true");
});

test("Date exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'now' in Date;")).toBe("true");
  expect(vm.run("'parse' in Date;")).toBe("true");
  expect(vm.run("'UTC' in Date;")).toBe("true");
});

test("Number exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'isNaN' in Number;")).toBe("true");
  expect(vm.run("'isFinite' in Number;")).toBe("true");
  expect(vm.run("'parseInt' in Number;")).toBe("true");
  expect(vm.run("'parseFloat' in Number;")).toBe("true");
});

test("String exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'fromCharCode' in String;")).toBe("true");
});

test("Symbol exists with iterator", () => {
  const vm = new Vm();
  expect(vm.run("'iterator' in Symbol;")).toBe("true");
});

test("Reflect exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'apply' in Reflect;")).toBe("true");
  expect(vm.run("'construct' in Reflect;")).toBe("true");
  expect(vm.run("'get' in Reflect;")).toBe("true");
  expect(vm.run("'set' in Reflect;")).toBe("true");
  expect(vm.run("'has' in Reflect;")).toBe("true");
});

test("Intl exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'DateTimeFormat' in Intl;")).toBe("true");
  expect(vm.run("'NumberFormat' in Intl;")).toBe("true");
});

test("process exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'env' in process;")).toBe("true");
  expect(vm.run("'argv' in process;")).toBe("true");
  expect(vm.run("'cwd' in process;")).toBe("true");
  expect(vm.run("'pid' in process;")).toBe("true");
  expect(vm.run("'platform' in process;")).toBe("true");
  expect(vm.run("'version' in process;")).toBe("true");
});

test("Buffer exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'alloc' in Buffer;")).toBe("true");
  expect(vm.run("'from' in Buffer;")).toBe("true");
  expect(vm.run("'concat' in Buffer;")).toBe("true");
  expect(vm.run("'isBuffer' in Buffer;")).toBe("true");
});

test("crypto exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'getRandomValues' in crypto;")).toBe("true");
  expect(vm.run("'randomUUID' in crypto;")).toBe("true");
  expect(vm.run("'subtle' in crypto;")).toBe("true");
});

test("navigator exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'userAgent' in navigator;")).toBe("true");
  expect(vm.run("'language' in navigator;")).toBe("true");
  expect(vm.run("'platform' in navigator;")).toBe("true");
});

test("performance exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'now' in performance;")).toBe("true");
});

test("location exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'href' in location;")).toBe("true");
  expect(vm.run("'protocol' in location;")).toBe("true");
  expect(vm.run("'host' in location;")).toBe("true");
  expect(vm.run("'pathname' in location;")).toBe("true");
});

test("localStorage exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'getItem' in localStorage;")).toBe("true");
  expect(vm.run("'setItem' in localStorage;")).toBe("true");
  expect(vm.run("'removeItem' in localStorage;")).toBe("true");
  expect(vm.run("'clear' in localStorage;")).toBe("true");
});

test("URL exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'createObjectURL' in URL;")).toBe("true");
  expect(vm.run("'revokeObjectURL' in URL;")).toBe("true");
});

test("WebSocket exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'CONNECTING' in WebSocket;")).toBe("true");
  expect(vm.run("'OPEN' in WebSocket;")).toBe("true");
  expect(vm.run("'CLOSING' in WebSocket;")).toBe("true");
  expect(vm.run("'CLOSED' in WebSocket;")).toBe("true");
});

test("Response exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'json' in Response;")).toBe("true");
  expect(vm.run("'text' in Response;")).toBe("true");
  expect(vm.run("'redirect' in Response;")).toBe("true");
});

test("ArrayBuffer exists with isView", () => {
  const vm = new Vm();
  expect(vm.run("'isView' in ArrayBuffer;")).toBe("true");
});

test("BigInt exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'asIntN' in BigInt;")).toBe("true");
  expect(vm.run("'asUintN' in BigInt;")).toBe("true");
});

test("module.exports exists", () => {
  const vm = new Vm();
  expect(vm.run("'exports' in module;")).toBe("true");
});

test("history exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'pushState' in history;")).toBe("true");
  expect(vm.run("'replaceState' in history;")).toBe("true");
  expect(vm.run("'back' in history;")).toBe("true");
  expect(vm.run("'forward' in history;")).toBe("true");
});

test("screen exists with members", () => {
  const vm = new Vm();
  expect(vm.run("'width' in screen;")).toBe("true");
  expect(vm.run("'height' in screen;")).toBe("true");
});
