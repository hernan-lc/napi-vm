import {readFile,writeFile,mkdir,stat,lstat,realpath,readdir,copyFile,chmod,rm} from 'node:fs/promises';
import {resolve,join,dirname,basename,sep,isAbsolute} from 'node:path';
import {createHash} from 'node:crypto';
export const sha256=bytes=>createHash('sha256').update(bytes).digest('hex');
export async function readJson(path){return JSON.parse(await readFile(path,'utf8'));}
export function relativePath(path){
 if(typeof path!=='string'||!path||path.includes('\0')||isAbsolute(path)||/^[A-Za-z]:/.test(path))throw new Error('Unsafe package path: '+path);
 const clean=path.replace(/\\/g,'/').replace(/^\.\//,'');
 if(clean.split('/').some(p=>!p||p==='.'||p==='..'))throw new Error('Unsafe package path: '+path);
 return clean;
}
export async function contained(root,path){
 path=relativePath(path);const base=await realpath(root);const full=await realpath(resolve(base,path));
 if(full===base||!full.startsWith(base+sep))throw new Error('Path escapes package: '+path);
 return full;
}
const forbidden=part=>['.git','.hg','.svn','.cache','__pycache__','target','.bin','.env','.npmrc','.yarnrc','.pypirc','.netrc','credentials','credentials.json','.aws','.ssh','.gnupg','.config','id_rsa','id_ed25519','.DS_Store'].includes(part)||part.startsWith('.env.')||/\.(pem|key|p12|pfx)$/i.test(part);
async function collect(root,path,files,ancestors=new Set()){
 path=relativePath(path);if(path.split('/').some(forbidden))throw new Error('Refusing sensitive/cache file: '+path);
 const full=await contained(root,path);const info=await stat(full);
 if(info.isDirectory()){
  if(ancestors.has(full))throw new Error('Symlink directory cycle: '+path);
  const next=new Set([...ancestors,full]);for(const name of (await readdir(full)).sort())await collect(root,path+'/'+name,files,next);
 }else if(info.isFile()){if(!files.has(path)&&files.size>=9999)throw new Error('Package inventory exceeds 10000 files including manifest');files.set(path,{source:full,mode:info.mode});}else throw new Error('Unsupported package file: '+path);
}
export function currentTarget(){
 const target={os:process.platform,arch:process.arch};
 if(process.platform==='linux')target.libc=process.report?.getReport().header.glibcVersionRuntime?'gnu':'musl';
 return target;
}
export async function pack(directory,out,{target,abi,runtime,profile}={}){
 directory=await realpath(directory);out=resolve(out);out=join(await realpath(dirname(out)),basename(out));if(out===directory||out.startsWith(directory+sep))throw new Error('Pack destination must be outside source directory');
 try{await lstat(out);throw new Error('Pack destination already exists: '+out);}catch(e){if(e.code!=='ENOENT')throw e;}
 const {readManifest}=await import('@napi-vm/plugin-host');
 const {manifest}=await readManifest(join(directory,'plugin.json'),{verifyIntegrity:false});
 if(manifest.launch.kind==='javascript'&&!/\.(?:mjs|cjs|js)$/.test(manifest.launch.entry))throw new Error('Production packages require built JavaScript entries (.js/.mjs/.cjs), not TypeScript source');
 const files=new Map();
 for(const path of [manifest.launch.entry,...(manifest.assets??[]),...(manifest.contracts??[]),...(manifest.dependencies?.native??[]).map(d=>d.path)])await collect(directory,path,files);
 for(const path of manifest.extensions?.['napi-vm.pack']?.include??[])await collect(directory,path,files);
 // Directory inventories never include an old manifest or lock supplied by an include list.
 files.delete('plugin.json');files.delete('plugin.lock.json');
 const interfaces=Object.create(null);
 for(const path of manifest.contracts??[]){const c=await readJson(await contained(directory,path));interfaces[c.descriptor.id]={version:c.descriptor.version,digest:c.digest};}
 const executable=manifest.launch.kind==='executable';
 const effectiveTarget=target??manifest.launch.target??{os:'any',arch:'any'};
 if(executable&&(!effectiveTarget.os||effectiveTarget.os==='any'||!effectiveTarget.arch||effectiveTarget.arch==='any'||effectiveTarget.os==='linux'&&!['gnu','musl'].includes(effectiveTarget.libc)))throw new Error('Native target requires actual OS, arch and Linux libc');
 if(executable&&(Object.keys(effectiveTarget).length!==Object.keys(manifest.launch.target).length||Object.keys(effectiveTarget).some(key=>effectiveTarget[key]!==manifest.launch.target[key])))throw new Error('Artifact target must match manifest launch target');
 const artifact={profile:profile??manifest.profile??(executable?'native-executable':'external-runtime'),target:effectiveTarget,runtime:runtime??(executable?[]:manifest.launch.runtimes),abi:abi??(executable?'native-executable':'javascript-esm'),availability:{built:true,distributed:false,tested:[]}};
 // Atomic no-overwrite claim. Any failure after this point removes only our own destination.
 await mkdir(out,{recursive:false});
 try{
  const inventory=Object.create(null);
  for(const [path,file]of [...files.entries()].sort(([a],[b])=>a<b?-1:a>b?1:0)){await mkdir(dirname(join(out,path)),{recursive:true});await copyFile(file.source,join(out,path));await chmod(join(out,path),file.mode&0o777);inventory[path]=sha256(await readFile(join(out,path)));}
  await writeFile(join(out,'plugin.json'),JSON.stringify(manifest,null,2)+'\n');inventory['plugin.json']=sha256(await readFile(join(out,'plugin.json')));
  const lock={lockVersion:1,pluginId:manifest.id,pluginVersion:manifest.version,artifact,interfaces,files:inventory};
  await writeFile(join(out,'plugin.lock.json'),JSON.stringify(lock,null,2)+'\n');
  await readManifest(join(out,'plugin.json'));
  return {directory:out,lock};
 }catch(error){await rm(out,{recursive:true,force:true});throw error;}
}
// Native dev adapters retain production integrity verification. Build first, then
// checksum a separate private launch directory before handing it to the host.
export async function prepareDevelopmentPackage(directory,format){
 if(format!=='executable')return directory;
 const output=directory+'-package';await pack(directory,output);return output;
}
