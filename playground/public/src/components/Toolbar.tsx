import type { VmStatus } from "../hooks/useVm.ts";
import type { Diagnostic } from "../types.ts";

interface ToolbarProps {
  status: VmStatus;
  loopLimit: number;
  diagnostic: Diagnostic | null;
  onRun: () => void;
  onReset: () => void;
  onClear: () => void;
  onLoopLimitChange: (n: number) => void;
}

export function Toolbar({
  status,
  loopLimit,
  diagnostic,
  onRun,
  onReset,
  onClear,
  onLoopLimitChange,
}: ToolbarProps) {
  const statusLabel = status === "loading" ? "loading wasm\u2026" : status === "ready" ? "ready" : "error";
  const statusClass = status === "ready" ? "open" : status === "error" ? "closed" : "";

  return (
    <header class="toolbar">
      <div class="brand">
        <span class="brand-icon">&#9654;</span>
        <span class="brand-name">napi-vm</span>
        <span class="brand-sub">playground</span>
      </div>

      <div class="controls">
        <button class="primary" onClick={onRun} disabled={status !== "ready"} title="Ctrl/&#8984;+Enter">
          &#9654; Run
        </button>
        <button onClick={onReset} disabled={status !== "ready"} title="Discard all VM state">
          Reset
        </button>
        <button onClick={onClear} title="Clear the console">
          Clear
        </button>

        <label class="loop">
          loop limit
          <select
            value={String(loopLimit)}
            onChange={(e) => onLoopLimitChange(Number((e.target as HTMLSelectElement).value))}
          >
            <option value="100000">100K</option>
            <option value="1000000">1M</option>
            <option value="5000000">5M</option>
            <option value="20000000">20M</option>
            <option value="100000000">100M</option>
          </select>
        </label>
      </div>

      <div class="right">
        {diagnostic && (
          <span class="diag bad">
            &#9888; {diagnostic.message} &middot; line {diagnostic.line}
          </span>
        )}
        <span class="hint">Ctrl/&#8984;+&#8629; run &middot; Ctrl+Space complete</span>
        <span class="status">
          <span class={"dot " + statusClass}></span>
          <span>{statusLabel}</span>
        </span>
      </div>
    </header>
  );
}
