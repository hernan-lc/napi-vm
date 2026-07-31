import { useMemo, useState } from "preact/hooks";
import type { Translations } from "../i18n/translations.ts";
import type { LogEntry, LoggerFilter } from "./types.ts";
import { DEFAULT_FILTER, LEVEL_ICONS, LEVEL_LABELS } from "./types.ts";

interface LoggerProps {
  entries: LogEntry[];
  t: Translations;
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export function Logger({ entries, t }: LoggerProps) {
  const [filter, setFilter] = useState<LoggerFilter>(DEFAULT_FILTER);
  const [search, setSearch] = useState("");
  const [collapsed, setCollapsed] = useState(false);

  const filtered = useMemo(() => {
    let result = entries.filter((e) => filter[e.level]);
    if (search) {
      const q = search.toLowerCase();
      result = result.filter((e) => e.text.toLowerCase().includes(q));
    }
    return result;
  }, [entries, filter, search]);

  const counts = useMemo(() => {
    const c: Record<string, number> = {};
    for (const e of entries) {
      c[e.level] = (c[e.level] || 0) + 1;
    }
    return c;
  }, [entries]);

  const copyAll = () => {
    const text = filtered.map((e) => `[${LEVEL_LABELS[e.level]}] ${e.text}`).join("\n");
    navigator.clipboard.writeText(text).catch(() => {});
  };

  const toggleFilter = (level: keyof LoggerFilter) => {
    setFilter((f) => ({ ...f, [level]: !f[level] }));
  };

  if (entries.length === 0) {
    return (
      <div class="logger empty">
        <div class="placeholder">{t.emptyConsole}</div>
      </div>
    );
  }

  return (
    <div class="logger">
      <div class="logger-toolbar">
        <div class="logger-filters">
          {(Object.keys(DEFAULT_FILTER) as (keyof LoggerFilter)[]).map((level) => {
            const active = filter[level];
            const count = counts[level] || 0;
            return (
              <button
                key={level}
                class={"filter-chip" + (active ? " active" : "") + " level-" + level}
                onClick={() => toggleFilter(level)}
                title={LEVEL_LABELS[level]}
              >
                <span class="chip-icon">{LEVEL_ICONS[level]}</span>
                <span class="chip-count">{count}</span>
              </button>
            );
          })}
        </div>

        <div class="logger-actions">
          <input
            class="logger-search"
            type="text"
            placeholder="filter output…"
            value={search}
            onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
          />
          <button class="icon-btn" onClick={copyAll} title="Copy all">
            ⧉
          </button>
          <button
            class="icon-btn"
            onClick={() => setCollapsed(!collapsed)}
            title={collapsed ? "Expand" : "Collapse"}
          >
            {collapsed ? "▸" : "▾"}
          </button>
        </div>
      </div>

      <div class={"logger-body" + (collapsed ? " collapsed" : "")}>
        {filtered.length === 0 ? (
          <div class="logger-empty-filter">No matching entries</div>
        ) : (
          filtered.map((entry) => (
            <div key={entry.id} class={"log-entry level-" + entry.level}>
              <span class="log-time">{formatTime(entry.timestamp)}</span>
              <span class={"log-icon icon-" + entry.level}>{LEVEL_ICONS[entry.level]}</span>
              <span class="log-content">
                {entry.html ? (
                  <span dangerouslySetInnerHTML={{ __html: entry.html }} />
                ) : (
                  entry.text
                )}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
