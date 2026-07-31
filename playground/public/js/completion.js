// Autocomplete popup. Candidates come from the shared Rust core via
// `WasmVm.complete`; this module only decides when to pop up, renders the
// list, handles selection, and inserts the accepted label.
import { complete } from "./vm.js";
import { escapeHtml } from "./console.js";

const KIND_LETTER = {
  variable: "x",
  function: "ƒ",
  method: "ƒ",
  property: "•",
  class: "C",
  module: "M",
  keyword: "k",
  global: "G",
  exposed: "h",
};

/**
 * @param {object} opts
 * @param {HTMLTextAreaElement} opts.editor
 * @param {HTMLElement} opts.popup
 * @param {() => any} opts.getVm  returns the live WasmVm (or null)
 */
export function createCompletion({ editor, popup, getVm }) {
  let items = [];
  let sel = 0;
  let prefix = "";

  function isOpen() {
    return !popup.hidden;
  }

  /**
   * Classify the text before the caret just enough to drive UX: member vs
   * bare identifier, and the word fragment to replace on accept. The actual
   * candidates come from the core, not from anything computed here.
   */
  function analyze(before) {
    const word = (before.match(/([\w$]*)$/) || [, ""])[1];
    const isMember = /[\w$)\]"']\.[\w$]*$/.test(before);
    return { kind: isMember ? "member" : "ident", prefix: word };
  }

  function request(force) {
    const vm = getVm();
    if (!vm) return;
    const caret = editor.selectionStart;
    const before = editor.value.slice(0, caret);
    const a = analyze(before);

    // Identifier completion is explicit-only (Ctrl+Space) to avoid noise;
    // member completion pops automatically after a dot.
    if (a.kind === "ident" && (!force || a.prefix.length === 0)) {
      close();
      return;
    }

    // The core works in UTF-8 byte offsets; the caret is a UTF-16 code unit
    // offset. Re-encode the prefix so non-ASCII lines stay aligned.
    const byteOffset = new TextEncoder().encode(before).length;
    prefix = a.prefix;
    show(complete(vm, editor.value, byteOffset));
  }

  function show(list) {
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
      div.addEventListener("mousedown", (e) => { e.preventDefault(); acceptItem(it); });
      div.addEventListener("mousemove", () => { if (sel !== i) { sel = i; paintSel(); } });
      popup.appendChild(div);
    });
    popup.hidden = false; // unhide first so offsetWidth/Height are measurable
    position();
  }

  function paintSel() {
    [...popup.children].forEach((el, i) => el.classList.toggle("sel", i === sel));
    const s = popup.children[sel];
    if (s) s.scrollIntoView({ block: "nearest" });
  }

  function move(delta) {
    if (items.length === 0) return;
    sel = (sel + delta + items.length) % items.length;
    paintSel();
  }

  function accept() {
    if (items[sel]) acceptItem(items[sel]);
  }

  function acceptItem(item) {
    const caret = editor.selectionStart;
    const start = caret - prefix.length;
    editor.setRangeText(item.label, start, caret, "end");
    close();
    editor.focus();
  }

  function close() {
    popup.hidden = true;
    items = [];
    sel = 0;
    prefix = "";
  }

  function position() {
    const caret = editor.selectionStart;
    const coords = caretCoordinates(editor, caret);
    const cs = getComputedStyle(editor);
    let lh = parseFloat(cs.lineHeight);
    if (isNaN(lh)) lh = parseFloat(cs.fontSize) * 1.55;

    let left = coords.left - editor.scrollLeft + 2;
    let top = coords.top - editor.scrollTop + lh;

    const wrap = editor.parentElement;
    const maxLeft = wrap.clientWidth - popup.offsetWidth - 8;
    const maxTop = wrap.clientHeight - popup.offsetHeight - 8;
    left = Math.max(4, Math.min(left, maxLeft));
    top = Math.max(4, Math.min(top, maxTop));

    popup.style.left = left + "px";
    popup.style.top = top + "px";
  }

  // Mirror-div technique to measure caret pixel coordinates in a textarea.
  function caretCoordinates(element, pos) {
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
    ];
    style.position = "absolute";
    style.visibility = "hidden";
    style.overflow = "hidden";
    for (const p of props) style[p] = cs[p];
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
