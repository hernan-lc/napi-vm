import {spawn} from 'node:child_process';
export async function run(command,args,options={}){await new Promise((resolve,reject)=>{const child=spawn(command,args,{stdio:'inherit',shell:false,...options});child.once('error',reject);child.once('exit',(code,signal)=>code===0?resolve():reject(new Error(`${command} exited ${code??signal}`)));});}
