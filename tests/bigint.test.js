import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// BigInt: arbitrary-precision integers, and the numeric literal forms that
// come with them.
// ---------------------------------------------------------------------------

test("typeof a BigInt", () => {
  expect(runCode("typeof 1n;")).toBe("bigint");
});

test("a BigInt exceeds the safe integer range", () => {
  expect(runCode("9007199254740993n.toString();")).toBe("9007199254740993");
  expect(runCode("(2n ** 64n).toString();")).toBe("18446744073709551616");
});

test("arithmetic", () => {
  expect(runCode("(1n + 2n).toString();")).toBe("3");
  expect(runCode("(10n - 3n).toString();")).toBe("7");
  expect(runCode("(6n * 7n).toString();")).toBe("42");
});

test("division truncates towards zero", () => {
  expect(runCode("(7n / 2n).toString();")).toBe("3");
  expect(runCode("(-7n / 2n).toString();")).toBe("-3");
});

test("remainder takes the dividend's sign", () => {
  expect(runCode("(-7n % 2n).toString();")).toBe("-1");
  expect(runCode("(7n % 2n).toString();")).toBe("1");
});

test("division by zero throws", () => {
  expect(() => runCode("1n / 0n;")).toThrow();
});

test("bit shifts", () => {
  expect(runCode("(1n << 64n).toString();")).toBe("18446744073709551616");
  expect(runCode("(5n >> 1n).toString();")).toBe("2");
});

test("a right shift floors a negative value", () => {
  expect(runCode("(-5n >> 1n).toString();")).toBe("-3");
});

test("bitwise operators use two's complement", () => {
  expect(runCode("(12n | 3n).toString();")).toBe("15");
  expect(runCode("(12n & 10n).toString();")).toBe("8");
  expect(runCode("(12n ^ 10n).toString();")).toBe("6");
  expect(runCode("(~5n).toString();")).toBe("-6");
});

test("negation", () => {
  expect(runCode("(-(5n)).toString();")).toBe("-5");
});

// --- Comparison -------------------------------------------------------------

test("BigInts compare by value", () => {
  expect(runCode("1n === 1n;")).toBe("true");
  expect(runCode("2n > 1n;")).toBe("true");
});

test("loose equality crosses the numeric types", () => {
  expect(runCode("1n == 1;")).toBe("true");
});

test("strict equality does not", () => {
  expect(runCode("1n === 1;")).toBe("false");
});

test("relational comparison crosses the numeric types", () => {
  expect(runCode("2n > 1;")).toBe("true");
  expect(runCode("1 < 2n;")).toBe("true");
});

// --- Mixing -----------------------------------------------------------------

test("mixed arithmetic is a TypeError", () => {
  expect(() => runCode("1n + 1;")).toThrow("Cannot mix BigInt");
});

test("unary plus on a BigInt is a TypeError", () => {
  expect(() => runCode("+1n;")).toThrow();
});

test("concatenation with a string works", () => {
  expect(runCode("1n + 'x';")).toBe("1x");
  expect(runCode("'x' + 1n;")).toBe("x1");
});

test("String() coerces a BigInt", () => {
  expect(runCode("String(1n);")).toBe("1");
});

test("a BigInt is falsy only at zero", () => {
  expect(runCode("0n ? 'y' : 'n';")).toBe("n");
  expect(runCode("1n ? 'y' : 'n';")).toBe("y");
});

// --- The BigInt global ------------------------------------------------------

test("BigInt converts a number", () => {
  expect(runCode("BigInt(42).toString();")).toBe("42");
});

test("BigInt parses a string, including radix prefixes", () => {
  expect(runCode("BigInt('0x10').toString();")).toBe("16");
  expect(runCode("BigInt('-5').toString();")).toBe("-5");
});

test("BigInt refuses a non-integer", () => {
  expect(() => runCode("BigInt(1.5);")).toThrow();
});

test("asIntN wraps to a signed width", () => {
  expect(runCode("BigInt.asIntN(8, 255n).toString();")).toBe("-1");
});

test("asUintN wraps to an unsigned width", () => {
  expect(runCode("BigInt.asUintN(8, -1n).toString();")).toBe("255");
});

// --- Literal forms ----------------------------------------------------------

test("radix-prefixed BigInt literals", () => {
  expect(runCode("0xffn.toString();")).toBe("255");
  expect(runCode("0b1010n.toString();")).toBe("10");
  expect(runCode("0o17n.toString();")).toBe("15");
});

test("radix-prefixed number literals", () => {
  expect(runCode("0xff;")).toBe("255");
  expect(runCode("0b1010;")).toBe("10");
  expect(runCode("0o17;")).toBe("15");
});

test("numeric separators", () => {
  expect(runCode("1_000_000;")).toBe("1000000");
  expect(runCode("1_000n.toString();")).toBe("1000");
});
