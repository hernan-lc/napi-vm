// Generated. Do not edit.
import { validate, PluginError, type Contract, type CallOptions } from "@napi-vm/plugin-protocol";
import { definePlugin, type CallContext, type LifecycleHooks, type Handler as RuntimeHandler } from "@napi-vm/plugin-sdk";
export type GreetInput = { "name": string };
export type GreetOutput = { "message": string };
export type InvalidNameData = { "reason": string };
export const CONTRACT: Contract = JSON.parse("{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{\"INVALID_NAME\":{\"data\":\"#/$defs/InvalidNameData\"}},\"events\":{},\"id\":\"example.greeter\",\"methods\":{\"greet\":{\"errors\":[\"INVALID_NAME\"],\"idempotent\":true,\"input\":\"#/$defs/GreetInput\",\"output\":\"#/$defs/GreetOutput\"}},\"version\":\"1.0.0\"},\"digest\":\"ca8aa1104513e5324f7aaaca699f3c4160f4b35ba934b4c1fc3d236aa77c9824\",\"schemas\":{\"$defs\":{\"GreetInput\":{\"additionalProperties\":false,\"properties\":{\"name\":{\"maxLength\":200,\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\"],\"type\":\"object\"},\"GreetOutput\":{\"additionalProperties\":false,\"properties\":{\"message\":{\"maxLength\":1024,\"type\":\"string\"}},\"required\":[\"message\"],\"type\":\"object\"},\"InvalidNameData\":{\"additionalProperties\":false,\"properties\":{\"reason\":{\"maxLength\":200,\"type\":\"string\"}},\"required\":[\"reason\"],\"type\":\"object\"}}}}") as Contract;
export function validateGreetInput(value: unknown): GreetInput { return validate(CONTRACT, "#/$defs/GreetInput", value) as GreetInput; }
export function validateGreetOutput(value: unknown): GreetOutput { return validate(CONTRACT, "#/$defs/GreetOutput", value) as GreetOutput; }
export function validateInvalidNameData(value: unknown): InvalidNameData { return validate(CONTRACT, "#/$defs/InvalidNameData", value) as InvalidNameData; }
export interface Invoker { invoke(contract: Contract, method: string, input: unknown, options?: CallOptions): Promise<unknown> }
export function client(target: Invoker) { return {
["greet"]: async (input: GreetInput, options?: CallOptions): Promise<GreetOutput> => { const result = await target.invoke(CONTRACT, "greet", validate(CONTRACT, "#/$defs/GreetInput", input), options); return validate(CONTRACT, "#/$defs/GreetOutput", result, "INVALID_RESULT") as GreetOutput; },
}; }
export interface Handlers {
["greet"](input: GreetInput, context: CallContext): Promise<GreetOutput> | GreetOutput;
}
export function register(handlers: Handlers, hooks?: LifecycleHooks) { const adapters: Record<string, RuntimeHandler> = Object.create(null);
adapters["greet"] = async (input, context) => validate(CONTRACT, "#/$defs/GreetOutput", await handlers["greet"](validate(CONTRACT, "#/$defs/GreetInput", input) as GreetInput, context), "INVALID_RESULT");
return definePlugin(CONTRACT, adapters, hooks); }
export const errors = {
["INVALID_NAME"]: (data: InvalidNameData, message = "INVALID_NAME") => PluginError.domain("INVALID_NAME", validate(CONTRACT, "#/$defs/InvalidNameData", data), message),
};
export interface Emitter { emit(contract: Contract, event: string, payload: unknown): Promise<void> }
export function events(target: Emitter) { return {
}; }
