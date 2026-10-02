// Generated. Do not edit.
import { validate, PluginError, type Contract, type CallOptions } from "@napi-vm/plugin-protocol";
import { definePlugin, type CallContext, type LifecycleHooks, type Handler as RuntimeHandler } from "@napi-vm/plugin-sdk";
export type WireTypes = { "__proto__": string; "choice": "🌍" | "ready" | "quote\"\\value" | "literal\\u0010" | "controls\b\f"; "created": string; "data": string; "integer": number; "literal": 1; "optionalNull"?: string | null; "optionalNumber"?: number | null; "self"?: string | null; "self-field"?: string; "self_field"?: string; "tagged": ({ "kind": "text"; "value": string }) | ({ "kind": "number"; "value": number }); "wide": string };
export const CONTRACT: Contract = JSON.parse("{\"descriptor\":{\"descriptorVersion\":1,\"errors\":{},\"events\":{},\"id\":\"example.wire-types\",\"methods\":{\"echo\":{\"errors\":[],\"input\":\"#/$defs/WireTypes\",\"output\":\"#/$defs/WireTypes\"}},\"version\":\"1.0.0\"},\"digest\":\"a0cdc912547e345bc9def20c0bb4110b60908cbcf4178fb68bea1e72734f2a7b\",\"schemas\":{\"$defs\":{\"WireTypes\":{\"additionalProperties\":false,\"properties\":{\"__proto__\":{\"type\":\"string\"},\"choice\":{\"enum\":[\"🌍\",\"ready\",\"quote\\\"\\\\value\",\"literal\\\\u0010\",\"controls\\b\\f\"],\"type\":\"string\"},\"created\":{\"type\":\"string\",\"x-wire-type\":\"utc-datetime\"},\"data\":{\"type\":\"string\",\"x-wire-type\":\"bytes-base64\"},\"integer\":{\"maximum\":9007199254740991,\"minimum\":-9007199254740991,\"type\":\"integer\"},\"literal\":{\"const\":1,\"type\":\"integer\"},\"optionalNull\":{\"type\":[\"string\",\"null\"]},\"optionalNumber\":{\"type\":[\"number\",\"null\"]},\"self\":{\"type\":[\"string\",\"null\"]},\"self-field\":{\"type\":\"string\"},\"self_field\":{\"type\":\"string\"},\"tagged\":{\"oneOf\":[{\"additionalProperties\":false,\"properties\":{\"kind\":{\"const\":\"text\",\"type\":\"string\"},\"value\":{\"type\":\"string\"}},\"required\":[\"kind\",\"value\"],\"type\":\"object\"},{\"additionalProperties\":false,\"properties\":{\"kind\":{\"const\":\"number\",\"type\":\"string\"},\"value\":{\"type\":\"number\"}},\"required\":[\"kind\",\"value\"],\"type\":\"object\"}]},\"wide\":{\"type\":\"string\",\"x-wire-type\":\"u64-decimal\"}},\"required\":[\"__proto__\",\"data\",\"wide\",\"created\",\"integer\",\"literal\",\"choice\",\"tagged\"],\"type\":\"object\"}}}}") as Contract;
export function validateWireTypes(value: unknown): WireTypes { return validate(CONTRACT, "#/$defs/WireTypes", value) as WireTypes; }
export interface Invoker { invoke(contract: Contract, method: string, input: unknown, options?: CallOptions): Promise<unknown> }
export function client(target: Invoker) { return {
["echo"]: async (input: WireTypes, options?: CallOptions): Promise<WireTypes> => { const result = await target.invoke(CONTRACT, "echo", validate(CONTRACT, "#/$defs/WireTypes", input), options); return validate(CONTRACT, "#/$defs/WireTypes", result, "INVALID_RESULT") as WireTypes; },
}; }
export interface Handlers {
["echo"](input: WireTypes, context: CallContext): Promise<WireTypes> | WireTypes;
}
export function register(handlers: Handlers, hooks?: LifecycleHooks) { const adapters: Record<string, RuntimeHandler> = Object.create(null);
adapters["echo"] = async (input, context) => validate(CONTRACT, "#/$defs/WireTypes", await handlers["echo"](validate(CONTRACT, "#/$defs/WireTypes", input) as WireTypes, context), "INVALID_RESULT");
return definePlugin(CONTRACT, adapters, hooks); }
export const errors = {
};
export interface Emitter { emit(contract: Contract, event: string, payload: unknown): Promise<void> }
export function events(target: Emitter) { return {
}; }
