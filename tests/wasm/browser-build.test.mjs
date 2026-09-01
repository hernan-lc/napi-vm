// Tests for the browser (`wasm32`) build.
//
// The main suite runs the *native* addon, so nothing else here exercises the
// browser path — and it has broken silently before, because the two targets
// compile different code. These load the built WASM module directly under
// Node, which is close enough to the browser for everything the VM does.
//
// Run `npm run playground:build` first; the suite skips itself if the package
// is not built.

import { test, before, describe } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const pkg = join(root, "playground", "pkg");
const built = existsSync(join(pkg, "napi_vm_bg.wasm"));

let vm;

before(async () => {
  if (!built) return;
  const module = await import(join(pkg, "napi_vm.js"));
  module.initSync({ module: readFileSync(join(pkg, "napi_vm_bg.wasm")) });
  vm = new module.WasmVm();
});

/// Run one program and return its value, asserting that it succeeded.
function run(source) {
  const result = vm.run(source);
  assert.ok(result.ok, `expected success, got: ${result.error}`);
  return result.value;
}

describe("browser build", { skip: built ? false : "playground/pkg is not built" }, () => {
  test("generators yield their values", () => {
    assert.equal(run("function* g() { yield 1; yield 2; } [...g()].join();"), "1,2");
    assert.equal(run("function* g() { yield 1; } g().next().value;"), "1");
  });

  test("a generator reports done and its return value", () => {
    assert.equal(
      run("function* g() { yield 1; return 9; } const it = g(); it.next(); it.next().value;"),
      "9",
    );
    assert.equal(
      run("function* g() { yield 1; } const it = g(); it.next(); String(it.next().done);"),
      "true",
    );
  });

  test("generators work in loops and comprehensions", () => {
    assert.equal(run("function* g() { for (let i = 0; i < 3; i++) yield i; } [...g()].join();"), "0,1,2");
    assert.equal(run("function* g() { yield 1; } let t = 0; for (const v of g()) t += v; t;"), "1");
    assert.equal(run("function* g() { yield 1; yield 2; } Array.from(g()).join();"), "1,2");
  });

  test("yield* delegates", () => {
    assert.equal(
      run(
        "function* inner() { yield 1; yield 2; } function* outer() { yield 0; yield* inner(); yield 3; } [...outer()].join();",
      ),
      "0,1,2,3",
    );
  });

  test("an unbounded generator is a catchable error, not a hang", () => {
    // The browser target cannot suspend a body, so it runs to completion under
    // a cap rather than streaming. The cap must be reachable and catchable.
    assert.equal(
      run(
        "function* f() { let a = 0, b = 1; while (true) { yield a; const n = a + b; a = b; b = n; } } try { [...f()]; 'no'; } catch (e) { 'caught'; }",
      ),
      "caught",
    );
  });

  test("the language features the native build has are present", () => {
    assert.equal(run("const s = new Set([1, 2]); s.size;"), "2");
    assert.equal(run("/ab+/.test('abbb');"), "true");
    assert.equal(run("(2n ** 64n).toString();"), "18446744073709551616");
    assert.equal(run("new Date(0).toISOString();"), "1970-01-01T00:00:00.000Z");
    assert.equal(run("new Uint8Array([1, 2, 3]).join();"), "1,2,3");
    assert.equal(run("const o = { a: 1 }; delete o.a; JSON.stringify(o);"), "{}");
    assert.equal(run("class A { #v = 3; get v() { return this.#v; } } new A().v;"), "3");
    assert.equal(run("let a = 0; a ||= 5; a;"), "5");
    assert.equal(run("new Map([['a', 1]]).get('a');"), "1");
    assert.equal(run("'a-b'.replace(/-/, '+');"), "a+b");
  });

  test("promise reactions run as microtasks", () => {
    assert.equal(
      run("let o = []; o.push('a'); Promise.resolve().then(() => o.push('c')); o.push('b'); await 0; o.join('');"),
      "abc",
    );
  });
});
