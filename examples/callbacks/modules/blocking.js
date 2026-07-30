/**
 * blocking.js — Functions that demonstrate event-loop blocking behavior.
 *
 * These are intentionally CPU-heavy operations that keep the VM busy,
 * starving the Node event loop until vm.run() returns.
 */

/**
 * Recursive Fibonacci — O(2^n) tree-recursive calls.
 * heavyFib(32) takes ~3-4 seconds in the interpreter.
 */
function heavyFib(n) {
  if (n <= 1) return n;
  return heavyFib(n - 1) + heavyFib(n - 2);
}

/**
 * Tight while loop — runs `iterations` increments.
 * Blocks the event loop for the duration.
 */
function whileLoop(iterations) {
  let i = 0;
  while (i < iterations) {
    i++;
  }
  return i;
}

/**
 * Nested loop — two levels of loops, O(n^2) work.
 */
function nestedLoop(n) {
  let count = 0;
  let i = 0;
  while (i < n) {
    let j = 0;
    while (j < n) {
      count++;
      j++;
    }
    i++;
  }
  return count;
}

/**
 * Recursive countdown — deep recursion that blocks until complete.
 */
function deepRecursion(depth) {
  if (depth <= 0) return 0;
  return 1 + deepRecursion(depth - 1);
}

export { heavyFib, whileLoop, nestedLoop, deepRecursion };
