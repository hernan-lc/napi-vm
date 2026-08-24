# Sandbox limits and crash safety

"Sandboxed" means both isolation and containment. Guest code cannot see
Node's `require`, filesystem, network, or process globals by default, and
known process-killing vectors are converted into catchable guest errors.

Run the executable safety catalogue with:

```bash
bun examples/crash.ts
```

| Vector | Behavior | Guard |
|--------|----------|-------|
| Deep recursion | Catchable `RangeError` | Call-depth counter, checked before every VM frame |
| Deep parsing | Catchable `RangeError` | Parser nesting cap and depth latch |
| Cyclic structures | Safe stringification | Visited pointers and depth cap |
| Cyclic JSON | Catchable `TypeError` | Visited set plus JSON parse/stringify depth cap |
| Deep teardown | Clean shutdown | Iterative `Drop` work stack, not native recursion |
| Array exhaustion | Catchable `RangeError` | Hard array-length cap on push, spread, and growth paths |
| String exhaustion | Catchable `RangeError` | Hard string-size cap on concat, repeat, and join paths |
| Infinite loops | Catchable `RangeError` | Per-execution loop budget, configurable with `setLoopLimit` |
| Generator misuse | Safe completion | Scoped generator execution and controlled suspension |
| Runtime/host errors | Catchable error objects | Error propagation into guest `catch` |

Host capabilities are opt-in and enforced host-side. The plugin host in
`plugins/` shows the intended shape: a manifest *requests* filesystem access,
the host policy decides, and every privileged call canonicalizes its path —
resolving `..` and symlinks — before matching it against the granted patterns.
See [Plugins](plugins.md).

The in-process VM is not a complete operating-system security boundary. CPU
time is bounded per execution but synchronous code blocks the Node event loop,
and allocation caps are not an aggregate heap quota. For strict CPU or memory
limits, run the VM in a worker or disposable child process with a watchdog and
OS limits such as cgroups or `ulimit`.
