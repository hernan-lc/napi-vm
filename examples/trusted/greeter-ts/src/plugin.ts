import { PluginError } from '@napi-vm/plugin-sdk';
import { register } from './generated/greeter.js';
import { client as configurationClient } from './generated/app-configuration.js';
export const plugin = register({
  async greet({ name }, context) {
    context.throwIfCancelled();
    if (name === 'invalid') throw PluginError.domain('INVALID_NAME', { reason: 'The demonstration rejects this name' });
    const { value } = await configurationClient(context).get({ key: 'greeting.prefix' });
    return { message: `${value}, ${name}` };
  },
});
