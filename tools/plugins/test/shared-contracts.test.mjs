import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {validateSchema,wireValue,FrameDecoder,encodeFrame} from '@napi-vm/plugin-protocol';
const cases=JSON.parse(await readFile(new URL('../../../fixtures/trusted-plugins/contracts/validation.json',import.meta.url),'utf8'));
for(const fixture of cases)test(fixture.name,()=>{const check=()=>validateSchema({},fixture.schema,wireValue(fixture.value));if(fixture.valid)assert.doesNotThrow(check);else assert.throws(check);});
test('codec all header/body boundaries and coalesced last-key-wins',()=>{const value={jsonrpc:'2.0',id:'h:test:1',result:'😀'};const frame=encodeFrame(value);for(let n=1;n<frame.length;n++){const decoder=new FrameDecoder();const result=[...decoder.push(frame.subarray(0,n)),...decoder.push(frame.subarray(n))];assert.equal(JSON.stringify(result),JSON.stringify([value]));decoder.finish();}const body=Buffer.from('{"x":0,"x":1}');const raw=Buffer.alloc(body.length+4);raw.writeUInt32BE(body.length);body.copy(raw,4);const decoder=new FrameDecoder();assert.equal(JSON.stringify(decoder.push(Buffer.concat([raw,raw]))),'[{"x":1},{"x":1}]');});
test('wire rejects invalid JS data without invoking accessors',()=>{let called=false;for(const value of [undefined,NaN,Infinity,1n,new Date(),{a:undefined},[undefined],{get secret(){called=true;return 1}},{toJSON(){called=true;return 1}}])assert.throws(()=>wireValue(value));assert.equal(called,false);assert.throws(()=>wireValue('\ud800'));assert.throws(()=>wireValue('\udc00'));assert.equal(wireValue(-0),0);});
