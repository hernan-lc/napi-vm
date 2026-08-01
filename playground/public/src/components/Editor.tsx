import { useMemo, useRef, useState } from "preact/hooks";
import type { Diagnostic, HoverInfo } from "../types.ts";
import { highlightToHtml } from "./editor/highlight.ts";
import { importSpecifierAt } from "./editor/imports.ts";
import { EDITOR, EDITOR_KEYS } from "../constants.ts";
import type { Translations } from "../i18n/translations.ts";
import { debugLog } from "../debug.ts";

interface EditorProps {
  value: string;
  onChange: (v: string) => void;
  onRun: () => void;
  onCompletionRequest: (force: boolean) => void;
  onCompletionClose: () => void;
  onCompletionMove: (delta: number) => void;
  onCompletionAccept: () => void;
  onHoverAt: (source: string, offset: number) => HoverInfo | null;
  onOpenImport: (specifier: string) => void;
  readOnly: boolean;
  completionOpen: boolean;
  diagnostic: Diagnostic | null;
  editorRef: { current: HTMLTextAreaElement | null };
  onCursorChange: (line: number, column: number) => void;
  t: Translations;
}

interface HoverState {
  info: HoverInfo;
  top: number;
  left: number;
}

function cursorPosition(value: string, offset: number) {
  const before = value.slice(0, offset);
  const lines = before.split(EDITOR.lineBreak);
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
  onHoverAt,
  onOpenImport,
  readOnly,
  completionOpen,
  diagnostic,
  editorRef,
  onCursorChange,
  t,
}: EditorProps) {
  const highlightRef = useRef<HTMLDivElement>(null);
  const gutterRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLCanvasElement | null>(null);
  const [hover, setHover] = useState<HoverState | null>(null);
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
    setHover(null);
    debugLog("editor:input", { caret: target.selectionStart, length: target.value.length });
    onChange(target.value);
    reportCursor(target);
  };

  const sourceOffsetAtPoint = (event: MouseEvent): { offset: number; top: number; left: number } | null => {
    const editor = event.currentTarget as HTMLTextAreaElement;
    const rect = editor.getBoundingClientRect();
    const style = getComputedStyle(editor);
    const lineHeight = parseFloat(style.lineHeight) || 21;
    const paddingTop = parseFloat(style.paddingTop) || 0;
    const paddingLeft = parseFloat(style.paddingLeft) || 0;
    const x = event.clientX - rect.left + editor.scrollLeft - paddingLeft;
    const y = event.clientY - rect.top + editor.scrollTop - paddingTop;
    if (x < 0 || y < 0) return null;

    const line = Math.floor(y / lineHeight);
    const lines = value.split("\n");
    if (line >= lines.length) return null;
    const lineText = lines[line];
    const canvas = measureRef.current ?? (measureRef.current = document.createElement("canvas"));
    const context = canvas.getContext("2d");
    if (context) context.font = `${style.fontSize} ${style.fontFamily}`;
    const measure = (text: string) => context?.measureText(text).width ?? text.length * 8;
    let column = 0;
    while (column < lineText.length && measure(lineText.slice(0, column + 1)) < x) column++;
    const lineStart = lines.slice(0, line).reduce((total, current) => total + current.length + 1, 0);
    return { offset: lineStart + column, top: event.clientY - rect.top + 8, left: event.clientX - rect.left + 12 };
  };

  const handleMouseMove = (event: MouseEvent) => {
    const point = sourceOffsetAtPoint(event);
    if (!point) {
      setHover(null);
      return;
    }
    const info = onHoverAt(value, point.offset);
    setHover(info ? { info, top: point.top, left: point.left } : null);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "r") {
      debugLog("editor:browser-refresh-key", { key: e.key, code: e.code });
    }
    if (mod || e.key === EDITOR_KEYS.tab || e.key === EDITOR_KEYS.run) {
      debugLog("editor:keydown", { key: e.key, code: e.code, mod, completionOpen });
    }

    if (mod && e.key === EDITOR_KEYS.run) {
      e.preventDefault();
      onRun();
      return;
    }

    if (e.key === EDITOR_KEYS.tab && !completionOpen) {
      e.preventDefault();
      const editor = editorRef.current;
      if (!editor) return;
      const start = editor.selectionStart;
      const end = editor.selectionEnd;
      editor.setRangeText(EDITOR.tabIndent, start, end, "end");
      onChange(editor.value);
      reportCursor(editor);
      return;
    }

    if (completionOpen) {
      if (e.key === EDITOR_KEYS.down) { e.preventDefault(); onCompletionMove(1); return; }
      if (e.key === EDITOR_KEYS.up) { e.preventDefault(); onCompletionMove(-1); return; }
      if (e.key === EDITOR_KEYS.run || e.key === EDITOR_KEYS.tab) { e.preventDefault(); onCompletionAccept(); return; }
      if (e.key === EDITOR_KEYS.escape) { e.preventDefault(); onCompletionClose(); return; }
      if ([EDITOR_KEYS.left, EDITOR_KEYS.right, EDITOR_KEYS.home, EDITOR_KEYS.end].includes(e.key as never)) onCompletionClose();
    }

    if (mod && e.code === EDITOR_KEYS.completion) {
      e.preventDefault();
      debugLog("editor:force-completion");
      onCompletionRequest(true);
    }
  };

  const handleSelectionChange = () => {
    const editor = editorRef.current;
    if (editor) reportCursor(editor);
  };

  const handleClick = (event: MouseEvent) => {
    const editor = event.currentTarget as HTMLTextAreaElement;
    reportCursor(editor);
    if (event.ctrlKey || event.metaKey) {
      const point = sourceOffsetAtPoint(event);
      const specifier = point ? importSpecifierAt(value, point.offset) : null;
      if (specifier) {
        event.preventDefault();
        onOpenImport(specifier);
      }
    }
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
        aria-label={t.editorAriaLabel}
        readOnly={readOnly}
        value={value}
        onInput={handleInput}
        onScroll={(e) => syncScroll(e.currentTarget as HTMLTextAreaElement)}
        onKeyDown={handleKeyDown}
        onClick={handleClick}
        onMouseMove={handleMouseMove}
        onMouseLeave={() => setHover(null)}
        onKeyUp={handleSelectionChange}
        onBlur={() => setTimeout(onCompletionClose, 120)}
      />
      {hover && (
        <div class="editor-hover" style={{ top: `${hover.top}px`, left: `${hover.left}px` }} role="tooltip">
          <div>{hover.info.detail}</div>
          {hover.info.documentation && <div class="editor-hover-doc">{hover.info.documentation}</div>}
        </div>
      )}
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
