//! Serialized allocation benchmark. Build first; compare identical binaries/workloads.
use napi_vm::Interpreter;
#[path = "support/allocations.rs"]
mod allocations;
use allocations::{ALLOCS, BYTES};
use std::sync::atomic::Ordering;
struct Bridge;
impl napi_vm::host::HostBridge for Bridge {
    fn call_host(
        &self,
        _id: usize,
        args: Vec<napi_vm::Value>,
    ) -> Result<napi_vm::Value, napi_vm::VmErr> {
        Ok(args.first().cloned().unwrap_or(napi_vm::Value::Undefined))
    }
}
fn main() {
    for (name, setup, call) in [
        ("global-loop", "", "var n=0;for(var i=0;i<100;i++)n+=i;n;"),
        ("zero", "function f(){return 42;}", "f();"),
        ("two", "function f(x,y){return x+y;}", "f(20,22);"),
        ("method", "var o={f(x){return x+1;}};", "o.f(41);"),
        (
            "closure",
            "var f=(()=>{var x=41;return ()=>++x;})();",
            "f();",
        ),
        ("recursive", "function f(n){return n?f(n-1)+1:0;}", "f(10);"),
        ("native", "", "Math.abs(-42);"),
        ("host", "", "host(42);"),
        (
            "spread",
            "function f(x,y){return x+y;}var a=[20,22];",
            "f(...a);",
        ),
        ("arguments", "function f(){return arguments[0];}", "f(42);"),
        ("rest", "function f(...a){return a[0];}", "f(42);"),
    ] {
        let mut vm = Interpreter::with_builtins();
        vm.set_host_bridge(std::rc::Rc::new(Bridge));
        vm.global
            .borrow_mut()
            .set("host", napi_vm::Value::host_function("host", 1));
        vm.eval_source(setup).unwrap();
        let program = Interpreter::compile(call).unwrap();
        for _ in 0..100 {
            vm.execute(&program).unwrap();
        }
        let count = ALLOCS.load(Ordering::Relaxed);
        let bytes = BYTES.load(Ordering::Relaxed);
        let started = std::time::Instant::now();
        for _ in 0..10000 {
            std::hint::black_box(vm.execute(&program).unwrap());
        }
        let micros = started.elapsed().as_micros();
        println!(
            "{}",
            serde_json::json!({"workload":name,"operations":10000,"allocations":ALLOCS.load(Ordering::Relaxed)-count,"bytes":BYTES.load(Ordering::Relaxed)-bytes,"elapsed_us":micros,"bytecode":program.stats().is_some()})
        );
    }
}
