import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// `Array.prototype` methods added alongside the rest, and the generator
// methods (`throw`, `return`) that complete the iterator protocol.
// ---------------------------------------------------------------------------

test("at supports negative indices", () => {
  expect(runCode("[1, 2, 3].at(-1);")).toBe("3");
  expect(runCode("[1, 2, 3].at(0);")).toBe("1");
  expect(runCode("String([1].at(5));")).toBe("undefined");
});

test("findIndex reports the position", () => {
  expect(runCode("[1, 2, 3].findIndex((x) => x > 1);")).toBe("1");
  expect(runCode("[1].findIndex((x) => x > 9);")).toBe("-1");
});

test("findLast and findLastIndex search from the end", () => {
  expect(runCode("[1, 2, 3].findLast((x) => x < 3);")).toBe("2");
  expect(runCode("[1, 2, 3].findLastIndex((x) => x < 3);")).toBe("1");
});

test("lastIndexOf searches from the end", () => {
  expect(runCode("[1, 2, 1].lastIndexOf(1);")).toBe("2");
  expect(runCode("[1].lastIndexOf(9);")).toBe("-1");
});

test("shift and unshift work at the front", () => {
  expect(runCode("const a = [1, 2]; a.shift(); a.join();")).toBe("2");
  expect(runCode("const a = [1, 2]; a.shift();")).toBe("1");
  expect(runCode("const a = [2]; a.unshift(1); a.join();")).toBe("1,2");
  expect(runCode("const a = [2]; a.unshift(1);")).toBe("2");
  expect(runCode("String([].shift());")).toBe("undefined");
});

test("fill writes a range", () => {
  expect(runCode("[1, 2, 3].fill(0).join();")).toBe("0,0,0");
  expect(runCode("[1, 2, 3].fill(0, 1).join();")).toBe("1,0,0");
  expect(runCode("[1, 2, 3].fill(0, 1, 2).join();")).toBe("1,0,3");
  expect(runCode("[1, 2, 3].fill(0, -1).join();")).toBe("1,2,0");
});

test("keys, values and entries iterate", () => {
  expect(runCode("[...[1, 2].keys()].join();")).toBe("0,1");
  expect(runCode("[...[1, 2].values()].join();")).toBe("1,2");
  expect(runCode("[...['a', 'b'].entries()].map((e) => e.join(':')).join();")).toBe("0:a,1:b");
});

// --- Generator throw / return -----------------------------------------------

test("gen.throw raises at the suspension point", () => {
  expect(
    runCode(
      "function* g() { try { yield 1; } catch (e) { yield 'caught ' + e; } } const it = g(); it.next(); it.throw('x').value;",
    ),
  ).toBe("caught x");
});

test("an uncaught gen.throw propagates to the caller", () => {
  expect(() =>
    runCode("function* g() { yield 1; } const it = g(); it.next(); it.throw(new Error('boom'));"),
  ).toThrow();
});

test("gen.return finishes the generator", () => {
  expect(
    runCode("function* g() { yield 1; yield 2; } const it = g(); it.next(); it.return(9).done;"),
  ).toBe("true");
  expect(
    runCode("function* g() { yield 1; yield 2; } const it = g(); it.next(); it.return(9).value;"),
  ).toBe("9");
});

test("gen.return runs a finally block", () => {
  expect(
    runCode(
      "let closed = false; function* g() { try { yield 1; } finally { closed = true; } } const it = g(); it.next(); it.return(); closed;",
    ),
  ).toBe("true");
});

test("a returned generator is exhausted", () => {
  expect(
    runCode(
      "function* g() { yield 1; yield 2; } const it = g(); it.next(); it.return(); String(it.next().done);",
    ),
  ).toBe("true");
});
