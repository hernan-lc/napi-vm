import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import { Toolbar } from "./Toolbar.tsx";
import { Editor } from "./Editor.tsx";
import { CompletionPopup } from "./CompletionPopup.tsx";
import { Logger } from "./logger/Logger.tsx";
import { Dialog } from "./Dialog.tsx";
import { Explorer } from "./Explorer.tsx";
import { useVm } from "../hooks/useVm.ts";
import { useCompletion } from "../hooks/useCompletion.ts";
import { useResizable } from "../hooks/useResizable.ts";
import { useI18n } from "../i18n/useI18n.ts";
import { useTheme } from "../hooks/useTheme.ts";
import { ASYNC_SAMPLE, LOOP_SAMPLE, MODULES, SAMPLE } from "../examples.ts";
import { hover as hoverCode } from "../vm.ts";
import { COMPLETION, EDITOR, RESIZER, UI, WORKSPACE } from "../constants.ts";
import type { WorkspaceFile } from "../types.ts";
import { resolveWorkspaceImport } from "./editor/imports.ts";

const SEED_FILES: WorkspaceFile[] = [
  { id: "playground", name: "playground.js", content: SAMPLE },
  { id: "async", name: "async.js", content: ASYNC_SAMPLE },
  { id: "loop", name: "loop-guard.js", content: LOOP_SAMPLE },
];

const READONLY_FILES: WorkspaceFile[] = MODULES.map((module) => ({
  id: `module-${module.name.replace(/^\.\//, "").replaceAll("/", "-").replaceAll(".", "-")}`,
  name: module.name.replace(/^\.\//, ""),
  content: module.source,
  readonly: true,
}));

const DEFAULT_FILES = [...SEED_FILES, ...READONLY_FILES];

function isWorkspaceFile(file: unknown): file is WorkspaceFile {
  return !!file && typeof file === "object" &&
    typeof (file as WorkspaceFile).id === "string" &&
    typeof (file as WorkspaceFile).name === "string" &&
    typeof (file as WorkspaceFile).content === "string";
}

function loadWorkspace(): WorkspaceFile[] {
  try {
    const raw = localStorage.getItem(WORKSPACE.storageKey);
    if (!raw) return DEFAULT_FILES;
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return DEFAULT_FILES;
    const readonlyIds = new Set(READONLY_FILES.map((file) => file.id));
    const files = parsed.filter(isWorkspaceFile).filter((file) => !readonlyIds.has(file.id));
    return files.length > 0 ? [...files, ...READONLY_FILES] : DEFAULT_FILES;
  } catch {
    return DEFAULT_FILES;
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

interface DialogState {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
}

export function App() {
  const { t, locale, setLocale } = useI18n();
  const { theme, toggleTheme } = useTheme();
  const { ratio, containerRef, handlePointerDown } = useResizable();
  const { getVm, status, entries, diagnostic, loopLimit, run, reset: resetVm, clearEntries, updateLoopLimit, refreshDiagnostic } = useVm(t);

  const [files, setFiles] = useState<WorkspaceFile[]>(loadWorkspace);
  const [activeFileId, setActiveFileId] = useState<string>(WORKSPACE.defaultFileId);
  const [openTabs, setOpenTabs] = useState<string[]>([WORKSPACE.defaultFileId]);
  const [cursor, setCursor] = useState({ line: 1, column: 1 });
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const renameRef = useRef<HTMLInputElement>(null);
  const diagnosticTimer = useRef<number | undefined>(undefined);
  const completionTimer = useRef<number | undefined>(undefined);

  const activeFile = files.find((file) => file.id === activeFileId) ?? files[0];
  const activeContent = activeFile?.content ?? "";

  const getHover = useCallback((source: string, offset: number) => {
    const vm = getVm();
    return vm ? hoverCode(vm as Parameters<typeof hoverCode>[0], source, offset) : null;
  }, [getVm]);

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
    setFiles((current) => current.map((file) => file.id === activeFileId && !file.readonly
      ? { ...file, content, dirty: true }
      : file));
  }, [activeFileId]);

  const handleRun = useCallback(() => {
    if (activeFile) run(activeFile.content, activeFile.name);
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
    if (file.readonly) return;
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

  const removeFile = useCallback((id: string) => {
    if (files.find((file) => file.id === id)?.readonly) return;
    const nextFiles = files.filter((item) => item.id !== id);
    setFiles(nextFiles);
    setOpenTabs((tabs) => tabs.filter((tabId) => tabId !== id));
    if (activeFileId === id) {
      const next = nextFiles[0];
      if (next) {
        setActiveFileId(next.id);
        setOpenTabs((tabs) => tabs.includes(next.id) ? tabs : [...tabs, next.id]);
      }
    }
    setRenamingId(null);
    compClose();
  }, [activeFileId, compClose, files]);

  const restoreFile = useCallback((file: WorkspaceFile, event: Event) => {
    event.stopPropagation();
    const original = DEFAULT_FILES.find((item) => item.id === file.id);
    if (!original || file.readonly) return;
    setDialog({
      title: t.restoreFile,
      message: t.restoreFileConfirm,
      confirmLabel: t.restoreFile,
      cancelLabel: t.cancel,
      onConfirm: () => {
        setFiles((current) => current.map((item) => item.id === file.id ? { ...original, dirty: false } : item));
        setDialog(null);
      },
    });
  }, [t.cancel, t.restoreFile, t.restoreFileConfirm]);

  const resetWorkspace = useCallback(() => {
    setDialog({
      title: t.resetWorkspace,
      message: t.resetWorkspaceConfirm,
      confirmLabel: t.resetWorkspace,
      cancelLabel: t.cancel,
      danger: true,
      onConfirm: () => {
        setFiles(DEFAULT_FILES.map((file) => ({ ...file })));
        setActiveFileId(WORKSPACE.defaultFileId);
        setOpenTabs([WORKSPACE.defaultFileId]);
        setRenamingId(null);
        setCursor({ line: 1, column: 1 });
        compClose();
        setDialog(null);
      },
    });
  }, [compClose, t.cancel, t.resetWorkspace, t.resetWorkspaceConfirm]);

  const openImport = useCallback((specifier: string) => {
    if (!activeFile) return;
    const resolved = resolveWorkspaceImport(activeFile.name, specifier);
    if (!resolved) return;
    const candidates = [resolved, `${resolved}.js`, `${resolved}/index.js`];
    const target = files.find((file) => candidates.includes(file.name));
    if (target) openFile(target.id);
  }, [activeFile, files, openFile]);

  const deleteFile = useCallback((file: WorkspaceFile, event: Event) => {
    event.stopPropagation();
    if (files.length <= 1) {
      setDialog({
        title: t.cannotDeleteLastFile,
        message: t.cannotDeleteLastFile,
        confirmLabel: t.ok,
        onConfirm: () => setDialog(null),
      });
      return;
    }
    setDialog({
      title: t.deleteFile,
      message: t.deleteFileConfirm,
      confirmLabel: t.deleteFile,
      cancelLabel: t.cancel,
      danger: true,
      onConfirm: () => {
        removeFile(file.id);
        setDialog(null);
      },
    });
  }, [files.length, removeFile, t.cancel, t.cannotDeleteLastFile, t.deleteFile, t.deleteFileConfirm, t.ok]);

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
        onReset={resetVm}
        onClear={clearEntries}
        onLoopLimitChange={updateLoopLimit}
        onToggleTheme={toggleTheme}
        onLocaleChange={setLocale}
      />

      <div class="workspace-shell">
        <aside class="sidebar">
          <Explorer
            files={files}
            activeFileId={activeFile?.id ?? ""}
            renamingId={renamingId}
            renameValue={renameValue}
            renameRef={renameRef}
            t={t}
            onCreateFile={createFile}
            onOpenFile={openFile}
            onBeginRename={beginRename}
            onDeleteFile={deleteFile}
            onResetFile={restoreFile}
            onCommitRename={commitRename}
            onRenameValueChange={setRenameValue}
            onCancelRename={() => setRenamingId(null)}
            onResetWorkspace={resetWorkspace}
          />

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
            <div class="editor-hints" title={t.openImportHint}>
              <span class="hint-breadcrumb"><span class="crumb-muted">{UI.brandSubtitle}</span><span>/</span><span>{activeFile?.name ?? t.fileName}</span><span>/</span><span>{UI.editorScope}</span></span>
              <span class="hint-actions"><span class="hint-key">⌘ Space</span> {t.autocomplete} <span class="hint-key">⌘ S</span> {t.runShortcut}</span>
            </div>
            <div class="editor-stage" title={t.openImportHint}>
              <Editor
                value={activeContent}
                onChange={handleCodeChange}
                onRun={handleRun}
                onCompletionRequest={compRequest}
                onCompletionClose={compClose}
                onCompletionMove={compMove}
                onCompletionAccept={handleCompletionAccept}
                onHoverAt={getHover}
                onOpenImport={openImport}
                readOnly={activeFile?.readonly ?? false}
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
            <span class="divider-label">{t.console}</span>
            <span class="divider-count">{entries.length} {entries.length === 1 ? t.entry : t.entries}</span>
          </div>
          <Logger entries={entries} t={t} />
        </main>
      </div>
      {dialog && (
        <Dialog
          title={dialog.title}
          message={dialog.message}
          confirmLabel={dialog.confirmLabel}
          cancelLabel={dialog.cancelLabel}
          danger={dialog.danger}
          onConfirm={dialog.onConfirm}
          onCancel={() => setDialog(null)}
        />
      )}
    </div>
  );
}
