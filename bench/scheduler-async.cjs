// Usage: node bench/scheduler-async.cjs [persistent]
const {performance} = require('node:perf_hooks');
const binding = require('../index.js');
(async () => {
  const persistent = process.argv[2] === 'persistent';
  const vm = persistent ? new binding.AsyncSession() : new binding.Vm();
  let admissionRetries = 0;
  const run = async source => {
    for (let attempts = 0; ; attempts++) {
      try { return await (persistent ? vm.run(source) : vm.runAsync(source)); }
      catch (error) {
        if (!/VM is busy with another execution/.test(error.message) || attempts >= 1000) throw error;
        admissionRetries++;
        await new Promise(resolve => setImmediate(resolve));
      }
    }
  };
  try {
    for(let i=0;i<20;i++) await run('1+1;');
    const times = [];
    const wakesBefore = persistent ? vm.wakeups() : null;
    const start = performance.now();
    const cpu = process.cpuUsage();
    for(let i=0;i<500;i++) {
      const t=performance.now();
      if(await run('1+1;') !== '2') throw Error('incorrect result');
      times.push(performance.now()-t);
    }
    const wall=performance.now()-start;
    const used=process.cpuUsage(cpu);
    times.sort((a,b)=>a-b);
    const idleStart=process.cpuUsage();
    await new Promise(resolve=>setTimeout(resolve,250));
    const idle=process.cpuUsage(idleStart);
    console.log(JSON.stringify({workload:'trivial-async',mode:persistent?'persistent':'legacy',calls:500,
      admission_retries:admissionRetries,throughput_calls_s:500000/wall,p50_ms:times[250],p95_ms:times[475],p99_ms:times[495],cpu_ms:(used.user+used.system)/1000,
      idle_cpu_ms_per_250ms:(idle.user+idle.system)/1000,allocations:null,timer_lateness_ms:null,queue_depth_peak:1,wakeups:persistent?vm.wakeups()-wakesBefore:null}));
    if (persistent) await vm.exposeFunction('answer', async () => 42, true);
    else vm.exposeAsyncFunction('answer', async () => 42);
    for (let i = 0; i < 20; i++) await run('await answer();');
    const hostTimes = [];
    const hostRetries = admissionRetries;
    const hostCpu = process.cpuUsage();
    const hostStart = performance.now();
    for (let i = 0; i < 500; i++) {
      const started = performance.now();
      if (await run('await answer();') !== '42') throw Error('incorrect host result');
      hostTimes.push(performance.now() - started);
    }
    const hostWall = performance.now() - hostStart;
    const hostUsed = process.cpuUsage(hostCpu);
    hostTimes.sort((a, b) => a - b);
    console.log(JSON.stringify({workload:'awaited-host-async',mode:persistent?'persistent':'legacy',calls:500,
      admission_retries:admissionRetries-hostRetries,throughput_calls_s:500000/hostWall,
      p50_ms:hostTimes[250],p95_ms:hostTimes[475],p99_ms:hostTimes[495],
      cpu_ms:(hostUsed.user+hostUsed.system)/1000,allocations:null,idle_cpu_ms_per_250ms:null}));
  } finally { vm.dispose(); }
})().catch(e=>{console.error(e);process.exitCode=1;});
