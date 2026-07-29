import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// ECMAScript conformance gaps.
//
// Each test below asserts CORRECT JavaScript behavior that this VM does NOT
// currently implement. They are marked `test.skip` so the suite stays green,
// but they document precisely what is missing and serve as a roadmap: when a
// feature is implemented, un-skip its test and it should pass unchanged.
//
// Grouped by feature area. See the evaluation report for details.
// ---------------------------------------------------------------------------

// --- Operators: bitwise -----------------------------------------------------

test.skip("bitwise AND &", () => {
  expect(runCode("6 & 3;")).toBe("2");
});

test.skip("bitwise OR |", () => {
  expect(runCode("6 | 1;")).toBe("7");
});

test.skip("bitwise XOR ^", () => {
  expect(runCode("6 ^ 3;")).toBe("5");
});

test.skip("bitwise NOT ~", () => {
  expect(runCode("~5;")).toBe("-6");
});

test.skip("left shift <<", () => {
  expect(runCode("1 << 3;")).toBe("8");
});

test.skip("signed right shift >>", () => {
  expect(runCode("8 >> 2;")).toBe("2");
});

test.skip("unsigned right shift >>>", () => {
  expect(runCode("-1 >>> 0;")).toBe("4294967295");
});

// --- Operators: exponentiation / nullish / optional chaining ----------------

test.skip("exponentiation operator **", () => {
  expect(runCode("2 ** 10;")).toBe("1024");
});

test.skip("nullish coalescing ??", () => {
  expect(runCode("null ?? 'default';")).toBe("default");
});

test.skip("optional chaining ?.", () => {
  expect(runCode("const o = null; o?.a;")).toBe("undefined");
});

// --- Template literals ------------------------------------------------------

test.skip("template literal basic", () => {
  expect(runCode("`hi`;")).toBe("hi");
});

test.skip("template literal interpolation", () => {
  expect(runCode("const n='Bob'; `hi ${n}`;")).toBe("hi Bob");
});

// --- Assignment & declarations ----------------------------------------------

test.skip("member assignment obj.prop = value", () => {
  expect(runCode("const o = {}; o.x = 5; o.x;")).toBe("5");
});

test.skip("compound assignment %=", () => {
  expect(runCode("let x = 10; x %= 3; x;")).toBe("1");
});

test.skip("chained assignment a = b = 5", () => {
  expect(runCode("let a; let b; a = b = 5; a;")).toBe("5");
});

test.skip("multiple variable declaration let a, b", () => {
  expect(runCode("let a = 1, b = 2; a + b;")).toBe("3");
});

// --- Destructuring & spread -------------------------------------------------

test.skip("array destructuring", () => {
  expect(runCode("const [a, b] = [1, 2]; a + b;")).toBe("3");
});

test.skip("object destructuring", () => {
  expect(runCode("const { a, b } = { a: 1, b: 2 }; a + b;")).toBe("3");
});

test.skip("spread in array literal flattens", () => {
  expect(runCode("const a = [...[1,2], 3]; a.length;")).toBe("3");
});

test.skip("spread in function call", () => {
  expect(runCode("Math.max(...[1,5,3]);")).toBe("5");
});

test.skip("object spread", () => {
  expect(runCode("const o = { ...{ a: 1 }, b: 2 }; o.a;")).toBe("1");
});

// --- Functions: parameters & arguments --------------------------------------

test.skip("arrow function single param without parens", () => {
  expect(runCode("const f = x => x * 2; f(4);")).toBe("8");
});

test.skip("arrow function multiple params", () => {
  expect(runCode("const f = (a, b) => a + b; f(2, 3);")).toBe("5");
});

test.skip("default parameters", () => {
  expect(runCode("function f(a = 10) { return a; } f();")).toBe("10");
});

test.skip("rest parameters", () => {
  expect(runCode("function f(...a) { return a.length; } f(1, 2, 3);")).toBe("3");
});

test.skip("arguments object", () => {
  expect(runCode("function f() { return arguments.length; } f(1, 2);")).toBe("2");
});

// --- Control flow -----------------------------------------------------------

test("do...while loop", () => {
  expect(runCode("let i = 0; do { i++; } while (i < 5); i;")).toBe("5");
});

test("break inside a loop", () => {
  expect(runCode("let i = 0; while (true) { if (i >= 3) { break; } i++; } i;")).toBe("3");
});

test("continue inside a loop", () => {
  expect(runCode("let s = 0; for (let i = 0; i < 5; i++) { if (i % 2) { continue; } s += i; } s;")).toBe("6");
});

test.skip("labeled break", () => {
  expect(
    runCode("let n = 0; outer: for (let i = 0; i < 3; i++) { for (let j = 0; j < 3; j++) { if (j === 1) { break outer; } n++; } } n;")
  ).toBe("1");
});

test.skip("braceless if statement", () => {
  expect(runCode("let r = 'n'; if (true) r = 'y'; r;")).toBe("y");
});

test.skip("for...of over a string", () => {
  expect(runCode("let r = ''; for (const c of 'abc') { r += c; } r;")).toBe("abc");
});

test.skip("comma operator / multi-init for loop", () => {
  expect(runCode("let s = 0; for (let i = 0, j = 10; i < j; i++, j--) { s++; } s;")).toBe("5");
});

// --- Error handling ---------------------------------------------------------

test.skip("finally runs after catch", () => {
  expect(runCode("let r = ''; try { throw 'x'; } catch (e) { r += 'c'; } finally { r += 'f'; } r;")).toBe("cf");
});

test.skip("try/catch catches runtime (non-throw) errors", () => {
  expect(runCode("let r = ''; try { undefinedVar; } catch (e) { r = 'caught'; } r;")).toBe("caught");
});

test.skip("typeof undeclared variable is 'undefined' (no throw)", () => {
  expect(runCode("typeof notDefinedAnywhere;")).toBe("undefined");
});

// --- Objects ----------------------------------------------------------------

test.skip("object property shorthand { x }", () => {
  expect(runCode("const x = 5; const o = { x }; o.x;")).toBe("5");
});

test.skip("object computed property key", () => {
  expect(runCode("const k = 'a'; const o = { [k]: 1 }; o.a;")).toBe("1");
});

test.skip("object method shorthand", () => {
  expect(runCode("const o = { f() { return 3; } }; o.f();")).toBe("3");
});

test.skip("object getter", () => {
  expect(runCode("const o = { get x() { return 5; } }; o.x;")).toBe("5");
});

// --- Classes & OOP ----------------------------------------------------------

test.skip("class constructor with this", () => {
  expect(runCode("class A { constructor() { this.x = 1; } } new A().x;")).toBe("1");
});

test.skip("class instance method", () => {
  expect(runCode("class A { foo() { return 5; } } new A().foo();")).toBe("5");
});

test.skip("class field", () => {
  expect(runCode("class A { x = 7; } new A().x;")).toBe("7");
});

test.skip("class static method", () => {
  expect(runCode("class A { static foo() { return 9; } } A.foo();")).toBe("9");
});

test.skip("class inheritance", () => {
  expect(runCode("class A { foo() { return 1; } } class B extends A {} new B().foo();")).toBe("1");
});

test.skip("super call in derived constructor", () => {
  expect(
    runCode("class A { constructor() { this.x = 1; } } class B extends A { constructor() { super(); this.y = 2; } } new B().x;")
  ).toBe("1");
});

test.skip("instanceof", () => {
  expect(runCode("class A {} new A() instanceof A;")).toBe("true");
});

test.skip("function constructor assigns via this", () => {
  expect(runCode("function P() { this.x = 5; } new P().x;")).toBe("5");
});

// --- Standard library: Array ------------------------------------------------

test.skip("Array.prototype.map", () => {
  expect(runCode("[1,2,3].map((x) => x * 2).length;")).toBe("3");
});

test.skip("Array.prototype.filter", () => {
  expect(runCode("[1,2,3,4].filter((x) => x % 2 === 0).length;")).toBe("2");
});

test.skip("Array.prototype.reduce", () => {
  expect(runCode("[1,2,3].reduce((a, b) => a + b, 0);")).toBe("6");
});

test.skip("Array.prototype.push mutates", () => {
  expect(runCode("const a = [1]; a.push(2); a.length;")).toBe("2");
});

test.skip("Array.prototype.join", () => {
  expect(runCode("[1,2,3].join('-');")).toBe("1-2-3");
});

test.skip("Array.isArray", () => {
  expect(runCode("Array.isArray([1,2]);")).toBe("true");
});

// --- Standard library: String -----------------------------------------------

test.skip("string index access", () => {
  expect(runCode("'abc'[1];")).toBe("b");
});

test.skip("String.prototype.toUpperCase", () => {
  expect(runCode("'abc'.toUpperCase();")).toBe("ABC");
});

test.skip("String.prototype.slice", () => {
  expect(runCode("'hello'.slice(1,3);")).toBe("el");
});

test.skip("String.prototype.split", () => {
  expect(runCode("'a,b,c'.split(',').length;")).toBe("3");
});

test.skip("String.prototype.includes", () => {
  expect(runCode("'hello'.includes('ell');")).toBe("true");
});

// --- Standard library: Number / Math ----------------------------------------

test.skip("Math.abs", () => {
  expect(runCode("Math.abs(-5);")).toBe("5");
});

test.skip("Math.floor", () => {
  expect(runCode("Math.floor(3.7);")).toBe("3");
});

test.skip("Math.sqrt", () => {
  expect(runCode("Math.sqrt(16);")).toBe("4");
});

test.skip("Math.max variadic", () => {
  expect(runCode("Math.max(1,5,3);")).toBe("5");
});

test.skip("Number.prototype.toFixed", () => {
  expect(runCode("(3.14159).toFixed(2);")).toBe("3.14");
});

test.skip("scientific notation literal", () => {
  expect(runCode("1e3;")).toBe("1000");
});

// --- Standard library: global functions -------------------------------------

test.skip("parseInt", () => {
  expect(runCode("parseInt('42');")).toBe("42");
});

test.skip("parseFloat", () => {
  expect(runCode("parseFloat('3.14');")).toBe("3.14");
});

test.skip("global isNaN", () => {
  expect(runCode("isNaN(NaN);")).toBe("true");
});

test.skip("Number.isNaN", () => {
  expect(runCode("Number.isNaN(NaN);")).toBe("true");
});

// --- Standard library: JSON / Object ----------------------------------------

test.skip("JSON.stringify", () => {
  expect(runCode("JSON.stringify({a:1});")).toBe('{"a":1}');
});

test.skip("JSON.parse", () => {
  expect(runCode("JSON.parse('{\"a\":1}').a;")).toBe("1");
});

test.skip("Object.keys", () => {
  expect(runCode("Object.keys({a:1,b:2}).length;")).toBe("2");
});

test.skip("Object.assign", () => {
  expect(runCode("Object.assign({a:1},{b:2}).b;")).toBe("2");
});

// --- Coercion correctness ---------------------------------------------------

test.skip("boolean coerces to number in arithmetic", () => {
  expect(runCode("true + 1;")).toBe("2");
});

test.skip("loose equality coerces number/string", () => {
  expect(runCode("'5' == 5;")).toBe("true");
});

test.skip("loose equality coerces boolean", () => {
  expect(runCode("0 == false;")).toBe("true");
});

// --- Async / generators (not started) ---------------------------------------

test.skip("async function returns a promise", () => {
  expect(runCode("async function f() { return 1; } typeof f();")).toBe("object");
});

test.skip("generator function", () => {
  expect(runCode("function* g() { yield 1; } typeof g;")).toBe("function");
});
