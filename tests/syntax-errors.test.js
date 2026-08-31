import { test, expect } from "bun:test";
import { Vm } from "../index.js";

/**
 * Syntax errors and parser termination.
 *
 * The parser used to have no error result at all: when a statement failed to
 * parse it skipped one token and carried on, so a malformed program produced a
 * partial AST that was then executed. Several delimited-list loops also had no
 * end-of-input guard, so truncated source spun forever — parsing happens before
 * the loop budget exists, so nothing interrupted it.
 */

function run(source) {
  return new Vm().run(source);
}

function fails(source) {
  return () => new Vm().run(source);
}

// ── errors are reported, not silently recovered ──────────────────────

test("a malformed statement is a SyntaxError with a position", () => {
  expect(fails(`let x = ;`)).toThrow(/SyntaxError: unexpected token .* at 1:9/);
});

test("a partial program is not executed", () => {
  // `@` begins no token. Previously it was dropped by the lexer and the rest
  // of the program ran anyway.
  expect(fails(`let a = 1; @@@; a;`)).toThrow(/SyntaxError/);
});

test("an unknown character is reported rather than skipped", () => {
  expect(fails(`x = #foo;`)).toThrow(/SyntaxError/);
});

test("a missing closing paren in an if head is rejected", () => {
  expect(fails(`if (true { }`)).toThrow(/SyntaxError: expected .*RParen/);
});

test("errors report the first problem, not a cascade", () => {
  const message = fails(`let x = ; let y = ; let z = ;`);
  expect(message).toThrow(/at 1:9/);
});

// ── truncated input terminates ───────────────────────────────────────

test.each([
  ["unclosed parameter list", `function f( { }`],
  ["unclosed array", `[1,2`],
  ["unclosed block", `{ let z = 1;`],
  ["unclosed call", `f(1,2`],
  ["unclosed object", `({ a: 1`],
])("%s reports end of input instead of hanging", (_name, source) => {
  const started = Date.now();
  expect(fails(source)).toThrow(/SyntaxError/);
  // Generous, but a regression here is an infinite loop, not a slow parse.
  expect(Date.now() - started).toBeLessThan(2000);
});

// ── valid programs still parse ───────────────────────────────────────

test("valid syntax is unaffected", () => {
  expect(run(`1 + 1;`)).toBe("2");
  expect(run(`let a = 1, b = 2; a + b;`)).toBe("3");
  expect(run(`class C { m() { return 1; } } new C().m();`)).toBe("1");
  expect(run(`const f = (x = 1, ...rest) => x + rest.length; f(1, 2, 3);`)).toBe("3");
  expect(run(`try { throw new Error("x"); } catch (e) { e.message; }`)).toBe("x");
  expect(run(`const { a, b: [c] } = { a: 1, b: [2] }; a + c;`)).toBe("3");
  expect(run(`label: for (const v of [1, 2]) { break label; } "done";`)).toBe("done");
});
