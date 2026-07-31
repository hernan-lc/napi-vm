// Playground frontend: drives the VM **in the browser**.
//
// The Rust interpreter is compiled to WebAssembly (`/pkg/napi_vm.js`), so there
// is no server round-trip: running code, capturing console output, and asking
// for completions/diagnostics all call the `WasmVm` directly. Completion and
// diagnostics come from the same shared Rust language core that a future LSP
// or native GUI will use — nothing is re-implemented here.
import init, { WasmVm } from "/pkg/napi_vm.js";

const editor = document.getElementById("editor");
const popup = document.getElementById("popup");
const consoleEl = document.getElementById("console");
const dot = document.getElementById("dot");
const statusText = document.getElementById("statusText");
const loopSelect = document.getElementById("loopLimit");
const diagEl = document.getElementById("diag");

const SAMPLE = `// napi-vm playground — a JS interpreter written in Rust, running
// in your browser via WebAssembly. Ctrl/⌘+Enter to run · Ctrl+Space to complete.
// Try "Math.", "arr.", "u." (an imported module), or "al" (an exposed function).

import * as u from "utils";

function fib(n) {
  return n < 2 ? n : fib(n - 1) + fib(n - 2);
}

const arr = [1, 2, 3, 4, 5];
console.log("fib(10) =", fib(10));
console.log("squares =", arr.map((x) => x * x));
console.log("double(21) =", u.double(21));

alert("hello from the VM");

class Counter {
  constructor() { this.n = 0; }
  bump() { this.n += 1; return this; }
}

const c = new Counter();
c.bump().bump().bump();
console.log("counter:", c);
c;
`;

// A module registered into the VM at startup, so `import * as u from "utils"`
// works and `u.` completion can offer its exports.
// Note: `String`/`Number` are namespace objects in this VM (not callable), so
// coerce with concatenation rather than `String(s)`.
const UTILS_SRC = `
export function double(x) { return x * 2; }
export function shout(s) { return (s + "").toUpperCase() + "!"; }
export const VERSION = "1.0.0";
`;

editor.value = SAMPLE;

// ---- VM lifecycle --------------------------------------------------------

let vm = null;

/** Wire the browser-side host surface into a VM: exposed fns + demo module. */
function setupHost(v) {
  // An exposed browser function: callable from the VM and offered as a
  // completion candidate (kind "exposed"). It renders into the playground
  // console so the host callback is visibly exercised.
  v.expose_function("alert", (msg) => {
    addLine("warn", `<span class="tag">alert</span>${escapeHtml(String(msg))}`);
  });
  try {
    v.register_module("utils", UTILS_SRC);
  } catch (e) {
    sys("failed to register utils module: " + e);
  }
}

function buildVm() {
  const v = new WasmVm();
  v.set_loop_limit(Number(loopSelect.value));
  setupHost(v);
  return v;
}

async function boot() {
  setStatus("", "loading wasm…");
  try {
    await init(); // fetch + stream-compile /pkg/napi_vm_bg.wasm
    vm = buildVm();
    setStatus("open", "ready");
    sys("WASM VM ready — running entirely in your browser");
    showDiagnostics();
  } catch (e) {
    setStatus("closed", "failed to load wasm");
    sys("could not initialise the WASM VM: " + e);
  }
}

// ---- console rendering ---------------------------------------------------

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function addLine(cls, html) {
  const div = document.createElement("div");
  div.className = "line " + cls;
  div.innerHTML = html;
  consoleEl.appendChild(div);
  consoleEl.scrollTop = consoleEl.scrollHeight;
}

function sys(text) {
  addLine("sys", escapeHtml(text));
}

function setStatus(cls, text) {
  dot.className = "dot " + cls;
  statusText.textContent = text;
}

const LEVEL_TAG = { warn: "warn", error: "error", dir: "dir", info: "info" };

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

// ---- run / reset / loop limit -------------------------------------------

function runCode() {
  if (!vm) return;
  const t0 = performance.now();
  const r = vm.run(editor.value);
  renderResult(r, performance.now() - t0);
}

document.getElementById("run").addEventListener("click", runCode);
document.getElementById("reset").addEventListener("click", () => {
  if (!vm) return;
  vm.reset();
  vm.set_loop_limit(Number(loopSelect.value));
  setupHost(vm);
  sys("VM state reset");
});
document.getElementById("clear").addEventListener("click", () => {
  consoleEl.innerHTML = "";
});
loopSelect.addEventListener("change", () => {
  if (vm) vm.set_loop_limit(Number(loopSelect.value));
});

// ---- editor keys ---------------------------------------------------------

editor.addEventListener("keydown", (e) => {
  const mod = e.ctrlKey || e.metaKey;

  if (mod && e.key === "Enter") {
    e.preventDefault();
    runCode();
    return;
  }
  if (e.key === "Tab" && !popupOpen()) {
    e.preventDefault();
    insertAtCursor("  ");
    return;
  }

  if (popupOpen()) {
    if (e.key === "ArrowDown") { e.preventDefault(); moveSel(1); return; }
    if (e.key === "ArrowUp") { e.preventDefault(); moveSel(-1); return; }
    if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); acceptSel(); return; }
    if (e.key === "Escape") { e.preventDefault(); closePopup(); return; }
    if (e.key === "ArrowLeft" || e.key === "ArrowRight" || e.key === "Home" || e.key === "End") {
      closePopup();
    }
  }

  if (mod && e.code === "Space") {
    e.preventDefault();
    requestCompletion(true);
  }
});

function insertAtCursor(text) {
  const start = editor.selectionStart;
  const end = editor.selectionEnd;
  editor.setRangeText(text, start, end, "end");
  editor.dispatchEvent(new Event("input"));
}

// ---- autocomplete (served by the shared Rust core) -----------------------

let acItems = [];
let acSel = 0;
let acPrefix = "";

function popupOpen() {
  return !popup.hidden;
}

/**
 * Classify the text before the caret just enough to drive UX: whether this is a
 * member completion (auto-popup after a dot) or a bare identifier (explicit
 * Ctrl+Space), and the word fragment to replace on accept. The actual
 * candidates — including scope analysis, exposed functions, and module exports
 * — come from `WasmVm.complete`, not from anything computed here.
 */
function analyze(before) {
  const word = (before.match(/([\w$]*)$/) || [, ""])[1];
  const isMember = /[\w$)\]"']\.[\w$]*$/.test(before);
  return { kind: isMember ? "member" : "ident", prefix: word };
}

let completeTimer = null;
editor.addEventListener("input", () => {
  clearTimeout(completeTimer);
  completeTimer = setTimeout(() => requestCompletion(false), 60);
  clearTimeout(diagTimer);
  diagTimer = setTimeout(showDiagnostics, 250);
});
editor.addEventListener("click", closePopup);
editor.addEventListener("blur", () => setTimeout(closePopup, 120));

function requestCompletion(force) {
  if (!vm) return;
  const caret = editor.selectionStart;
  const before = editor.value.slice(0, caret);
  const a = analyze(before);

  // Identifier completion is explicit-only to avoid noise; member completion
  // pops automatically after a dot.
  if (a.kind === "ident" && (!force || a.prefix.length === 0)) {
    closePopup();
    return;
  }

  // The core works in UTF-8 byte offsets; the textarea caret is a UTF-16 code
  // unit offset. They differ only for non-ASCII, so re-encode the prefix.
  const byteOffset = new TextEncoder().encode(before).length;
  const items = vm.complete(editor.value, byteOffset);
  acPrefix = a.prefix;
  showCompletions(items);
}

const KIND_LETTER = {
  variable: "x",
  function: "ƒ",
  method: "ƒ",
  property: "•",
  class: "C",
  module: "M",
  keyword: "k",
  global: "G",
  exposed: "h",
};

function showCompletions(items) {
  if (!items || items.length === 0) {
    closePopup();
    return;
  }
  acItems = items.slice(0, 50);
  acSel = 0;
  popup.innerHTML = "";
  acItems.forEach((it, i) => {
    const div = document.createElement("div");
    div.className = "item" + (i === acSel ? " sel" : "");
    const letter = KIND_LETTER[it.kind] || "?";
    const detail = it.detail ? `<span class="detail">${escapeHtml(it.detail)}</span>` : "";
    div.innerHTML = `<span class="kind ${it.kind}">${letter}</span>${escapeHtml(it.label)}${detail}`;
    div.addEventListener("mousedown", (e) => { e.preventDefault(); acceptItem(it); });
    div.addEventListener("mousemove", () => { if (acSel !== i) { acSel = i; paintSel(); } });
    popup.appendChild(div);
  });
  popup.hidden = false;
  positionPopup();
}

function paintSel() {
  [...popup.children].forEach((el, i) => el.classList.toggle("sel", i === acSel));
  const sel = popup.children[acSel];
  if (sel) sel.scrollIntoView({ block: "nearest" });
}

function moveSel(delta) {
  if (acItems.length === 0) return;
  acSel = (acSel + delta + acItems.length) % acItems.length;
  paintSel();
}

function acceptSel() {
  if (acItems[acSel]) acceptItem(acItems[acSel]);
}

function acceptItem(item) {
  const caret = editor.selectionStart;
  const start = caret - acPrefix.length;
  editor.setRangeText(item.label, start, caret, "end");
  closePopup();
  editor.focus();
}

function closePopup() {
  popup.hidden = true;
  acItems = [];
  acSel = 0;
  acPrefix = "";
}

// ---- live diagnostics (shared Rust core) ---------------------------------

let diagTimer = null;

function showDiagnostics() {
  if (!vm) return;
  let d;
  try {
    d = vm.diagnose(editor.value);
  } catch {
    return;
  }
  if (d && d.length) {
    diagEl.textContent = `⚠ ${d[0].message} · line ${d[0].line}`;
    diagEl.classList.add("bad");
  } else {
    diagEl.textContent = "";
    diagEl.classList.remove("bad");
  }
}

// ---- caret positioning for the popup -------------------------------------

function positionPopup() {
  const caret = editor.selectionStart;
  const coords = getCaretCoordinates(editor, caret);
  const cs = getComputedStyle(editor);
  let lh = parseFloat(cs.lineHeight);
  if (isNaN(lh)) lh = parseFloat(cs.fontSize) * 1.55;

  let left = coords.left - editor.scrollLeft + 2;
  let top = coords.top - editor.scrollTop + lh;

  const wrap = editor.parentElement;
  const maxLeft = wrap.clientWidth - popup.offsetWidth - 8;
  const maxTop = wrap.clientHeight - popup.offsetHeight - 8;
  left = Math.max(4, Math.min(left, maxLeft));
  top = Math.max(4, Math.min(top, maxTop));

  popup.style.left = left + "px";
  popup.style.top = top + "px";
}

// Mirror-div technique to measure caret pixel coordinates in a textarea.
function getCaretCoordinates(element, position) {
  const div = document.createElement("div");
  const style = div.style;
  const cs = getComputedStyle(element);
  const props = [
    "direction", "boxSizing", "width", "height", "overflowX", "overflowY",
    "borderTopWidth", "borderRightWidth", "borderBottomWidth", "borderLeftWidth",
    "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
    "fontStyle", "fontVariant", "fontWeight", "fontStretch", "fontSize",
    "fontSizeAdjust", "lineHeight", "fontFamily", "textAlign", "textTransform",
    "textIndent", "textDecoration", "letterSpacing", "wordSpacing", "tabSize",
    "whiteSpace",
  ];
  style.position = "absolute";
  style.visibility = "hidden";
  style.overflow = "hidden";
  for (const p of props) style[p] = cs[p];
  div.textContent = element.value.substring(0, position);
  const span = document.createElement("span");
  span.textContent = element.value.substring(position) || ".";
  div.appendChild(span);
  document.body.appendChild(div);
  const coords = {
    top: span.offsetTop + parseInt(cs.borderTopWidth, 10),
    left: span.offsetLeft + parseInt(cs.borderLeftWidth, 10),
  };
  document.body.removeChild(div);
  return coords;
}

boot();
