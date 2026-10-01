import { definePlugin } from '@napi-vm/plugin-sdk';
/** @type {import('@napi-vm/plugin-protocol').Contract} */
export const CONTRACT = JSON.parse("{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{\"INVALID_NAME\":{\"data\":\"#/$defs/InvalidNameData\"}},\"events\":{},\"id\":\"example.greeter\",\"methods\":{\"greet\":{\"errors\":[\"INVALID_NAME\"],\"idempotent\":true,\"input\":\"#/$defs/GreetInput\",\"output\":\"#/$defs/GreetOutput\"}},\"version\":\"1.0.0\"},\"digest\":\"ca8aa1104513e5324f7aaaca699f3c4160f4b35ba934b4c1fc3d236aa77c9824\",\"schemas\":{\"$defs\":{\"GreetInput\":{\"additionalProperties\":false,\"properties\":{\"name\":{\"maxLength\":200,\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\"],\"type\":\"object\"},\"GreetOutput\":{\"additionalProperties\":false,\"properties\":{\"message\":{\"maxLength\":1024,\"type\":\"string\"}},\"required\":[\"message\"],\"type\":\"object\"},\"InvalidNameData\":{\"additionalProperties\":false,\"properties\":{\"reason\":{\"maxLength\":200,\"type\":\"string\"}},\"required\":[\"reason\"],\"type\":\"object\"}}}}");
/** @param {{greet(input: {name:string}, context: import('@napi-vm/plugin-sdk').CallContext): Promise<{message:string}>}} handlers */
export function register(handlers) {
  return definePlugin(CONTRACT, {greet: async (input, context) => handlers.greet(/** @type {{name:string}} */(input), context)});
}
