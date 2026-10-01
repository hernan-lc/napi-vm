import test from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { RpcPeer, encodeFrame, FrameDecoder, PluginError } from '../dist/index.js';

class MemorySocket extends EventEmitter {
  frames = [];
  setNoDelay() {}
  write(frame, callback) { const value = new FrameDecoder().push(frame)[0]; this.frames.push(value); this.emit('written', value); callback(); }
  destroy() {}
}

test('event fence preserves initialize reply order while host callbacks remain live', { timeout: 1000 }, async () => {
  const socket = new MemorySocket(), peer = new RpcPeer(socket, { role: 'p', sessionId: 'fence' });
  peer.holdEventsUntilReply('system.initialize');
  try {
    peer.notify('system.event', { sequence: '1' });
    assert.equal(socket.frames.length, 0);
    const callback = peer.request('system.invoke', {});
    assert.equal(socket.frames[0].method, 'system.invoke');
    socket.emit('data', encodeFrame({ jsonrpc: '2.0', id: 'p:fence:1', result: null }));
    await callback;
    peer.onRequest('system.initialize', () => { peer.notify('system.event', { sequence: '2' }); return null; });
    const replied = new Promise(resolve => socket.on('written', value => { if (value.id === 'h:fence:1') resolve(); }));
    socket.emit('data', encodeFrame({ jsonrpc: '2.0', id: 'h:fence:1', method: 'system.initialize', params: {} }));
    await replied;
    assert.deepEqual(socket.frames.slice(1).map(value => value.method ?? 'reply'), ['reply', 'system.event', 'system.event']);
    assert.deepEqual(socket.frames.slice(2).map(value => value.params.sequence), ['1', '2']);
  } finally { peer.close(); }
});

test('held events are bounded and failed initialization keeps them fenced until close', { timeout: 1000 }, async () => {
  const socket = new MemorySocket(), peer = new RpcPeer(socket, { role: 'p', sessionId: 'failure', limits: { maxFrameBytes: 256, maxQueuedBytes: 256 } });
  peer.holdEventsUntilReply('system.initialize');
  try {
    peer.notify('system.event', { value: 'x'.repeat(80) });
    assert.ok(peer.queuedBytes > 0 && peer.queuedBytes <= peer.limits.maxQueuedBytes);
    assert.throws(() => peer.notify('system.event', { value: 'x'.repeat(80) }), error => error.code === 'OVERLOADED');
    peer.onRequest('system.initialize', () => { throw new PluginError('INTERNAL_ERROR', 'failed'); });
    const replied = new Promise(resolve => socket.once('written', resolve));
    socket.emit('data', encodeFrame({ jsonrpc: '2.0', id: 'h:failure:1', method: 'system.initialize', params: {} }));
    await replied;
    assert.equal(socket.frames[0].error.data.code, 'INTERNAL_ERROR');
    assert.equal(socket.frames.length, 1);
    assert.equal(peer.writes.length, 1);
    peer.close();
    assert.equal(peer.writes.length, 0);
    assert.equal(peer.queuedBytes, 0);
    assert.throws(() => peer.notify('system.event', {}), error => error.code === 'CONNECTION_CLOSED');
  } finally { peer.close(); }
});

test('transport close before initialize reply discards held events', { timeout: 1000 }, async () => {
  const socket = new MemorySocket(), peer = new RpcPeer(socket, { role: 'p', sessionId: 'closed-fence' });
  peer.holdEventsUntilReply('system.initialize');
  peer.notify('system.event', { sequence: '1' });
  peer.onRequest('system.initialize', () => { peer.close(); return null; });
  const closed = new Promise(resolve => peer.once('closed', resolve));
  socket.emit('data', encodeFrame({ jsonrpc: '2.0', id: 'h:closed-fence:1', method: 'system.initialize', params: {} }));
  await closed;
  assert.equal(socket.frames.length, 0);
  assert.equal(peer.writes.length, 0);
  assert.equal(peer.queuedBytes, 0);
});

test('result admission precedes a following event in the same read', { timeout: 1000 }, async () => {
  const socket = new MemorySocket(), peer = new RpcPeer(socket, { role: 'h', sessionId: 'order' });
  let ready = false;
  const seen = [];
  peer.on('event', () => seen.push(ready));
  try {
    const result = peer.request('system.initialize', {}, {}, value => { assert.equal(value, null); ready = true; });
    socket.emit('data', Buffer.concat([
      encodeFrame({ jsonrpc: '2.0', id: 'h:order:1', result: null }),
      encodeFrame({ jsonrpc: '2.0', method: 'system.event', params: {} }),
    ]));
    assert.deepEqual(seen, [true]);
    assert.equal(await result, null);
  } finally { peer.close(); }
});

test('invalid or late successful results never admit a pending lifecycle', { timeout: 1000 }, async () => {
  for (const mode of ['invalid', 'timeout', 'cancel', 'closed', 'remote-error']) {
    const socket = new MemorySocket(), peer = new RpcPeer(socket, { role: 'h', sessionId: mode });
    const controller = new AbortController();
    let admitted = false;
    const result = peer.request('system.initialize', {}, { timeoutMs: mode === 'timeout' ? 5 : 500, signal: controller.signal }, value => {
      if (value !== null) throw new PluginError('INVALID_RESULT', 'Initialize must return null');
      admitted = true;
    });
    const rejected = assert.rejects(result);
    if (mode === 'timeout') await rejected;
    if (mode === 'cancel') controller.abort();
    if (mode === 'closed') peer.close();
    const reply = mode === 'remote-error'
      ? { error: new PluginError('INTERNAL_ERROR', 'initialize failed').toWire() }
      : { result: mode === 'invalid' ? {} : null };
    socket.emit('data', encodeFrame({ jsonrpc: '2.0', id: `h:${mode}:1`, ...reply }));
    await rejected;
    assert.equal(admitted, false, mode);
    peer.close();
  }
});
