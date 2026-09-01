import { test, expect } from "bun:test";
import { Vm } from "../index.js";

// ---------------------------------------------------------------------------
// ES module linking: live bindings, re-exports, `export *`, namespace objects,
// dynamic `import()`, and cyclic graphs.
// ---------------------------------------------------------------------------

function withCounter() {
  const vm = new Vm();
  vm.registerModule(
    "counter",
    "export let n = 0; export function bump() { n = n + 1; }",
  );
  return vm;
}

// --- Live bindings ----------------------------------------------------------

test("an imported binding tracks the exporting module", () => {
  const vm = withCounter();
  expect(vm.run("import { n, bump } from 'counter'; bump(); n;")).toBe("1");
});

test("a renamed import is still live", () => {
  const vm = withCounter();
  expect(vm.run("import { n as m, bump } from 'counter'; bump(); m;")).toBe("1");
});

test("a namespace object is live", () => {
  const vm = withCounter();
  expect(
    vm.run("import * as c from 'counter'; const before = c.n; c.bump(); before + ':' + c.n;"),
  ).toBe("0:1");
});

test("export let declares in module scope", () => {
  const vm = new Vm();
  vm.registerModule("m", "export let v = 1; export function read() { return v; }");
  expect(vm.run("import { read } from 'm'; read();")).toBe("1");
});

// --- Namespace objects ------------------------------------------------------

test("a namespace exposes the default under 'default'", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1; export default 9;");
  expect(vm.run("import * as ns from 'a'; ns.default;")).toBe("9");
});

test("a namespace lists its exports", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1; export const w = 2; export default 9;");
  expect(vm.run("import * as ns from 'a'; Object.keys(ns).join();")).toBe("v,w,default");
});

// --- Renamed imports --------------------------------------------------------

test("import renames with `as`", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1;");
  expect(vm.run("import { v as renamed } from 'a'; renamed;")).toBe("1");
});

test("import { default as x } names the default export", () => {
  const vm = new Vm();
  vm.registerModule("a", "export default 9;");
  expect(vm.run("import { default as nine } from 'a'; nine;")).toBe("9");
});

// --- Re-exports -------------------------------------------------------------

test("export ... from forwards a named export", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1;");
  vm.registerModule("b", "export { v } from 'a';");
  expect(vm.run("import { v } from 'b'; v;")).toBe("1");
});

test("export ... from renames while forwarding", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1;");
  vm.registerModule("b", "export { v as alias } from 'a';");
  expect(vm.run("import { alias } from 'b'; alias;")).toBe("1");
});

test("a re-exported binding is still live", () => {
  const vm = withCounter();
  vm.registerModule("proxy", "export { n, bump } from 'counter';");
  expect(vm.run("import { n, bump } from 'proxy'; bump(); n;")).toBe("1");
});

test("export { default as x } from forwards the default", () => {
  const vm = new Vm();
  vm.registerModule("a", "export default 9;");
  vm.registerModule("b", "export { default as nine } from 'a';");
  expect(vm.run("import { nine } from 'b'; nine;")).toBe("9");
});

// --- export * ---------------------------------------------------------------

test("export * forwards every named export", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1; export const w = 2;");
  vm.registerModule("c", "export * from 'a';");
  expect(vm.run("import { v, w } from 'c'; v + w;")).toBe("3");
});

test("export * excludes the default export", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1; export default 9;");
  vm.registerModule("c", "export * from 'a';");
  expect(vm.run("import * as ns from 'c'; String(ns.default);")).toBe("undefined");
});

test("export * as names the namespace", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1;");
  vm.registerModule("e", "export * as inner from 'a';");
  expect(vm.run("import { inner } from 'e'; inner.v;")).toBe("1");
});

// --- Dynamic import ---------------------------------------------------------

test("import() resolves to the namespace", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const v = 1;");
  expect(vm.run("let out; import('a').then((m) => { out = m.v; }); out;")).toBe("1");
});

test("await import() works inside an async function", () => {
  const vm = new Vm();
  vm.registerModule("a", "export const w = 2;");
  expect(
    vm.run("let out; async function load() { const m = await import('a'); out = m.w; } load(); out;"),
  ).toBe("2");
});

test("import() of a missing module rejects", () => {
  const vm = new Vm();
  expect(vm.run("let out; import('zzz').catch(() => { out = 'caught'; }); out;")).toBe("caught");
});

// --- Deferred definition and cycles ----------------------------------------

test("defineModule defers evaluation until the first import", () => {
  const vm = new Vm();
  vm.defineModule("lazy", "export const x = 5;");
  expect(vm.run("import { x } from 'lazy'; x;")).toBe("5");
});

test("a body error in a deferred module surfaces at the import", () => {
  const vm = new Vm();
  vm.defineModule("bad", "throw new Error('boom');");
  expect(() => vm.run("import { z } from 'bad'; z;")).toThrow("boom");
});

test("a cyclic module graph links", () => {
  const vm = new Vm();
  vm.defineModule(
    "even",
    "import { isOdd } from 'odd'; export function isEven(n) { return n === 0 ? true : isOdd(n - 1); }",
  );
  vm.defineModule(
    "odd",
    "import { isEven } from 'even'; export function isOdd(n) { return n === 0 ? false : isEven(n - 1); }",
  );
  expect(vm.run("import { isEven } from 'even'; isEven(4) + ':' + isEven(3);")).toBe(
    "true:false",
  );
});

test("a cycle links from either entry point", () => {
  const vm = new Vm();
  vm.defineModule("even", "import { isOdd } from 'odd'; export function isEven(n) { return n === 0 ? true : isOdd(n - 1); }");
  vm.defineModule("odd", "import { isEven } from 'even'; export function isOdd(n) { return n === 0 ? false : isEven(n - 1); }");
  expect(vm.run("import { isOdd } from 'odd'; isOdd(3);")).toBe("true");
});
