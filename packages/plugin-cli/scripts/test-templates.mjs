// Exercise the developer's actual install and package scripts, not parent imports.
import {access,mkdtemp,readFile,writeFile,rm,realpath} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {join,dirname,resolve} from 'node:path';
import {tmpdir} from 'node:os';
import {spawn} from 'node:child_process';
const cliArg=process.argv.indexOf('--cli');
const cli=cliArg===-1?fileURLToPath(new URL('../src/index.mjs',import.meta.url)):resolve(process.argv[cliArg+1]);
const npmCandidates=[process.env.npm_execpath,join(dirname(process.execPath),'../lib/node_modules/npm/bin/npm-cli.js'),join(dirname(process.execPath),'node_modules/npm/bin/npm-cli.js')].filter(Boolean);
let npm;for(const path of npmCandidates)try{await access(path);npm=resolve(path);break;}catch{}
if(!npm)throw new Error('Set npm_execpath to npm-cli.js to run the Node workflow without a platform-specific command shim');
async function run(command,args,cwd){await new Promise((resolveDone,reject)=>{const child=spawn(command,args,{cwd,shell:false,stdio:'inherit'});const timer=setTimeout(()=>{child.kill('SIGKILL');reject(new Error('Command timed out: '+command+' '+args.join(' ')));},120000);child.once('error',e=>{clearTimeout(timer);reject(e);});child.once('close',(code,signal)=>{clearTimeout(timer);code===0?resolveDone():reject(new Error('Command failed: '+command+' '+args.join(' ')+' ('+(code??signal)+')'));});});}
export async function development(command,args,cwd,language){
 if(process.platform==='win32')throw new Error('This automated dev shutdown smoke requires POSIX signal injection; Windows child.kill(SIGINT) force-terminates instead.');
 const source=join(cwd,'src/plugin.'+language),original=await readFile(source,'utf8');let log='',edited=false,reloaded=false,timedOut=false;
 const child=spawn(command,args,{cwd,shell:false,detached:process.platform!=='win32',stdio:['ignore','pipe','pipe']});
 const stop=signal=>{try{process.platform==='win32'?child.kill(signal):process.kill(-child.pid,signal);}catch(e){if(e.code!=='ESRCH')throw e;}};
 const timer=setTimeout(()=>{timedOut=true;stop('SIGKILL');},120000);
 const observe=async bytes=>{process.stderr.write(bytes);log+=bytes;if(!edited&&log.includes('Development host ready')){edited=true;await writeFile(source,original+'\n// verified development reload\n');}if(!reloaded&&log.includes('Reloaded successfully')){reloaded=true;stop('SIGINT');}};
 child.stderr.on('data',bytes=>{void observe(bytes);});child.stdout.on('data',bytes=>{void observe(bytes);});
 try{const outcome=await new Promise((res,rej)=>{child.once('error',rej);child.once('close',(code,signal)=>res({code,signal}));});if(timedOut||!edited||!reloaded||outcome.code!==0&&outcome.code!==130&&outcome.signal!=='SIGINT')throw new Error('Developer watcher did not rebuild/reload/shutdown cleanly: '+JSON.stringify(outcome));}
 finally{clearTimeout(timer);stop('SIGTERM');await writeFile(source,original);}
}
if(process.argv[1]&&await realpath(process.argv[1]).catch(()=>null)===fileURLToPath(import.meta.url)){
const root=await mkdtemp(join(tmpdir(),'napi-template-workflows-'));
try{
 for(const language of ['ts','js']){
  const directory=join(root,language);
  await run(process.execPath,[cli,'create',directory,'--language',language],root);
  await run(process.execPath,[npm,'install','--prefer-offline','--ignore-scripts','--no-audit','--no-fund'],directory);
  for(const script of ['typecheck','build','test:node','test:artifact'])await run(process.execPath,[npm,'run',script,...(script==='test:artifact'?['--','--runtime','node']:[])],directory);
  await development(process.execPath,[npm,'run','dev','--','--runtime','node'],directory,language);
  // Bun can consume the explicit npm installation and migrate its lock offline.
  await run('bun',['install','--offline','--ignore-scripts'],directory);
  await run('bun',['run','typecheck'],directory);await run('bun',['test'],directory);await run('bun',['run','build'],directory);await run('bun',['run','test:artifact','--runtime','bun'],directory);
  await development('bun',['run','dev','--runtime','bun'],directory,language);
 }
 console.log('TEMPLATE_WORKFLOWS_PASS');
}finally{if(process.argv.includes('--keep'))console.log('TEMPLATE_WORKFLOW_ROOT='+root);else await rm(root,{recursive:true,force:true});}

}
