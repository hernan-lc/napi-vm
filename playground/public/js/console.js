// Console pane rendering: append log/result/error/system lines into the
// console element. Pure presentation — no VM knowledge.

const LEVEL_TAG = { warn: "warn", error: "error", dir: "dir", info: "info" };

export function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/**
 * Bind a console renderer to a container element.
 * @param {HTMLElement} el
 */
export function createConsole(el) {
  function addLine(cls, html) {
    const div = document.createElement("div");
    div.className = "line " + cls;
    div.innerHTML = html;
    el.appendChild(div);
    el.scrollTop = el.scrollHeight;
  }

  function sys(text) {
    addLine("sys", escapeHtml(text));
  }

  function clear() {
    el.innerHTML = "";
  }

  /** Render a `WasmVm.run` result (`{ ok, value, error, logs }`) and its logs. */
  function renderResult(r, ms) {
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
