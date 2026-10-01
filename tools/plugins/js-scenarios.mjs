import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {TrustedPluginHost} from '@napi-vm/plugin-host';
const [greeterDir,counterDir,runtime]=process.argv.slice(2);
const load=async (dir,name)=>JSON.parse(await readFile(join(dir,'contracts',name+'.contract.json'),'utf8'));
const greeter=await load(greeterDir,'greeter'),config=await load(greeterDir,'app-configuration'),counter=await load(counterDir,'counter');
const host=new TrustedPluginHost({contracts:[greeter,config,counter]});host.registerService(config,{get:async()=>({value:'Hola'})});
try{const greeting=await host.load(join(greeterDir,'plugin.json'),{runtime});assert.equal((await greeting.invoke(greeter,'greet',{name:'Ana'})).message,'Hola, Ana');await assert.rejects(greeting.invoke(greeter,'greet',{name:'invalid'}),e=>e.code==='APPLICATION_ERROR'&&e.domainCode==='INVALID_NAME');await host.unload(greeting.instanceId);
const handle=await host.load(join(counterDir,'plugin.json'),{runtime});const client=handle.client(counter);let acceptEvent;let eventTimer;const queue=[];const cancel=handle.subscribe(counter,'changed',payload=>{if(acceptEvent){const r=acceptEvent;acceptEvent=undefined;clearTimeout(eventTimer);r(payload);}else queue.push(payload);});const nextEvent=()=>queue.length?Promise.resolve(queue.shift()):new Promise((resolve,reject)=>{acceptEvent=resolve;eventTimer=setTimeout(()=>{acceptEvent=undefined;reject(new Error('event deadline'));},2000);});
const initialSession=handle.sessionId;assert.equal((await client.add({amount:12})).count,'12');const firstEvent=await nextEvent();assert.equal(firstEvent.count,'12');await host.reload(handle.instanceId);assert.notEqual(handle.sessionId,initialSession);assert.equal((await client.get({})).count,'12');assert.equal((await client.add({amount:3})).count,'15');const secondEvent=await nextEvent();assert.equal(secondEvent.count,'15');cancel();clearTimeout(eventTimer);console.log(JSON.stringify({greeting:'Hola, Ana',domainError:'INVALID_NAME',firstEvent,secondEvent,stableInstance:handle.instanceId,sessions:[initialSession,handle.sessionId],restored:'12',final:'15'}));
}finally{await host.shutdown();}
