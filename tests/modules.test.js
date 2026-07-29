import { test, expect } from "bun:test";
import { Vm, runCode } from "../index.js";

test("import.meta.url", () => {
  const vm = new Vm();
  expect(vm.run("import.meta.url;")).toBe("vm://module");
});

test("import.meta.main defaults to false", () => {
  const vm = new Vm();
  expect(vm.run("import.meta.main;")).toBe("false");
});

test("setImportMetaMain changes import.meta.main", () => {
  const vm = new Vm();
  vm.setImportMetaMain(true);
  expect(vm.run("import.meta.main;")).toBe("true");
  vm.setImportMetaMain(false);
  expect(vm.run("import.meta.main;")).toBe("false");
});

test("import.meta as expression", () => {
  const vm = new Vm();
  expect(vm.run("typeof import.meta;")).toBe("object");
});

test("import.meta.main in conditional", () => {
  const vm = new Vm();
  vm.setImportMetaMain(true);
  expect(vm.run("const m = import.meta.main; m ? 'main' : 'module';")).toBe("main");
});

test("register_module does not throw", () => {
  const vm = new Vm();
  expect(() => vm.registerModule("mymod", "const x = 1;")).not.toThrow();
});

test("register_module with exports", () => {
  const vm = new Vm();
  expect(() => vm.registerModule("utils", "const add = (x) => x + 1; export { add };")).not.toThrow();
});

test("register_module with default export", () => {
  const vm = new Vm();
  expect(() => vm.registerModule("main", "export default 42;")).not.toThrow();
});

test("import from unregistered module throws", () => {
  const vm = new Vm();
  expect(() => vm.run("import { x } from 'nonexistent';")).toThrow();
});

test("module state persists across runs", () => {
  const vm = new Vm();
  vm.run("const x = 42;");
  expect(vm.run("x;")).toBe("42");
});

test("module state: functions persist", () => {
  const vm = new Vm();
  vm.run("function add(a, b) { return a + b; }");
  expect(vm.run("add(3, 4);")).toBe("7");
});

test("module state: variables mutable across runs", () => {
  const vm = new Vm();
  vm.run("let count = 0;");
  vm.run("count++;");
  vm.run("count++;");
  expect(vm.run("count;")).toBe("2");
});
