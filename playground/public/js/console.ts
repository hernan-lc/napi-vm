import type { ConsoleView, RunResult } from "./types";

const LEVEL_TAG: Record<string, string> = {
  warn: "warn",
  error: "error",
  dir: "dir",
  info: "info",
};

export function escapeHtml(s: string | number | boolean): string {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

export function createConsole(el: HTMLElement): ConsoleView {
  function addLine(cls: string, html: string): void {
    const div = document.createElement("div");
    div.className = "line " + cls;
    div.innerHTML = html;
    el.appendChild(div);
    el.scrollTop = el.scrollHeight;
  }

  function sys(text: string): void {
    addLine("sys", escapeHtml(text));
  }

  function clear(): void {
    el.innerHTML = "";
  }

  function renderResult(r: RunResult, ms: number): void {
    for (const log of r.logs || []) {
      const tag = LEVEL_TAG[log.level];
      const prefix = tag ? `<span class="tag">${tag}</span>` : "";
      addLine(log.level, prefix + escapeHtml(log.text));
    }
    const msHtml = `<span class="ms">${ms.toFixed(1)} ms</span>`;
    if (r.ok) {
      addLine("result", `<span class="arrow">←</span>${escapeHtml(r.value)}${msHtml}`);
    } else {
      addLine("error", `${escapeHtml(r.error || "error")}${msHtml}`);
    }
  }

  return { addLine, sys, clear, renderResult };
}
