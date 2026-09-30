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
