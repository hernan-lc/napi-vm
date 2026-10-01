import { readFile, writeFile, mkdir, copyFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const manifest = JSON.parse(await readFile(new URL('./plugin.json', import.meta.url), 'utf8'));
await mkdir(new URL('./src/generated/', import.meta.url), {recursive:true});
await writeFile(new URL('./src/generated/plugin-metadata.ts', import.meta.url), '// Generated from plugin.json.\nexport const PLUGIN_METADATA = '+JSON.stringify({id:manifest.id,version:manifest.version})+';\n');
const result = spawnSync(process.execPath, [fileURLToPath(import.meta.resolve('typescript/bin/tsc')), '-p','tsconfig.json'], {cwd:new URL('.',import.meta.url),stdio:'inherit',shell:false});
if (result.status !== 0) process.exit(result.status ?? 1);
