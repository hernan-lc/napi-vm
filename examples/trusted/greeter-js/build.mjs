import { readFile, writeFile, mkdir, copyFile } from 'node:fs/promises';
const here = new URL('.', import.meta.url);
const manifest = JSON.parse(await readFile(new URL('./plugin.json', here), 'utf8'));
await mkdir(new URL('./src/generated/', here), { recursive: true });
const metadata = '// Generated from plugin.json.\nexport const PLUGIN_METADATA = ' + JSON.stringify({ id: manifest.id, version: manifest.version }) + ';\n';
await writeFile(new URL('./src/generated/plugin-metadata.mjs', here), metadata);
await mkdir(new URL('./dist/generated/', here), { recursive: true });
for (const name of ['main.mjs', 'plugin.mjs', 'generated/plugin-metadata.mjs']) await copyFile(new URL('./src/' + name, here), new URL('./dist/' + name, here));
