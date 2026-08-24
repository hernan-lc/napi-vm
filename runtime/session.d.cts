export interface HostFunctionMetadata {
  params?: Array<{ name: string; typeName: string }>;
  returns?: string;
  documentation?: string;
  async?: boolean;
  languageService?: boolean;
  public?: boolean;
}

export interface LanguageShape {
  kind: "unknown" | "any" | "void" | "undefined" | "null" | "boolean" | "number" | "string" | "array" | "promise" | "object" | "function";
  documentation?: string;
  properties?: Record<string, LanguageShape | string>;
  items?: LanguageShape | string;
  value?: LanguageShape | string;
  params?: Array<{ name: string; type?: LanguageShape | string; shape?: LanguageShape | string; typeName?: string }>;
  returns?: LanguageShape | string;
  async?: boolean;
}

export interface GlobalMetadataOptions {
  documentation?: string;
}

export interface VmSessionOptions {
  workspace?: string;
  sessionId?: string;
  vm?: any;
}

export interface VmSessionModule {
  name: string;
  source: string;
}

export interface VmSessionHandlerShape {
  name: string;
  shape: unknown;
}

export declare class VmSession {
  constructor(options?: VmSessionOptions);
  readonly vm: any;
  readonly workspace: string;
  readonly runtimeFile: string;
  attach(vm: any, options?: { modules?: VmSessionModule[] }): any;
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
  snapshot(): object;
}

export declare function runtimePath(workspace: string): string;
