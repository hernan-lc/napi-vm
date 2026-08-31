import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// Symbol identity: a symbol is a unique value, not its description.
// ---------------------------------------------------------------------------

test("a symbol is strictly equal to itself", () => {
  expect(runCode("const s = Symbol('a'); s === s;")).toBe("true");
});

test("two symbols with the same description differ", () => {
  expect(runCode("Symbol('a') === Symbol('a');")).toBe("false");
});

test("typeof a symbol", () => {
  expect(runCode("typeof Symbol('a');")).toBe("symbol");
});

test("a symbol carries its description", () => {
  expect(runCode("Symbol('a').description;")).toBe("a");
});

test("a symbol without a description", () => {
  expect(runCode("Symbol().description;")).toBe("undefined");
});

test("a symbol renders as Symbol(description)", () => {
  expect(runCode("Symbol('a').toString();")).toBe("Symbol(a)");
});

test("String() coerces a symbol", () => {
  expect(runCode("String(Symbol('a'));")).toBe("Symbol(a)");
});

// --- The registry -----------------------------------------------------------

test("Symbol.for returns the same symbol for a key", () => {
  expect(runCode("Symbol.for('k') === Symbol.for('k');")).toBe("true");
});

test("Symbol.for differs from a fresh symbol", () => {
  expect(runCode("Symbol.for('k') === Symbol('k');")).toBe("false");
});

test("Symbol.keyFor finds a registered symbol", () => {
  expect(runCode("Symbol.keyFor(Symbol.for('k'));")).toBe("k");
});

test("Symbol.keyFor ignores an unregistered symbol", () => {
  expect(runCode("Symbol.keyFor(Symbol('k'));")).toBe("undefined");
});

// --- Well-known symbols -----------------------------------------------------

test("Symbol.iterator is one value", () => {
  expect(runCode("Symbol.iterator === Symbol.iterator;")).toBe("true");
});

test("well-known symbols are distinct from each other", () => {
  expect(runCode("Symbol.iterator === Symbol.asyncIterator;")).toBe("false");
});

// --- Symbol-keyed properties ------------------------------------------------

test("a symbol key round-trips", () => {
  expect(runCode("const o = {}; const s = Symbol('k'); o[s] = 1; o[s];")).toBe("1");
});

test("same-description symbols key separate slots", () => {
  expect(
    runCode("const o = {}; const a = Symbol('k'), b = Symbol('k'); o[a] = 1; o[b] = 2; o[a];"),
  ).toBe("1");
});

test("symbol keys are not string keys", () => {
  expect(runCode("const o = {}; o[Symbol('k')] = 1; Object.keys(o).length;")).toBe("0");
});

test("symbol keys are skipped by JSON.stringify", () => {
  expect(runCode("const o = { a: 1 }; o[Symbol('k')] = 2; JSON.stringify(o);")).toBe('{"a":1}');
});

test("a computed symbol key in an object literal", () => {
  expect(runCode("const s = Symbol('x'); const o = { [s]: 7 }; o[s];")).toBe("7");
});

test("a custom Symbol.iterator drives spread", () => {
  expect(
    runCode(
      "const o = { [Symbol.iterator]() { let i = 0; return { next: () => i < 2 ? { value: i++, done: false } : { value: undefined, done: true } }; } }; [...o].join();",
    ),
  ).toBe("0,1");
});
