import { useCallback, useEffect, useRef, useState } from "preact/hooks";
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
import { COMPLETION, EDITOR, RESIZER, UI, WORKSPACE } from "../constants.ts";
import type { Translations } from "../i18n/translations.ts";
import type { WorkspaceFile } from "../types.ts";

const ASYNC_SAMPLE = `async function loadUser(id) {
  const response = await Promise.resolve({ id, name: "Ada" });
  return response;
}

loadUser(42).then((user) => {
  console.log("loaded", user.name);
  user;
});`;

const LOOP_SAMPLE = `const start = Date.now();
let total = 0;

for (let i = 0; i < 1000; i++) {
  total += i;
}

console.log("sum", total);
console.log("elapsed", Date.now() - start, "ms");
total;`;

const SEED_FILES: WorkspaceFile[] = [
  { id: "playground", name: "playground.js", content: SAMPLE },
  { id: "async", name: "async.js", content: ASYNC_SAMPLE },
  { id: "loop", name: "loop-guard.js", content: LOOP_SAMPLE },
];

function loadWorkspace(): WorkspaceFile[] {
  try {
    const raw = localStorage.getItem(WORKSPACE.storageKey);
    if (!raw) return SEED_FILES;
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return SEED_FILES;
    const files = parsed.filter((file): file is WorkspaceFile => (
      !!file && typeof file === "object" &&
      typeof (file as WorkspaceFile).id === "string" &&
      typeof (file as WorkspaceFile).name === "string" &&
      typeof (file as WorkspaceFile).content === "string"
    ));
    return files.length > 0 ? files : SEED_FILES;
  } catch {
    return SEED_FILES;
  }
}

function uniqueFileName(files: WorkspaceFile[], requested: string, ignoredId?: string): string {
  const base = requested.trim() || "untitled";
  const name = base.includes(".") ? base : `${base}${WORKSPACE.fileExtension}`;
  const taken = new Set(files.filter((file) => file.id !== ignoredId).map((file) => file.name));
  if (!taken.has(name)) return name;
  const dot = name.lastIndexOf(".");
  const stem = dot > 0 ? name.slice(0, dot) : name;
  const extension = dot > 0 ? name.slice(dot) : WORKSPACE.fileExtension;
  let index = 1;
  while (taken.has(`${stem}-${index}${extension}`)) index++;
  return `${stem}-${index}${extension}`;
}

function nextFileId(files: WorkspaceFile[]): string {
  const used = new Set(files.map((file) => file.id));
  let index = 1;
  while (used.has(`file-${index}`)) index++;
  return `file-${index}`;
}

export function App() {
  const { t, locale, setLocale } = useI18n();
  const { theme, toggleTheme } = useTheme();
  const { ratio, containerRef, handlePointerDown } = useResizable();
  const { getVm, status, entries, diagnostic, loopLimit, run, reset, clearEntries, updateLoopLimit, refreshDiagnostic } = useVm(t);

  const [files, setFiles] = useState<WorkspaceFile[]>(loadWorkspace);
  const [activeFileId, setActiveFileId] = useState<string>(WORKSPACE.defaultFileId);
  const [openTabs, setOpenTabs] = useState<string[]>([WORKSPACE.defaultFileId]);
  const [cursor, setCursor] = useState({ line: 1, column: 1 });
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const renameRef = useRef<HTMLInputElement>(null);
  const diagnosticTimer = useRef<number | undefined>(undefined);
  const completionTimer = useRef<number | undefined>(undefined);

  const activeFile = files.find((file) => file.id === activeFileId) ?? files[0];
  const activeContent = activeFile?.content ?? "";

  const { state: compState, close: compClose, request: compRequest, move: compMove, accept: compAccept, isOpen: compOpen, kindLetter } = useCompletion(
    getVm,
    editorRef,
    popupRef,
  );

  useEffect(() => {
    localStorage.setItem(WORKSPACE.storageKey, JSON.stringify(files));
  }, [files]);

  useEffect(() => {
    if (files.some((file) => file.id === activeFileId)) return;
    const fallback = files[0];
    if (fallback) {
      setActiveFileId(fallback.id);
      setOpenTabs((tabs) => tabs.includes(fallback.id) ? tabs : [fallback.id, ...tabs.filter((id) => files.some((file) => file.id === id))]);
    }
  }, [activeFileId, files]);

  useEffect(() => {
    if (renamingId) requestAnimationFrame(() => renameRef.current?.focus());
  }, [renamingId]);

  const updateActiveContent = useCallback((content: string) => {
    setFiles((current) => current.map((file) => file.id === activeFileId ? { ...file, content, dirty: true } : file));
  }, [activeFileId]);

  const handleRun = useCallback(() => {
    if (activeFile) run(activeFile.content);
  }, [activeFile, run]);

  const handleCodeChange = useCallback((next: string) => {
    updateActiveContent(next);
    compClose();
    if (diagnosticTimer.current) window.clearTimeout(diagnosticTimer.current);
    diagnosticTimer.current = window.setTimeout(() => refreshDiagnostic(next), EDITOR.diagnosticDelayMs);
    if (completionTimer.current) window.clearTimeout(completionTimer.current);
    completionTimer.current = window.setTimeout(() => compRequest(false), COMPLETION.requestDelayMs);
  }, [compClose, compRequest, refreshDiagnostic, updateActiveContent]);

  const openFile = useCallback((id: string) => {
    setActiveFileId(id);
    setOpenTabs((current) => current.includes(id) ? current : [...current, id]);
    setRenamingId(null);
    compClose();
    setCursor({ line: 1, column: 1 });
    requestAnimationFrame(() => {
      const editor = editorRef.current;
      if (!editor) return;
      editor.scrollTop = 0;
      editor.scrollLeft = 0;
      editor.focus();
    });
  }, [compClose]);

  const createFile = useCallback(() => {
    setFiles((current) => {
      const file: WorkspaceFile = {
        id: nextFileId(current),
        name: uniqueFileName(current, t.untitledFile),
        content: "",
        dirty: true,
      };
      setActiveFileId(file.id);
      setOpenTabs((tabs) => [...tabs, file.id]);
      setRenamingId(file.id);
      setRenameValue(file.name);
      return [...current, file];
    });
  }, [t.untitledFile]);

  const beginRename = useCallback((file: WorkspaceFile, event: Event) => {
    event.stopPropagation();
    setRenamingId(file.id);
    setRenameValue(file.name);
  }, []);

  const commitRename = useCallback(() => {
    if (!renamingId) return;
    setFiles((current) => current.map((file) => file.id === renamingId
      ? { ...file, name: uniqueFileName(current, renameValue, renamingId), dirty: true }
      : file));
    setRenamingId(null);
  }, [renameValue, renamingId]);

  const deleteFile = useCallback((file: WorkspaceFile, event: Event) => {
    event.stopPropagation();
    if (files.length <= 1) {
      window.alert(t.cannotDeleteLastFile);
      return;
    }
    if (!window.confirm(t.deleteFileConfirm)) return;
    const nextFiles = files.filter((item) => item.id !== file.id);
    setFiles(nextFiles);
    setOpenTabs((tabs) => tabs.filter((id) => id !== file.id));
    if (activeFileId === file.id) {
      const next = nextFiles[0];
      if (next) {
        setActiveFileId(next.id);
        setOpenTabs((tabs) => tabs.includes(next.id) ? tabs : [...tabs, next.id]);
      }
    }
    setRenamingId(null);
    compClose();
  }, [activeFileId, compClose, files, t.cannotDeleteLastFile, t.deleteFileConfirm]);

  const closeTab = useCallback((id: string, event: Event) => {
    event.stopPropagation();
    const tabIndex = openTabs.indexOf(id);
    const remaining = openTabs.filter((tab) => tab !== id);
    setOpenTabs(remaining);
    if (activeFileId === id) {
      const nextId = remaining[tabIndex - 1] ?? remaining[tabIndex] ?? files[0]?.id;
      if (nextId) setActiveFileId(nextId);
    }
  }, [activeFileId, files, openTabs]);

  const handleCompletionAccept = useCallback(() => {
    compAccept();
    if (editorRef.current) updateActiveContent(editorRef.current.value);
  }, [compAccept, updateActiveContent]);

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
          <div class="sidebar-section files-section">
            <div class="sidebar-heading">
              <span>{t.explorer}</span>
              <span class="sidebar-actions">
                <button class="sidebar-action" onClick={createFile} title={t.newFile} aria-label={t.newFile}>＋</button>
                <button class="sidebar-action" title={t.moreActions} aria-label={t.moreActions}>•••</button>
              </span>
            </div>
            <div class="workspace-root"><span class="chevron">⌄</span><span class="folder-icon">▱</span><span>{t.workspace}</span></div>
            <div class="sidebar-heading files-heading"><span>{t.files}</span><span class="count-badge">{files.length}</span></div>
            <div class="file-tree">
              {files.map((file) => (
                <div key={file.id} class={"file-row" + (activeFile?.id === file.id ? " active" : "")}>
                  {renamingId === file.id ? (
                    <input
                      ref={renameRef}
                      class="file-rename-input"
                      value={renameValue}
                      aria-label={t.renameFile}
                      onInput={(event) => setRenameValue((event.target as HTMLInputElement).value)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter") commitRename();
                        if (event.key === "Escape") setRenamingId(null);
                      }}
                      onBlur={commitRename}
                    />
                  ) : (
                    <button class="file-open" onClick={() => openFile(file.id)}>
                      <span class="js-icon">JS</span><span class="file-name">{file.name}</span>{file.dirty && <span class="file-dirty">●</span>}
                    </button>
                  )}
                  {renamingId !== file.id && (
                    <span class="file-actions">
                      <button onClick={(event) => beginRename(file, event)} title={t.renameFile} aria-label={t.renameFile}>✎</button>
                      <button onClick={(event) => deleteFile(file, event)} title={t.deleteFile} aria-label={t.deleteFile}>×</button>
                    </span>
                  )}
                </div>
              ))}
            </div>
          </div>

          <div class="sidebar-footer">
            <div class="feature-note"><span class="spark">✦</span><div><strong>{t.languageTools}</strong><small>{t.languageToolsDescription}</small></div></div>
          </div>
        </aside>

        <main class="layout" ref={containerRef}>
          <section class="editor-wrap editor-panel" style={{ flex: `0 0 calc(${ratio * 100}% - ${ratio * RESIZER.dividerHeight}px)` }}>
            <div class="editor-panel-head tab-strip">
              <div class="open-tabs">
                {openTabs.map((id) => {
                  const file = files.find((item) => item.id === id);
                  if (!file) return null;
                  return (
                    <button key={file.id} class={"editor-tab" + (file.id === activeFile?.id ? " active" : "")} onClick={() => openFile(file.id)}>
                      <span class="js-icon">JS</span><span class="tab-name">{file.name}</span>{file.dirty && <span class="tab-dirty">●</span>}
                      <span class="tab-close" onClick={(event) => closeTab(file.id, event)} title={t.closeTab}>×</span>
                    </button>
                  );
                })}
              </div>
              <div class="editor-head-meta"><span class="language-pill">{t.editorLanguage}</span><span>{t.encoding}</span><span>{t.indentation}</span></div>
            </div>
            <div class="editor-hints">
              <span class="hint-breadcrumb"><span class="crumb-muted">{UI.brandSubtitle}</span><span>/</span><span>{activeFile?.name ?? t.fileName}</span><span>/</span><span>{UI.editorScope}</span></span>
              <span class="hint-actions"><span class="hint-key">⌘ Space</span> {t.autocomplete} <span class="hint-key">⌘ S</span> {t.runShortcut}</span>
            </div>
            <div class="editor-stage">
              <Editor
                value={activeContent}
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
                  updateActiveContent(editor.value);
                  compClose();
                  editor.focus();
                }}
                popupRef={popupRef}
              />
            </div>
            <div class="editor-statusbar">
              <span><span class="statusbar-green" /> {t.editorLanguage}</span>
              <span>Ln {cursor.line}, Col {cursor.column}</span>
              <span>{activeContent.length.toLocaleString()} chars</span>
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
