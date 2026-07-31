import { diagnose } from "./vm";
import type { DiagnosticsController, Diagnostic } from "./types";

declare class WasmVm {
  diagnose(source: string): Diagnostic[];
}

interface DiagnosticsOpts {
  editor: HTMLTextAreaElement;
  el: HTMLElement;
  getVm: () => WasmVm | null;
}

export function createDiagnostics({ editor, el, getVm }: DiagnosticsOpts): DiagnosticsController {
  function refresh(): void {
    const vm = getVm();
    if (!vm) return;
    let d: Diagnostic[];
    try {
      d = diagnose(vm as any, editor.value) as Diagnostic[];
    } catch {
      return;
    }
    if (d && d.length) {
      el.textContent = `\u26A0 ${d[0].message} \u00B7 line ${d[0].line}`;
      el.classList.add("bad");
    } else {
      el.textContent = "";
      el.classList.remove("bad");
    }
  }

  return { refresh };
}
