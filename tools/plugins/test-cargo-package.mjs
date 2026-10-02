import {mkdtemp,mkdir,rm,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {capture} from './capture.mjs';
const root=fileURLToPath(new URL('../../',import.meta.url));
export const names=['napi-vm-plugin-protocol','napi-vm-plugin-sdk','napi-vm-plugin-host'];
// Unpublished sibling crates are simulated with Cargo registry patches pointing ONLY
// to previously extracted .crate archives. Cargo still creates and verifies each
// real package; no source checkout dependency is available to its verification.
export async function packageCrates(dir){
 const target=join(dir,'package-target');const extracted=join(dir,'archives');await mkdir(extracted,{recursive:true});const paths={};
 for(const name of names){
  const manifest=await readFile(join(root,'crates',name,'Cargo.toml'),'utf8');
  const version=manifest.match(/^version\s*=\s*"([^"]+)"/m)?.[1];if(!version)throw new Error('Missing package version: '+name);
  const args=['package','--manifest-path',join(root,'crates',name,'Cargo.toml'),'--allow-dirty','--target-dir',target];
  for(const [dependency,path] of Object.entries(paths))args.push('--config',`patch.crates-io.${dependency}.path=${JSON.stringify(path)}`);
  const result=await capture('cargo',args,{cwd:dir,timeoutMs:600000});process.stderr.write(result.stderr);
  const archive=join(target,'package',`${name}-${version}.crate`);
  const listing=await capture('tar',['-tzf',archive],{cwd:dir});
  const entries=listing.stdout.trim().split(/\r?\n/).map(entry=>entry.replace(`${name}-${version}/`,''));
  for(const entry of entries)if(/(^|\/)(node_modules|target|\.git)(\/|$)|\.log$/.test(entry))throw new Error(`Forbidden Cargo archive entry: ${entry}`);
  for(const required of ['Cargo.toml','README.md','LICENSE','src/lib.rs'])if(!entries.includes(required))throw new Error(`Missing ${required} in ${name} archive`);
  await capture('tar',['-xzf',archive,'-C',extracted],{cwd:dir});
  paths[name]=join(extracted,`${name}-${version}`);
 }
 return paths;
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 const dir=await mkdtemp(join(tmpdir(),'napi-vm-cargo-packages-'));
 try{await packageCrates(dir);console.log('cargo-package: all three real archives verified with extracted sibling dependencies');}
 finally{await rm(dir,{recursive:true,force:true});}
}
