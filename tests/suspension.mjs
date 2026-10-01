import { runCode } from "../index.js";

// Targets without stack-switching (notably aarch64-pc-windows-msvc and
// wasm32 — see `build.rs`) cannot suspend a generator body or an async
// function. There a generator body runs once, to completion, on the first
// `next()` and its yields are buffered, `next(v)` cannot send a value in,
// `throw()` cannot be delivered to the suspension point, and `await`
// resolves eagerly. Tests that pin true-suspension semantics branch on this
// probe and assert the documented buffered behavior instead.
//
// Detected behaviorally, so no new native export is needed: with true
// suspension only the code before the first `yield` has run when the first
// `next()` returns.
let cached;
export function hasTrueSuspension() {
  if (cached === undefined) {
    cached =
      runCode(
        "let log = []; function* g() { log.push('a'); yield 1; log.push('b'); yield 2; } const it = g(); it.next(); log.join(',');",
      ) === "a";
  }
  return cached;
}
