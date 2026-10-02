import {run} from './run.mjs';
for(const name of ['plugin-protocol','plugin-sdk','plugin-host','plugin-codegen','plugin-cli'])await run(process.execPath,['node_modules/typescript/bin/tsc','-p',`packages/${name}/tsconfig.json`,'--noEmit']);
await run(process.execPath,['node_modules/typescript/bin/tsc','-p','contracts/trusted-plugins/tsconfig.json','--noEmit']);
