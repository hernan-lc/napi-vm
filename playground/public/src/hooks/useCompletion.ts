import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import { complete } from "../vm.ts";
import type { CompletionItem, CompletionPosition } from "../types.ts";

const TEXT_ENCODER = new TextEncoder();
const MEASURE_CANVAS = document.createElement("canvas");

type WasmVm = Parameters<typeof complete>[0];

const KIND_LETTER: Record<string, string> = {
  variable: "x",
  function: "\u0192",
  method: "\u0192",
  property: "\u2022",
  class: "C",
  module: "M",
  keyword: "k",
  global: "G",
  exposed: "h",
};

export interface CompletionState {
  items: CompletionItem[];
  sel: number;
  prefix: string;
  open: boolean;
  position: CompletionPosition;
}

export function useCompletion(
  getVm: () => unknown,
  editorRef: { current: HTMLTextAreaElement | null },
  popupRef: { current: HTMLDivElement | null }
) {
  const [state, setState] = useState<CompletionState>({
    items: [],
    sel: 0,
    prefix: "",
    open: false,
    position: { top: 0, left: 0 },
  });
  const stateRef = useRef(state);
  stateRef.current = state;

  const close = useCallback(() => {
    setState((s) => (s.open ? { ...s, items: [], sel: 0, prefix: "", open: false } : s));
  }, []);

  const caretPosition = (editor: HTMLTextAreaElement, before: string): CompletionPosition => {
    const style = getComputedStyle(editor);
    const lineHeight = parseFloat(style.lineHeight) || 21;
    const paddingTop = parseFloat(style.paddingTop) || 0;
    const paddingLeft = parseFloat(style.paddingLeft) || 0;
    const lines = before.split("\n");
    const line = lines.length - 1;
    const lastLine = lines[line] || "";
    const ctx = MEASURE_CANVAS.getContext("2d");
    if (ctx) ctx.font = `${style.fontSize} ${style.fontFamily}`;
    const width = ctx?.measureText(lastLine).width ?? lastLine.length * 8;
    return {
      top: Math.max(8, paddingTop + (line + 1) * lineHeight - editor.scrollTop),
      left: Math.max(paddingLeft + 8, paddingLeft + width - editor.scrollLeft),
    };
  };

  const afterLast = (s: string, sep: string): string => {
    const idx = s.lastIndexOf(sep);
    return idx >= 0 ? s.slice(idx + sep.length) : s;
  };

  const analyze = (before: string): { kind: "member" | "ident"; prefix: string } => {
    // @playground/<module> namespace: offer module completions.
    if (before.includes("@playground/")) {
      const after = afterLast(before, "@playground/");
      // If the text after @playground/ contains a dot, it's a member
      // completion for module exports (e.g. @playground/math.floor).
      if (after.includes(".")) {
        const dotIdx = after.lastIndexOf(".");
        return { kind: "member", prefix: after.slice(dotIdx + 1) };
      }
      return { kind: "ident", prefix: after };
    }
    const word = (before.match(/([\w$]*)$/) || [, ""])[1]!;
    const isMember = /[\w$)\]"']\.[\w$]*$/.test(before);
    return { kind: isMember ? "member" : "ident", prefix: word };
  };

  const request = useCallback(
    (force: boolean) => {
      const vm = getVm();
      const editor = editorRef.current;
      if (!vm || !editor) return;
      const caret = editor.selectionStart;
      const before = editor.value.slice(0, caret);
      const a = analyze(before);

      if (a.kind === "ident" && (!force || a.prefix.length === 0)) {
        // Keep open for @playground/ even with empty prefix so all
        // registered modules are offered as completions.
        if (!before.includes("@playground/")) {
          close();
          return;
        }
      }

      const byteOffset = TEXT_ENCODER.encode(before).length;
      const list = complete(vm as unknown as WasmVm, editor.value, byteOffset);

      if (!list || list.length === 0) {
        close();
        return;
      }

      setState({
        items: list.slice(0, 50),
        sel: 0,
        prefix: a.prefix,
        open: true,
        position: caretPosition(editor, before),
      });
    },
    [getVm, editorRef, close]
  );

  const move = useCallback((delta: number) => {
    setState((s) => {
      if (s.items.length === 0) return s;
      return { ...s, sel: (s.sel + delta + s.items.length) % s.items.length };
    });
  }, []);

  const accept = useCallback(() => {
    const editor = editorRef.current;
    const { items, sel, prefix } = stateRef.current;
    if (!editor || !items[sel]) return;

    const caret = editor.selectionStart;
    const start = caret - prefix.length;
    editor.setRangeText(items[sel].label, start, caret, "end");
    close();
    editor.focus();
  }, [editorRef, close]);

  useEffect(() => {
    if (!state.open || !popupRef.current) return;
    const el = popupRef.current.querySelector(".item.sel");
    if (el) (el as HTMLElement).scrollIntoView({ block: "nearest" });
  }, [state.sel, state.open, popupRef]);

  return {
    state,
    close,
    request,
    move,
    accept,
    isOpen: state.open,
    kindLetter: KIND_LETTER,
  };
}
