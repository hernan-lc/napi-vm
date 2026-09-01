import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// Class expressions, private members, static blocks, and async/generator
// methods — the pieces the class implementation previously omitted.
// ---------------------------------------------------------------------------

// --- Class expressions ------------------------------------------------------

test("an anonymous class expression", () => {
  expect(runCode("const C = class { m() { return 1; } }; new C().m();")).toBe("1");
});

test("a named class expression binds its name inside the body", () => {
  expect(runCode("const C = class Named { m() { return Named.name; } }; new C().m();")).toBe(
    "Named",
  );
});

test("a class expression's name does not leak", () => {
  expect(() => runCode("const C = class Named {}; Named;")).toThrow();
});

test("a class expression can extend", () => {
  expect(
    runCode("class Base { m() { return 'base'; } } const C = class extends Base {}; new C().m();"),
  ).toBe("base");
});

test("a class expression is usable inline", () => {
  expect(runCode("new (class { constructor() { this.x = 4; } })().x;")).toBe("4");
});

// --- Private members --------------------------------------------------------

test("a private field is readable from inside", () => {
  expect(runCode("class A { #v = 3; get v() { return this.#v; } } new A().v;")).toBe("3");
});

test("a private field is not an own enumerable property", () => {
  expect(runCode("class A { #v = 1; } Object.keys(new A()).length;")).toBe("0");
});

test("a private field is invisible to JSON", () => {
  expect(runCode("class A { #v = 1; constructor() { this.pub = 2; } } JSON.stringify(new A());")).toBe(
    '{"pub":2}',
  );
});

test("a private method is callable from inside", () => {
  expect(runCode("class A { #m() { return 4; } call() { return this.#m(); } } new A().call();")).toBe(
    "4",
  );
});

test("a private static member", () => {
  expect(
    runCode("class A { static #c = 0; static bump() { return ++A.#c; } } A.bump(); A.bump();"),
  ).toBe("2");
});

test("a private field can be written", () => {
  expect(
    runCode("class A { #v = 1; set(x) { this.#v = x; return this.#v; } } new A().set(9);"),
  ).toBe("9");
});

// --- Static blocks ----------------------------------------------------------

test("a static block runs against the class", () => {
  expect(runCode("class A { static x = 1; static { A.y = A.x + 1; } } A.y;")).toBe("2");
});

test("a static block sees this as the class", () => {
  expect(runCode("class A { static { this.tag = 'A'; } } A.tag;")).toBe("A");
});

test("static blocks run in order", () => {
  expect(
    runCode("class A { static { A.o = '1'; } static { A.o = A.o + '2'; } } A.o;"),
  ).toBe("12");
});

// --- Static assignment ------------------------------------------------------

test("a static can be assigned after the class is declared", () => {
  expect(runCode("class A {} A.tag = 'x'; A.tag;")).toBe("x");
});

// --- async / generator methods ----------------------------------------------

test("an async class method returns a promise", () => {
  expect(runCode("class A { async m() { return 7; } } typeof new A().m().then;")).toBe("function");
});

test("a generator class method yields", () => {
  expect(runCode("class A { *g() { yield 1; yield 2; } } [...new A().g()].join();")).toBe("1,2");
});

test("an async generator class method", () => {
  expect(
    runCode(
      "class A { async *g() { yield 1; yield 2; } } await (async () => { const out = []; for await (const v of new A().g()) out.push(v); return out.join(); })();",
    ),
  ).toBe("1,2");
});

test("a static async method", () => {
  expect(runCode("class A { static async m() { return 3; } } await A.m();")).toBe("3");
});
