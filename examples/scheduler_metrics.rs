//! Additional metrics requiring the new clock/wake APIs, deliberately separate
//! from the common baseline benchmark. Run after all builds have completed.
use napi_vm::{Clock, ClockMode, HostBridge, Interpreter, RealTimeClock, Value, VmErr, WakeSignal};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
struct TimingBridge {
    clock: Rc<RealTimeClock>,
    lateness: RefCell<Vec<f64>>,
}
impl HostBridge for TimingBridge {
    fn call_host(&self, _: usize, args: Vec<Value>) -> Result<Value, VmErr> {
        let Value::Number(deadline) = args[0] else {
            panic!()
        };
        self.lateness
            .borrow_mut()
            .push((self.clock.now_ms() - deadline) * 1000.0);
        Ok(Value::Undefined)
    }
}
#[cfg(unix)]
fn cpu_us() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage initializes the out pointer on success; no guest values.
    assert_eq!(
        unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) },
        0
    );
    let usage = unsafe { usage.assume_init() };
    (usage.ru_utime.tv_sec + usage.ru_stime.tv_sec) as f64 * 1e6
        + (usage.ru_utime.tv_usec + usage.ru_stime.tv_usec) as f64
}
#[cfg(not(unix))]
fn cpu_us() -> f64 {
    0.0
}
fn percentiles(values: &mut [f64]) -> [f64; 3] {
    values.sort_by(f64::total_cmp);
    [
        values[values.len() / 2],
        values[values.len() * 95 / 100],
        values[values.len() * 99 / 100],
    ]
}
fn main() {
    let mut lateness = Vec::new();
    let start = Instant::now();
    let mut peak = 0;
    for _ in 0..25 {
        let mut vm = Interpreter::with_builtins();
        let clock = Rc::new(RealTimeClock::default());
        let host = Rc::new(TimingBridge {
            clock: clock.clone(),
            lateness: RefCell::new(Vec::with_capacity(100)),
        });
        vm.set_host_bridge(host.clone());
        vm.jobs
            .borrow_mut()
            .set_clock(ClockMode::RealTime(clock.clone()))
            .unwrap();
        for i in 0..100 {
            let delay = (i % 5 + 1) as f64;
            let deadline = clock.now_ms() + delay;
            vm.jobs.borrow_mut().push_timer(
                delay,
                Value::host_function("mark", 0),
                vec![Value::Number(deadline)],
            );
        }
        while !vm.jobs.borrow().is_empty() {
            vm.run_event_loop_once(Duration::from_millis(20)).unwrap();
        }
        peak = peak.max(vm.jobs.borrow().peak_depth());
        lateness.extend(host.lateness.borrow().iter().copied());
    }
    let throughput = 2500.0 / start.elapsed().as_secs_f64();
    let p = percentiles(&mut lateness);
    println!(
        "{}",
        serde_json::json!({"workload":"real-time/timers","samples":lateness.len(),"throughput_jobs_s":throughput,"lateness_p50_us":p[0],"lateness_p95_us":p[1],"lateness_p99_us":p[2],"queue_depth_peak":peak})
    );
    let signal = Arc::new(WakeSignal::default());
    let (tx, rx) = mpsc::sync_channel(32);
    let wake = signal.clone();
    let start = Instant::now();
    let producer = std::thread::spawn(move || {
        for i in 0..10000 {
            tx.send((i, Instant::now())).unwrap();
            wake.fire();
        }
    });
    let mut latencies = Vec::with_capacity(10000);
    let mut received = 0;
    while received < 10000 {
        while let Ok((id, sent)) = rx.try_recv() {
            assert_eq!(id, received);
            received += 1;
            latencies.push(sent.elapsed().as_secs_f64() * 1e6);
        }
        if received < 10000 {
            signal.wait(None);
        }
    }
    producer.join().unwrap();
    let throughput = 10000.0 / start.elapsed().as_secs_f64();
    let p = percentiles(&mut latencies);
    let idle_cpu = cpu_us();
    signal.wait(Some(Duration::from_millis(250)));
    let idle_cpu = cpu_us() - idle_cpu;
    println!(
        "{}",
        serde_json::json!({"workload":"wake/ingress","samples":latencies.len(),"throughput_events_s":throughput,"latency_p50_us":p[0],"latency_p95_us":p[1],"latency_p99_us":p[2],"wakeups":signal.wakeups(),"queue_capacity":32,"idle_cpu_us_per_250ms":idle_cpu})
    );
    for (name, source) in [
        (
            "vm/timers",
            "for(var i=0;i<1000;i++)setTimeout(()=>1,i%17);",
        ),
        (
            "vm/cancel",
            "for(var i=0;i<1000;i++){var id=setTimeout(()=>1,i);clearTimeout(id);}",
        ),
        (
            "vm/promises",
            "var p=Promise.resolve();for(var i=0;i<1000;i++)p=p.then(()=>1);",
        ),
    ] {
        let mut vm = Interpreter::with_builtins();
        vm.eval_source(source).unwrap();
        println!(
            "{}",
            serde_json::json!({"workload":name,"queue_depth_peak":vm.jobs.borrow().peak_depth()})
        );
    }
}
