// Usage: node bench/scheduler-async.cjs [persistent]
const {performance} = require('node:perf_hooks');
const binding = require('../index.js');
(async () => {
  const persistent = process.argv[2] === 'persistent';
  const vm = persistent ? new binding.AsyncSession() : new binding.Vm();
  const run = source => persistent ? vm.run(source) : vm.runAsync(source);
  try {
    for(let i=0;i<20;i++) await run('1+1;');
    const times = [];
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
      throughput_calls_s:500000/wall,p50_ms:times[250],p95_ms:times[475],p99_ms:times[495],cpu_ms:(used.user+used.system)/1000,
      idle_cpu_ms_per_250ms:(idle.user+idle.system)/1000,allocations:null,timer_lateness_ms:null,queue_depth_upper_bound:1,wakeups:null}));
  } finally { vm.dispose(); }
})().catch(e=>{console.error(e);process.exitCode=1;});
