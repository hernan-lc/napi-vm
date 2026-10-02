import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer, connect } from 'node:net';
import { readFile } from 'node:fs/promises';
import { FrameDecoder, encodeFrame, wireValue, validate, assertContract, canonical, decimal, bytes, decodeBytes, RpcPeer, PluginError } from '../dist/index.js';
const counter=JSON.parse(await readFile(new URL('../../../contracts/trusted-plugins/generated/counter.contract.json',import.meta.url),'utf8'));
test('W01/W02 all fragmentation boundaries and coalesced Unicode frames',()=>{
 const values=[{value:'🙂漢字'},null,{a:[1,2,3]}], framed=Buffer.concat(values.map(v=>encodeFrame(v)));
 for(let i=1;i<framed.length;i++){const decoder=new FrameDecoder();const got=[...decoder.push(framed.subarray(0,i)),...decoder.push(framed.subarray(i))];assert.equal(JSON.stringify(got),JSON.stringify(values));decoder.finish();}
 const decoder=new FrameDecoder(); const got=[];for(const byte of framed)got.push(...decoder.push(Uint8Array.of(byte)));assert.equal(JSON.stringify(got),JSON.stringify(values));
});
test('W03/W04 bounded malformed framing, UTF8, JSON, nesting and truncation',()=>{
 for(const header of [Buffer.from([0,0,0,0]),Buffer.from([0,0,1,1])])assert.throws(()=>new FrameDecoder(256).push(header));
 const raw=body=>Buffer.concat([Buffer.from([0,0,0,body.length]),body]);
 for(const body of [Buffer.from([0xff]),Buffer.from('{'),Buffer.from('\ufeff{}')])assert.throws(()=>new FrameDecoder().push(raw(body)));
 const decoder=new FrameDecoder();decoder.push(Buffer.from([0]));assert.throws(()=>decoder.finish());
 assert.throws(()=>wireValue([[[[1]]]],2));assert.throws(()=>new FrameDecoder().push(Buffer.concat(Array.from({length:257},()=>encodeFrame(null)))));
});
test('W05 duplicate keys last wins; prototype-sensitive keys are data',()=>{
 const data=Buffer.from('{"a":1,"a":2,"__proto__":{"polluted":true}}');const frame=Buffer.alloc(4+data.length);frame.writeUInt32BE(data.length);data.copy(frame,4);
 const parsed=new FrameDecoder().push(frame)[0];assert.equal(parsed.a,2);assert.equal(parsed.__proto__.polluted,true);assert.equal({}.polluted,undefined);
});
test('D01-D05 data rejects undefined/sparse/class/getter/cycle and preserves finite semantics',()=>{
 let called=false;for(const value of [undefined,{x:undefined},[undefined],Array(1),new Date(),{get x(){called=true;return 1}},{toJSON(){called=true;return 'ok'}}])assert.throws(()=>wireValue(value));assert.equal(called,false);
 const cycle={};cycle.c=cycle;assert.throws(()=>wireValue(cycle));assert.throws(()=>wireValue('\ud800'));assert.throws(()=>wireValue('\udc00'));assert.equal(wireValue('🙂'),'🙂');assert.equal(Object.is(wireValue(-0),-0),false);assert.throws(()=>wireValue(Infinity));
 assert.equal(decimal(-(1n<<63n)),'-9223372036854775808');assert.throws(()=>decimal(1n<<63n));assert.equal(bytes(Uint8Array.of(0,255)),'AP8=');assert.deepEqual([...decodeBytes('AP8=')],[0,255]);assert.throws(()=>decodeBytes('AP8'));assert.equal(bytes(new Uint8Array()),'');
});
test('D02 runtime contract identities/profile and integer validation',()=>{
 assertContract(counter);assert.throws(()=>assertContract({...counter,digest:'0'.repeat(64)}));
 assert.throws(()=>validate(counter,'#/$defs/AddInput',{amount:Number.MAX_SAFE_INTEGER+1}));assert.throws(()=>validate(counter,'#/$defs/AddInput',{amount:1,extra:true}));
 assert.throws(()=>validate(counter,'#/$defs/Count',{count:'-0'}));assert.throws(()=>validate(counter,'#/$defs/Count',{count:'9223372036854775808'}));assert.equal(validate(counter,'#/$defs/Count',{count:'12'}).count,'12');
});
async function pair(){const server=createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const got=new Promise(r=>server.once('connection',r));const outgoing=connect(server.address().port,'127.0.0.1');const incoming=await got;server.close();return [new RpcPeer(outgoing,{role:'h',sessionId:'test'}),new RpcPeer(incoming,{role:'p',sessionId:'test'})];}
test('W07 out-of-order responses correlate by ID; control remains live',async()=>{
 const [h,p]=await pair();p.onRequest('system.ping',async params=>{await new Promise(r=>setTimeout(r,params.delay));return params.value;});
 try{const one=h.request('system.ping',{delay:25,value:1});const two=h.request('system.ping',{delay:1,value:2});assert.equal(await two,2);assert.equal(await one,1);}finally{h.close();p.close();}
});
test('R07/R08/R09 busy independent chains fail promptly; timeout preserves active slot',async()=>{
 const [h,p]=await pair();let finish;let entered;const started=new Promise(r=>entered=r);p.onRequest('system.invoke',async()=>{entered();return await new Promise(r=>finish=r)});p.onRequest('system.ping',()=>null);
 const envelope={interface:'test.interface',version:'1.0.0',method:'wait',input:{},context:{timeoutMs:1000,callChain:[]}};
 try {const first=h.request('system.invoke',envelope,{timeoutMs:20});const failed=assert.rejects(first,e=>e.code==='DEADLINE_EXCEEDED');await started;await assert.rejects(h.request('system.invoke',envelope),e=>e.code==='REENTRANT_CALL');assert.equal(await h.request('system.ping',{}),null);await failed;assert.equal(p.busy,true);await assert.rejects(h.request('system.invoke',envelope),e=>e.code==='REENTRANT_CALL');finish(null);await p.whenIdle(1000);assert.equal(p.busy,false);}finally{h.close();p.close();}
});
test('R08 pre-aborted calls settle without dispatch',async()=>{const[h,p]=await pair();let count=0;p.onRequest('system.ping',()=>{count++;return null});const c=new AbortController();c.abort();try{await assert.rejects(h.request('system.ping',{}, {signal:c.signal}),e=>e.code==='CANCELLED');assert.equal(count,0);}finally{h.close();p.close();}});
test('W06 duplicate active IDs terminate without double dispatch; unknown replies ignored',async()=>{
 const[h,p]=await pair();let entered,release;const started=new Promise(r=>entered=r);let calls=0;p.onRequest('system.ping',async()=>{calls++;entered();return new Promise(r=>release=r)});
 try{const first=h.request('system.ping',{});const rejection=assert.rejects(first,e=>e.code==='CONNECTION_CLOSED');await started;h.socket.write(encodeFrame({jsonrpc:'2.0',id:'h:test:1',method:'system.ping',params:{}}));await rejection;assert.equal(calls,1);}finally{release?.(null);h.close();p.close();}
 const[a,b]=await pair();b.onRequest('system.ping',()=>null);try{b.socket.write(encodeFrame({jsonrpc:'2.0',id:'h:test:999',result:'late'}));assert.equal(await a.request('system.ping',{}),null);assert.equal(a.diagnostics.unknownReplies,1);}finally{a.close();b.close();}
});
test('W08 batches, malformed envelopes, and no-ID business calls close deterministically',async()=>{
 for(const message of [[],{jsonrpc:'1.0',id:'h:test:1',method:'system.ping',params:{}},{jsonrpc:'2.0',method:'system.invoke',params:{}},{jsonrpc:'2.0',id:'h:test:2',result:null,error:{}}]){const[h,p]=await pair();try{const closed=new Promise(r=>p.once('closed',r));h.socket.write(encodeFrame(message));await closed;assert.equal(p.isClosed,true);}finally{h.close();p.close();}}
});
test('R10 pending business capacity rejects without losing independent control response',async()=>{
 const[h,p]=await pair();h.limits.maxPendingCalls=1;let entered,release;const started=new Promise(r=>entered=r);p.onRequest('system.invoke',async()=>{entered();return await new Promise(r=>release=r)});p.onRequest('system.ping',()=>null);const params={interface:'test.interface',version:'1.0.0',method:'wait',input:{},context:{timeoutMs:1000,callChain:[]}};
 try{const first=h.request('system.invoke',params);await started;await assert.rejects(h.request('system.invoke',params),e=>e.code==='OVERLOADED');assert.equal(await h.request('system.ping',{}),null);release(null);assert.equal(await first,null);}finally{release?.(null);h.close();p.close();}
});
test('D01/D04/D09/D10 nullable presence, tagged unions, u64, UTC calendar, and literal identities',async()=>{
 const c=JSON.parse(await readFile(new URL('../../../contracts/trusted-plugins/generated/wire-types.contract.json',import.meta.url),'utf8'));assertContract(c);
 const base=JSON.parse('{"__proto__":"ordinary data","choice":"🌍","created":"2024-02-29T23:59:59.999Z","data":"AP8=","integer":9007199254740991,"literal":1,"tagged":{"kind":"number","value":1.0},"wide":"18446744073709551615"}');
 const missing=validate(c,'#/$defs/WireTypes',base);assert.equal(Object.hasOwn(missing,'optionalNull'),false);assert.equal(missing.__proto__,'ordinary data');assert.equal(validate(c,'#/$defs/WireTypes',{...base,optionalNull:null}).optionalNull,null);assert.throws(()=>validate(c,'#/$defs/WireTypes',{...base,optionalNull:undefined}));
 for(const created of ['0001-01-01T00:00:00.000Z','9999-12-31T23:59:59.999Z'])assert.equal(validate(c,'#/$defs/WireTypes',{...base,created}).created,created);
 for(const created of ['2023-02-29T00:00:00.000Z','0000-01-01T00:00:00.000Z','2024-01-01T23:59:60.000Z','2024-01-01T00:00:00Z'])assert.throws(()=>validate(c,'#/$defs/WireTypes',{...base,created}));
 for(const change of [{wide:'18446744073709551616'},{wide:'-1'},{wide:'01'},{literal:2},{data:'AP8'},{tagged:{kind:'unknown',value:1}},{integer:9007199254740992}])assert.throws(()=>validate(c,'#/$defs/WireTypes',{...base,...change}));
});

test('invalid handler wire output returns INVALID_RESULT and releases the business slot', async()=>{
 const [h,p]=await pair();
 const envelope={context:{timeoutMs:1000,callChain:[]}};
 let accessed=false;const invalid=[undefined,Infinity,new Date(),{get value(){accessed=true;return 1;}}];
 p.onRequest('system.invoke',()=>invalid.length?invalid.shift():null);
 try{for(let i=0;i<4;i++){await assert.rejects(h.request('system.invoke',envelope),e=>e.code==='INVALID_RESULT');await p.whenIdle(1000);}assert.equal(accessed,false);assert.equal(await h.request('system.invoke',envelope),null);}finally{h.close();p.close();}
});
test('malformed application error data closes the transport',async()=>{
 for(const data of [{code:'APPLICATION_ERROR',domainCode:12,data:{}},{code:'APPLICATION_ERROR',domainCode:'ERR'},{code:'INTERNAL_ERROR',domainCode:'ERR',data:{}},{code:'INTERNAL_ERROR',extra:true}]){
  const [h,p]=await pair();p.onRequest('system.ping',()=>new Promise(()=>{}));
  try{const pending=h.request('system.ping',{});const rejected=assert.rejects(pending,e=>e.code==='INVALID_REQUEST'||e.code==='INVALID_ARGUMENT');p.socket.write(encodeFrame({jsonrpc:'2.0',id:'h:test:1',error:{code:data.code==='APPLICATION_ERROR'?-32012:-32603,message:'bad',data}}));await rejected;assert.equal(h.isClosed,true);}finally{h.close();p.close();}
 }
});
