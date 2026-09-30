use napi_vm::interpreter::{ExecutionBudget, Job};
use napi_vm::{HostBridge, HostEvent, Interpreter, Value, VmErr};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

struct ProbeBridge {
    polls: Cell<usize>,
    blocking: Cell<usize>,
    events: RefCell<Vec<HostEvent>>,
}
impl HostBridge for ProbeBridge {
    fn call_host(&self, _: usize, _: Vec<Value>) -> Result<Value, VmErr> {
        unreachable!()
    }
    fn poll_host_events(&self, timeout: Duration) -> Result<Vec<HostEvent>, VmErr> {
        self.polls.set(self.polls.get() + 1);
        if !timeout.is_zero() {
            self.blocking.set(self.blocking.get() + 1);
        }
        Ok(std::mem::take(&mut *self.events.borrow_mut()))
    }
}
fn probe() -> Rc<ProbeBridge> {
    Rc::new(ProbeBridge {
        polls: Cell::new(0),
        blocking: Cell::new(0),
        events: RefCell::new(vec![]),
    })
}
fn callback(vm: &mut Interpreter, source: &str) -> Value {
    vm.eval_source(source).unwrap()
}
#[test]
fn hard_job_boundary_keeps_next_job() {
    let mut vm = Interpreter::with_builtins();
    let cb = callback(&mut vm, "var hits=0; ()=>hits++;");
    for _ in 0..2 {
        vm.jobs.borrow_mut().push_microtask(Job::Callback {
            callback: cb.clone(),
            args: vec![],
        });
    }
    vm.set_execution_budget(ExecutionBudget {
        max_jobs: 1,
        ..ExecutionBudget::default()
    });
    assert!(
        vm.drain_jobs()
            .unwrap_err()
            .to_string()
            .contains("Maximum job count")
    );
    assert!(vm.jobs.borrow().has_microtasks());
    assert!(matches!(
        vm.global.borrow().get("hits"),
        Some(Value::Number(1.0))
    ));
}
#[test]
fn queued_work_counts_as_progress_without_waiting() {
    for micro in [true, false] {
        let mut vm = Interpreter::with_builtins();
        let cb = callback(&mut vm, "()=>1;");
        let p = probe();
        vm.set_host_bridge(p.clone());
        let job = Job::Callback {
            callback: cb,
            args: vec![],
        };
        if micro {
            vm.jobs.borrow_mut().push_microtask(job);
        } else {
            vm.jobs.borrow_mut().push_timer_job(0., job);
        }
        assert!(vm.run_event_loop_once(Duration::from_secs(1)).unwrap());
        assert_eq!(p.blocking.get(), 0);
        assert!(!vm.run_event_loop_once(Duration::ZERO).unwrap());
    }
}
#[test]
fn host_is_sampled_at_checkpoints_not_per_microtask() {
    let mut vm = Interpreter::with_builtins();
    let cb = callback(&mut vm, "var hits=0; ()=>hits++;");
    let p = probe();
    vm.set_host_bridge(p.clone());
    for _ in 0..100 {
        vm.jobs.borrow_mut().push_microtask(Job::Callback {
            callback: cb.clone(),
            args: vec![],
        });
    }
    vm.drain_jobs().unwrap();
    assert_eq!(p.polls.get(), 1);
    assert!(matches!(
        vm.global.borrow().get("hits"),
        Some(Value::Number(100.0))
    ));
}
#[test]
fn nested_timers_and_recursive_microtasks_keep_order() {
    let mut vm = Interpreter::with_builtins();
    vm.eval_source("var seen=[]; setTimeout(()=>{seen.push('a'); queueMicrotask(()=>{seen.push('micro');queueMicrotask(()=>seen.push('nested'));});setTimeout(()=>seen.push('inner'),0);},1); setTimeout(()=>seen.push('b'),1);").unwrap();
    assert!(
        matches!(vm.eval_source("seen.join(',')").unwrap(),Value::String(ref s) if s=="a,micro,nested,inner,b")
    );
}

use napi_vm::{
    CancellationToken, Clock, ClockMode, EventLoopOptions, Fairness, RealTimeClock, TurnBudget,
    VirtualClock, YieldReason,
};
#[test]
fn virtual_deadlines_and_nested_timers_are_absolute() {
    let mut vm = Interpreter::with_builtins();
    let clock = VirtualClock::default();
    vm.jobs
        .borrow_mut()
        .set_clock(ClockMode::Virtual(clock.clone()))
        .unwrap();
    vm.eval_source(
        "var seen=[];setTimeout(()=>{seen.push('a');setTimeout(()=>seen.push('b'),5);},10);",
    )
    .unwrap();
    assert_eq!(
        vm.poll_event_loop(TurnBudget::jobs(10))
            .unwrap()
            .next_deadline,
        Some(10.)
    );
    clock.advance(9.).unwrap();
    assert_eq!(
        vm.poll_event_loop(TurnBudget::jobs(10))
            .unwrap()
            .executed_jobs,
        0
    );
    clock.advance(1.).unwrap();
    let turn = vm.poll_event_loop(TurnBudget::jobs(10)).unwrap();
    assert_eq!(turn.executed_jobs, 1);
    assert_eq!(turn.next_deadline, Some(15.));
    assert!(!turn.runnable);
    clock.advance(5.).unwrap();
    assert_eq!(
        vm.poll_event_loop(TurnBudget::jobs(10))
            .unwrap()
            .executed_jobs,
        1
    );
    assert!(matches!(vm.eval_source("seen.join(',')").unwrap(),Value::String(ref s) if s=="a,b"));
    for invalid in [-1., f64::NAN, f64::INFINITY] {
        assert!(clock.advance(invalid).is_err());
    }
    assert_eq!(clock.now_ms(), 15.);
}
#[test]
fn soft_yield_resumes_microtasks_before_timers_or_new_evaluation() {
    let mut vm = Interpreter::with_builtins();
    let cb = callback(
        &mut vm,
        "var seen=[];()=>{seen.push('a');queueMicrotask(()=>{seen.push('m');queueMicrotask(()=>seen.push('n'));});}",
    );
    vm.jobs.borrow_mut().push_timer(0., cb, vec![]);
    let cb = callback(&mut vm, "()=>seen.push('b')"); // previous timer completes before installing this one
    // Clear prior observations and queue a fresh macro that creates two microtasks.
    vm.eval_source("seen=[]").unwrap();
    let cb_a = callback(
        &mut vm,
        "()=>{seen.push('a');queueMicrotask(()=>{seen.push('m');queueMicrotask(()=>seen.push('n'));});}",
    );
    vm.jobs.borrow_mut().push_timer(0., cb_a, vec![]);
    vm.jobs.borrow_mut().push_timer(0., cb, vec![]);
    let first = vm.poll_event_loop(TurnBudget::jobs(1)).unwrap();
    assert!(first.checkpoint_pending);
    assert!(
        vm.eval_source("seen.push('intruder')")
            .unwrap_err()
            .to_string()
            .contains("checkpoint")
    );
    for _ in 0..3 {
        vm.poll_event_loop(TurnBudget::jobs(1)).unwrap();
    }
    assert!(
        matches!(vm.eval_source("seen.join(',')").unwrap(),Value::String(ref s) if s=="a,m,n,b")
    );
}
#[test]
fn hard_limits_survive_soft_yields() {
    let mut vm = Interpreter::with_builtins();
    let cb = callback(&mut vm, "var hits=0;()=>hits++;");
    vm.set_execution_budget(ExecutionBudget {
        max_jobs: 2,
        ..ExecutionBudget::default()
    });
    for _ in 0..3 {
        vm.jobs.borrow_mut().push_microtask(Job::Callback {
            callback: cb.clone(),
            args: vec![],
        });
    }
    for _ in 0..2 {
        assert_eq!(
            vm.poll_event_loop(TurnBudget::jobs(1))
                .unwrap()
                .executed_jobs,
            1
        );
    }
    vm.begin_execution(); // a pending checkpoint must not refill the budget
    assert!(vm.poll_event_loop(TurnBudget::jobs(1)).is_err());
    assert!(vm.jobs.borrow().has_microtasks());
}
#[test]
fn zero_and_time_budgets_keep_the_next_job() {
    let mut vm = Interpreter::with_builtins();
    let cb = callback(&mut vm, "()=>1");
    vm.jobs.borrow_mut().push_timer(0., cb, vec![]);
    let outcome = vm.poll_event_loop(TurnBudget::jobs(0)).unwrap();
    assert_eq!(outcome.executed_jobs, 0);
    assert!(outcome.runnable);
    let outcome = vm
        .poll_event_loop(TurnBudget {
            max_jobs: 10,
            max_duration: Some(Duration::ZERO),
        })
        .unwrap();
    assert_eq!(outcome.yield_reason, YieldReason::TimeBudget);
    assert!(outcome.runnable);
    assert_eq!(
        vm.poll_event_loop(TurnBudget::jobs(1))
            .unwrap()
            .executed_jobs,
        1
    );
}
#[test]
fn real_time_waits_only_for_due_timers_and_reports_progress() {
    let mut vm = Interpreter::with_builtins();
    let cb = callback(&mut vm, "var hit=false;()=>{hit=true}");
    vm.jobs
        .borrow_mut()
        .set_clock(ClockMode::RealTime(Rc::new(RealTimeClock::default())))
        .unwrap();
    vm.jobs.borrow_mut().push_timer(20., cb, vec![]);
    assert_eq!(
        vm.poll_event_loop(TurnBudget::jobs(10))
            .unwrap()
            .executed_jobs,
        0
    );
    assert!(vm.run_event_loop_once(Duration::from_secs(1)).unwrap());
    assert!(matches!(
        vm.global.borrow().get("hit"),
        Some(Value::Bool(true))
    ));
}
struct FloodBridge {
    callback: Value,
}
impl HostBridge for FloodBridge {
    fn call_host(&self, _: usize, _: Vec<Value>) -> Result<Value, VmErr> {
        unreachable!()
    }
    fn poll_host_events_bounded(&self, _: Duration, limit: usize) -> Result<Vec<HostEvent>, VmErr> {
        assert_eq!(limit, 1);
        Ok(vec![HostEvent::Callback(napi_vm::HostCallback {
            callback: self.callback.clone(),
            this_value: Value::Undefined,
            args: vec![],
            kind: napi_vm::HostCallbackKind::Call,
        })])
    }
}
#[test]
fn opt_in_alternation_prevents_timer_starvation_under_host_flood() {
    let mut vm = Interpreter::with_builtins();
    let e = callback(&mut vm, "var seen=[];()=>seen.push('e')");
    let t = callback(&mut vm, "()=>seen.push('t')");
    vm.set_event_loop_options(EventLoopOptions {
        fairness: Fairness::Alternate,
        host_batch_size: 1,
        external_capacity: 2,
    })
    .unwrap();
    vm.set_host_bridge(Rc::new(FloodBridge { callback: e }));
    vm.jobs.borrow_mut().push_timer(0., t.clone(), vec![]);
    vm.jobs.borrow_mut().push_timer(0., t, vec![]);
    assert_eq!(
        vm.poll_event_loop(TurnBudget::jobs(4))
            .unwrap()
            .executed_jobs,
        4
    );
    vm.host = None;
    // Read without admitting another evaluation/macrotask.
    let seen = vm.global.borrow().get("seen").unwrap();
    assert_eq!(napi_vm::format::to_string(&seen), "[e, t, e, t]");
    assert!(vm.jobs.borrow().external_len() <= 2);
}
#[test]
fn oversized_legacy_ingress_reports_backpressure_without_losing_work() {
    let mut vm = Interpreter::with_builtins();
    let cb = callback(&mut vm, "var hits=0;()=>hits++");
    let p = probe();
    for _ in 0..7 {
        p.events
            .borrow_mut()
            .push(HostEvent::Callback(napi_vm::HostCallback {
                callback: cb.clone(),
                this_value: Value::Undefined,
                args: vec![],
                kind: napi_vm::HostCallbackKind::Call,
            }));
    }
    vm.set_event_loop_options(EventLoopOptions {
        external_capacity: 2,
        host_batch_size: 1,
        ..EventLoopOptions::default()
    })
    .unwrap();
    vm.set_host_bridge(p);
    let turn = vm.poll_event_loop(TurnBudget::jobs(1)).unwrap();
    assert_eq!(turn.yield_reason, YieldReason::Backpressure);
    assert!(vm.jobs.borrow().external_len() <= 2);
    vm.drain_jobs().unwrap();
    assert!(matches!(
        vm.global.borrow().get("hits"),
        Some(Value::Number(7.0))
    ));
}
#[test]
fn cancellation_and_deadlines_interrupt_long_callbacks_and_coroutine_bodies() {
    let mut vm = Interpreter::with_builtins();
    let cb = callback(&mut vm, "()=>{while(true){}};");
    vm.jobs.borrow_mut().push_timer(0., cb, vec![]);
    vm.set_execution_timeout(Some(Duration::from_millis(5)));
    assert!(
        vm.poll_event_loop(TurnBudget::jobs(1))
            .unwrap_err()
            .to_string()
            .contains("deadline")
    );
    vm.set_execution_timeout(None);
    let token = CancellationToken::default();
    vm.set_cancellation_token(token.clone());
    let producer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(5));
        token.cancel();
    });
    assert!(
        vm.eval_source("async function spin(){while(true){}} spin();")
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    producer.join().unwrap();
}
