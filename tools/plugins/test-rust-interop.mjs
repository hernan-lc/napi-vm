import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {artifacts,root} from './stage-fixtures.mjs';
import {capture} from './capture.mjs';

// Stage/build through plugins:test:interop first; missing fixtures are errors.
const fixtures=JSON.parse(await readFile(join(artifacts,'fixtures.json'),'utf8'));
const env={...process.env};
for(const [key,name] of Object.entries({
  NAPI_VM_GREETER_JS_MANIFEST:'greeter-ts',
  NAPI_VM_GREETER_RUST_MANIFEST:'greeter-rust',
  NAPI_VM_COUNTER_JS_MANIFEST:'counter-ts',
  NAPI_VM_COUNTER_RUST_MANIFEST:'counter-rust',
})){
  if(typeof fixtures[name]!=='string')throw new Error(`Missing staged fixture ${name}`);
  env[key]=fixtures[name];
}
const result=await capture('cargo',['test','-p','napi-vm-plugin-conformance','--test','interop','--','--ignored'],{cwd:root,env,timeoutMs:300000});
process.stdout.write(result.stdout);
process.stderr.write(result.stderr);
