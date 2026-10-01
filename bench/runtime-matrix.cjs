// Build baseline and modified binaries first; interleave process runs externally.
'use strict';
const {performance}=require('node:perf_hooks');
const binding=require(process.env.RUNTIME_BINDING || '../index.js');
const workloads=[
  ['tiny','function f(x,y){return x+y;}f(20,22);','42'],
  ['arithmetic','var n=0;for(var i=0;i<1000;i++)n+=i;n;','499500'],
  ['closure','var f=(()=>{var n=0;return ()=>++n;})();f();','1'],
  ['class','class C{constructor(x){this.x=x;}f(){return this.x;}}new C(42).f();','42'],
  ['property','var o={x:0};for(var i=0;i<100;i++)o.x++;o.x;','100'],
  ['promise','var n=0;Promise.resolve(42).then(x=>n=x);42;','42'],
  ['cycle','var o={};o.self=o;42;','42'],
];
async function measure(mode,name,source,expected,make) {
  const {run,close,stats}=await make();
  try {
    for(let i=0;i<30;i++)if(String(await run(source))!==expected)throw Error(`${mode}/${name}: wrong result`);
    const rss=process.memoryUsage().rss,times=[],cpu=process.cpuUsage(),start=performance.now();
    for(let i=0;i<200;i++){
      const t=performance.now();const result=await run(source);times.push(performance.now()-t);
      if(String(result)!==expected)throw Error(`${mode}/${name}: wrong result`);
    }
    const wall=performance.now()-start,used=process.cpuUsage(cpu);times.sort((a,b)=>a-b);
    console.log(JSON.stringify({mode,workload:name,source_bytes:Buffer.byteLength(source),operations:200,ops_s:200000/wall,
      p50_ms:times[100],p95_ms:times[190],p99_ms:times[198],cpu_ms:(used.user+used.system)/1000,
      rss_before:rss,rss_after:process.memoryUsage().rss,diagnostics:await stats(),allocations:null,bytes_allocated:null}));
  }finally{await close();}
}
(async()=>{
  for(const [name,source,expected] of workloads) {
    for(const mode of ['runCode','VM.run','VM.runAsync','AsyncSession.run','AsyncSession.evaluate']) {
      await measure(mode,name,source,expected,async()=>{
        if(mode==='runCode')return {run:s=>binding.runCode(s),close:()=>{},stats:()=>null};
        const session=mode.startsWith('AsyncSession'),vm=session?new binding.AsyncSession():new binding.Vm();
        const method=mode.split('.')[1];
        return {run:async s=>{
          // Baseline runAsync can resolve before releasing admission; count wall time
          // including retries rather than masking the race with a fixed delay.
          for(let attempt=0;;attempt++)try{return await vm[method](s);}catch(e){if(attempt>=1000 || !/VM is busy/.test(e.message))throw e;await new Promise(r=>setImmediate(r));}
        },close:()=>vm.dispose(),stats:async()=>typeof vm.evaluationStats==='function'?JSON.parse(await vm.evaluationStats()):null};
      });
    }
  }
  for(const mode of ['callFunction','guest-to-host','module-import']) {
    const vm=new binding.Vm();
    vm.run('function answer(){return 42;}');vm.exposeFunction('host',()=>42);
    vm.defineModule('metrics','export const answer=42;');
    await measure(mode,mode,mode==='module-import'?'import {answer} from "metrics";answer;':'42;','42',async()=>({
      run:s=>mode==='callFunction'?vm.callFunction('answer',[]):vm.run(mode==='guest-to-host'?'host();':s),
      close:()=>vm.dispose(),stats:()=>typeof vm.evaluationStats==='function'?JSON.parse(vm.evaluationStats()):null,
    }));
  }
})().catch(e=>{console.error(e);process.exitCode=1;});
