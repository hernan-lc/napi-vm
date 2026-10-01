import test from 'node:test';import assert from 'node:assert/strict';import {readFile}from'node:fs/promises';
import {definePlugin,createHarness,PluginError,serve}from'../dist/index.js';
const root=new URL('../../../contracts/trusted-plugins/generated/',import.meta.url);const read=async n=>JSON.parse(await readFile(new URL(n+'.contract.json',root),'utf8'));const [G,C]=await Promise.all([read('greeter'),read('app-configuration')]);
test('SDK definition is side-effect-free and harness validates callbacks/domain errors',async()=>{
 const definition=definePlugin(G,{greet:async(input,ctx)=>{if(input.name==='bad')throw PluginError.domain('INVALID_NAME',{reason:'test'});const config=await ctx.invoke(C,'get',{key:'greeting.prefix'});return{message:config.value+', '+input.name}}});
 const h=await createHarness(definition,{services:[definePlugin(C,{get:()=>({value:'Hola'})})]});try{assert.equal((await h.invoke(G,'greet',{name:'Ana'})).message,'Hola, Ana');await assert.rejects(h.invoke(G,'greet',{name:'bad'}),e=>e.code==='APPLICATION_ERROR'&&e.domainCode==='INVALID_NAME');await assert.rejects(h.invoke(G,'greet',{name:''}),e=>e.code==='INVALID_ARGUMENT');}finally{await h.shutdown();await h.shutdown();}
});
test('D06 invalid output and undeclared error are INVALID_RESULT',async()=>{
 for(const handler of [()=>({message:undefined}),()=>{throw PluginError.domain('UNKNOWN',{})}]){const h=await createHarness(definePlugin(G,{greet:handler}));try{await assert.rejects(h.invoke(G,'greet',{name:'Ana'}),e=>e.code==='INVALID_RESULT');}finally{await h.shutdown();}}
});
test('R11 startup without managed environment fails immediately',async()=>{await assert.rejects(serve(definePlugin(G,{greet:()=>({message:'x'})}),{metadata:{id:'example.greeter',version:'0.1.0'}}),/Missing managed bootstrap/);});
test('tracked background author tasks cancel and drain before snapshot',async()=>{
 let ended=false;const definition=definePlugin(G,{greet:()=>({message:'x'})},{initialize(_,ctx){void ctx.spawnTask(async signal=>{await new Promise(r=>signal.addEventListener('abort',r,{once:true}));ended=true;});}});
 const h=await createHarness(definition);assert.equal(ended,false);assert.equal(await h.snapshot(),null);assert.equal(ended,true);await h.shutdown();
});
test('harness caller cancellation settles promptly while ignored handler retains its slot',async()=>{
 let entered,release;const started=new Promise(r=>entered=r);const h=await createHarness(definePlugin(G,{greet:async()=>{entered();await new Promise(r=>release=r);return{message:'late'}}}));const controller=new AbortController();
 try{const first=h.invoke(G,'greet',{name:'Ana'},{signal:controller.signal});const cancelled=assert.rejects(first,e=>e.code==='CANCELLED');await started;controller.abort();await cancelled;await assert.rejects(h.invoke(G,'greet',{name:'Other'}),e=>e.code==='REENTRANT_CALL');}finally{release();await h.shutdown();}
});
test('harness validates host event injection and drains the event callback before snapshot',async()=>{
 const K=await read('counter');let seen;const delivered=new Promise(r=>seen=r);let count;
 const plugin=definePlugin(G,{greet:()=>({message:String(count)})},{initialize(_,ctx){ctx.subscribe(K,'changed',payload=>{count=payload.count;seen()})}});
 const h=await createHarness(plugin,{services:[definePlugin(K,{get:()=>({count:'0'}),add:()=>({count:'0'})})]});
 try{await assert.rejects(h.emitHostEvent(K,'changed',{count:'bad'}),e=>e.code==='INVALID_ARGUMENT');await h.emitHostEvent(K,'changed',{count:'12'});await delivered;assert.equal((await h.invoke(G,'greet',{name:'Ana'})).message,'12');await h.snapshot();}finally{await h.shutdown();}
});
test('wire timeout remaining duration is a positive integer for Rust callbacks',async()=>{
 const {invokeRemote}=await import('../dist/index.js');let transmitted;
 const peer={endpointId:'p:session',sessionId:'session',limits:{callTimeoutMs:30000},async request(_method,params){transmitted=params.context.timeoutMs;return{value:'ok'}}};
 assert.equal((await invokeRemote(peer,C,'get',{key:'prefix'},{context:{callChain:['p:session'],remainingMs:()=>12.875}})).value,'ok');assert.equal(transmitted,12);assert.equal(Number.isSafeInteger(transmitted),true);
});

test('harness preserves explicit null initialization data and defaults only absence',async()=>{
 for(const options of [{configuration:null,context:null},{}]){let seen;const h=await createHarness(definePlugin(G,{greet:()=>({message:'ok'})},{initialize(input){seen=input;}}),options);try{assert.equal(JSON.stringify(seen.configuration),Object.hasOwn(options,'configuration')?'null':'{}');assert.equal(JSON.stringify(seen.context),Object.hasOwn(options,'context')?'null':'{}');}finally{await h.shutdown();}}
});
