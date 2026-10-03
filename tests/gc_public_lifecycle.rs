use napi_vm::{Interpreter, Value};
#[test]
fn pins_and_multiple_interpreters_survive_collection() {
    let mut first = Interpreter::with_builtins();
    let mut second = Interpreter::with_builtins();
    let value = first.eval_source("var o={answer:42};o.self=o;o;").unwrap();
    let pin = napi_vm::heap::RootPin::new(value.clone());
    first.eval_source("o=undefined;").unwrap();
    second
        .eval_source("var keep={answer:7};keep.self=keep;")
        .unwrap();
    first.collect_cycles();
    assert!(matches!(value.get_prop("answer"), Some(Value::Number(42.))));
    assert!(matches!(
        second.eval_source("keep.answer;").unwrap(),
        Value::Number(7.)
    ));
    drop(value);
    drop(pin);
    assert!(first.collect_cycles().collected > 0);
}
#[test]
fn debt_and_suspended_generator_barrier() {
    let mut vm = Interpreter::with_builtins();
    vm.set_collection_threshold(1);
    vm.eval_source("var g=(function*(){var o={};o.self=o;yield o;})();g.next();")
        .unwrap();
    let collection = vm.maybe_collect_cycles().unwrap();
    #[cfg(stackful_coroutines)]
    assert!(collection.skipped.is_some());
    // Buffered generators store yielded values in the traced heap instead of
    // an opaque suspended stack, so collection can proceed on Windows ARM.
    #[cfg(not(stackful_coroutines))]
    assert!(collection.skipped.is_none());
    vm.eval_source("g.return();g=undefined;").unwrap();
    assert!(vm.maybe_collect_cycles().unwrap().skipped.is_none());
    assert!(vm.maybe_collect_cycles().is_none());
}
