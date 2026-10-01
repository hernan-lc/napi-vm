// Prepare self-contained scaffolding assets before packing the developer CLI.
// This is explicit packaging tooling, never part of loading or launching a plugin.
import {access,cp,mkdir,readFile,writeFile,rm} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {join,resolve} from 'node:path';
const cli=fileURLToPath(new URL('../',import.meta.url));
const root=resolve(cli,'../..');
try{await access(join(root,'crates/napi-vm-plugin-sdk/Cargo.toml'));}catch{await access(join(cli,'templates/rust/vendor/napi-vm-plugin-sdk/src/lib.rs'));process.stderr.write('Using packaged template assets\n');process.exit(0);}
for(const name of ['napi-vm-plugin-sdk','napi-vm-plugin-protocol']){
 const destination=join(cli,'templates/rust/vendor',name);await rm(destination,{recursive:true,force:true});await mkdir(destination,{recursive:true});
 for(const path of ['src','Cargo.toml'])await cp(join(root,'crates',name,path),join(destination,path),{recursive:true});
 for(const license of ['LICENSE','LICENSE.md'])try{await cp(join(root,license),join(destination,license));break;}catch(error){if(error.code!=='ENOENT')throw error;}
}
for(const language of ['ts','js','rust']){
 for(const name of ['greeter.interface.json','greeter.types.schema.json'])await cp(join(root,'contracts/trusted-plugins/examples',name),join(cli,'templates',language,'contracts',name));
 await cp(join(root,'contracts/trusted-plugins/generated/greeter.contract.json'),join(cli,'templates',language,'contracts/greeter.contract.json'));
}
await cp(join(root,'contracts/trusted-plugins/generated/greeter.ts'),join(cli,'templates/ts/src/generated/greeter.ts'));
await cp(join(root,'contracts/trusted-plugins/generated/greeter.rs'),join(cli,'templates/rust/src/generated.rs'));
const c=JSON.parse(await readFile(join(root,'contracts/trusted-plugins/generated/greeter.contract.json'),'utf8'));
const js=join(cli,'templates/js/src/generated/greeter.js');let text=await readFile(js,'utf8');text=text.replace(/export const CONTRACT = JSON.parse\(.*\);/,`export const CONTRACT = JSON.parse(${JSON.stringify(JSON.stringify(c))});`);await writeFile(js,text);
process.stderr.write('Prepared checked-in contract bindings and independent Rust SDK template assets\n');
