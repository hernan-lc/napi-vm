import { useMemo, useState } from "preact/hooks";

type InspectNode =
  | { kind: "object"; entries: Array<{ key: string; value: InspectNode }> }
  | { kind: "array"; items: InspectNode[] }
  | { kind: "primitive"; value: string };

type Segment = string | InspectNode;

interface ParsedValue {
  node: InspectNode;
  next: number;
}

function skipWhitespace(source: string, index: number): number {
  while (/\s/.test(source[index] || "")) index++;
  return index;
}

function readString(source: string, index: number): ParsedValue {
  const quote = source[index];
  let cursor = index + 1;
  while (cursor < source.length) {
    if (source[cursor] === "\\") {
      cursor += 2;
      continue;
    }
    if (source[cursor] === quote) {
      return { node: { kind: "primitive", value: source.slice(index, cursor + 1) }, next: cursor + 1 };
    }
    cursor++;
  }
  return { node: { kind: "primitive", value: source.slice(index) }, next: source.length };
}

function readPrimitive(source: string, index: number): ParsedValue | null {
  const start = index;
  while (index < source.length && source[index] !== "," && source[index] !== "]" && source[index] !== "}") index++;
  const value = source.slice(start, index).trim();
  return value ? { node: { kind: "primitive", value }, next: index } : null;
}

function parseValue(source: string, start: number): ParsedValue | null {
  const index = skipWhitespace(source, start);
  if (source[index] === "\"" || source[index] === "'") return readString(source, index);

  if (source[index] === "{") {
    const entries: Array<{ key: string; value: InspectNode }> = [];
    let cursor = skipWhitespace(source, index + 1);
    if (source[cursor] === "}") return { node: { kind: "object", entries }, next: cursor + 1 };
    while (cursor < source.length) {
      const keyStart = cursor;
      while (cursor < source.length && source[cursor] !== ":" && source[cursor] !== "}") cursor++;
      if (source[cursor] !== ":") return null;
      const key = source.slice(keyStart, cursor).trim();
      const parsed = parseValue(source, cursor + 1);
      if (!key || !parsed) return null;
      entries.push({ key, value: parsed.node });
      cursor = skipWhitespace(source, parsed.next);
      if (source[cursor] === "}") return { node: { kind: "object", entries }, next: cursor + 1 };
      if (source[cursor] !== ",") return null;
      cursor = skipWhitespace(source, cursor + 1);
    }
    return null;
  }

  if (source[index] === "[") {
    if (/^\[(Function|object|circular|function)/.test(source.slice(index))) return readPrimitive(source, index);
    const items: InspectNode[] = [];
    let cursor = skipWhitespace(source, index + 1);
    if (source[cursor] === "]") return { node: { kind: "array", items }, next: cursor + 1 };
    while (cursor < source.length) {
      const parsed = parseValue(source, cursor);
      if (!parsed) return null;
      items.push(parsed.node);
      cursor = skipWhitespace(source, parsed.next);
      if (source[cursor] === "]") return { node: { kind: "array", items }, next: cursor + 1 };
      if (source[cursor] !== ",") return null;
      cursor = skipWhitespace(source, cursor + 1);
    }
    return null;
  }

  return readPrimitive(source, index);
}

function parseSegments(source: string): Segment[] {
  const segments: Segment[] = [];
  let cursor = 0;
  let textStart = 0;
  while (cursor < source.length) {
    if (source[cursor] === "\"" || source[cursor] === "'") {
      const quote = source[cursor++];
      while (cursor < source.length) {
        if (source[cursor] === "\\") {
          cursor += 2;
          continue;
        }
        if (source[cursor] === quote) {
          cursor++;
          break;
        }
        cursor++;
      }
      continue;
    }
    if (source[cursor] === "{" || source[cursor] === "[") {
      const parsed = parseValue(source, cursor);
      if (parsed && (parsed.node.kind === "object" || parsed.node.kind === "array")) {
        if (textStart < cursor) segments.push(source.slice(textStart, cursor));
        segments.push(parsed.node);
        cursor = parsed.next;
        textStart = cursor;
        continue;
      }
    }
    cursor++;
  }
  if (textStart < source.length) segments.push(source.slice(textStart));
  return segments.length ? segments : [source];
}

function preview(node: InspectNode, depth = 0): string {
  if (node.kind === "primitive") return node.value;
  if (depth > 1) return node.kind === "array" ? "[…]" : "{…}";
  if (node.kind === "array") {
    const items = node.items.slice(0, 3).map((item) => preview(item, depth + 1));
    return `[${items.join(", ")}${node.items.length > items.length ? ", …" : ""}]`;
  }
  const entries = node.entries.slice(0, 3).map((entry) => `${entry.key}: ${preview(entry.value, depth + 1)}`);
  return `{${entries.join(", ")}${node.entries.length > entries.length ? ", …" : ""}}`;
}

function InspectableNode({ node }: { node: InspectNode }) {
  if (node.kind === "primitive") return <span class="inspect-primitive">{node.value}</span>;

  const [expanded, setExpanded] = useState(false);
  const entries = node.kind === "object"
    ? node.entries.map((entry) => ({ key: entry.key, value: entry.value }))
    : node.items.map((value, index) => ({ key: String(index), value }));

  return (
    <span class={"inspect-node " + node.kind + (expanded ? " expanded" : "") }>
      <button class="inspect-toggle" onClick={() => setExpanded((value) => !value)} aria-expanded={expanded}>
        <span class="inspect-caret">{expanded ? "▾" : "▸"}</span>
        <span class="inspect-preview">{expanded ? (node.kind === "object" ? "{" : "[") : preview(node)}</span>
      </button>
      {expanded && (
        <span class="inspect-children">
          {entries.map((entry, index) => (
            <span class="inspect-entry" key={entry.key + index}>
              <span class="inspect-key">{node.kind === "object" ? entry.key : `${entry.key}:`}</span>
              <InspectableNode node={entry.value} />
              {index < entries.length - 1 && <span class="inspect-comma">,</span>}
            </span>
          ))}
          <span class="inspect-closing">{node.kind === "object" ? "}" : "]"}</span>
        </span>
      )}
    </span>
  );
}

export function InspectableText({ text }: { text: string }) {
  const segments = useMemo(() => parseSegments(text), [text]);
  return (
    <span class="inspectable-text">
      {segments.map((segment, index) => typeof segment === "string"
        ? <span key={index}>{segment}</span>
        : <InspectableNode key={index} node={segment} />)}
    </span>
  );
}
