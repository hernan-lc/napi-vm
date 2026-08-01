import init, { WasmVm } from "/pkg/napi_vm.js";
import { debugLog } from "./debug.ts";
import { MODULES, SAMPLE } from "./examples.ts";
import type { CompletionItem, Diagnostic, HostOptions, ModuleDef, RunResult } from "./types.ts";

export { SAMPLE, MODULES };
export type { CompletionItem, Diagnostic, RunResult, ModuleDef };

export async function initWasm(): Promise<void> {
  await init();
}

function setupHost(vm: WasmVm, { onAlert }: { onAlert: (msg: string) => void }): string[] {
  vm.expose_function("alert", (msg: unknown) => onAlert(String(msg)));
  const failed: string[] = [];
  for (const { name, source } of MODULES) {
    try {
      vm.register_module(name, source);
    } catch (e) {
      failed.push(`${name}: ${e}`);
    }
  }
  return failed;
}

export function createVm(opts: HostOptions): { vm: WasmVm; failed: string[] } {
  const vm = new WasmVm();
  vm.set_loop_limit(opts.loopLimit);
  const failed = setupHost(vm, { onAlert: opts.onAlert });
  return { vm, failed };
}

export function rehost(vm: WasmVm, opts: HostOptions): string[] {
  vm.set_loop_limit(opts.loopLimit);
  return setupHost(vm, { onAlert: opts.onAlert });
}

export function runCode(vm: WasmVm, code: string): RunResult {
  return vm.run(code) as RunResult;
}

export function complete(vm: WasmVm, code: string, byteOffset: number): CompletionItem[] {
  const result = vm.complete(code, byteOffset) as CompletionItem[];
  debugLog("wasm:complete", {
    byteOffset,
    resultCount: result?.length ?? 0,
    labels: (result || []).slice(0, 20).map((item) => item.label),
  });
  return result;
}

export function diagnose(vm: WasmVm, code: string): Diagnostic[] {
  return vm.diagnose(code) as Diagnostic[];
}

export function setLoopLimit(vm: WasmVm, n: number): void {
  vm.set_loop_limit(n);
}
