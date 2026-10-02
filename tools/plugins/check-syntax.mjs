import {readdir} from 'node:fs/promises';
import {join} from 'node:path';
import {run} from './run.mjs';
async function visit(path){for(const item of await readdir(path,{withFileTypes:true})){if(['node_modules','dist','target','vendor'].includes(item.name))continue;const next=join(path,item.name);if(item.isDirectory())await visit(next);else if(/\.(?:mjs|js)$/.test(item.name))await run(process.execPath,['--check',next]);}}
for(const path of ['packages','tools/plugins','examples/trusted'])await visit(path);
