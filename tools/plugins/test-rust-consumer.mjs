import {mkdtemp,mkdir,cp,readFile,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {capture} from './capture.mjs';
import {packageCrates} from './test-cargo-package.mjs';
import {pack} from '../../packages/plugin-cli/src/pack.mjs';
const root=fileURLToPath(new URL('../../',import.meta.url));
const dir=await mkdtemp(join(tmpdir(),'napi-vm rust consumer café '));
try{
 const crates=await packageCrates(dir);const consumer=join(dir,'consumer');await mkdir(join(consumer,'src'),{recursive:true});
 const dependencies=Object.entries(crates).map(([name,path])=>`${name} = { path = ${JSON.stringify(path)} }`).join('\n');
 const patches=Object.entries(crates).map(([name,path])=>`${name} = { path = ${JSON.stringify(path)} }`).join('\n');
 await writeFile(join(consumer,'Cargo.toml'),`[package]\nname="external-rust-consumer"\nversion="0.0.0"\nedition="2024"\n[dependencies]\n${dependencies}\nserde={version="1",features=["derive"]}\nserde_json="1"\ntokio={version="1",features=["full"]}\n[patch.crates-io]\n${patches}\n`);
 await cp(join(root,'tools/plugins/fixtures/rust-consumer.rs'),join(consumer,'src/main.rs'));
 for(const name of ['counter','greeter','app-configuration','wire-types'])for(const ext of ['rs','contract.json'])await cp(join(root,'contracts/trusted-plugins/generated',`${name}.${ext}`),join(consumer,'src',`${name}.${ext}`));
 const fixtureTarget=join(dir,'fixture-target');
 await capture('cargo',['build','-p','trusted-greeter-rust','-p','trusted-counter-rust','--target-dir',fixtureTarget],{cwd:root,timeoutMs:600000});
 const manifests=[];const suffix=process.platform==='win32'?'.exe':'';
 for(const name of ['greeter','counter']){
  const staging=join(dir,`stage-${name}`);await mkdir(join(staging,'bin'),{recursive:true});await mkdir(join(staging,'contracts'));
  await cp(join(fixtureTarget,'debug',`trusted-${name}-rust${suffix}`),join(staging,'bin',`plugin${suffix}`));
  for(const c of [name,...(name==='greeter'?['app-configuration']:[])])await cp(join(root,'contracts/trusted-plugins/generated',`${c}.contract.json`),join(staging,'contracts',`${c}.contract.json`));
  const manifest=JSON.parse(await readFile(join(root,'examples/trusted',`${name}-rust/plugin.json`),'utf8'));
  manifest.launch.entry=`bin/plugin${suffix}`;manifest.launch.target={os:process.platform,arch:process.arch,...(process.platform==='linux'?{libc:process.report.getReport().header.glibcVersionRuntime?'gnu':'musl'}:{})};
  await writeFile(join(staging,'plugin.json'),JSON.stringify(manifest));const destination=join(dir,`relocated ${name}`);await pack(staging,destination);manifests.push(join(destination,'plugin.json'));
 }
 await capture('cargo',['build'],{cwd:consumer,timeoutMs:600000});
 const emptyPath=join(dir,'empty-runtime-path');await mkdir(emptyPath);
 const binary=join(consumer,'target/debug',`external-rust-consumer${suffix}`);
 const runtimeEnv={PATH:emptyPath};
 for(const key of (process.platform==='win32'?['SystemRoot','WINDIR','TEMP','TMP']:['TMPDIR','TMP','TEMP']))if(process.env[key])runtimeEnv[key]=process.env[key];
 const result=await capture(binary,manifests,{cwd:dir,env:runtimeEnv,timeoutMs:60000});process.stdout.write(result.stdout);process.stderr.write(result.stderr);
}finally{await rm(dir,{recursive:true,force:true});}
