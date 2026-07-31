import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
  createVm,
  diagnose,
  initWasm,
  rehost,
  runCode,
  setLoopLimit,
} from "../vm.ts";
import type { Diagnostic, HostOptions, RunResult } from "../types.ts";

declare class WasmVm {
  set_loop_limit(n: number): void;
  reset(): void;
  expose_function(name: string, fn: (msg: unknown) => void): void;
  register_module(name: string, source: string): void;
  run(code: string): RunResult;
  complete(source: string, offset: number): import("../types.ts").CompletionItem[];
  diagnose(source: string): Diagnostic[];
}

export type VmStatus = "loading" | "ready" | "error";

export interface ConsoleLine {
  id: number;
  cls: string;
  html: string;
}

export function useVm() {
  const vmRef = useRef<unknown>(null);
  const [status, setStatus] = useState<VmStatus>("loading");
  const [loopLimit, setLoopLimitState] = useState(5_000_000);
  const [lines, setLines] = useState<ConsoleLine[]>([]);
  const [failedModules, setFailedModules] = useState<string[]>([]);
  const [diagnostic, setDiagnostic] = useState<Diagnostic | null>(null);
  const lineId = useRef(0);

  const addLine = useCallback((cls: string, html: string) => {
    const id = ++lineId.current;
    setLines((prev) => [...prev, { id, cls, html }]);
  }, []);

  const sys = useCallback(
    (text: string) => addLine("sys", escapeHtml(text)),
    [addLine]
  );

  const clearLines = useCallback(() => {
    setLines([]);
    setDiagnostic(null);
  }, []);

  const opts = useCallback(
    (): HostOptions => ({
      loopLimit,
      onAlert: (msg: string) =>
        addLine("warn", `<span class="tag">alert</span>${escapeHtml(msg)}`),
    }),
    [loopLimit, addLine]
  );

  const run = useCallback(
    (code: string) => {
      const vm = vmRef.current;
      if (!vm) return;
      const t0 = performance.now();
      const r = runCode(vm as unknown as WasmVm, code);
      const ms = performance.now() - t0;

      for (const log of r.logs || []) {
        const tag = log.level !== "log" ? `<span class="tag">${log.level}</span>` : "";
        addLine(log.level, tag + escapeHtml(log.text));
      }

      const msHtml = `<span class="ms">${ms.toFixed(1)} ms</span>`;
      if (r.ok) {
        addLine("result", `<span class="arrow">&larr;</span>${escapeHtml(r.value)}${msHtml}`);
      } else {
        addLine("error", `${escapeHtml(r.error || "error")}${msHtml}`);
      }
    },
    [addLine]
  );

  const reset = useCallback(() => {
    const vm = vmRef.current;
    if (!vm) return;
    (vm as unknown as WasmVm).reset();
    const failed = rehost(vm as unknown as WasmVm, opts());
    setFailedModules(failed);
    for (const f of failed) sys("failed to register module " + f);
    sys("VM state reset");
  }, [opts, sys]);

  const updateLoopLimit = useCallback(
    (n: number) => {
      setLoopLimitState(n);
      if (vmRef.current) setLoopLimit(vmRef.current as unknown as WasmVm, n);
    },
    []
  );

  const refreshDiagnostic = useCallback((code: string) => {
    const vm = vmRef.current;
    if (!vm) return;
    try {
      const d = diagnose(vm as unknown as WasmVm, code);
      setDiagnostic(d && d.length ? d[0] : null);
    } catch {
      setDiagnostic(null);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function boot() {
      try {
        await initWasm();
        if (cancelled) return;
        const built = createVm(opts());
        vmRef.current = built.vm;
        setFailedModules(built.failed);
        setStatus("ready");
        sys("WASM VM ready \u2014 running entirely in your browser");
      } catch (e) {
        if (cancelled) return;
        setStatus("error");
        sys("could not initialise the WASM VM: " + e);
      }
    }
    boot();
    return () => { cancelled = true; };
  }, []);

  return {
    status,
    lines,
    diagnostic,
    failedModules,
    loopLimit,
    run,
    reset,
    clearLines,
    sys,
    updateLoopLimit,
    refreshDiagnostic,
  };
}

function escapeHtml(s: string | number | boolean): string {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
