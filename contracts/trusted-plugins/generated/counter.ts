// Generated. Do not edit.
import { validate, PluginError, type Contract, type CallOptions } from "@napi-vm/plugin-protocol";
import { definePlugin, type CallContext, type LifecycleHooks, type Handler as RuntimeHandler } from "@napi-vm/plugin-sdk";
export type AddInput = { "amount": number };
export type Count = { "count": string };
export type Empty = {  };
export const CONTRACT: Contract = JSON.parse("{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{},\"events\":{\"changed\":{\"payload\":\"#/$defs/Count\"}},\"id\":\"example.counter\",\"methods\":{\"add\":{\"errors\":[],\"input\":\"#/$defs/AddInput\",\"output\":\"#/$defs/Count\"},\"get\":{\"errors\":[],\"idempotent\":true,\"input\":\"#/$defs/Empty\",\"output\":\"#/$defs/Count\"}},\"state\":{\"contract\":\"example.counter.state\",\"schema\":\"#/$defs/Count\",\"version\":1},\"version\":\"1.0.0\"},\"digest\":\"a6cc7c1e4c4ceedaaa1dde5626c052d5320c1b26ee21c63e3d581d4d972fc90f\",\"schemas\":{\"$defs\":{\"AddInput\":{\"additionalProperties\":false,\"properties\":{\"amount\":{\"maximum\":1000000,\"minimum\":-1000000,\"type\":\"integer\"}},\"required\":[\"amount\"],\"type\":\"object\"},\"Count\":{\"additionalProperties\":false,\"properties\":{\"count\":{\"type\":\"string\",\"x-wire-type\":\"i64-decimal\"}},\"required\":[\"count\"],\"type\":\"object\"},\"Empty\":{\"additionalProperties\":false,\"properties\":{},\"required\":[],\"type\":\"object\"}}}}") as Contract;
export function validateAddInput(value: unknown): AddInput { return validate(CONTRACT, "#/$defs/AddInput", value) as AddInput; }
export function validateCount(value: unknown): Count { return validate(CONTRACT, "#/$defs/Count", value) as Count; }
export function validateEmpty(value: unknown): Empty { return validate(CONTRACT, "#/$defs/Empty", value) as Empty; }
export interface Invoker { invoke(contract: Contract, method: string, input: unknown, options?: CallOptions): Promise<unknown> }
export function client(target: Invoker) { return {
["add"]: async (input: AddInput, options?: CallOptions): Promise<Count> => { const result = await target.invoke(CONTRACT, "add", validate(CONTRACT, "#/$defs/AddInput", input), options); return validate(CONTRACT, "#/$defs/Count", result, "INVALID_RESULT") as Count; },
["get"]: async (input: Empty, options?: CallOptions): Promise<Count> => { const result = await target.invoke(CONTRACT, "get", validate(CONTRACT, "#/$defs/Empty", input), options); return validate(CONTRACT, "#/$defs/Count", result, "INVALID_RESULT") as Count; },
}; }
export interface Handlers {
["add"](input: AddInput, context: CallContext): Promise<Count> | Count;
["get"](input: Empty, context: CallContext): Promise<Count> | Count;
}
export function register(handlers: Handlers, hooks?: LifecycleHooks) { const adapters: Record<string, RuntimeHandler> = Object.create(null);
adapters["add"] = async (input, context) => validate(CONTRACT, "#/$defs/Count", await handlers["add"](validate(CONTRACT, "#/$defs/AddInput", input) as AddInput, context), "INVALID_RESULT");
adapters["get"] = async (input, context) => validate(CONTRACT, "#/$defs/Count", await handlers["get"](validate(CONTRACT, "#/$defs/Empty", input) as Empty, context), "INVALID_RESULT");
return definePlugin(CONTRACT, adapters, hooks); }
export const errors = {
};
export interface Emitter { emit(contract: Contract, event: string, payload: unknown): Promise<void> }
export function events(target: Emitter) { return {
["changed"]: (payload: Count) => target.emit(CONTRACT, "changed", validate(CONTRACT, "#/$defs/Count", payload)),
}; }
