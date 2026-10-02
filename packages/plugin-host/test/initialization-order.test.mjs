import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { TrustedPluginHost } from '../dist/index.js';
import { RpcPeer, encodeFrame } from '../../plugin-protocol/dist/index.js';

const counter = JSON.parse(await readFile(new URL('../../../contracts/trusted-plugins/generated/counter.contract.json', import.meta.url), 'utf8'));
const artifact = fileURLToPath(new URL('../../../examples/trusted/counter-ts', import.meta.url));

async function withInitializeFrames(mode, verify) {
  const host = new TrustedPluginHost({ runtimePaths: { node: process.env.NAPI_VM_NODE ?? (process.versions.bun ? 'node' : process.execPath) }, limits: { startupTimeoutMs: 5000, shutdownTimeoutMs: 300 } });
  const statuses = [];
  let delivered;
  const event = new Promise(resolve => { delivered = resolve; });
  host.on('status', update => {
    statuses.push(update.status);
    if (update.status === 'STARTING') host.get(update.instanceId).subscribe(counter, 'changed', delivered);
  });
  const request = RpcPeer.prototype.request;
  RpcPeer.prototype.request = function (method, params, options, acceptResult) {
    const response = request.call(this, method, params, options, acceptResult);
    if (method === 'system.initialize') {
      const reply = { jsonrpc: '2.0', id: `h:${this.sessionId}:${this.counter}`, result: mode === 'invalid' ? {} : null };
      const changed = { jsonrpc: '2.0', method: 'system.event', params: { interface: 'example.counter', version: '1.0.0', event: 'changed', sessionId: this.sessionId, sequence: '1', payload: { count: '1' } } };
      if (mode === 'failed') host.list()[0].status = 'FAILED';
      if (mode === 'retired') host.list()[0].sessionId = 'retired';
      if (mode === 'malformed-event') changed.params.payload.count = 'not-an-integer';
      // One synchronous data callback deterministically models coalesced TCP frames.
      // Waiting for real packets to coalesce would make this scheduler-dependent.
      this.socket.emit('data', Buffer.concat((mode === 'premature' ? [changed, reply] : [reply, changed]).map(value => encodeFrame(value))));
    }
    return response;
  };
  try {
    await verify(host, statuses, event);
  } finally {
    RpcPeer.prototype.request = request;
    await host.shutdown();
  }
}

test('initialize success admits READY before a coalesced first event', { timeout: 15000 }, async () => {
  await withInitializeFrames('valid', async (host, statuses, event) => {
    const handle = await host.load(artifact, { runtime: 'node', development: true });
    assert.equal(handle.status, 'READY');
    assert.equal(handle.peer.isClosed, false);
    assert.equal(handle.lastEvent, 1n);
    assert.deepEqual(statuses, ['STARTING', 'READY']);
    assert.equal((await event).count, '1');
  });
});

for (const mode of ['premature', 'invalid', 'failed', 'retired']) {
  test(`initialize ${mode} response cannot admit READY`, { timeout: 15000 }, async () => {
    await withInitializeFrames(mode, async (host, statuses) => {
      const code = mode === 'premature' ? 'NOT_READY' : mode === 'invalid' ? 'INVALID_RESULT' : 'PLUGIN_EXITED';
      await assert.rejects(host.load(artifact, { runtime: 'node', development: true }), error => error.code === code);
      assert.equal(statuses.includes('READY'), false);
      assert.equal(host.list()[0].status, 'FAILED');
      assert.equal(host.list()[0].child, undefined);
    });
  });
}

test('a coalesced malformed event cannot make load report a dead READY session', { timeout: 15000 }, async () => {
  await withInitializeFrames('malformed-event', async (host, statuses) => {
    await assert.rejects(host.load(artifact, { runtime: 'node', development: true }), error => error.code === 'PLUGIN_EXITED');
    assert.equal(host.list()[0].status, 'FAILED');
    assert.equal(host.list()[0].child, undefined);
    assert.ok(statuses.indexOf('READY') < statuses.indexOf('FAILED'));
    assert.equal(statuses.lastIndexOf('READY'), statuses.indexOf('READY'));
  });
});
