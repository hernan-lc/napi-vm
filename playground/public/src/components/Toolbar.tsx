import type { VmStatus } from "../hooks/useVm.ts";
import type { Diagnostic } from "../types.ts";
import type { Translations } from "../i18n/translations.ts";
import type { Locale } from "../i18n/translations.ts";
import { LOCALE_LABELS } from "../i18n/translations.ts";
import type { Theme } from "../hooks/useTheme.ts";

interface ToolbarProps {
  status: VmStatus;
  loopLimit: number;
  diagnostic: Diagnostic | null;
  theme: Theme;
  locale: Locale;
  t: Translations;
  onRun: () => void;
  onReset: () => void;
  onClear: () => void;
  onLoopLimitChange: (n: number) => void;
  onToggleTheme: () => void;
  onLocaleChange: (l: Locale) => void;
}

export function Toolbar({
  status,
  loopLimit,
  diagnostic,
  theme,
  locale,
  t,
  onRun,
  onReset,
  onClear,
  onLoopLimitChange,
  onToggleTheme,
  onLocaleChange,
}: ToolbarProps) {
  const statusLabel = status === "loading" ? t.loadingWasm : status === "ready" ? t.ready : t.error;
  const statusClass = status === "ready" ? "open" : status === "error" ? "closed" : "";

  return (
    <header class="toolbar">
      <div class="brand">
        <span class="brand-mark"><span></span><span></span><span></span></span>
        <span class="brand-name">napi-vm</span>
        <span class="brand-sub">/ playground</span>
      </div>

      <div class="toolbar-separator" />

      <div class="toolbar-file"><span class="file-dot" /> playground.js <span class="file-state">·</span></div>

      <div class="controls">
        <button class="primary run-button" onClick={onRun} disabled={status !== "ready"} title={t.runHint}>
          <span class="button-play">▶</span> {t.run}
          <kbd>⌘ ↵</kbd>
        </button>
        <button class="toolbar-button" onClick={onReset} disabled={status !== "ready"} title="Discard all VM state">
          ↻ <span class="button-label">{t.reset}</span>
        </button>
        <button class="toolbar-button" onClick={onClear} title="Clear the console">
          ◌ <span class="button-label">{t.clear}</span>
        </button>
      </div>

      <div class="right">
        {diagnostic && <span class="diag bad"><span class="diag-dot" /> {t.diagWarning}</span>}
        <label class="loop compact-loop">
          <span>limit</span>
          <select value={String(loopLimit)} onChange={(e) => onLoopLimitChange(Number((e.target as HTMLSelectElement).value))}>
            <option value="100000">100K</option>
            <option value="1000000">1M</option>
            <option value="5000000">5M</option>
            <option value="20000000">20M</option>
            <option value="100000000">100M</option>
          </select>
        </label>
        <span class="status"><span class={"dot " + statusClass}></span><span>{statusLabel}</span></span>
        <select class="lang-select" value={locale} onChange={(e) => onLocaleChange((e.target as HTMLSelectElement).value as Locale)} title={t.language}>
          {(Object.keys(LOCALE_LABELS) as Locale[]).map((l) => <option key={l} value={l}>{LOCALE_LABELS[l]}</option>)}
        </select>
        <button class="icon-btn theme-button" onClick={onToggleTheme} title={t.theme} aria-label={t.theme}>{theme === "dark" ? "☼" : "☾"}</button>
      </div>
    </header>
  );
}
