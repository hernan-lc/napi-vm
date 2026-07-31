import { complete } from "./vm";
import { escapeHtml } from "./console";
import type { CompletionController, CompletionItem } from "./types";

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

interface CompletionOpts {
  editor: HTMLTextAreaElement;
  popup: HTMLElement;
  getVm: () => WasmVm | null;
}

export function createCompletion({ editor, popup, getVm }: CompletionOpts): CompletionController {
  let items: CompletionItem[] = [];
  let sel = 0;
  let prefix = "";

  function isOpen(): boolean {
    return !popup.hidden;
  }

  function analyze(before: string): { kind: "member" | "ident"; prefix: string } {
    const word = (before.match(/([\w$]*)$/) || [, ""])[1]!;
    const isMember = /[\w$)\]"']\.[\w$]*$/.test(before);
    return { kind: isMember ? "member" : "ident", prefix: word };
  }

  function request(force: boolean): void {
    const vm = getVm();
    if (!vm) return;
    const caret = editor.selectionStart;
    const before = editor.value.slice(0, caret);
    const a = analyze(before);

    if (a.kind === "ident" && (!force || a.prefix.length === 0)) {
      close();
      return;
    }

    const byteOffset = new TextEncoder().encode(before).length;
    prefix = a.prefix;
    show(complete(vm as any, editor.value, byteOffset) as CompletionItem[]);
  }

  function show(list: CompletionItem[]): void {
    if (!list || list.length === 0) {
      close();
      return;
    }
    items = list.slice(0, 50);
    sel = 0;
    popup.innerHTML = "";
    items.forEach((it, i) => {
      const div = document.createElement("div");
      div.className = "item" + (i === sel ? " sel" : "");
      const letter = KIND_LETTER[it.kind] || "?";
      const detail = it.detail ? `<span class="detail">${escapeHtml(it.detail)}</span>` : "";
      div.innerHTML = `<span class="kind ${it.kind}">${letter}</span>${escapeHtml(it.label)}${detail}`;
      div.addEventListener("mousedown", (e) => {
        e.preventDefault();
        acceptItem(it);
      });
      div.addEventListener("mousemove", () => {
        if (sel !== i) {
          sel = i;
          paintSel();
        }
      });
      popup.appendChild(div);
    });
    popup.hidden = false;
    position();
  }

  function paintSel(): void {
    [...popup.children].forEach((el, i) => el.classList.toggle("sel", i === sel));
    const s = popup.children[sel];
    if (s) s.scrollIntoView({ block: "nearest" });
  }

  function move(delta: number): void {
    if (items.length === 0) return;
    sel = (sel + delta + items.length) % items.length;
    paintSel();
  }

  function accept(): void {
    if (items[sel]) acceptItem(items[sel]);
  }

  function acceptItem(item: CompletionItem): void {
    const caret = editor.selectionStart;
    const start = caret - prefix.length;
    editor.setRangeText(item.label, start, caret, "end");
    close();
    editor.focus();
  }

  function close(): void {
    popup.hidden = true;
    items = [];
    sel = 0;
    prefix = "";
  }

  function position(): void {
    const caret = editor.selectionStart;
    const coords = caretCoordinates(editor, caret);
    const cs = getComputedStyle(editor);
    let lh = parseFloat(cs.lineHeight);
    if (isNaN(lh)) lh = parseFloat(cs.fontSize) * 1.55;

    let left = coords.left - editor.scrollLeft + 2;
    let top = coords.top - editor.scrollTop + lh;

    const wrap = editor.parentElement!;
    const maxLeft = wrap.clientWidth - popup.offsetWidth - 8;
    const maxTop = wrap.clientHeight - popup.offsetHeight - 8;
    left = Math.max(4, Math.min(left, maxLeft));
    top = Math.max(4, Math.min(top, maxTop));

    popup.style.left = left + "px";
    popup.style.top = top + "px";
  }

  function caretCoordinates(element: HTMLTextAreaElement, pos: number): { top: number; left: number } {
    const div = document.createElement("div");
    const style = div.style;
    const cs = getComputedStyle(element);
    const props = [
      "direction", "boxSizing", "width", "height", "overflowX", "overflowY",
      "borderTopWidth", "borderRightWidth", "borderBottomWidth", "borderLeftWidth",
      "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
      "fontStyle", "fontVariant", "fontWeight", "fontStretch", "fontSize",
      "fontSizeAdjust", "lineHeight", "fontFamily", "textAlign", "textTransform",
      "textIndent", "textDecoration", "letterSpacing", "wordSpacing", "tabSize",
      "whiteSpace",
    ] as const;
    style.position = "absolute";
    style.visibility = "hidden";
    style.overflow = "hidden";
    for (const p of props) {
      (style as unknown as Record<string, string>)[p] = cs.getPropertyValue(p);
    }
    div.textContent = element.value.substring(0, pos);
    const span = document.createElement("span");
    span.textContent = element.value.substring(pos) || ".";
    div.appendChild(span);
    document.body.appendChild(div);
    const coords = {
      top: span.offsetTop + parseInt(cs.borderTopWidth, 10),
      left: span.offsetLeft + parseInt(cs.borderLeftWidth, 10),
    };
    document.body.removeChild(div);
    return coords;
  }

  return { isOpen, close, request, move, accept };
}
