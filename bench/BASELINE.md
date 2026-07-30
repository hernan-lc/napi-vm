# Performance Baseline

Captured at commit `2a6a610` (clean tree), Linux x64, Node v26.4.0, Rust release profile (default, no LTO).
Test suite: **535 pass / 0 fail**.

## End-to-end through NAPI (`npm run bench`)

Each workload measured ≥ 250 ms after 20 warmup iterations.

| workload        | vm/op     | vm ops/s | native/op  | ratio |
|-----------------|-----------|----------|------------|-------|
| arithmetic_loop | 3.69 ms   | 271      | 5.02 µs    | 736x  |
| recursion_fib   | 20.06 ms  | 50       | 102.19 µs  | 196x  |
| array_chain     | 2.11 ms   | 473      | 16.24 µs   | 130x  |
| string_ops      | 1.13 ms   | 888      | 69.38 µs   | 16x   |
| class_methods   | 2.48 ms   | 403      | 40.17 µs   | 62x   |
| closures        | 5.65 ms   | 177      | 32.63 µs   | 173x  |
| json_roundtrip  | 1.58 ms   | 635      | 164.31 µs  | 10x   |

ratio = vm time / native time (higher = slower than the host engine).

## Criterion microbenchmarks (`npm run bench:rust`)

Full pipeline (lex → parse → eval) unless noted; 100 samples each.

| benchmark                 | time (estimate)          |
|---------------------------|--------------------------|
| run/arithmetic_loop       | 3.78 ms (3.75–3.81)      |
| run/recursion_fib         | 18.38 ms (18.19–18.61)   |
| run/array_chain           | 2.02 ms (2.00–2.04)      |
| run/string_ops            | 1.11 ms (1.10–1.13)      |
| run/class_methods         | 2.55 ms (2.52–2.60)      |
| run/closures              | 5.77 ms (5.74–5.82)      |
| run/json_roundtrip        | 1.67 ms (1.65–1.70)      |
| frontend/lex_big_source   | 2.66 ms (2.60–2.74)      |
| frontend/parse_big_source | 5.33 ms (5.27–5.39)      |

Criterion also persists these as the saved baseline in `target/criterion/`,
so subsequent `cargo bench` runs report % change automatically.

## Observations

- Call-heavy workloads (fib, closures, class_methods) carry the largest
  overhead: every call allocates an environment (`Rc<RefCell<Environment>>` +
  `HashMap`) and eagerly builds an `arguments` object.
- Parsing is a significant share of end-to-end time: `parse_big_source`
  (5.3 ms) exceeds `lex_big_source` (2.7 ms) ~2:1, and `bench.js` re-parses on
  every `runCode` iteration.
- Operators are stored as `String`s in the AST and string-matched on every
  evaluation; control-flow signals (`break`/`continue`) allocate `String`s.
