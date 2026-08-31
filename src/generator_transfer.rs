//! The generator thread-transfer boundary.
//!
//! Every `unsafe impl Send` in this crate that covers `Rc`/`RefCell` state
//! lives here, in one place, so the proof below can be checked against all of
//! them at once. Nothing else in the crate may add such an impl.
//!
//! # Why this is needed
//!
//! Generators are implemented with a dedicated OS thread that runs the
//! generator body and blocks at each `yield` (see
//! `interpreter::call::spawn_generator_thread`). That gives true mid-body
//! suspension — infinite generators, `yield` inside loops, `try`/`finally`
//! around a `yield` — without a CPS transform of the interpreter.
//!
//! The cost is that VM values, environments and AST nodes are reference
//! counted with `Rc`, whose refcount updates are **not** atomic. Moving them
//! to another thread is what `Send` exists to prevent, so the transfer types
//! assert it manually.
//!
//! # Safety proof
//!
//! The claim is: no two threads ever touch the same non-atomic refcount at the
//! same time. That rests on three invariants, all of which must hold together.
//!
//! 1. **Mutual exclusion while running.** The channel protocol makes the two
//!    threads strictly alternate. `generator_next` sends a `GenResume` and
//!    immediately blocks on `from_gen.recv()`; the generator thread only runs
//!    between receiving that resume and sending its next `GenYield`. Neither
//!    side executes VM code while the other does.
//!
//! 2. **Exclusive ownership of transferred values.** A value moved through
//!    `GeneratorValue` is not retained by the sender. The sender constructs it
//!    from a value it is handing over and does not read it afterwards, so the
//!    receiving thread has sole access to the refcounts it reaches.
//!
//! 3. **Teardown is joined, not concurrent.** This is the invariant that
//!    ordinary channel discipline does *not* give you, and it is the reason
//!    `GeneratorInner` owns a `JoinHandle`.
//!
//!    When the generator thread finishes it drops its interpreter, the body
//!    `Rc<Vec<Statement>>`, the closure `Env` and every intermediate value.
//!    Those `Rc`s are *clones of ones the main thread still holds*: the
//!    closure environment in particular is the enclosing scope, which the main
//!    thread keeps using. Without a join, the main thread resumes the moment
//!    it receives `Returned`/`Threw` while the generator thread is still
//!    running its drop glue — two threads decrementing the same non-atomic
//!    refcount. The same race exists on abandonment, where dropping the
//!    sender wakes the generator thread to unwind at an arbitrary point in
//!    the main thread's execution.
//!
//!    Both paths are therefore joined before the main thread proceeds:
//!    `generator_next` joins on any terminal outcome, and
//!    `GeneratorInner::drop` closes the channel and joins. A join is
//!    guaranteed to make progress because a generator thread is only ever
//!    blocked on `from_main.recv()`, which fails as soon as the sender is
//!    dropped.
//!
//! # Status: this proof does not currently hold in full
//!
//! Invariant 3 is enforced but **incomplete**, and this is measured, not
//! suspected. Running `tests/generator_stress.rs` under ThreadSanitizer:
//!
//! | Build                          | Races per full run |
//! |--------------------------------|--------------------|
//! | Without the joins (historical) | ~7.6               |
//! | With the joins (current)       | ~1.0               |
//!
//! The joins remove the large majority of the reported races, but one class
//! survives. Its shape is: a generator thread running its final
//! `drop_glue::<Interpreter>` -- releasing the closure `Environment` after its
//! last channel message -- overlapping the main thread's own
//! `Environment::drain_chain`. It reproduces most readily in
//! `generators_sharing_one_closure_environment` and
//! `nested_generators_abandoned_mid_flight`, i.e. where a generator shares a
//! closure environment with its driver and is abandoned rather than exhausted.
//!
//! Reproduce with:
//!
//! ```text
//! rustup component add rust-src --toolchain nightly
//! RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test -Zbuild-std \
//!     --release --no-default-features --test generator_stress \
//!     --target x86_64-unknown-linux-gnu -- --test-threads=1
//! ```
//!
//! Treat the `Send` impls below as a known-unsound-but-bounded compromise, not
//! as a discharged proof. The durable fix is to remove the thread boundary
//! entirely -- a same-thread resumable interpreter or a CPS transform of
//! generator bodies -- rather than to keep adding happens-before edges to a
//! design that needs them everywhere.
//!
//! # What is *not* proven
//!
//! These invariants are maintained by hand and are not checked by the
//! compiler. Any of the following would break them, and none of them will
//! produce a compile error:
//!
//! - letting the main thread run between a terminal `GenYield` and the join;
//! - retaining a value after wrapping it in `GeneratorValue`;
//! - giving the generator thread a second way to block, so that a join can
//!   deadlock and the join is then "optimised" away;
//! - sharing any other `Rc`-bearing type across the boundary by adding an
//!   `unsafe impl Send` elsewhere.
//!
//! Until the boundary is removed, changes to `interpreter::call`'s generator
//! functions should be reviewed against the three invariants above, and
//! `tests/generator_stress.rs` exercises the create/drop/abandon/nest paths
//! that the proof depends on.

use crate::value::{GenResume, GenYield, GeneratorInit, GeneratorValue};

// SAFETY: invariants 1 and 2 above. These carry values across the channel at a
// suspension point, at which exactly one of the two threads is running.
unsafe impl Send for GenResume {}
unsafe impl Send for GenYield {}
unsafe impl Send for GeneratorValue {}

// SAFETY: invariants 1-3 above. This is the one-time transfer of the body,
// closure and arguments to a freshly spawned generator thread. The spawning
// thread hands over ownership and then blocks; the receiving thread's eventual
// release of those `Rc`s is ordered before the main thread continues by the
// join described in invariant 3.
unsafe impl Send for GeneratorInit {}
