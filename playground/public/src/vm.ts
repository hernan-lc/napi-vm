import init, { WasmVm } from "/pkg/napi_vm.js";
import { debugLog } from "./debug.ts";
import { MODULES, SAMPLE } from "./examples.ts";
import type { CompletionItem, Diagnostic, HoverInfo, HostOptions, ModuleDef, RunResult } from "./types.ts";

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

export function runCode(vm: WasmVm, code: string, moduleName?: string): RunResult {
  return (moduleName ? vm.run_file(moduleName, code) : vm.run(code)) as RunResult;
}

export function complete(vm: WasmVm, code: string, codeUnitOffset: number): CompletionItem[] {
  const byteOffset = utf8ByteOffset(code, codeUnitOffset);
  const result = vm.complete(code, byteOffset) as CompletionItem[];
  debugLog("wasm:complete", {
    byteOffset,
    resultCount: result?.length ?? 0,
    labels: (result || []).slice(0, 20).map((item) => item.label),
  });
  return result;
}

export function hover(vm: WasmVm, code: string, codeUnitOffset: number): HoverInfo | null {
  return vm.hover(code, utf8ByteOffset(code, codeUnitOffset)) as HoverInfo | null;
}

export function diagnose(vm: WasmVm, code: string): Diagnostic[] {
  return vm.diagnose(code) as Diagnostic[];
}

export function setLoopLimit(vm: WasmVm, n: number): void {
  vm.set_loop_limit(n);
}

// Textarea selectionStart and mouse offsets use UTF-16 code units. Rust
// receives UTF-8 byte offsets, so convert at the WASM boundary once rather
// than making every editor component understand two offset models.
function utf8ByteOffset(source: string, codeUnitOffset: number): number {
  const end = Math.max(0, Math.min(codeUnitOffset, source.length));
  let bytes = 0;
  for (let index = 0; index < end;) {
    const codePoint = source.codePointAt(index) ?? 0;
    bytes += codePoint <= 0x7f ? 1 : codePoint <= 0x7ff ? 2 : codePoint <= 0xffff ? 3 : 4;
    index += codePoint > 0xffff ? 2 : 1;
  }
  return bytes;
}
