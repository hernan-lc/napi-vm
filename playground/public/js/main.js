// Playground entry point: wire the DOM to the wasm VM, console, completion,
// and diagnostics modules, then boot. This is the only module the HTML loads
// directly; everything else is imported.
import { SAMPLE } from "./examples.js";
import { initWasm, createVm, rehost, runCode, setLoopLimit } from "./vm.js";
import { createConsole, escapeHtml } from "./console.js";
import { createCompletion } from "./completion.js";
import { createDiagnostics } from "./diagnostics.js";

const editor = document.getElementById("editor");
const popup = document.getElementById("popup");
const consoleEl = document.getElementById("console");
const dot = document.getElementById("dot");
const statusText = document.getElementById("statusText");
const loopSelect = document.getElementById("loopLimit");
const diagEl = document.getElementById("diag");

editor.value = SAMPLE;

const consoleView = createConsole(consoleEl);

let vm = null;

const hostOpts = () => ({
  loopLimit: Number(loopSelect.value),
  onAlert: (msg) =>
    consoleView.addLine("warn", `<span class="tag">alert</span>${escapeHtml(msg)}`),
});

function setStatus(cls, text) {
  dot.className = "dot " + cls;
  statusText.textContent = text;
}

// ---- run / reset ---------------------------------------------------------

function run() {
  if (!vm) return;
  const t0 = performance.now();
  const r = runCode(vm, editor.value);
  consoleView.renderResult(r, performance.now() - t0);
}

function reset() {
  if (!vm) return;
  vm.reset();
  const failed = rehost(vm, hostOpts());
  for (const f of failed) consoleView.sys("failed to register module " + f);
  consoleView.sys("VM state reset");
}

// ---- completion + diagnostics (debounced on input) -----------------------

const completion = createCompletion({ editor, popup, getVm: () => vm });
const diagnostics = createDiagnostics({ editor, el: diagEl, getVm: () => vm });

let completeTimer = null;
let diagTimer = null;
editor.addEventListener("input", () => {
  clearTimeout(completeTimer);
  completeTimer = setTimeout(() => completion.request(false), 60);
  clearTimeout(diagTimer);
  diagTimer = setTimeout(() => diagnostics.refresh(), 250);
});
editor.addEventListener("click", () => completion.close());
editor.addEventListener("blur", () => setTimeout(() => completion.close(), 120));

// ---- editor keys ---------------------------------------------------------

function insertAtCursor(text) {
  const start = editor.selectionStart;
  const end = editor.selectionEnd;
  editor.setRangeText(text, start, end, "end");
  editor.dispatchEvent(new Event("input"));
}

editor.addEventListener("keydown", (e) => {
  const mod = e.ctrlKey || e.metaKey;

  if (mod && e.key === "Enter") {
    e.preventDefault();
    run();
    return;
  }
  if (e.key === "Tab" && !completion.isOpen()) {
    e.preventDefault();
    insertAtCursor("  ");
    return;
  }

  if (completion.isOpen()) {
    if (e.key === "ArrowDown") { e.preventDefault(); completion.move(1); return; }
    if (e.key === "ArrowUp") { e.preventDefault(); completion.move(-1); return; }
    if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); completion.accept(); return; }
    if (e.key === "Escape") { e.preventDefault(); completion.close(); return; }
    if (e.key === "ArrowLeft" || e.key === "ArrowRight" || e.key === "Home" || e.key === "End") {
      completion.close();
    }
  }

  if (mod && e.code === "Space") {
    e.preventDefault();
    completion.request(true);
  }
});

// ---- toolbar buttons -----------------------------------------------------

document.getElementById("run").addEventListener("click", run);
document.getElementById("reset").addEventListener("click", reset);
document.getElementById("clear").addEventListener("click", () => consoleView.clear());
loopSelect.addEventListener("change", () => {
  if (vm) setLoopLimit(vm, Number(loopSelect.value));
});

// ---- boot ----------------------------------------------------------------

async function boot() {
  setStatus("", "loading wasm…");
  try {
    await initWasm(); // fetch + stream-compile /pkg/napi_vm_bg.wasm
    const built = createVm(hostOpts());
    vm = built.vm;
    for (const f of built.failed) consoleView.sys("failed to register module " + f);
    setStatus("open", "ready");
    consoleView.sys("WASM VM ready — running entirely in your browser");
    diagnostics.refresh();
  } catch (e) {
    setStatus("closed", "failed to load wasm");
    consoleView.sys("could not initialise the WASM VM: " + e);
  }
}

boot();
