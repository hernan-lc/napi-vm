import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { TrustedPluginHost } from '../../plugin-host/dist/index.js';

const counter = JSON.parse(await readFile(new URL('../../../contracts/trusted-plugins/generated/counter.contract.json', import.meta.url), 'utf8'));
const sdk = new URL('../dist/index.js', import.meta.url).href;
const protocol = new URL('../../plugin-protocol/dist/index.js', import.meta.url).href;

for (const mode of ['success', 'initialize-error', 'disconnect']) {
  test(`SDK initialization event task preserves reply order and cleanup: ${mode}`, { timeout: 15000 }, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'napi-sdk-initialize-'));
    const host = new TrustedPluginHost({ runtimePaths: { node: process.env.NAPI_VM_NODE ?? (process.versions.bun ? 'node' : process.execPath) }, limits: { startupTimeoutMs: 5000, shutdownTimeoutMs: 300 } });
    const source = `import {serve,definePlugin} from ${JSON.stringify(sdk)};
      import {RpcPeer} from ${JSON.stringify(protocol)};
      const K=${JSON.stringify(counter)};
      ${mode === 'disconnect' ? `const reply=RpcPeer.prototype.reply;RpcPeer.prototype.reply=function(...args){if(args[3]==='system.initialize')this.close();return reply.apply(this,args);};` : ''}
      await serve(definePlugin(K,{get:()=>({count:'1'}),add:()=>({count:'1'})},{
        initialize(_,ctx){
          ${mode === 'initialize-error' ? "throw new Error('controlled initialize failure');" : "ctx.spawnTask(async()=>{await Promise.resolve();await ctx.emit(K,'changed',{count:'1'});});"}
        },snapshot(){return{stateVersion:1,contract:'example.counter.state',data:{count:'1'}};}
      }),{metadata:{id:'example.counter',version:'0.1.0'}});`;
    await writeFile(join(directory, 'main.mjs'), source);
    await writeFile(join(directory, 'counter.json'), JSON.stringify(counter));
    await writeFile(join(directory, 'plugin.json'), JSON.stringify({ manifestVersion: 2, execution: 'trusted-process', id: 'example.counter', version: '0.1.0', protocol: { major: 1, minMinor: 0, maxMinor: 0 }, provides: { 'example.counter': '1.0.0' }, requiresHost: {}, contracts: ['counter.json'], launch: { kind: 'javascript', entry: 'main.mjs', runtimes: ['node'], preferredRuntime: 'node', args: [] }, assets: [] }));
    let received, immediateInvoke;
    const event = new Promise(resolve => { received = resolve; });
    host.on('status', status => {
      const handle = host.get(status.instanceId);
      if (status.status === 'STARTING') handle.subscribe(counter, 'changed', received);
      if (status.status === 'READY') immediateInvoke = handle.invoke(counter, 'get', {});
    });
    try {
      if (mode === 'success') {
        const handle = await host.load(directory, { runtime: 'node', development: true });
        assert.equal((await immediateInvoke).count, '1');
        assert.equal((await event).count, '1');
        assert.equal(handle.status, 'READY');
        await handle.unload();
        assert.equal(handle.status, 'STOPPED');
      } else {
        await assert.rejects(host.load(directory, { runtime: 'node', development: true }));
        assert.equal(host.list()[0].status, 'FAILED');
        assert.equal(host.list()[0].child, undefined);
      }
    } finally {
      await host.shutdown();
      await rm(directory, { recursive: true, force: true });
    }
  });
}
