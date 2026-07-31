import { useEffect, useRef } from "preact/hooks";
import type { Diagnostic } from "../types.ts";

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
}: EditorProps) {
  const highlightRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (highlightRef.current && editorRef.current) {
      highlightRef.current.scrollTop = editorRef.current.scrollTop;
      highlightRef.current.scrollLeft = editorRef.current.scrollLeft;
    }
  }, [value]);

  const handleInput = (e: Event) => {
    const target = e.target as HTMLTextAreaElement;
    onChange(target.value);
  };

  const handleScroll = () => {
    if (highlightRef.current && editorRef.current) {
      highlightRef.current.scrollTop = editorRef.current.scrollTop;
      highlightRef.current.scrollLeft = editorRef.current.scrollLeft;
    }
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
      return;
    }

    if (completionOpen) {
      if (e.key === "ArrowDown") { e.preventDefault(); onCompletionMove(1); return; }
      if (e.key === "ArrowUp") { e.preventDefault(); onCompletionMove(-1); return; }
      if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); onCompletionAccept(); return; }
      if (e.key === "Escape") { e.preventDefault(); onCompletionClose(); return; }
      if (e.key === "ArrowLeft" || e.key === "ArrowRight" || e.key === "Home" || e.key === "End") {
        onCompletionClose();
      }
    }

    if (mod && e.code === "Space") {
      e.preventDefault();
      onCompletionRequest(true);
    }
  };

  return (
    <div class="editor-container">
      <div class="editor-highlight" ref={highlightRef} aria-hidden="true">
        <pre><code>{value}</code></pre>
      </div>
      <textarea
        ref={editorRef}
        class="editor-input"
        spellcheck={false}
        autocapitalize="off"
        autocomplete="off"
        value={value}
        onInput={handleInput}
        onScroll={handleScroll}
        onKeyDown={handleKeyDown}
        onBlur={() => setTimeout(onCompletionClose, 120)}
      />
      {diagnostic && (
        <div class="editor-diagnostic">
          <span class="diag-icon">&#9888;</span>
          <span>{diagnostic.message}</span>
          <span class="diag-loc">line {diagnostic.line}</span>
        </div>
      )}
    </div>
  );
}
