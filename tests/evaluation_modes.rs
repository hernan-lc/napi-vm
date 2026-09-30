use napi_vm::interpreter::{DrainPolicy, EvaluationOptions};
use napi_vm::{ClockMode, Interpreter, Value, VirtualClock};

#[test]
fn evaluation_tiers_caching_and_drain_policies() {
    let mut vm = Interpreter::with_builtins();
    let source = "var count=(typeof count==='undefined'?0:count)+1;count;";
    assert!(matches!(vm.eval_source(source).unwrap(), Value::Number(1.)));
    assert!(matches!(vm.eval_source(source).unwrap(), Value::Number(2.)));
    let (hits, misses, entries, _) = vm.prepared_cache_stats();
    assert_eq!((hits, misses, entries), (1, 1, 1));
    assert!(vm.evaluation_diagnostics().contains("bytecode"));
    let clock = VirtualClock::default();
    vm.jobs
        .borrow_mut()
        .set_clock(ClockMode::Virtual(clock.clone()))
        .unwrap();
    vm.eval_source_with_options(
        "var seen=0;setTimeout(()=>seen++,10);queueMicrotask(()=>seen+=2);",
        EvaluationOptions {
            drain: DrainPolicy::None,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        vm.global.borrow().get("seen"),
        Some(Value::Number(0.))
    ));
    vm.eval_source_with_options(
        "",
        EvaluationOptions {
            drain: DrainPolicy::Microtasks,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        vm.global.borrow().get("seen"),
        Some(Value::Number(2.))
    ));
    clock.advance(10.).unwrap();
    vm.drain_jobs().unwrap();
    assert!(matches!(
        vm.global.borrow().get("seen"),
        Some(Value::Number(3.))
    ));
    for i in 0..80 {
        vm.eval_source(&format!("{i};")).unwrap();
    }
    assert!(vm.prepared_cache_stats().2 <= 64);
    assert!(vm.prepared_cache_stats().3 <= 2 * 1024 * 1024);
}

#[test]
fn lazy_source_lines_match_standard_lines() {
    for source in [
        "",
        "first\nmid\nlast",
        "first\r\nmid\r\nlast\r\n",
        "first\n",
        "last\r",
    ] {
        let mut vm = Interpreter::new();
        vm.set_source(source);
        assert_eq!(vm.get_source_line(0), None);
        let expected: Vec<_> = source.lines().collect();
        for (i, line) in expected.iter().enumerate() {
            assert_eq!(vm.get_source_line(i + 1), Some(*line));
        }
        assert_eq!(vm.get_source_line(expected.len() + 1), None);
    }
}

#[test]
fn compact_property_cache_verification() {
    use napi_vm::bytecode::{Instr, VerifyError, compile_program, verify_module};
    use napi_vm::lexer::Lexer;
    use napi_vm::parser::Parser;
    let statements =
        Parser::new_with_spans(Lexer::new("var o={x:1};o.x=2;o.x;").tokenize_with_spans())
            .parse_program()
            .unwrap();
    let mut module = compile_program(&statements).unwrap();
    assert_eq!(module.main.caches.len(), 2);
    assert!(module.main.caches.len() < module.main.code.len());
    let function = std::rc::Rc::make_mut(&mut module.main);
    for instr in &mut function.code {
        if let Instr::GetProp { cache, .. } = instr {
            *cache = u32::MAX;
            break;
        }
    }
    assert!(matches!(
        verify_module(&module),
        Err(VerifyError::BadCache { .. })
    ));
}

#[test]
fn host_panic_restores_bytecode_scope_and_allows_reuse() {
    struct PanickingBridge;
    impl napi_vm::host::HostBridge for PanickingBridge {
        fn call_host(&self, _: usize, _: Vec<Value>) -> Result<Value, napi_vm::VmErr> {
            panic!("deliberate host panic")
        }
    }
    let mut vm = Interpreter::with_builtins();
    vm.set_host_bridge(std::rc::Rc::new(PanickingBridge));
    vm.global
        .borrow_mut()
        .set("host", Value::host_function("host", 1));
    vm.eval_source("var keep=42;function f(){let local=7;return ()=>host();}var g=f();")
        .unwrap();
    let global = vm.global.clone();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| vm.eval_source("g();")));
    assert!(outcome.is_err());
    assert!(std::rc::Rc::ptr_eq(&global, &vm.global));
    assert!(matches!(
        vm.eval_source("keep;").unwrap(),
        Value::Number(42.)
    ));
}
