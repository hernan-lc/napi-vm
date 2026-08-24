import type { Vm } from "../index";

export interface HostFunctionParam {
  readonly name: string;
  readonly typeName: string;
}

export interface HostFunctionMetadata {
  readonly params?: ReadonlyArray<HostFunctionParam>;
  readonly returns?: string;
  readonly documentation?: string;
  readonly async?: boolean;
  readonly languageService?: boolean;
  readonly public?: boolean;
}

export type LanguageShapeKind =
  | "unknown"
  | "any"
  | "void"
  | "undefined"
  | "null"
  | "boolean"
  | "number"
  | "string"
  | "array"
  | "promise"
  | "object"
  | "function";

export interface LanguageShapeParam {
  readonly name: string;
  readonly type?: LanguageShape | string;
  readonly shape?: LanguageShape | string;
  readonly typeName?: string;
}

export interface LanguageShape {
  readonly kind: LanguageShapeKind;
  readonly documentation?: string;
  readonly properties?: { readonly [key: string]: LanguageShape | string };
  readonly items?: LanguageShape | string;
  readonly value?: LanguageShape | string;
  readonly params?: ReadonlyArray<LanguageShapeParam>;
  readonly returns?: LanguageShape | string;
  readonly async?: boolean;
}

export interface GlobalMetadataOptions {
  readonly documentation?: string;
}

export interface VmSessionOptions {
  readonly workspace?: string;
  readonly sessionId?: string;
  readonly vm?: Vm | null;
}

export interface VmSessionModule {
  readonly name: string;
  readonly source: string;
}

export interface VmSessionHandlerShape {
  readonly name: string;
  readonly shape: unknown;
}

export declare class VmSession {
  constructor(options?: VmSessionOptions);
  readonly vm: Vm | null;
  readonly workspace: string;
  readonly runtimeFile: string;
  attach<T extends Vm = Vm>(vm: T, options?: { readonly modules?: ReadonlyArray<VmSessionModule> }): T;
  detach(): void;
  exposeFunction(name: string, fn: (...args: unknown[]) => unknown, info?: HostFunctionMetadata): void;
  exposeAsyncFunction(name: string, fn: (...args: unknown[]) => unknown, info?: HostFunctionMetadata): void;
  registerGlobal(name: string, shape: LanguageShape, options?: GlobalMetadataOptions): void;
  removeGlobal(name: string): boolean;
  registerModule(name: string, source: string): void;
  removeModule(name: string): boolean;
  observeHandler(name: string, value: unknown): boolean;
  run(source: string): string;
  runAsync(source: string): Promise<string>;
  start(): this;
  stop(): void;
  snapshot(): Record<string, unknown>;
}

export declare function runtimePath(workspace: string): string;


