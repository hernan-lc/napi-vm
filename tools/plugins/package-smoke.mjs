import {mkdtemp,cp,readFile,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {root,artifacts} from './stage-fixtures.mjs';
import {capture} from './capture.mjs';
const dir=await mkdtemp(join(tmpdir(),'trusted plugins café '));const results=[];
try{
 const relocation=join(dir,'greeter moved');await cp(join(artifacts,'greeter-ts'),relocation,{recursive:true});
 const {TrustedPluginHost}=await import('@napi-vm/plugin-host');const configuration=JSON.parse(await readFile(join(relocation,'contracts/app-configuration.contract.json'),'utf8'));const greeter=JSON.parse(await readFile(join(relocation,'contracts/greeter.contract.json'),'utf8'));
 for(const runtime of ['node','bun']){const host=new TrustedPluginHost({contracts:[greeter,configuration]});host.registerService(configuration,{get:async()=>({value:'Hola'})});try{const plugin=await host.load(join(relocation,'plugin.json'),{runtime});const result=await plugin.invoke(greeter,'greet',{name:'Ana'});if(result.message!=='Hola, Ana')throw new Error('relocated result mismatch');results.push({test:'relocated-'+runtime,status:'pass'});}finally{await host.shutdown();}}
 const entry=JSON.parse(await readFile(join(relocation,'plugin.json'),'utf8')).launch.entry;await writeFile(join(relocation,entry),'throw new Error("tampered")');let rejected=false;const host=new TrustedPluginHost();try{await host.load(join(relocation,'plugin.json'));}catch(e){if(!/integrity|checksum|hash/i.test(e.message))throw e;rejected=true;}finally{await host.shutdown();}if(!rejected)throw new Error('tampered file accepted');results.push({test:'tampered-byte-rejected',status:'pass'});
 const nativeHost=join(dir,'standalone host'+(process.platform==='win32'?'.exe':''));await cp(join(root,'target/debug/trusted-host-rust'+(process.platform==='win32'?'.exe':'')),nativeHost);
 for(const name of ['greeter-rust','greeter-bun']){const relocated=join(dir,name+' moved');await cp(join(artifacts,name),relocated,{recursive:true});const {stdout}=await capture(nativeHost,[join(relocated,'plugin.json'),'native','greeter'],{cwd:dir,env:{...process.env,PATH:process.platform==='win32'?process.env.SystemRoot+'\\System32':'/usr/bin:/bin'}});const output=JSON.parse(stdout);if(output.greeting?.message!=='Hola, Ana')throw new Error('Relocated native artifact result mismatch');results.push({test:'relocated-'+name+'-no-JS-or-Cargo-PATH',status:'pass'});}
 await writeFile(join(artifacts,'package-results.json'),JSON.stringify({results},null,2)+'\n');console.log(JSON.stringify({results},null,2));
}finally{await rm(dir,{recursive:true,force:true});}
