import test from 'node:test';
import assert from 'node:assert/strict';
import {createHarness} from '@napi-vm/plugin-sdk';
const {plugin} = await import(globalThis.Bun ? '../src/plugin.ts' : '../dist/plugin.js');
const {CONTRACT} = await import(globalThis.Bun ? '../src/generated/greeter.ts' : '../dist/generated/greeter.js');
test('greeter logic and validation through the SDK harness', async () => {
  const harness = await createHarness(plugin);
  try {
    assert.deepEqual(JSON.parse(JSON.stringify(await harness.invoke(CONTRACT,'greet',{name:'Ana'}))), {message:'Hola, Ana'});
    await assert.rejects(harness.invoke(CONTRACT,'greet',{name:''}));
  } finally { await harness.shutdown(); }
});
