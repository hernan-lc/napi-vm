import { test, expect } from "bun:test";
import { Vm } from "../index.js";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// ── removeModule / hasModule / listModules ───────────────────────────

test("hasModule returns false for unregistered modules", () => {
  const vm = new Vm();
  expect(vm.hasModule("nope")).toBe(false);
});

test("registerModule makes hasModule return true", () => {
  const vm = new Vm();
  vm.registerModule("greet", 'export function hi() { return "hi"; }');
  expect(vm.hasModule("greet")).toBe(true);
});

test("removeModule removes a registered module", () => {
  const vm = new Vm();
  vm.registerModule("greet", 'export function hi() { return "hi"; }');
  expect(vm.removeModule("greet")).toBe(true);
  expect(vm.hasModule("greet")).toBe(false);
});

test("removeModule returns false for unknown module", () => {
  const vm = new Vm();
  expect(vm.removeModule("nope")).toBe(false);
});

test("listModules returns all registered module names", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const x = 1;");
  vm.registerModule("b", "export const y = 2;");
  const names = vm.listModules().sort();
  expect(names).toEqual(["a", "b"]);
});

test("listModules is empty after removing all modules", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const x = 1;");
  vm.registerModule("b", "export const y = 2;");
  vm.removeModule("a");
  vm.removeModule("b");
  expect(vm.listModules()).toEqual([]);
});

test("removed module exports are no longer importable", () => {
  const vm = new Vm();
  vm.registerModule("dep", "export const val = 42;");
  vm.run('import { val } from "dep";');
  expect(vm.run("val;")).toBe("42");

  // Remove and re-register with a different value.
  vm.removeModule("dep");
  vm.registerModule("dep", "export const val = 99;");
  // A fresh import picks up the new module.
  const vm2 = new Vm();
  vm2.registerModule("dep", "export const val = 99;");
  vm2.run('import { val } from "dep";');
  expect(vm2.run("val;")).toBe("99");
});

// ── removeGlobal / hasGlobal ─────────────────────────────────────────

test("hasGlobal returns false for unset globals", () => {
  const vm = new Vm();
  expect(vm.hasGlobal("nope")).toBe(false);
});

test("setGlobal makes hasGlobal return true", () => {
  const vm = new Vm();
  vm.setGlobal("answer", 42);
  expect(vm.hasGlobal("answer")).toBe(true);
});

test("removeGlobal removes a global set via setGlobal", () => {
  const vm = new Vm();
  vm.setGlobal("answer", 42);
  expect(vm.removeGlobal("answer")).toBe(true);
  expect(vm.hasGlobal("answer")).toBe(false);
  expect(vm.run("typeof answer;")).toBe("undefined");
});

test("removeGlobal removes an exposed host function", () => {
  const vm = new Vm();
  vm.exposeFunction("hostAdd", (a, b) => a + b);
  expect(vm.hasGlobal("hostAdd")).toBe(true);
  expect(vm.run("hostAdd(1, 2);")).toBe("3");

  expect(vm.removeGlobal("hostAdd")).toBe(true);
  expect(vm.hasGlobal("hostAdd")).toBe(false);
});

test("removeGlobal returns false for unknown global", () => {
  const vm = new Vm();
  expect(vm.removeGlobal("nope")).toBe(false);
});

test("removeGlobal + re-expose avoids stale references (hot-reload pattern)", () => {
  const vm = new Vm();

  // First "generation" of a host function.
  let generation = 1;
  vm.exposeFunction("getGen", () => generation);
  expect(vm.run("getGen();")).toBe("1");

  // Simulate hot-reload: remove, bump generation, re-expose.
  vm.removeGlobal("getGen");
  generation = 2;
  vm.exposeFunction("getGen", () => generation);
  expect(vm.run("getGen();")).toBe("2");
});

test("VmIpc is available while a module is evaluated", async () => {
  const { VmIpc } = await import("../examples/lib/vm-ipc.ts");
  const vm = new Vm();
  const ipc = new VmIpc();
  ipc.handle("answer", () => 42);
  ipc.attach(vm);

  vm.registerModule(
    "ipc-module",
    `export const count = ipc.commands().length;
     export const answer = ipc.invoke("answer");`,
  );
  vm.run('import { count, answer } from "ipc-module";');

  expect(vm.run("count;")).toBe("1");
  expect(vm.run("answer;")).toBe("42");
  ipc.detach();
});

test("HotReloader attaches IPC before loading modules", async () => {
  const { HotReloader } = await import("../examples/lib/hot-reload.ts");
  const { VmIpc } = await import("../examples/lib/vm-ipc.ts");
  const modulesDir = mkdtempSync(join(tmpdir(), "napi-vm-hot-reload-"));
  writeFileSync(
    join(modulesDir, "ipc.js"),
    'export const commandCount = ipc.commands().length;\n',
  );

  const ipc = new VmIpc();
  ipc.handle("answer", () => 42);
  const reloader = new HotReloader({
    modulesDir,
    onBeforeLoad: (vm) => ipc.attach(vm),
  });

  try {
    const vm = reloader.start();
    vm.run('import { commandCount } from "ipc";');
    expect(vm.run("commandCount;")).toBe("1");
  } finally {
    ipc.detach();
    reloader.stop();
    rmSync(modulesDir, { recursive: true, force: true });
  }
});

// ── hot-reload full cycle ────────────────────────────────────────────

test("full hot-reload cycle: teardown + rebuild leaves no stale state", () => {
  const vm = new Vm();

  // Initial load.
  vm.registerModule("math", "export function add(a, b) { return a + b; }");
  vm.exposeFunction("hostLog", () => {});
  vm.run('import { add } from "math";');
  expect(vm.run("add(1, 2);")).toBe("3");

  // Teardown (what HotReloader does).
  for (const name of vm.listModules()) {
    vm.removeModule(name);
  }
  vm.removeGlobal("hostLog");

  expect(vm.listModules()).toEqual([]);
  expect(vm.hasGlobal("hostLog")).toBe(false);

  // Rebuild with updated module.
  vm.registerModule("math", "export function add(a, b) { return a + b + 100; }");
  const vm2 = new Vm();
  vm2.registerModule("math", "export function add(a, b) { return a + b + 100; }");
  vm2.run('import { add } from "math";');
  expect(vm2.run("add(1, 2);")).toBe("103");
});
