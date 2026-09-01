import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// `ArrayBuffer`, the typed-array views, and `DataView`.
//
// A buffer is bytes; a view is a window onto one. Two views over the same
// buffer see each other's writes, which is what these types are for.
// ---------------------------------------------------------------------------

test("the constructors report as functions", () => {
  expect(runCode("typeof ArrayBuffer;")).toBe("function");
  expect(runCode("typeof Uint8Array;")).toBe("function");
  expect(runCode("typeof DataView;")).toBe("function");
});

// --- ArrayBuffer ------------------------------------------------------------

test("a buffer has a byte length", () => {
  expect(runCode("new ArrayBuffer(8).byteLength;")).toBe("8");
});

test("a view over a buffer sizes itself", () => {
  expect(runCode("new Int32Array(new ArrayBuffer(8)).length;")).toBe("2");
});

test("isView distinguishes buffers from views", () => {
  expect(runCode("ArrayBuffer.isView(new Uint8Array(1));")).toBe("true");
  expect(runCode("ArrayBuffer.isView(new ArrayBuffer(1));")).toBe("false");
});

test("views over one buffer share its storage", () => {
  expect(
    runCode("const b = new ArrayBuffer(4); const x = new Uint8Array(b); const y = new Uint8Array(b); x[0] = 9; y[0];"),
  ).toBe("9");
});

test("a buffer slice is a copy", () => {
  expect(
    runCode("const b = new ArrayBuffer(4); const c = b.slice(0, 2); c.byteLength;"),
  ).toBe("2");
});

// --- Element access and conversion ------------------------------------------

test("elements round-trip", () => {
  expect(runCode("const a = new Int32Array(3); a[0] = 5; a[0];")).toBe("5");
});

test("a typed array is built from an array", () => {
  expect(runCode("new Uint8Array([1, 2, 3]).length;")).toBe("3");
  expect(runCode("new Uint8Array([1, 2, 3]).join();")).toBe("1,2,3");
});

test("integer views wrap", () => {
  expect(runCode("const a = new Uint8Array(1); a[0] = 300; a[0];")).toBe("44");
  expect(runCode("const a = new Int8Array(1); a[0] = 200; a[0];")).toBe("-56");
});

test("a clamped view saturates", () => {
  expect(runCode("const a = new Uint8ClampedArray(1); a[0] = 300; a[0];")).toBe("255");
  expect(runCode("const a = new Uint8ClampedArray(1); a[0] = -5; a[0];")).toBe("0");
});

test("float views keep the fraction", () => {
  expect(runCode("new Float64Array([1.5])[0];")).toBe("1.5");
});

test("a float32 view rounds to single precision", () => {
  expect(runCode("new Float32Array([0.5])[0];")).toBe("0.5");
});

test("out-of-range writes are ignored", () => {
  expect(runCode("const a = new Uint8Array(1); a[5] = 1; a.length;")).toBe("1");
});

test("an out-of-range read is undefined", () => {
  expect(runCode("String(new Uint8Array(1)[5]);")).toBe("undefined");
});

test("BigInt views hold BigInt elements", () => {
  expect(runCode("const a = new BigInt64Array(1); a[0] = 5n; a[0].toString();")).toBe("5");
});

// --- Metadata ---------------------------------------------------------------

test("byteLength and BYTES_PER_ELEMENT", () => {
  expect(runCode("new Int32Array(2).byteLength;")).toBe("8");
  expect(runCode("new Uint16Array(3).BYTES_PER_ELEMENT;")).toBe("2");
});

test("byteOffset reflects the window", () => {
  expect(
    runCode("const b = new ArrayBuffer(8); new Int32Array(b, 4).byteOffset;"),
  ).toBe("4");
});

test("buffer exposes the underlying storage", () => {
  expect(runCode("new Uint8Array(4).buffer.byteLength;")).toBe("4");
});

// --- Methods ----------------------------------------------------------------

test("of and from build views", () => {
  expect(runCode("Int32Array.of(1, 2).join();")).toBe("1,2");
  expect(runCode("Int32Array.from([1, 2], (x) => x * 3).join();")).toBe("3,6");
});

test("set copies elements in", () => {
  expect(runCode("const a = new Uint8Array(2); a.set([7, 8]); a.join();")).toBe("7,8");
});

test("set rejects an oversized source", () => {
  expect(() => runCode("new Uint8Array(1).set([1, 2]);")).toThrow();
});

test("subarray shares storage", () => {
  expect(
    runCode("const a = new Uint8Array([1, 2, 3, 4]); const s = a.subarray(1, 3); s[0] = 9; a[1];"),
  ).toBe("9");
});

test("slice copies storage", () => {
  expect(
    runCode("const a = new Uint8Array([1, 2, 3]); const s = a.slice(1); s[0] = 9; a[1];"),
  ).toBe("2");
});

test("fill writes a range", () => {
  expect(runCode("const a = new Uint8Array(4); a.fill(7); a.join();")).toBe("7,7,7,7");
});

test("at supports negative indices", () => {
  expect(runCode("new Uint8Array([1, 2, 3]).at(-1);")).toBe("3");
});

test("iteration methods work on the elements", () => {
  expect(runCode("new Int32Array([1, 2, 3]).map((x) => x * 2).join();")).toBe("2,4,6");
  expect(runCode("new Int32Array([1, 2, 3]).filter((x) => x > 1).join();")).toBe("2,3");
  expect(runCode("new Int32Array([1, 2, 3]).reduce((a, b) => a + b, 0);")).toBe("6");
  expect(runCode("new Uint8Array([3, 1, 2]).sort().join();")).toBe("1,2,3");
});

test("a typed array is iterable", () => {
  expect(runCode("[...new Uint8Array([1, 2])].join();")).toBe("1,2");
  expect(
    runCode("const a = new Uint8Array([1, 2]); let t = 0; for (const v of a) t += v; t;"),
  ).toBe("3");
});

test("a typed array stringifies as its elements", () => {
  expect(runCode("String(new Uint8Array([1, 2]));")).toBe("1,2");
});

// --- DataView ---------------------------------------------------------------

test("a DataView reads what it writes", () => {
  expect(
    runCode("const d = new DataView(new ArrayBuffer(4)); d.setInt32(0, 258); d.getInt32(0);"),
  ).toBe("258");
});

test("byte order is explicit", () => {
  expect(
    runCode("const d = new DataView(new ArrayBuffer(4)); d.setInt32(0, 258, true); d.getInt32(0, true);"),
  ).toBe("258");
});

test("big-endian is the default and differs from little-endian", () => {
  expect(
    runCode("const d = new DataView(new ArrayBuffer(4)); d.setInt32(0, 1); d.getInt32(0, true);"),
  ).toBe("16777216");
});

test("signed and unsigned views of the same bytes", () => {
  expect(
    runCode("const d = new DataView(new ArrayBuffer(2)); d.setInt16(0, -2); d.getUint16(0);"),
  ).toBe("65534");
});

test("floats round-trip", () => {
  expect(
    runCode("const d = new DataView(new ArrayBuffer(8)); d.setFloat64(0, 1.5); d.getFloat64(0);"),
  ).toBe("1.5");
});

test("an out-of-range access throws", () => {
  expect(() => runCode("new DataView(new ArrayBuffer(2)).getInt32(0);")).toThrow();
});

test("a DataView reports its window", () => {
  expect(runCode("new DataView(new ArrayBuffer(4), 1).byteLength;")).toBe("3");
  expect(runCode("new DataView(new ArrayBuffer(4), 1).byteOffset;")).toBe("1");
});
