import {readFile,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {cpus} from 'node:os';
import {artifacts,root} from './stage-fixtures.mjs';
import {capture} from './capture.mjs';

const fixtures=JSON.parse(await readFile(join(artifacts,'fixtures.json'),'utf8'));
const executable=join(root,'target/debug/trusted-host-rust'+(process.platform==='win32'?'.exe':''));
const results=[];
for(const [runtime,name] of [['node','counter-ts'],['bun','counter-ts'],['native','counter-rust']]){
  if(!fixtures[name])throw new Error(`Missing staged fixture ${name}; run plugins:test:interop first`);
  const {stdout}=await capture(executable,[fixtures[name],runtime,'benchmark'],{timeoutMs:120000});
  const result=JSON.parse(stdout);
  if(result.final?.count!=='40'||result.samples!==30||result.warmup!==10)throw new Error('Equivalent counter workload did not complete');
  results.push({plugin:name,requestedRuntime:runtime,...result});
}
const rust=await capture('rustc',['--version']);
const report={environment:{os:process.platform,arch:process.arch,cpu:cpus()[0]?.model,rust:rust.stdout.trim()},dataset:{method:'example.counter.add',input:{amount:1},warmup:10,samples:30,expectedFinal:{count:'40'}},results,limitations:['Trivial counter workload includes RPC and validation overhead; it does not establish a native CPU speedup','Host and Rust plugin are debug builds','Separate processes and cache state are not controlled cold starts','Performance has no CI pass/fail threshold']};
await writeFile(join(artifacts,'benchmark-rust-host.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report,null,2));
