// Live diagnostics indicator: debounced `WasmVm.diagnose` on the editor
// content, surfacing the first problem in the toolbar.
import { diagnose } from "./vm.js";

/**
 * @param {object} opts
 * @param {HTMLTextAreaElement} opts.editor
 * @param {HTMLElement} opts.el      the indicator element
 * @param {() => any} opts.getVm     returns the live WasmVm (or null)
 */
export function createDiagnostics({ editor, el, getVm }) {
  function refresh() {
    const vm = getVm();
    if (!vm) return;
    let d;
    try {
      d = diagnose(vm, editor.value);
    } catch {
      return;
    }
    if (d && d.length) {
      el.textContent = `⚠ ${d[0].message} · line ${d[0].line}`;
      el.classList.add("bad");
    } else {
      el.textContent = "";
      el.classList.remove("bad");
    }
  }

  return { refresh };
}
