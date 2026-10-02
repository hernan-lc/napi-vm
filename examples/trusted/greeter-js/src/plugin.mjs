import { readFileSync } from 'node:fs';
import { definePlugin, PluginError } from '@napi-vm/plugin-sdk';
/** @type {import('@napi-vm/plugin-protocol').Contract} */
export const CONTRACT = JSON.parse(readFileSync(new URL('../contracts/greeter.contract.json', import.meta.url), 'utf8'));
/** @type {import('@napi-vm/plugin-protocol').Contract} */
export const CONFIGURATION = JSON.parse(readFileSync(new URL('../contracts/app-configuration.contract.json', import.meta.url), 'utf8'));
export const plugin = definePlugin(CONTRACT, {
  async greet(input, context) {
    context.throwIfCancelled();
    // The SDK has already validated this against the generated business schema.
    if (!input || typeof input !== 'object' || Array.isArray(input) || typeof input.name !== 'string') throw new PluginError('INVALID_ARGUMENT', 'Expected name');
    if (input.name === 'invalid') throw PluginError.domain('INVALID_NAME', { reason: 'The demonstration rejects this name' });
    const configuration = await context.invoke(CONFIGURATION, 'get', { key: 'greeting.prefix' });
    if (!configuration || typeof configuration !== 'object' || Array.isArray(configuration) || typeof configuration.value !== 'string') throw new PluginError('INVALID_RESULT', 'Expected configuration value');
    return { message: `${configuration.value}, ${input.name}` };
  },
});
