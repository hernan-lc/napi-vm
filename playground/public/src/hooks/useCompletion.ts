import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import { complete } from "../vm.ts";
import type { CompletionItem } from "../types.ts";

declare class WasmVm {
  complete(source: string, offset: number): CompletionItem[];
}

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
  });
  const stateRef = useRef(state);
  stateRef.current = state;

  const close = useCallback(() => {
    setState((s) => (s.open ? { items: [], sel: 0, prefix: "", open: false } : s));
  }, []);

  const analyze = (before: string): { kind: "member" | "ident"; prefix: string } => {
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
        close();
        return;
      }

      const byteOffset = new TextEncoder().encode(before).length;
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
