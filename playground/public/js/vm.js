// VM lifecycle: load the wasm, build/reset a `WasmVm`, wire the browser-side
// host surface (an exposed `alert`, the registered demo modules), and expose
// thin wrappers over the shared language services.
//
// The heavy lifting — execution, completion, diagnostics — lives in the Rust
// core; this module only adapts it to the browser.
import init, { WasmVm } from "/pkg/napi_vm.js";
import { MODULES } from "./examples.js";

/** Fetch and stream-compile `/pkg/napi_vm_bg.wasm`. Call once before use. */
export async function initWasm() {
  await init();
}

/**
 * Expose the browser-side host surface on a VM: an `alert` function (callable
 * from the VM and offered as a completion) plus the registered demo modules.
 * Returns the list of modules that failed to register, if any.
 */
function setupHost(vm, { onAlert }) {
  vm.expose_function("alert", (msg) => onAlert(String(msg)));
  const failed = [];
  for (const { name, source } of MODULES) {
    try {
      vm.register_module(name, source);
    } catch (e) {
      failed.push(`${name}: ${e}`);
    }
  }
  return failed;
}

/**
 * Create a fresh VM with the loop cap and host surface applied.
 * @returns {{ vm: WasmVm, failed: string[] }}
 */
export function createVm({ loopLimit, onAlert }) {
  const vm = new WasmVm();
  vm.set_loop_limit(loopLimit);
  const failed = setupHost(vm, { onAlert });
  return { vm, failed };
}

/** Re-apply the loop cap and host surface after `vm.reset()`. */
export function rehost(vm, { loopLimit, onAlert }) {
  vm.set_loop_limit(loopLimit);
  return setupHost(vm, { onAlert });
}

// -- Language-service wrappers (kept thin so callers depend on this module,
//    not on the wasm glue directly) ----------------------------------------

export function runCode(vm, code) {
  return vm.run(code);
}

export function complete(vm, code, byteOffset) {
  return vm.complete(code, byteOffset);
}

export function diagnose(vm, code) {
  return vm.diagnose(code);
}

export function setLoopLimit(vm, n) {
  vm.set_loop_limit(n);
}
