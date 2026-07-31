import { useCallback, useRef, useState } from "preact/hooks";
import { Toolbar } from "./Toolbar.tsx";
import { Editor } from "./Editor.tsx";
import { Console } from "./Console.tsx";
import { CompletionPopup } from "./CompletionPopup.tsx";
import { useVm } from "../hooks/useVm.ts";
import { useCompletion } from "../hooks/useCompletion.ts";
import { useI18n } from "../i18n/useI18n.ts";
import { useTheme } from "../hooks/useTheme.ts";
import { SAMPLE } from "../examples.ts";

export function App() {
  const { t, locale, setLocale } = useI18n();
  const { theme, toggleTheme } = useTheme();

  const {
    status,
    lines,
    diagnostic,
    loopLimit,
    run,
    reset,
    clearLines,
    updateLoopLimit,
    refreshDiagnostic,
  } = useVm(t);

  const [code, setCode] = useState(SAMPLE);
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);

  const getVmRef = useRef<unknown>(null);

  const { state: compState, close: compClose, request: compRequest, move: compMove, accept: compAccept, isOpen: compOpen, kindLetter } = useCompletion(
    () => getVmRef.current,
    editorRef as any,
    popupRef as any
  );

  const handleRun = useCallback(() => {
    run(code);
  }, [run, code]);

  const handleCompletionClose = useCallback(() => {
    compClose();
  }, [compClose]);

  const handleCompletionRequest = useCallback(
    (force: boolean) => {
      compRequest(force);
    },
    [compRequest]
  );

  const handleCompletionMove = useCallback(
    (delta: number) => {
      compMove(delta);
    },
    [compMove]
  );

  const handleCompletionAccept = useCallback(() => {
    compAccept();
    setCode(editorRef.current?.value ?? "");
  }, [compAccept]);

  const handleCodeChange = useCallback(
    (v: string) => {
      setCode(v);
      handleCompletionClose();
      setTimeout(() => refreshDiagnostic(v), 250);
      setTimeout(() => handleCompletionRequest(false), 60);
    },
    [handleCompletionClose, refreshDiagnostic, handleCompletionRequest]
  );

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
        onClear={clearLines}
        onLoopLimitChange={updateLoopLimit}
        onToggleTheme={toggleTheme}
        onLocaleChange={setLocale}
      />
      <main class="layout">
        <section class="editor-wrap">
          <Editor
            value={code}
            onChange={handleCodeChange}
            onRun={handleRun}
            onCompletionRequest={handleCompletionRequest}
            onCompletionClose={handleCompletionClose}
            onCompletionMove={handleCompletionMove}
            onCompletionAccept={handleCompletionAccept}
            completionOpen={compOpen}
            diagnostic={diagnostic}
            editorRef={editorRef as any}
          />
          <CompletionPopup
            items={compState.items}
            sel={compState.sel}
            open={compState.open}
            kindLetter={kindLetter}
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
            popupRef={popupRef as any}
          />
        </section>
        <div class="divider">
          <span>{t.console}</span>
          <span class="divider-count">{lines.length} {lines.length === 1 ? t.entry : t.entries}</span>
        </div>
        <Console lines={lines} t={t} />
      </main>
    </div>
  );
}
