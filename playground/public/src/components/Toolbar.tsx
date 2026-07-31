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
        <span class="brand-icon">&#9654;</span>
        <span class="brand-name">napi-vm</span>
        <span class="brand-sub">playground</span>
      </div>

      <div class="controls">
        <button class="primary" onClick={onRun} disabled={status !== "ready"} title={t.runHint}>
          &#9654; {t.run}
        </button>
        <button onClick={onReset} disabled={status !== "ready"} title="Discard all VM state">
          {t.reset}
        </button>
        <button onClick={onClear} title="Clear the console">
          {t.clear}
        </button>

        <label class="loop">
          {t.loopLimit}
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
            {t.diagWarning} {diagnostic.message} &middot; line {diagnostic.line}
          </span>
        )}
        <span class="hint">{t.runHint} &middot; {t.completeHint}</span>

        <select
          class="lang-select"
          value={locale}
          onChange={(e) => onLocaleChange((e.target as HTMLSelectElement).value as Locale)}
          title={t.language}
        >
          {(Object.keys(LOCALE_LABELS) as Locale[]).map((l) => (
            <option key={l} value={l}>{LOCALE_LABELS[l]}</option>
          ))}
        </select>

        <button
          class="icon-btn"
          onClick={onToggleTheme}
          title={t.theme}
          aria-label={t.theme}
        >
          {theme === "dark" ? "☀" : "☾"}
        </button>

        <span class="status">
          <span class={"dot " + statusClass}></span>
          <span>{statusLabel}</span>
        </span>
      </div>
    </header>
  );
}
