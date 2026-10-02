# Reproducible measurements

Build and stage the real artifacts with `npm run plugins:build` and
`npm run plugins:test:interop` before measuring. On a POSIX shell:

```sh
PLUGIN_RUNTIME=node PLUGIN_BENCH_SAMPLES=30 npm run plugins:bench
PLUGIN_RUNTIME=bun PLUGIN_BENCH_SAMPLES=30 npm run plugins:bench
npm run plugins:bench:rust
```

The JS host runner measures process startup through READY, warmed echo RPC
latency at 64 B / 4 KiB / 256 KiB / 1 MiB, sequential throughput, eight simultaneous offered
calls, Linux resident memory and 1 / 5 / 20 live instances. Overlapping business calls
may be rejected by the documented fail-fast admission policy; offered load is
reported with accepted/rejected counts. It also reports separate quiesce,
snapshot and shutdown/reap timings, plus combined restart/initialization time.
The echo fixture is stateless, so its restart does not measure state restoration.

The Rust-host runner executes the same generated counter contract and identical
dataset against TS under Node, TS under Bun and the Rust executable. Each has ten
warmup increments and thirty timed increments, with a validated final value 40.
The runner reports actual selected runtimes and build modes; debug Rust results
are not presented as optimized native performance. State-preserving restoration
is separately verified by the bidirectional migration correctness suite.

Machine-readable reports are written to
`artifacts/trusted-plugins/benchmark-node.json`, `benchmark-bun.json` and
`benchmark-rust-host.json`. Binary/package footprints distinguish the JS package,
debug Rust binaries and the Bun-compiled executable. Missing binaries are labeled
not-built rather than assigned invented sizes.

These are noisy single-machine observations, without performance thresholds.
New processes do not imply cold OS/disk caches. Linux `/proc` RSS includes shared
resident pages and is not exclusive allocation. Small echo/counter methods mostly
measure RPC, serialization and validation, so they cannot justify a native CPU
speedup claim. One process per plugin adds memory overhead; standalone runtimes
can duplicate installation size. Larger application operations may benefit from
explicit batching while keeping the versioned business contract intact.

## Captured Linux observation (2026-10-01)

The Rust-host counter workload measured median RPC latency of 7.415 ms for
TS/Node, 2.097 ms for TS/Bun and 47.138 ms for the debug Rust executable.
Corresponding p95 values were 25.113, 18.125 and 127.305 ms. These include
transport/validation overhead and shared-machine scheduling; they do not rank
production runtime performance. All runs reached the expected count 40.
The full measured payload/load/lifecycle results are preserved in
[evidence](evidence/benchmark-node.json), [Bun report](evidence/benchmark-bun.json)
and [Rust-host report](evidence/benchmark-rust-host.json).
