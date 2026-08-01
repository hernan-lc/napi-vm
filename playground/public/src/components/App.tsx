import { useCallback, useRef, useState } from "preact/hooks";
import { Toolbar } from "./Toolbar.tsx";
import { Editor } from "./Editor.tsx";
import { CompletionPopup } from "./CompletionPopup.tsx";
import { Logger } from "./logger/Logger.tsx";
import { useVm } from "../hooks/useVm.ts";
import { useCompletion } from "../hooks/useCompletion.ts";
import { useResizable } from "../hooks/useResizable.ts";
import { useI18n } from "../i18n/useI18n.ts";
import { useTheme } from "../hooks/useTheme.ts";
import { SAMPLE } from "../examples.ts";
import { COMPLETION, EDITOR, EXAMPLE_FILE_NAMES, EXAMPLE_ICONS, EXAMPLE_IDS, RESIZER, UI, type ExampleId } from "../constants.ts";
import type { Translations } from "../i18n/translations.ts";

const EXAMPLES = [
  { id: EXAMPLE_IDS.modules, name: EXAMPLE_FILE_NAMES[EXAMPLE_IDS.modules], labelKey: "examplesModules", code: SAMPLE },
  {
    id: EXAMPLE_IDS.async,
    name: EXAMPLE_FILE_NAMES[EXAMPLE_IDS.async],
    labelKey: "examplesAsync",
    code: `async function loadUser(id) {
  const response = await Promise.resolve({ id, name: "Ada" });
  return response;
}

loadUser(42).then((user) => {
  console.log("loaded", user.name);
  user;
});`,
  },
  {
    id: EXAMPLE_IDS.loop,
    name: EXAMPLE_FILE_NAMES[EXAMPLE_IDS.loop],
    labelKey: "examplesLoop",
    code: `const start = Date.now();
let total = 0;

for (let i = 0; i < 1000; i++) {
  total += i;
}

console.log("sum", total);
console.log("elapsed", Date.now() - start, "ms");
total;`,
  },
];

type Example = (typeof EXAMPLES)[number];
type ExampleLabelKey = Example["labelKey"] & keyof Translations;

export function App() {
  const { t, locale, setLocale } = useI18n();
  const { theme, toggleTheme } = useTheme();
  const { ratio, containerRef, handlePointerDown } = useResizable();
  const { getVm, status, entries, diagnostic, loopLimit, run, reset, clearEntries, updateLoopLimit, refreshDiagnostic } = useVm(t);

  const [code, setCode] = useState(SAMPLE);
  const [activeExample, setActiveExample] = useState<ExampleId>(EXAMPLE_IDS.modules);
  const [cursor, setCursor] = useState({ line: 1, column: 1 });
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const diagnosticTimer = useRef<number | undefined>(undefined);
  const completionTimer = useRef<number | undefined>(undefined);

  const { state: compState, close: compClose, request: compRequest, move: compMove, accept: compAccept, isOpen: compOpen, kindLetter } = useCompletion(
    getVm,
    editorRef,
    popupRef,
  );

  const handleRun = useCallback(() => run(code), [run, code]);

  const handleCodeChange = useCallback((next: string) => {
    setCode(next);
    compClose();
    if (diagnosticTimer.current) window.clearTimeout(diagnosticTimer.current);
    diagnosticTimer.current = window.setTimeout(() => refreshDiagnostic(next), EDITOR.diagnosticDelayMs);
    if (completionTimer.current) window.clearTimeout(completionTimer.current);
    completionTimer.current = window.setTimeout(() => compRequest(false), COMPLETION.requestDelayMs);
  }, [compClose, compRequest, refreshDiagnostic]);

  const handleCompletionAccept = useCallback(() => {
    compAccept();
    setCode(editorRef.current?.value ?? "");
  }, [compAccept]);

  const openExample = useCallback((id: ExampleId) => {
    const example = EXAMPLES.find((item) => item.id === id);
    if (!example) return;
    setActiveExample(id);
    setCode(example.code);
    compClose();
    requestAnimationFrame(() => editorRef.current?.focus());
  }, [compClose]);

  return (
    <div class="app">
      <Toolbar
        status={status}
        loopLimit={loopLimit}
        diagnostic={diagnostic}
        theme={theme}
        locale={locale}
        t={t}
        onRun={handleRun}
        onReset={reset}
        onClear={clearEntries}
        onLoopLimitChange={updateLoopLimit}
        onToggleTheme={toggleTheme}
        onLocaleChange={setLocale}
      />

      <div class="workspace-shell">
        <aside class="sidebar">
          <div class="sidebar-section">
            <div class="sidebar-heading"><span>{t.explorer}</span><button class="sidebar-action" title={t.moreActions}>•••</button></div>
            <div class="workspace-root"><span class="chevron">⌄</span><span class="folder-icon">▱</span><span>{t.workspace}</span></div>
            <button class="file-row active"><span class="js-icon">JS</span><span>{t.fileName}</span><span class="file-state">·</span></button>
          </div>

          <div class="sidebar-section examples-section">
            <div class="sidebar-heading"><span>{t.examples}</span><span class="count-badge">{EXAMPLES.length}</span></div>
            {EXAMPLES.map((example) => (
              <button key={example.id} class={"example-row" + (activeExample === example.id ? " selected" : "")} onClick={() => openExample(example.id)}>
                <span class="example-icon">{EXAMPLE_ICONS[example.id]}</span>
                <span><strong>{example.name}</strong><small>{t[example.labelKey as ExampleLabelKey]}</small></span>
              </button>
            ))}
          </div>

          <div class="sidebar-footer">
            <div class="feature-note"><span class="spark">✦</span><div><strong>{t.languageTools}</strong><small>{t.languageToolsDescription}</small></div></div>
          </div>
        </aside>

        <main class="layout" ref={containerRef}>
          <section class="editor-wrap editor-panel" style={{ flex: `0 0 calc(${ratio * 100}% - ${ratio * RESIZER.dividerHeight}px)` }}>
            <div class="editor-panel-head">
              <div class="editor-tab active"><span class="js-icon">JS</span> {t.fileName} <span class="tab-close">×</span></div>
              <div class="editor-head-meta"><span class="language-pill">{t.editorLanguage}</span><span>{t.encoding}</span><span>{t.indentation}</span></div>
            </div>
            <div class="editor-hints">
              <span class="hint-breadcrumb"><span class="crumb-muted">{UI.brandSubtitle}</span><span>/</span><span>{t.fileName}</span><span>/</span><span>{UI.editorScope}</span></span>
              <span class="hint-actions"><span class="hint-key">⌘ Space</span> {t.autocomplete} <span class="hint-key">⌘ S</span> {t.runShortcut}</span>
            </div>
            <div class="editor-stage">
              <Editor
                value={code}
                onChange={handleCodeChange}
                onRun={handleRun}
                onCompletionRequest={compRequest}
                onCompletionClose={compClose}
                onCompletionMove={compMove}
                onCompletionAccept={handleCompletionAccept}
                completionOpen={compOpen}
                diagnostic={diagnostic}
                editorRef={editorRef}
                onCursorChange={(line, column) => setCursor({ line, column })}
                t={t}
              />
              <CompletionPopup
                items={compState.items}
                sel={compState.sel}
                open={compState.open}
                kindLetter={kindLetter}
                position={compState.position}
                onAccept={(item) => {
                  const editor = editorRef.current;
                  if (!editor) return;
                  const caret = editor.selectionStart;
                  const start = caret - compState.prefix.length;
                  editor.setRangeText(item.label, start, caret, "end");
                  setCode(editor.value);
                  compClose();
                  editor.focus();
                }}
                popupRef={popupRef}
              />
            </div>
            <div class="editor-statusbar">
              <span><span class="statusbar-green" /> {t.editorLanguage}</span>
              <span>Ln {cursor.line}, Col {cursor.column}</span>
              <span>{code.length.toLocaleString()} chars</span>
              <span class="statusbar-right">{diagnostic ? <span class="statusbar-error">! 1 {t.problems}</span> : <span class="statusbar-ok">✓ {t.noProblems}</span>} <span>⌁ {UI.wasmLabel}</span></span>
            </div>
          </section>

          <div class="divider-resizable" onPointerDown={handlePointerDown}>
            <span class="divider-label"><span class="console-icon">›_</span> {t.console}</span>
            <span class="divider-count">{entries.length} {entries.length === 1 ? t.entry : t.entries}</span>
          </div>
          <Logger entries={entries} t={t} />
        </main>
      </div>
    </div>
  );
}
