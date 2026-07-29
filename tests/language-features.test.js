import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// Additional conformance tests for the subset of JavaScript that this VM
// implements correctly. Every assertion below is verified against the current
// interpreter. Features that are NOT implemented are documented separately in
// tests/ecma-gaps.test.js (as skipped tests).
// ---------------------------------------------------------------------------

// --- Numeric special values & formatting -----------------------------------

test("IEEE-754 double precision addition", () => {
  expect(runCode("0.1 + 0.2;")).toBe("0.30000000000000004");
});

test("positive division by zero yields inf", () => {
  expect(runCode("1 / 0;")).toBe("inf");
});

test("negative division by zero yields -inf", () => {
  expect(runCode("-1 / 0;")).toBe("-inf");
});

test("zero divided by zero yields NaN", () => {
  expect(runCode("0 / 0;")).toBe("NaN");
});

test("very large integer literal round-trips", () => {
  expect(runCode("100000000000000000000;")).toBe("100000000000000000000");
});

test("infinity constant prints as inf", () => {
  // Diverges from JS ("Infinity"); documents current formatting.
  expect(runCode("Infinity;")).toBe("inf");
});

test("NaN constant is a number", () => {
  expect(runCode("typeof NaN;")).toBe("number");
});

// --- String escape sequences ------------------------------------------------

test("newline escape", () => {
  expect(runCode("'a\\nb';")).toBe("a\nb");
});

test("tab escape", () => {
  expect(runCode("'a\\tb';")).toBe("a\tb");
});

test("backslash escape", () => {
  expect(runCode("'a\\\\b';")).toBe("a\\b");
});

test("quote escapes", () => {
  expect(runCode("'\\'';")).toBe("'");
  expect(runCode("'\"';")).toBe('"');
});

test("null escape", () => {
  expect(runCode("'a\\0b';")).toBe("a\0b");
});

// --- typeof coverage --------------------------------------------------------

test("typeof array is object", () => {
  expect(runCode("typeof [1,2];")).toBe("object");
});

test("typeof object is object", () => {
  expect(runCode("typeof {a:1};")).toBe("object");
});

test("typeof arrow function is function", () => {
  expect(runCode("typeof (()=>{});")).toBe("function");
});

test("typeof function declaration is function", () => {
  expect(runCode("function f(){} typeof f;")).toBe("function");
});

test("typeof typeof number is string", () => {
  expect(runCode("typeof typeof 1;")).toBe("string");
});

// --- Equality & NaN semantics -----------------------------------------------

test("NaN is not equal to itself (strict)", () => {
  expect(runCode("NaN === NaN;")).toBe("false");
});

test("NaN is not equal to itself (loose)", () => {
  expect(runCode("NaN == NaN;")).toBe("false");
});

test("null loosely equals undefined", () => {
  expect(runCode("null == undefined;")).toBe("true");
});

test("null does not strictly equal undefined", () => {
  expect(runCode("null === undefined;")).toBe("false");
});

test("null strictly equals null", () => {
  expect(runCode("const x = null; x === null;")).toBe("true");
});

test("undefined strictly equals undefined", () => {
  expect(runCode("const u = undefined; u === undefined;")).toBe("true");
});

// --- Coercion in concatenation ----------------------------------------------

test("number + number + string concatenates left to right", () => {
  expect(runCode("1 + 2 + 'px';")).toBe("3px");
});

test("string first forces concatenation", () => {
  expect(runCode("'v' + 1 + 2;")).toBe("v12");
});

test("array to string joins with commas", () => {
  expect(runCode("[1,2,3] + '';")).toBe("1,2,3");
});

test("object to string is [object Object]", () => {
  expect(runCode("({}) + '';")).toBe("[object Object]");
});

test("boolean true stringifies in concat", () => {
  expect(runCode("'x=' + true;")).toBe("x=true");
});

// --- Increment / decrement --------------------------------------------------

test("postfix decrement returns old value", () => {
  expect(runCode("let i = 5; i--;")).toBe("5");
});

test("prefix decrement returns new value", () => {
  expect(runCode("let i = 5; --i;")).toBe("4");
});

test("double negation of boolean", () => {
  expect(runCode("!!!true;")).toBe("false");
});

// --- Control flow edge cases ------------------------------------------------

test("switch with no matching case yields empty result", () => {
  expect(runCode("let r=''; switch(3){ case 1: r='a'; break; case 2: r='b'; break; } r;")).toBe("");
});

test("nested for loops count correctly", () => {
  expect(runCode("let s=0; for (let i=0; i<3; i++) { for (let j=0; j<3; j++) { s += 1; } } s;")).toBe("9");
});

test("for...in iterates all keys in order", () => {
  expect(runCode("let s=''; for (const k in {x:1,y:2,z:3}) { s += k; } s;")).toBe("xyz");
});

test("for...of sums array values", () => {
  expect(runCode("let n=0; for (const v of [10,20,30]) { n += v; } n;")).toBe("60");
});

test("block can mutate outer variable", () => {
  expect(runCode("let x = 1; { x = 2; } x;")).toBe("2");
});

test("chained comparison via &&", () => {
  expect(runCode("1 < 2 && 2 < 3;")).toBe("true");
});

// --- Function edge cases ----------------------------------------------------

test("bare return yields undefined", () => {
  expect(runCode("function f(){ return; } f();")).toBe("undefined");
});

test("statements after return are unreachable", () => {
  expect(runCode("function f(){ return 1; return 2; } f();")).toBe("1");
});

test("arrow returning object literal in parens", () => {
  expect(runCode("const f = () => ({a: 1}); f().a;")).toBe("1");
});

test("function composition", () => {
  expect(runCode("const f=(x)=>x+1; const g=(y)=>y*2; g(f(3));")).toBe("8");
});

test("three-argument function", () => {
  expect(runCode("function sum(a,b,c){return a+b+c;} sum(1,2,3);")).toBe("6");
});

test("identity function", () => {
  expect(runCode("function id(x){return x;} id(42);")).toBe("42");
});

test("function expression assigned and called", () => {
  expect(runCode("const g = (function(){ return 7; }); g();")).toBe("7");
});

// --- Closures ---------------------------------------------------------------

test("closure accumulator retains state", () => {
  expect(
    runCode(
      "function makeAcc() { let total = 0; return (x) => { total += x; return total; }; } const acc = makeAcc(); acc(5); acc(3); acc(2);"
    )
  ).toBe("10");
});

test("two closures have independent state", () => {
  expect(
    runCode(
      "function counter() { let n = 0; return () => ++n; } const a = counter(); const b = counter(); a(); a(); b();"
    )
  ).toBe("1");
});

// --- Nested data access -----------------------------------------------------

test("deeply nested object access", () => {
  expect(runCode("const o={a:{b:{c:42}}}; o.a.b.c;")).toBe("42");
});

test("nested array indexing", () => {
  expect(runCode("[[1,2],[3,4]][1][0];")).toBe("3");
});

test("object property arithmetic", () => {
  expect(runCode("const o = { a: 1, b: 2, c: 3 }; o.a + o.b + o.c;")).toBe("6");
});

// --- Integration programs ---------------------------------------------------

test("program: FizzBuzz (1-15)", () => {
  expect(
    runCode(
      "let r=''; for (let i=1; i<=15; i++) { if (i%15===0) { r+='FB'; } else if (i%3===0) { r+='F'; } else if (i%5===0) { r+='B'; } else { r+=i; } r+=','; } r;"
    )
  ).toBe("1,2,F,4,B,F,7,8,F,B,11,F,13,14,FB,");
});

test("program: greatest common divisor", () => {
  expect(
    runCode("function gcd(a,b) { while (b !== 0) { let t = b; b = a % b; a = t; } return a; } gcd(48, 18);")
  ).toBe("6");
});

test("program: iterative factorial", () => {
  expect(
    runCode("function fact(n) { let r = 1; while (n > 1) { r *= n; n--; } return r; } fact(6);")
  ).toBe("720");
});

test("program: primality test", () => {
  expect(
    runCode(
      "function isPrime(n) { if (n < 2) { return false; } for (let d=2; d*d<=n; d++) { if (n%d===0) { return false; } } return true; } isPrime(17);"
    )
  ).toBe("true");
});

test("program: collatz steps", () => {
  expect(
    runCode(
      "function collatz(n) { let steps = 0; while (n !== 1) { if (n % 2 === 0) { n = n / 2; } else { n = 3 * n + 1; } steps++; } return steps; } collatz(6);"
    )
  ).toBe("8");
});

test("program: max of array via for...of", () => {
  expect(runCode("let max = 0; for (const x of [3,7,2,9,4]) { if (x > max) { max = x; } } max;")).toBe("9");
});

test("program: sum of nested array (grid)", () => {
  expect(
    runCode("const grid = [[1,2],[3,4],[5,6]]; let s=0; for (const row of grid) { for (const v of row) { s += v; } } s;")
  ).toBe("21");
});

test("program: countdown string", () => {
  expect(runCode("let r=''; let i=5; while (i > 0) { r += i; i--; } r;")).toBe("54321");
});

test("program: csv builder with separator logic", () => {
  expect(
    runCode("let csv = ''; for (let i=1; i<=4; i++) { csv += i; if (i < 4) { csv += '-'; } } csv;")
  ).toBe("1-2-3-4");
});
