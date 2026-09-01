import { test, expect } from "bun:test";
import { runCode } from "../index.js";

test("addition", () => {
  expect(runCode("2 + 2;")).toBe("4");
  expect(runCode("0 + 0;")).toBe("0");
  expect(runCode("-1 + 1;")).toBe("0");
  expect(runCode("100 + 200 + 300;")).toBe("600");
});

test("subtraction", () => {
  expect(runCode("10 - 3;")).toBe("7");
  expect(runCode("0 - 5;")).toBe("-5");
  expect(runCode("100 - 50 - 25;")).toBe("25");
});

test("multiplication", () => {
  expect(runCode("4 * 5;")).toBe("20");
  expect(runCode("0 * 100;")).toBe("0");
  expect(runCode("-3 * 4;")).toBe("-12");
  expect(runCode("2 * 3 * 4;")).toBe("24");
});

test("division", () => {
  expect(runCode("15 / 3;")).toBe("5");
  expect(runCode("7 / 2;")).toBe("3.5");
  expect(runCode("1 / 3;")).toContain("0.333");
  expect(runCode("100 / 10 / 2;")).toBe("5");
});

test("modulo", () => {
  expect(runCode("10 % 3;")).toBe("1");
  expect(runCode("15 % 5;")).toBe("0");
  expect(runCode("7 % 2;")).toBe("1");
  expect(runCode("-7 % 3;")).toBe("-1");
});

test("operator precedence: mul before add", () => {
  expect(runCode("2 + 3 * 4;")).toBe("14");
  expect(runCode("10 - 2 * 3;")).toBe("4");
  expect(runCode("2 * 3 + 4 * 5;")).toBe("26");
});

test("operator precedence: parentheses override", () => {
  expect(runCode("(2 + 3) * 4;")).toBe("20");
  expect(runCode("(10 - 2) * 3;")).toBe("24");
  expect(runCode("2 * (3 + 4);")).toBe("14");
});

test("nested parentheses", () => {
  expect(runCode("((2 + 3) * (4 - 1));")).toBe("15");
  expect(runCode("(((1 + 1)));")).toBe("2");
});

test("unary minus", () => {
  expect(runCode("-5;")).toBe("-5");
  expect(runCode("-(3 + 2);")).toBe("-5");
  expect(runCode("-(-5);")).toBe("5");
});

test("unary plus", () => {
  expect(runCode("+5;")).toBe("5");
  expect(runCode("+(-3);")).toBe("-3");
});

test("floating point arithmetic", () => {
  expect(runCode("0.1 + 0.2;")).toContain("0.3");
  expect(runCode("1.5 * 2;")).toBe("3");
  expect(runCode("10.5 / 0.5;")).toBe("21");
});

test("large numbers", () => {
  expect(runCode("999999999 * 999999999;")).toBe("999999998000000000");
  expect(runCode("1000000000 + 1;")).toBe("1000000001");
});

test("division by zero produces Infinity", () => {
  expect(runCode("1 / 0;")).toBe("Infinity");
  expect(runCode("-1 / 0;")).toBe("-Infinity");
});

test("NaN propagation", () => {
  expect(runCode("0 / 0;")).toBe("NaN");
});

test("complex expressions", () => {
  expect(runCode("2 + 3 * 4 - 6 / 2;")).toBe("11");
  expect(runCode("(1 + 2) * (3 + 4) - 5;")).toBe("16");
});
