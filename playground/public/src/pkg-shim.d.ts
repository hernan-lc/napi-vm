declare module "/pkg/napi_vm.js" {
  export default function init(): Promise<void>;
  export class WasmVm {
    constructor();
    set_loop_limit(n: number): void;
    reset(): void;
    expose_function(name: string, fn: (msg: unknown) => void): void;
    expose_function_with_info(name: string, fn: (...args: unknown[]) => unknown, metadata: HostFunctionInfo): void;
    register_module(name: string, source: string): void;
    run(code: string): unknown;
    run_file(name: string, code: string): unknown;
    complete(source: string, offset: number): unknown;
    hover(source: string, offset: number): unknown;
    diagnose(source: string): unknown;
  }

  interface HostFunctionInfo {
    params?: { name: string; type: string }[];
    returns?: string;
    documentation?: string;
    async?: boolean;
  }
}
