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
import type { Translations } from "../i18n/translations.ts";
import { LOG_LEVELS, type LogEntry, type LogLevel } from "../components/logger/types.ts";

type WasmVm = ReturnType<typeof createVm>["vm"];

export type VmStatus = "loading" | "ready" | "error";

export function useVm(t: Translations) {
  const vmRef = useRef<unknown>(null);
  const getVm = useCallback(() => vmRef.current, []);
  const [status, setStatus] = useState<VmStatus>("loading");
  const [loopLimit, setLoopLimitState] = useState(5_000_000);
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [diagnostic, setDiagnostic] = useState<Diagnostic | null>(null);
  const entryId = useRef(0);

  const addEntry = useCallback((level: LogLevel, text: string, html?: string) => {
    const id = ++entryId.current;
    const timestamp = Date.now();
    setEntries((prev) => [...prev, { id, level, text, timestamp, html }]);
  }, []);

  const clearEntries = useCallback(() => {
    setEntries([]);
    setDiagnostic(null);
  }, []);

  const opts = useCallback(
    (): HostOptions => ({
      loopLimit,
      onAlert: (msg: string) =>
        addEntry(LOG_LEVELS.warn, msg, `<span class="tag">${t.alert}</span>${escapeHtml(msg)}`),
    }),
    [loopLimit, addEntry, t]
  );

  const run = useCallback(
    (code: string) => {
      const vm = vmRef.current;
      if (!vm) return;
      const t0 = performance.now();
      const r = runCode(vm as unknown as WasmVm, code);
      const ms = performance.now() - t0;

      for (const log of r.logs || []) {
        const level = log.level as LogLevel;
        addEntry(level, log.text);
      }

      if (r.ok) {
        addEntry(LOG_LEVELS.result, `${r.value}  ${ms.toFixed(1)} ms`, `<span class="arrow">&larr;</span>${escapeHtml(r.value)}<span class="ms">${ms.toFixed(1)} ms</span>`);
      } else {
        addEntry(LOG_LEVELS.error, `${r.error || t.error}  ${ms.toFixed(1)} ms`, `${escapeHtml(r.error || t.error)}<span class="ms">${ms.toFixed(1)} ms</span>`);
      }
    },
    [addEntry]
  );

  const reset = useCallback(() => {
    const vm = vmRef.current;
    if (!vm) return;
    (vm as unknown as WasmVm).reset();
    const failed = rehost(vm as unknown as WasmVm, opts());
    for (const f of failed) addEntry(LOG_LEVELS.sys, `${t.failedModule} ${f}`);
    addEntry(LOG_LEVELS.sys, t.vmReset);
  }, [opts, addEntry]);

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
        setStatus("ready");
        addEntry(LOG_LEVELS.sys, t.vmReady);
      } catch (e) {
        if (cancelled) return;
        setStatus("error");
        addEntry(LOG_LEVELS.sys, t.vmFailed + " " + e);
      }
    }
    boot();
    return () => { cancelled = true; };
  }, []);

  return {
    getVm,
    status,
    entries,
    diagnostic,
    loopLimit,
    run,
    reset,
    clearEntries,
    addEntry,
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
