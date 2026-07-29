import { test, expect } from "bun:test";
import { Vm, createVm, runCode, debugParse } from "../index.js";

test("createVm returns VM instance", () => {
  const vm = createVm();
  expect(vm).toBeDefined();
  expect(vm.run).toBeDefined();
});

test("new Vm() creates instance", () => {
  const vm = new Vm();
  expect(vm).toBeDefined();
});

test("vm.run returns string", () => {
  const vm = new Vm();
  const result = vm.run("1 + 1;");
  expect(typeof result).toBe("string");
  expect(result).toBe("2");
});

test("vm.run preserves state", () => {
  const vm = new Vm();
  vm.run("let x = 10;");
  vm.run("x += 5;");
  expect(vm.run("x;")).toBe("15");
});

test("vm.run handles errors gracefully", () => {
  const vm = new Vm();
  expect(() => vm.run("nonExistent;")).toThrow();
});

test("vm.getGlobal returns defined variable", () => {
  const vm = new Vm();
  vm.run("const myVar = 42;");
  expect(vm.getGlobal("myVar")).toBe("42");
});

test("vm.getGlobal returns undefined for missing", () => {
  const vm = new Vm();
  expect(vm.getGlobal("doesNotExist")).toBe("undefined");
});

test("vm.getGlobal returns string values", () => {
  const vm = new Vm();
  vm.run("const name = 'Alice';");
  expect(vm.getGlobal("name")).toBe("Alice");
});

test("vm.getGlobal returns object representation", () => {
  const vm = new Vm();
  vm.run("const obj = {a: 1};");
  expect(vm.getGlobal("obj")).toContain("a: 1");
});

test("vm.getGlobal returns array representation", () => {
  const vm = new Vm();
  vm.run("const arr = [1, 2, 3];");
  expect(vm.getGlobal("arr")).toBe("[1, 2, 3]");
});

test("vm.getGlobal returns function representation", () => {
  const vm = new Vm();
  vm.run("function myFn() {}");
  expect(vm.getGlobal("myFn")).toContain("Function");
});

test("runCode standalone function", () => {
  expect(runCode("2 + 2;")).toBe("4");
  expect(runCode("'hello';")).toBe("hello");
  expect(runCode("true;")).toBe("true");
});

test("runCode returns last expression", () => {
  expect(runCode("1; 2; 3;")).toBe("3");
});

test("runCode throws on runtime error", () => {
  expect(() => runCode("undefinedVar;")).toThrow();
});

test("debugParse returns AST string", () => {
  const ast = debugParse("const x = 1;");
  expect(typeof ast).toBe("string");
  expect(ast).toContain("VarDecl");
});

test("debugParse shows function structure", () => {
  const ast = debugParse("function foo(a) { return a; }");
  expect(ast).toContain("FnDecl");
  expect(ast).toContain("foo");
});

test("debugParse shows expression structure", () => {
  const ast = debugParse("1 + 2;");
  expect(ast).toContain("Binary");
});

test("vm handles complex program", () => {
  const vm = new Vm();
  vm.run(`
    function fibonacci(n) {
      if (n <= 1) { return n; }
      return fibonacci(n - 1) + fibonacci(n - 2);
    }
  `);
  expect(vm.run("fibonacci(10);")).toBe("55");
  expect(vm.run("fibonacci(15);")).toBe("610");
});

test("vm handles class-like patterns", () => {
  const vm = new Vm();
  vm.run(`
    function Animal(name) {
      return { name: name };
    }
  `);
  expect(vm.run("const a = new Animal('Dog'); typeof a;")).toBe("object");
});

test("vm handles closures across runs", () => {
  const vm = new Vm();
  vm.run("function makeCounter() { let n = 0; return () => ++n; }");
  vm.run("const counter = makeCounter();");
  expect(vm.run("counter();")).toBe("1");
  expect(vm.run("counter();")).toBe("2");
  expect(vm.run("counter();")).toBe("3");
});

test("multiple VMs run independently", () => {
  const vm1 = new Vm();
  const vm2 = new Vm();
  const vm3 = new Vm();

  vm1.run("const val = 'one';");
  vm2.run("const val = 'two';");
  vm3.run("const val = 'three';");

  expect(vm1.run("val;")).toBe("one");
  expect(vm2.run("val;")).toBe("two");
  expect(vm3.run("val;")).toBe("three");
});

test("vm handles empty source", () => {
  const vm = new Vm();
  expect(vm.run("")).toBe("undefined");
});

test("vm handles comment-only source", () => {
  const vm = new Vm();
  expect(vm.run("// just a comment")).toBe("undefined");
});

test("vm handles multiline programs", () => {
  const vm = new Vm();
  const result = vm.run(`
    const data = [1, 2, 3, 4, 5];
    let sum = 0;
    for (const x of data) {
      sum += x;
    }
    sum;
  `);
  expect(result).toBe("15");
});
