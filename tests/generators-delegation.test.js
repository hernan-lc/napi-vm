import { test, expect } from "bun:test";
import { Vm } from "../index.js";

/**
 * `yield*` delegation and iterator-protocol spread.
 *
 * `yield*` previously did not parse at all: the `*` was silently skipped, so
 * `yield* g()` ran as `yield g()` — yielding the generator object itself.
 */

function run(source) {
  return new Vm().run(source);
}

test("yield* delegates to an array", () => {
  expect(run(`function* g() { yield* [1, 2, 3]; } [...g()].join(",");`)).toBe("1,2,3");
});

test("yield* delegates to another generator", () => {
  expect(
    run(`
      function* a() { yield 1; yield 2; }
      function* b() { yield 0; yield* a(); yield 3; }
      [...b()].join(",");
    `),
  ).toBe("0,1,2,3");
});

test("yield* evaluates to the delegate's return value", () => {
  expect(
    run(`
      function* a() { yield 1; return 9; }
      function* b() { const r = yield* a(); yield r; }
      [...b()].join(",");
    `),
  ).toBe("1,9");
});

test("yield* delegates to a string", () => {
  expect(run(`function* g() { yield* "ab"; } [...g()].join(",");`)).toBe("a,b");
});

test("yield* forwards values sent with next(v)", () => {
  expect(
    run(`
      function* a() { const x = yield 1; yield x; }
      function* b() { yield* a(); }
      const it = b(); it.next(); it.next(7).value;
    `),
  ).toBe("7");
});

test("closing the outer generator closes the delegate", () => {
  expect(
    run(`
      let log = "";
      function* a() { try { yield 1; } finally { log = "fin"; } }
      function* b() { yield* a(); }
      for (const v of b()) break;
      log;
    `),
  ).toBe("fin");
});

test("nested delegation composes", () => {
  expect(
    run(`
      function* a() { yield 1; }
      function* b() { yield* a(); yield 2; }
      function* c() { yield* b(); yield 3; }
      [...c()].join(",");
    `),
  ).toBe("1,2,3");
});

// ── spread uses the iterator protocol ────────────────────────────────

test("a generator can be spread", () => {
  // This produced an empty array: spread only understood arrays and strings.
  expect(run(`function* g() { yield 1; yield 2; } [...g()].join(",");`)).toBe("1,2");
});

test("spreading an infinite generator is bounded, not a hang", () => {
  expect(() => run(`function* g() { while (true) yield 1; } [...g()];`)).toThrow(/RangeError/);
});
