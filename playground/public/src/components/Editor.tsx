import { useMemo, useRef } from "preact/hooks";
import type { Diagnostic } from "../types.ts";
import { highlightToHtml } from "./editor/highlight.ts";

interface EditorProps {
  value: string;
  onChange: (v: string) => void;
  onRun: () => void;
  onCompletionRequest: (force: boolean) => void;
  onCompletionClose: () => void;
  onCompletionMove: (delta: number) => void;
  onCompletionAccept: () => void;
  completionOpen: boolean;
  diagnostic: Diagnostic | null;
  editorRef: { current: HTMLTextAreaElement | null };
  onCursorChange: (line: number, column: number) => void;
}

function cursorPosition(value: string, offset: number) {
  const before = value.slice(0, offset);
  const lines = before.split("\n");
  return { line: lines.length, column: (lines.at(-1) || "").length + 1 };
}

export function Editor({
  value,
  onChange,
  onRun,
  onCompletionRequest,
  onCompletionClose,
  onCompletionMove,
  onCompletionAccept,
  completionOpen,
  diagnostic,
  editorRef,
  onCursorChange,
}: EditorProps) {
  const highlightRef = useRef<HTMLDivElement>(null);
  const gutterRef = useRef<HTMLDivElement>(null);
  const highlighted = useMemo(() => highlightToHtml(value) + "\n", [value]);
  const lineCount = useMemo(() => Math.max(1, value.split("\n").length), [value]);
  const lines = useMemo(() => Array.from({ length: lineCount }, (_, i) => i + 1), [lineCount]);

  const syncScroll = (target: HTMLTextAreaElement) => {
    if (highlightRef.current) {
      highlightRef.current.scrollTop = target.scrollTop;
      highlightRef.current.scrollLeft = target.scrollLeft;
    }
    const gutterContent = gutterRef.current?.firstElementChild as HTMLElement | null;
    if (gutterContent) gutterContent.style.transform = `translateY(-${target.scrollTop}px)`;
  };

  const reportCursor = (target: HTMLTextAreaElement) => {
    const position = cursorPosition(target.value, target.selectionStart);
    onCursorChange(position.line, position.column);
  };

  const handleInput = (e: Event) => {
    const target = e.target as HTMLTextAreaElement;
    onChange(target.value);
    reportCursor(target);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    const mod = e.ctrlKey || e.metaKey;

    if (mod && e.key === "Enter") {
      e.preventDefault();
      onRun();
      return;
    }

    if (e.key === "Tab" && !completionOpen) {
      e.preventDefault();
      const editor = editorRef.current;
      if (!editor) return;
      const start = editor.selectionStart;
      const end = editor.selectionEnd;
      editor.setRangeText("  ", start, end, "end");
      onChange(editor.value);
      reportCursor(editor);
      return;
    }

    if (completionOpen) {
      if (e.key === "ArrowDown") { e.preventDefault(); onCompletionMove(1); return; }
      if (e.key === "ArrowUp") { e.preventDefault(); onCompletionMove(-1); return; }
      if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); onCompletionAccept(); return; }
      if (e.key === "Escape") { e.preventDefault(); onCompletionClose(); return; }
      if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) onCompletionClose();
    }

    if (mod && e.code === "Space") {
      e.preventDefault();
      onCompletionRequest(true);
    }
  };

  const handleSelectionChange = () => {
    const editor = editorRef.current;
    if (editor) reportCursor(editor);
  };

  return (
    <div class="editor-container">
      <div class="editor-gutter" ref={gutterRef} aria-hidden="true">
        <div class="gutter-content">
          {lines.map((line) => (
            <div key={line} class={"gutter-line" + (diagnostic?.line === line ? " has-error" : "")}>{line}</div>
          ))}
        </div>
      </div>
      <div class="editor-highlight" ref={highlightRef} aria-hidden="true">
        <pre><code dangerouslySetInnerHTML={{ __html: highlighted }} /></pre>
      </div>
      <textarea
        ref={editorRef}
        class="editor-input"
        spellcheck={false}
        autocapitalize="off"
        autocomplete="off"
        aria-label="JavaScript source editor"
        value={value}
        onInput={handleInput}
        onScroll={(e) => syncScroll(e.currentTarget as HTMLTextAreaElement)}
        onKeyDown={handleKeyDown}
        onClick={handleSelectionChange}
        onKeyUp={handleSelectionChange}
        onBlur={() => setTimeout(onCompletionClose, 120)}
      />
      {diagnostic && (
        <div class="editor-diagnostic" role="status">
          <span class="diag-icon">!</span>
          <span>{diagnostic.message}</span>
          <span class="diag-loc">Ln {diagnostic.line}, Col {diagnostic.col}</span>
        </div>
      )}
    </div>
  );
}
