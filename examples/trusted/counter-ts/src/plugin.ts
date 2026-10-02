import { decimal } from '@napi-vm/plugin-protocol';
import { CONTRACT, register, validateCount } from './generated/counter.js';
export function createCounter() {
  let count = 0n;
  return register({
    async add({ amount }, context) { context.throwIfCancelled(); const next = count + BigInt(amount); decimal(next); count = next; const result = { count: count.toString() }; await context.emit(CONTRACT, 'changed', result); return result; },
    get(_input, context) { context.throwIfCancelled(); return { count: count.toString() }; },
  }, {
    initialize({ snapshot }) { if (snapshot) count = BigInt(validateCount(snapshot.data).count); },
    snapshot() { return { stateVersion: 1, contract: 'example.counter.state', data: { count: count.toString() } }; },
  });
}
export const plugin = createCounter();
