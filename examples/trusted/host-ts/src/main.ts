import { TrustedPluginHost, readManifest } from '@napi-vm/plugin-host';
const path = process.argv[2];
if (!path) throw new Error('Usage: node dist/main.js <packaged-plugin-directory> [node|bun]');
const resolved = await readManifest(path);
const greeter = resolved.contracts.find(c => c.descriptor.id === 'example.greeter');
const configuration = resolved.contracts.find(c => c.descriptor.id === 'app.configuration');
if (!greeter || !configuration) throw new Error('This example requires a packaged greeter and app.configuration contract');
const host = new TrustedPluginHost({ contracts: [greeter] });
host.registerService(configuration, { get: () => ({ value: 'Hola' }) });
try {
  const selected = process.argv[3];
  const handle = await host.load(path, selected === 'node' || selected === 'bun' ? { runtime: selected } : {});
  console.log(JSON.stringify(await handle.invoke(greeter, 'greet', { name: 'Ana' })));
} finally { await host.shutdown(); }
