import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// Error objects: subclassing, `stack`, and `toString`.
// ---------------------------------------------------------------------------

// --- The built-in types -----------------------------------------------------

test("each built-in error carries its name", () => {
  expect(runCode("new TypeError('x').name;")).toBe("TypeError");
  expect(runCode("new RangeError('x').name;")).toBe("RangeError");
  expect(runCode("new SyntaxError('x').name;")).toBe("SyntaxError");
  expect(runCode("new ReferenceError('x').name;")).toBe("ReferenceError");
});

test("message round-trips", () => {
  expect(runCode("new Error('boom').message;")).toBe("boom");
  expect(runCode("new Error().message;")).toBe("");
});

// --- Subclassing ------------------------------------------------------------

test("a subclass inherits the constructor", () => {
  expect(runCode("class E extends Error {} new E('x').message;")).toBe("x");
  expect(runCode("class E extends Error {} new E('x') instanceof Error;")).toBe("true");
});

test("a subclass can add its own state", () => {
  expect(
    runCode("class E extends Error { constructor(m) { super(m); this.code = 1; } } const e = new E('z'); e.message + ':' + e.code;"),
  ).toBe("z:1");
});

test("a subclass is catchable", () => {
  expect(
    runCode("class E extends Error {} try { throw new E('boom'); } catch (e) { e.message; }"),
  ).toBe("boom");
});

// --- Implicit derived constructors ------------------------------------------

test("a derived class without a constructor forwards its arguments", () => {
  expect(
    runCode("class A { constructor(x) { this.x = x; } } class B extends A {} new B(5).x;"),
  ).toBe("5");
});

test("an explicit derived constructor is used instead", () => {
  expect(
    runCode("class A { constructor(x) { this.x = x; } } class B extends A { constructor(y) { super(y * 2); } } new B(3).x;"),
  ).toBe("6");
});

// --- stack ------------------------------------------------------------------

test("stack begins with the error line", () => {
  expect(runCode("new Error('x').stack.split('\\n')[0];")).toBe("Error: x");
});

test("stack names the frames it was constructed in", () => {
  expect(
    runCode("function f() { return new Error('boom').stack; } f().includes('at f');"),
  ).toBe("true");
});

test("stack names the whole call chain", () => {
  expect(
    runCode("function g() { throw new Error('e'); } function f() { g(); } try { f(); } catch (e) { e.stack.includes('at g') && e.stack.includes('at f'); }"),
  ).toBe("true");
});

test("an internally raised error carries a stack too", () => {
  expect(
    runCode("try { null.x; } catch (e) { e.stack.split('\\n')[0]; }"),
  ).toBe("TypeError: Cannot read properties of null (reading 'x')");
  expect(
    runCode("function f() { null.x; } try { f(); } catch (e) { e.stack.includes('at f'); }"),
  ).toBe("true");
});

// --- toString ---------------------------------------------------------------

test("toString joins the name and message", () => {
  expect(runCode("new Error('x').toString();")).toBe("Error: x");
  expect(runCode("new TypeError('y').toString();")).toBe("TypeError: y");
  expect(runCode("new Error().toString();")).toBe("Error");
});

test("String() uses toString", () => {
  expect(runCode("String(new Error('x'));")).toBe("Error: x");
  expect(runCode("`${new Error('e')}`;")).toBe("Error: e");
});

test("an internally raised error stringifies too", () => {
  expect(runCode("try { null.x; } catch (e) { e.toString().slice(0, 9); }")).toBe("TypeError");
});

// --- A guest toString -------------------------------------------------------

test("a custom toString is used by String and templates", () => {
  expect(runCode("String({ toString() { return 'custom'; } });")).toBe("custom");
  expect(runCode("`${{ toString() { return 'custom'; } }}`;")).toBe("custom");
});

test("a non-string return is coerced", () => {
  expect(runCode("String({ toString() { return 42; } });")).toBe("42");
});

test("an object without toString still renders opaquely", () => {
  expect(runCode("String({});")).toBe("[object Object]");
});
