export interface HostFunctionMetadata {
  params?: Array<{ name: string; typeName: string }>;
  returns?: string;
  documentation?: string;
  async?: boolean;
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
