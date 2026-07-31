import { SAMPLE } from "./examples";
import { initWasm, createVm, rehost, runCode, setLoopLimit } from "./vm";
import { createConsole, escapeHtml } from "./console";
import { createCompletion } from "./completion";
import { createDiagnostics } from "./diagnostics";

const editor = document.getElementById("editor") as HTMLTextAreaElement;
const popup = document.getElementById("popup") as HTMLElement;
const consoleEl = document.getElementById("console") as HTMLElement;
const dot = document.getElementById("dot") as HTMLElement;
const statusText = document.getElementById("statusText") as HTMLElement;
const loopSelect = document.getElementById("loopLimit") as HTMLSelectElement;
const diagEl = document.getElementById("diag") as HTMLElement;

editor.value = SAMPLE;

const consoleView = createConsole(consoleEl);

let vm: any | null = null;

const hostOpts = () => ({
  loopLimit: Number(loopSelect.value),
  onAlert: (msg: string) =>
    consoleView.addLine("warn", `<span class="tag">alert</span>${escapeHtml(msg)}`),
});

function setStatus(cls: string, text: string): void {
  dot.className = "dot " + cls;
  statusText.textContent = text;
}

function run(): void {
  if (!vm) return;
  const t0 = performance.now();
  const r = runCode(vm, editor.value);
  consoleView.renderResult(r, performance.now() - t0);
}

function reset(): void {
  if (!vm) return;
  vm.reset();
  const failed = rehost(vm, hostOpts());
  for (const f of failed) consoleView.sys("failed to register module " + f);
  consoleView.sys("VM state reset");
}

const completion = createCompletion({ editor, popup, getVm: () => vm });
const diagnostics = createDiagnostics({ editor, el: diagEl, getVm: () => vm });

let completeTimer: ReturnType<typeof setTimeout> | null = null;
let diagTimer: ReturnType<typeof setTimeout> | null = null;
editor.addEventListener("input", () => {
  if (completeTimer !== null) clearTimeout(completeTimer);
  completeTimer = setTimeout(() => completion.request(false), 60);
  if (diagTimer !== null) clearTimeout(diagTimer);
  diagTimer = setTimeout(() => diagnostics.refresh(), 250);
});
editor.addEventListener("click", () => completion.close());
editor.addEventListener("blur", () => setTimeout(() => completion.close(), 120));

function insertAtCursor(text: string): void {
  const start = editor.selectionStart;
  const end = editor.selectionEnd;
  editor.setRangeText(text, start, end, "end");
  editor.dispatchEvent(new Event("input"));
}

editor.addEventListener("keydown", (e: KeyboardEvent) => {
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

document.getElementById("run")!.addEventListener("click", run);
document.getElementById("reset")!.addEventListener("click", reset);
document.getElementById("clear")!.addEventListener("click", () => consoleView.clear());
loopSelect.addEventListener("change", () => {
  if (vm) setLoopLimit(vm, Number(loopSelect.value));
});

async function boot(): Promise<void> {
  setStatus("", "loading wasm\u2026");
  try {
    await initWasm();
    const built = createVm(hostOpts());
    vm = built.vm;
    for (const f of built.failed) consoleView.sys("failed to register module " + f);
    setStatus("open", "ready");
    consoleView.sys("WASM VM ready \u2014 running entirely in your browser");
    diagnostics.refresh();
  } catch (e) {
    setStatus("closed", "failed to load wasm");
    consoleView.sys("could not initialise the WASM VM: " + e);
  }
}

boot();
