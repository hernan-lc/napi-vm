import type { CompletionItem } from "../types.ts";

interface CompletionPopupProps {
  items: CompletionItem[];
  sel: number;
  open: boolean;
  kindLetter: Record<string, string>;
  onAccept: (item: CompletionItem) => void;
  popupRef: { current: HTMLDivElement | null };
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

export function CompletionPopup({ items, sel, open, kindLetter, onAccept, popupRef }: CompletionPopupProps) {
  if (!open || items.length === 0) return null;

  return (
    <div class="popup" role="listbox" ref={popupRef}>
      {items.map((it, i) => {
        const letter = kindLetter[it.kind] || "?";
        const detail = it.detail ? (
          <span class="detail">{escapeHtml(it.detail)}</span>
        ) : null;
        return (
          <div
            key={it.label + i}
            class={"item" + (i === sel ? " sel" : "")}
            role="option"
            aria-selected={i === sel}
            onMouseDown={(e) => {
              e.preventDefault();
              onAccept(it);
            }}
          >
            <span class={"kind " + it.kind}>{letter}</span>
            <span>{escapeHtml(it.label)}</span>
            {detail}
          </div>
        );
      })}
    </div>
  );
}
