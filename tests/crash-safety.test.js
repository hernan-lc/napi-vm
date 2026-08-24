import { test, expect } from "bun:test";
import { Vm, runCode } from "../index.js";

// Crash-safety regressions: every one of these used to kill the host process
// (SIGSEGV / SIGTRAP) or freeze it forever. They must stay catchable. The
// executable matrix with subprocess isolation lives in examples/crash.ts.

test("unbounded recursion throws a catchable RangeError", () => {
  const vm = new Vm();
  const r = vm.run("function f() { return f(); } try { f(); } catch (e) { e.message; }");
  expect(r).toContain("Maximum call stack size exceeded");
});

test("unbounded mutual recursion throws a catchable RangeError", () => {
  const vm = new Vm();
  const r = vm.run(
    "function a() { return b(); } function b() { return a(); } try { a(); } catch (e) { e.message; }"
  );
  expect(r).toContain("Maximum call stack size exceeded");
});

test("recursion limit still allows deep-but-bounded recursion", () => {
  const vm = new Vm();
  // Depth 200 < the 256 cap.
  expect(vm.run("function c(n) { return n <= 0 ? 0 : 1 + c(n - 1); } c(200);")).toBe("200");
});

test("deeply nested source is a parse error, not a crash", () => {
  const src = "(".repeat(10_000) + "1" + ")".repeat(10_000);
  expect(() => runCode(src)).toThrow(/Maximum parse depth exceeded/);
});

test("returning a cyclic object prints [Circular] instead of crashing", () => {
  const vm = new Vm();
  const r = vm.run("let o = {}; o.self = o; o;");
  expect(r).toContain("[Circular]");
});

test("returning a cyclic array prints [Circular] instead of crashing", () => {
  const vm = new Vm();
  const r = vm.run("let a = []; a.push(a); a;");
  expect(r).toContain("[Circular]");
});

test("console.log of a cyclic array does not crash", () => {
  const vm = new Vm();
  expect(() => vm.run("let a = []; a.push(a); console.log(a);")).not.toThrow();
});

test("JSON.stringify of a cyclic structure throws a catchable TypeError", () => {
  const vm = new Vm();
  const r = vm.run(
    "let o = {}; o.self = o; try { JSON.stringify(o); } catch (e) { e.name + ': ' + e.message; }"
  );
  expect(r).toContain("circular structure");
});

test("JSON.stringify of a very deep structure throws a catchable RangeError", () => {
  const vm = new Vm();
  const r = vm.run(`
    let root = {}; let cur = root;
    for (let i = 0; i < 1000; i++) { cur.child = {}; cur = cur.child; }
    try { JSON.stringify(root); } catch (e) { e.message; }
  `);
  expect(r).toContain("Maximum JSON depth exceeded");
});

test("JSON.parse of a very deep document throws a catchable RangeError", () => {
  const vm = new Vm();
  const deep = "[".repeat(10_000) + "1" + "]".repeat(10_000);
  const r = vm.run(
    `try { JSON.parse(${JSON.stringify(deep)}); } catch (e) { e.message; }`
  );
  expect(r).toContain("Maximum JSON depth exceeded");
});

test("infinite loop hits the loop budget and throws", () => {
  const vm = new Vm();
  vm.setLoopLimit(1_000_000);
  expect(() => vm.run("while (true) {}")).toThrow(/Maximum loop iterations exceeded/);
});

test("loop budget refills on the next run", () => {
  const vm = new Vm();
  vm.setLoopLimit(1_000_000);
  expect(() => vm.run("while (true) {}")).toThrow();
  // A fresh execution gets a fresh budget.
  expect(vm.run("let s = 0; for (let i = 0; i < 10; i++) { s += i; } s;")).toBe("45");
});

test("array growth past the cap throws a catchable RangeError", () => {
  const vm = new Vm();
  const r = vm.run(
    "let a = []; try { for (let i = 0; i < 300000; i++) { a.push(1); } } catch (e) { e.message; }"
  );
  expect(r).toContain("Maximum array length exceeded");
});

test("indexed array assignment enforces the hard cap", () => {
  const vm = new Vm();
  const r = vm.run("let a = []; try { a[1000000000] = 1; } catch (e) { e.message; }");
  expect(r).toContain("Maximum array length exceeded");
});

test("deep Array.prototype.flat is iterative and bounded", () => {
  const vm = new Vm();
  const r = vm.run(
    "let a = [0]; for (let i = 0; i < 100000; i++) { a = [a]; } a.flat(100000);",
  );
  expect(r).toBe("[0]");
});

test("Array.sort does not hold a RefCell borrow across a comparator", () => {
  const vm = new Vm();
  expect(
    vm.run("let a = [3, 2, 1]; a.sort((x, y) => { a.push(4); return x - y; }); a.length;"),
  ).toBe("3");
});

test("string doubling past the cap throws a catchable RangeError", () => {
  const vm = new Vm();
  const r = vm.run(
    "let s = 'x'; try { while (true) { s = s + s; } } catch (e) { e.message; }"
  );
  expect(r).toContain("Maximum string length exceeded");
});

test("deeply nested value builds fine and teardown does not crash", () => {
  const vm = new Vm();
  // 300k-deep nesting: construction is iterative; the old derived Drop would
  // SIGSEGV at process exit when this value is freed. If this regression
  // returns, the test process itself dies — a very loud failure.
  const r = vm.run("let a = [0]; for (let i = 0; i < 300000; i++) { a = [a]; } 'built';");
  expect(r).toBe("built");
});

test("recursive yield* is absorbed, not fatal", () => {
  const vm = new Vm();
  expect(() => vm.run("function* g() { yield* g(); } let it = g(); it.next(); 'ok';")).not.toThrow();
});

test("callFunction into unbounded recursion throws a catchable JS error", async () => {
  const vm = new Vm();
  vm.run("function f() { return f(); }");
  expect(() => vm.callFunction("f", [])).toThrow(/Maximum call stack size exceeded/);
});
