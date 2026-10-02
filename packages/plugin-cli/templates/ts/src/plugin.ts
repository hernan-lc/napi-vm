import { register } from './generated/greeter.js';
export const plugin = register({
  async greet({name}, context) {
    context.throwIfCancelled();
    return {message: `Hola, ${name}`};
  },
});
