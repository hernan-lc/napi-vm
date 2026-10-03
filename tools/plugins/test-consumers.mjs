// Install published payloads into a real external application. Never use workspace links.
import assert from 'node:assert/strict';
import {mkdtemp, mkdir, readFile, writeFile, rm, cp, realpath} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join, relative, isAbsolute, sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {capture} from './capture.mjs';
const root=fileURLToPath(new URL('../../',import.meta.url));
const names=['plugin-protocol','plugin-sdk','plugin-host','plugin-codegen','plugin-cli'];
export async function testConsumers({templates=true,rustHost=join(root,'target/debug/trusted-host-rust'+(process.platform==='win32'?'.exe':''))}={}) {
 const directory=await mkdtemp(join(tmpdir(),'napi consumer café '));
 const results=[];
 try {
  const tarballs=[];
  for(const name of names) {
   const {stdout}=await capture('npm',['pack',join(root,'packages',name),'--pack-destination',directory,'--json','--ignore-scripts']);
   const packed=JSON.parse(stdout)[0];
   for(const {path} of packed.files) assert(!path.split('/').some(part=>['node_modules','target','.git','.cache','test'].includes(part)),`${name} unexpectedly ships ${path}`);
   assert(packed.files.some(({path})=>path==='LICENSE'),`${name} missing license`);
   assert(packed.files.some(({path})=>path==='README.md'),`${name} missing README`);
   tarballs.push(join(directory,packed.filename));
  }
  const consumer=join(directory,'application');await mkdir(consumer);
  await writeFile(join(consumer,'package.json'),JSON.stringify({private:true,type:'module'}));
  const nodeTypes=JSON.parse(await readFile(join(root,'node_modules/@types/node/package.json'),'utf8')).version;
  await capture('npm',['install','--prefer-offline','--ignore-scripts','--no-audit','--no-fund',...tarballs,`@types/node@${nodeTypes}`],{cwd:consumer,timeoutMs:120000});
  // macOS resolves /var temporary directories through /private/var. Compare
  // canonical paths and require a descendant, not a matching string prefix.
  const canonicalConsumer=await realpath(consumer);
  for(const name of names) {
   const packagePath=await realpath(join(consumer,'node_modules/@napi-vm',name));
   const location=relative(canonicalConsumer,packagePath);
   assert(location && location!=='..' && !location.startsWith('..'+sep) && !isAbsolute(location),`${name} linked back to workspace`);
  }
  const sources=join(consumer,'contracts');await mkdir(sources);
  for(const name of ['greeter.interface.json','greeter.types.schema.json','wire-types.interface.json','wire-types.types.schema.json'])await cp(join(root,'contracts/trusted-plugins/examples',name),join(sources,name));
  await writeFile(join(consumer,'generate.mjs'),`import {generate, generateAll} from '@napi-vm/plugin-codegen';
await generate('contracts/greeter.interface.json','generated');
await generate('contracts/wire-types.interface.json','generated');
await generate('contracts/greeter.interface.json','generated',{check:true});
await generate('contracts/wire-types.interface.json','generated',{check:true});
await generateAll({sourceDir:'contracts',outDir:'generated',check:true});\n`);
  await capture(process.execPath,['generate.mjs'],{cwd:consumer});
  await writeFile(join(consumer,'consumer.ts'),`import {validate, type Contract} from '@napi-vm/plugin-protocol';
import {createHarness, type PluginDefinition} from '@napi-vm/plugin-sdk';
import {TrustedPluginHost, type LoadOptions} from '@napi-vm/plugin-host';
import {generate, type GenerationResult} from '@napi-vm/plugin-codegen';
import {main, pack, type ArtifactLock} from '@napi-vm/plugin-cli';
import {CONTRACT, client, register, type GreetInput} from './generated/greeter.js';
const input: GreetInput = {name: 'Ana'};
// @ts-expect-error generated public input rejects an incompatible value
const bad: GreetInput = {name: 123};
const contract: Contract = CONTRACT;
const definition: PluginDefinition = register({greet: ({name}) => ({message: 'Hola, '+name})});
const harness = await createHarness(definition);
try { const output: string = (await client(harness).greet(input)).message; if(output !== 'Hola, Ana') throw new Error(output); } finally {await harness.shutdown();}
validate(contract, '#/$defs/GreetInput', input);
const host = new TrustedPluginHost({contracts:[contract]});
const options: LoadOptions = {runtime:'node'};
await host.shutdown();
const generated: Promise<GenerationResult> = generate('contracts/greeter.interface.json','generated',{check:true});
await generated;
const packed: (directory:string, out:string) => Promise<{directory:string;lock:ArtifactLock}> = pack;
if(typeof packed !== 'function') throw new Error('Missing pack export');
await main(['help']);
void options; void bad;\n`);
  await writeFile(join(consumer,'tsconfig.json'),JSON.stringify({compilerOptions:{target:'ES2022',module:'NodeNext',moduleResolution:'NodeNext',strict:true,skipLibCheck:false,outDir:'dist',types:['node']},include:['consumer.ts','generated/*.ts']}));
  await capture(process.execPath,['node_modules/typescript/bin/tsc','--noEmit','-p','tsconfig.json'],{cwd:consumer});
  await capture(process.execPath,['node_modules/typescript/bin/tsc','-p','tsconfig.json'],{cwd:consumer});
  await capture(process.execPath,['dist/consumer.js'],{cwd:consumer});
  results.push({test:'all-five-tarball-imports-strict-declarations-generated-runtime',status:'pass'});
  if(templates) {
   const cli=join(consumer,'node_modules/@napi-vm/plugin-cli/src/index.mjs');
   for(const language of ['ts','js','rust']) {
    const project=join(directory,language+' template');
    await capture(process.execPath,[cli,'create',project,'--language',language],{cwd:consumer});
    if(language==='rust') {
     for(const name of ['napi-vm-plugin-sdk','napi-vm-plugin-protocol']) {
      await readFile(join(project,'vendor',name,'LICENSE'));
      const metadata=await readFile(join(project,'vendor',name,'Cargo.toml'),'utf8');
      assert(!metadata.includes('workspace = true'),`${name} leaks workspace metadata`);
     }
     await capture('cargo',['metadata','--locked','--no-deps','--format-version','1'],{cwd:project});
     await capture('cargo',['test','--locked'],{cwd:project,timeoutMs:120000});
     results.push({test:'packed-cli-rust-template-independent-metadata-generated-bindings-tests',status:'pass'});
     if(rustHost) {
      await capture(process.execPath,[cli,'build',project,'--format','executable'],{cwd:consumer,timeoutMs:120000});
      const out=join(directory,'rust packed');
      await capture(process.execPath,[cli,'pack',project,'--out',out],{cwd:consumer});
      await capture(process.execPath,[cli,'test',project,'--artifact','--host-executable',rustHost],{cwd:consumer,timeoutMs:120000});
      const {stdout}=await capture(process.execPath,[cli,'invoke',out,'--interface','example.greeter','--method','greet','--input','{"name":"Ana"}','--host-executable',rustHost],{cwd:consumer,timeoutMs:120000});
      assert.equal(JSON.parse(stdout).message,'Hola, Ana');
      results.push({test:'packed-cli-rust-template-build-pack-native-invoke',status:'pass'});
     }
     continue;
    }
    await capture('npm',['install','--prefer-offline','--ignore-scripts','--no-audit','--no-fund'],{cwd:project,timeoutMs:120000});
    for(const script of ['typecheck','build','test:node'])await capture('npm',['run',script],{cwd:project,timeoutMs:120000});
    await capture('npm',['run','test:artifact','--','--runtime','node'],{cwd:project,timeoutMs:120000});
    const out=join(directory,language+' packed');
    await capture(process.execPath,[cli,'pack',project,'--out',out],{cwd:consumer,timeoutMs:120000});
    await capture(process.execPath,[cli,'validate',out],{cwd:consumer});
    await capture(process.execPath,[cli,'invoke',out,'--interface','example.greeter','--method','greet','--input','{"name":"Ana"}','--runtime','node'],{cwd:consumer});
    results.push({test:'packed-cli-'+language+'-template-install-typecheck-build-test-pack-invoke',status:'pass'});
    if(language==='ts'&&process.platform!=='win32'){const {development}=await import('../../packages/plugin-cli/scripts/test-templates.mjs');await development(process.execPath,[cli,'dev',project,'--runtime','node'],project,language);results.push({test:'packed-cli-ts-dev-signals',status:'pass'});}
    if(language==='ts'&&process.platform==='win32')results.push({test:'packed-cli-ts-dev-signals',status:'not-tested',reason:'Windows child.kill(SIGINT) force-terminates; it cannot test graceful console interrupt shutdown. Host shutdown is exercised by artifact invocation tests.'});
   }
  }
  return {results};
 } finally {await rm(directory,{recursive:true,force:true});}
}
if(process.argv[1]&&await realpath(process.argv[1]).catch(()=>null)===fileURLToPath(import.meta.url))console.log(JSON.stringify(await testConsumers(),null,2));
