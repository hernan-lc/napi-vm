//! Queue-only schedule/drain and cancel/churn matrix (no parsing in timings).
use napi_vm::{Interpreter, Value};
#[path = "support/allocations.rs"]
mod allocations;
use allocations::{ALLOCS, BYTES};
use std::sync::atomic::Ordering;
fn main() {
    for depth in [1, 10, 32, 64, 100, 1000, 10000] {
        for mode in ["drain", "cancel", "partial-cancel"] {
            let vm = Interpreter::new();
            let allocations = ALLOCS.load(Ordering::Relaxed);
            let bytes = BYTES.load(Ordering::Relaxed);
            let started = std::time::Instant::now();
            let repeats = (100000 / depth).max(10);
            for _ in 0..repeats {
                let mut q = vm.jobs.borrow_mut();
                let mut ids = Vec::with_capacity(depth);
                for i in 0..depth {
                    ids.push(q.push_timer((i % 7) as f64, Value::Undefined, vec![]));
                }
                for (i, id) in ids.into_iter().enumerate() {
                    if mode == "cancel" || mode == "partial-cancel" && i % 3 == 0 {
                        q.cancel_timer(id);
                    }
                }
                while q.take_timer().is_some() {}
            }
            println!(
                "{}",
                serde_json::json!({"workload":mode,"depth":depth,"repeats":repeats,"elapsed_us":started.elapsed().as_micros(),"peak":vm.jobs.borrow().peak_depth(),"allocations": ALLOCS.load(Ordering::Relaxed)-allocations,"bytes":BYTES.load(Ordering::Relaxed)-bytes})
            );
        }
    }
}
