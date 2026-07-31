import init, { WasmVm } from "/pkg/napi_vm.js";
import { MODULES } from "./examples";
import type { HostOptions } from "./types";

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

export function runCode(vm: WasmVm, code: string): ReturnType<WasmVm["run"]> {
  return vm.run(code);
}

export function complete(vm: WasmVm, code: string, byteOffset: number): ReturnType<WasmVm["complete"]> {
  return vm.complete(code, byteOffset);
}

export function diagnose(vm: WasmVm, code: string): ReturnType<WasmVm["diagnose"]> {
  return vm.diagnose(code);
}

export function setLoopLimit(vm: WasmVm, n: number): void {
  vm.set_loop_limit(n);
}
