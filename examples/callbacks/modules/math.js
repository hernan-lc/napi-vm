import { validate, clamp } from "utils";

function add(a, b) {
  validate(a, "number");
  validate(b, "number");
  return a + b;
}

function multiply(a, b) {
  validate(a, "number");
  validate(b, "number");
  return a * b;
}

function factorial(n) {
  validate(n, "number");
  if (n < 0) throw new Error("factorial: n must be non-negative");
  if (n === 0 || n === 1) return 1;
  let result = 1;
  for (let i = 2; i <= n; i++) {
    result = result * i;
  }
  return result;
}

function fib(n) {
  validate(n, "number");
  if (n < 0) throw new Error("fib: n must be non-negative");
  if (n === 0) return 0;
  if (n === 1) return 1;
  let a = 0;
  let b = 1;
  for (let i = 2; i <= n; i++) {
    const temp = a + b;
    a = b;
    b = temp;
  }
  return b;
}

function clampValue(value, min, max) {
  validate(value, "number");
  validate(min, "number");
  validate(max, "number");
  return clamp(value, min, max);
}

export { add, multiply, factorial, fib, clampValue };
