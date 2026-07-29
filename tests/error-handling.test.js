import { test, expect } from "bun:test";
import { runCode } from "../index.js";

test("try/catch basic", () => {
  expect(runCode("try { throw 'oops'; } catch(e) { 'caught: ' + e; }")).toBe("caught: oops");
});

test("try/catch with number throw", () => {
  expect(runCode("try { throw 42; } catch(e) { e; }")).toBe("42");
});

test("try/catch with string throw", () => {
  expect(runCode("try { throw 'error message'; } catch(e) { e; }")).toBe("error message");
});

test("try without throw executes normally", () => {
  expect(runCode("let r = 'ok'; try { r = 'try'; } catch(e) { r = 'catch'; } r;")).toBe("try");
});

test("catch variable scoped", () => {
  expect(runCode("try { throw 'x'; } catch(e) { e; }")).toBe("x");
});

test("nested try/catch", () => {
  expect(runCode("try { try { throw 'inner'; } catch(e) { throw 'outer: ' + e; } } catch(e) { e; }")).toBe("outer: inner");
});

test("try/catch does not catch non-throw errors silently", () => {
  expect(runCode("let r = ''; try { r += 'a'; } catch(e) { r += 'b'; } r += 'c'; r;")).toBe("ac");
});

test("throw stops execution in try block", () => {
  expect(runCode("let r = ''; try { r += 'a'; throw 'x'; r += 'b'; } catch(e) { r += 'c'; } r;")).toBe("ac");
});

test("try/catch/finally - finally runs after try", () => {
  expect(runCode("let r = ''; try { r += 'try'; } catch(e) { r += 'catch'; } finally { r += 'finally'; } r;")).toBe("tryfinally");
});

test("try/catch/finally - finally runs after catch", () => {
  expect(runCode("let r = ''; try { throw 'x'; } catch(e) { r += 'catch'; } r;")).toBe("catch");
});

test("error in function caught by caller", () => {
  expect(runCode("function f() { throw 'fn error'; } try { f(); } catch(e) { e; }")).toBe("fn error");
});

test("error propagates through call stack", () => {
  expect(runCode("function a() { throw 'deep'; } function b() { a(); } function c() { b(); } try { c(); } catch(e) { e; }")).toBe("deep");
});

test("throw with expression", () => {
  expect(runCode("try { throw 'code: ' + 404; } catch(e) { e; }")).toBe("code: 404");
});

test("conditional throw", () => {
  expect(runCode("function check(x) { if (x < 0) { throw 'negative'; } return x; } try { check(-1); } catch(e) { e; }")).toBe("negative");
  expect(runCode("function check(x) { if (x < 0) { throw 'negative'; } return x; } check(5);")).toBe("5");
});

test("try/catch returns value from try", () => {
  expect(runCode("function f() { try { return 'from try'; } catch(e) { return 'from catch'; } } f();")).toBe("from try");
});

test("try/catch returns value from catch", () => {
  expect(runCode("function f() { try { throw 'x'; return 'from try'; } catch(e) { return 'from catch'; } } f();")).toBe("from catch");
});

test("multiple statements in catch", () => {
  expect(runCode("try { throw 'err'; } catch(e) { const msg = 'Error: ' + e; msg; }")).toBe("Error: err");
});

test("undefined variable access throws (not caught by try/catch)", () => {
  expect(() => runCode("nonExistentVar;")).toThrow();
});

test("calling non-function throws (not caught by try/catch)", () => {
  expect(() => runCode("const x = 5; x();")).toThrow();
});
