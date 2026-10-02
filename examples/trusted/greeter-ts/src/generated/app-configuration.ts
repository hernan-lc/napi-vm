// Generated. Do not edit.
import { validate, PluginError, type Contract, type CallOptions } from "@napi-vm/plugin-protocol";
import { definePlugin, type CallContext, type LifecycleHooks, type Handler as RuntimeHandler } from "@napi-vm/plugin-sdk";
export type GetInput = { "key": string };
export type GetOutput = { "value": string };
export const CONTRACT: Contract = JSON.parse("{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{},\"events\":{},\"id\":\"app.configuration\",\"methods\":{\"get\":{\"errors\":[],\"input\":\"#/$defs/GetInput\",\"output\":\"#/$defs/GetOutput\"}},\"version\":\"1.0.0\"},\"digest\":\"db06a9046edec0cff9afe336ca7bea7011133ac037f87dff207f652d2b47113f\",\"schemas\":{\"$defs\":{\"GetInput\":{\"additionalProperties\":false,\"properties\":{\"key\":{\"maxLength\":200,\"type\":\"string\"}},\"required\":[\"key\"],\"type\":\"object\"},\"GetOutput\":{\"additionalProperties\":false,\"properties\":{\"value\":{\"maxLength\":4096,\"type\":\"string\"}},\"required\":[\"value\"],\"type\":\"object\"}}}}") as Contract;
export function validateGetInput(value: unknown): GetInput { return validate(CONTRACT, "#/$defs/GetInput", value) as GetInput; }
export function validateGetOutput(value: unknown): GetOutput { return validate(CONTRACT, "#/$defs/GetOutput", value) as GetOutput; }
export interface Invoker { invoke(contract: Contract, method: string, input: unknown, options?: CallOptions): Promise<unknown> }
export function client(target: Invoker) { return {
["get"]: async (input: GetInput, options?: CallOptions): Promise<GetOutput> => { const result = await target.invoke(CONTRACT, "get", validate(CONTRACT, "#/$defs/GetInput", input), options); return validate(CONTRACT, "#/$defs/GetOutput", result, "INVALID_RESULT") as GetOutput; },
}; }
export interface Handlers {
["get"](input: GetInput, context: CallContext): Promise<GetOutput> | GetOutput;
}
export function register(handlers: Handlers, hooks?: LifecycleHooks) { const adapters: Record<string, RuntimeHandler> = Object.create(null);
adapters["get"] = async (input, context) => validate(CONTRACT, "#/$defs/GetOutput", await handlers["get"](validate(CONTRACT, "#/$defs/GetInput", input) as GetInput, context), "INVALID_RESULT");
return definePlugin(CONTRACT, adapters, hooks); }
export const errors = {
};
export interface Emitter { emit(contract: Contract, event: string, payload: unknown): Promise<void> }
export function events(target: Emitter) { return {
}; }
