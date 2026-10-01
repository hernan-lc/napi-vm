const {test} = require('node:test');
const assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const {Vm, AsyncSession, runCode} = require('../../index.js');

test('public owners use the same bytecode tier and reuse prepared source', async () => {
  const vm = new Vm(), session = new AsyncSession();
  const source = 'var n=(typeof n === "undefined"?0:n)+1;n;';
  try {
    for (let i=1;i<=5;i++) {
      assert.equal(vm.run(source), String(i));
      assert.equal(await session.run(source), String(i));
    }
    for (const stats of [JSON.parse(vm.evaluationStats()), JSON.parse(await session.evaluationStats())]) {
      assert.equal(stats.tier,'bytecode');assert.equal(stats.cacheHits,4);assert.equal(stats.cacheMisses,1);
    }
  } finally {vm.dispose();session.dispose();}
});
test('legacy sync and async calls share globals closures modules symbols and shapes', async () => {
  const vm = new Vm();
  try {
    vm.run('var counter=0;var fn=(()=>{var n=40;return ()=>++n;})();var sym=Symbol.for("shared");var obj={x:3};');
    vm.defineModule('m','export let n=7;export function increment(){return ++n;}');
    vm.exposeFunction('host', x=>x+1);
    for (let i=0;i<100;i++) {
      assert.equal(await vm.runAsync('counter++;var cycle={};cycle.self=cycle;obj.x+counter;'), String(4+i));
      assert.equal(vm.run('counter;'),String(i+1));
      vm.collectCycles();
    }
    assert.equal(await vm.runAsync('sym===Symbol.for("shared");'),'true');
    assert.equal(await vm.runAsync('fn();'),'41');assert.equal(vm.run('fn();'),'42');
    assert.equal(await vm.runAsync('import {increment} from "m";host(increment());'),'9');
  } finally {vm.dispose();}
});
test('released exports stay bounded and finalization cannot wait for a Node-dependent worker',()=>{
  const child=spawnSync(process.execPath,['--expose-gc','-e',`
    const {Vm}=require('./index.js');
    (async()=>{
      const vm=new Vm();vm.run('function make(){return ()=>42;}');
      for(let i=0;i<2000;i++){vm.callFunction('make',[]);if(i%50===0)global.gc();}
      vm.exposeAsyncFunction('host',async()=>{global.gc();await new Promise(r=>setImmediate(r));return 42;});
      if(await vm.runAsync('await host();')!=='42')throw Error('wrong value');
      vm.collectCycles();vm.dispose();
    })().catch(e=>{console.error(e);process.exitCode=1});
  `],{cwd:require('node:path').resolve(__dirname,'../..'),timeout:10000,encoding:'utf8'});
  assert.equal(child.status,0,child.stderr||String(child.error));
});
test('fresh cycle workloads stabilize without sharing mutable realm objects',()=>{
  const source='var o={};o.self=o;class C {}new C();42;';
  for(let i=0;i<1000;i++)assert.equal(runCode(source),'42');
  for(let i=0;i<1000;i++)assert.equal(runCode('Map.prototype=null;var m=new Map();var p=Object.getPrototypeOf(m);p.self=p;42;'),'42');
  assert.equal(runCode('Map.prototype.leaked=1;42;'),'42');
  assert.equal(runCode('typeof Map.prototype.leaked;'),'undefined');
});
test('automatic collection bounds retained cyclic containers in a persistent owner',()=>{
  const vm=new Vm();
  const source='var value={};value.self=value;42;';
  try {
    for(let i=0;i<1000;i++)vm.run(source);
    vm.collectCycles();
    const before=JSON.parse(vm.heapStats()).tracked;
    for(let i=0;i<20000;i++)vm.run(source);
    const after=JSON.parse(vm.heapStats()).tracked;
    assert.ok(after-before<4096,`retained ${after-before} cyclic containers`);
    assert.ok(JSON.parse(vm.heapStats()).collectedTotal>0);
  }finally{vm.dispose();}
});
