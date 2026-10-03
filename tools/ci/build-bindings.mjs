import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {NapiCli, readNapiConfig, writeJsBinding} from '@napi-rs/cli';

// The CLI rebuilds the crate when changing module formats to recover macro
// metadata. Build once, then use its public loader generator for the second
// format with the exports from the freshly generated CommonJS loader.
const {task} = await new NapiCli().build({
  cwd: process.cwd(), platform: true, release: true,
  format: 'commonjs', jsBinding: 'index.js', cargoOptions: ['--lib', '--locked'],
});
await task;
const commonjs = await readFile('index.js', 'utf8');
const idents = [...commonjs.matchAll(/^module\.exports\.([\w$]+) = nativeBinding\.\1$/gm)]
  .map(match => match[1]);
if (!idents.length) throw new Error('Fresh NAPI loader has no named exports');
const config = await readNapiConfig(resolve('package.json'));
await writeJsBinding({
  platform: true, format: 'esm', jsBinding: 'index.mjs', idents,
  binaryName: config.binaryName, packageName: config.packageName,
  version: config.packageJson.version, outputDir: process.cwd(),
});
