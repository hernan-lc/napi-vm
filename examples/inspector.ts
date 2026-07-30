/**
 * Approach A — interactive terminal object inspector, DevTools-style.
 *
 * Zero dependencies, works in both Bun and Node. Guest VM values reach it
 * through the same `exposeFunction` marshalling path as `pretty()` in
 * pretty-print.ts: the Rust `to_napi` walker turns the guest `Value` into a
 * real host JS object, and this file renders it as a foldable tree.
 *
 *   ▶/▼  fold state        ↑/↓ (or j/k)  move cursor
 *   → / space / enter      expand node    ←            collapse / go to parent
 *   q / ctrl-c             quit
 *
 * Type colors match the native `console.dir`: keys cyan, strings green,
 * numbers blue, booleans yellow, null/undefined/circular dimmed. Circular
 * references print as `[Circular *1]` instead of recursing forever.
 *
 * Run (in a real terminal):  bun examples/inspector.ts
 * When stdin/stdout is not a TTY (pipes, CI), it falls back to a static
 * pre-expanded dump so it never blocks.
 *
 * Note: circular *guest* structures cannot cross the NAPI boundary — the
 * marshaller in bindings.rs is depth-bounded, not cycle-aware — so the
 * circular demo below builds the cycle on the host side.
 */
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { Vm } = require("../index");

// ── ANSI helpers ─────────────────────────────────────────────────────
const useColor =
  !!process.stdout.isTTY && !("NO_COLOR" in process.env);
const paint = (code: number | string) => (s: string) =>
  useColor ? `\x1b[${code}m${s}\x1b[0m` : s;
const cyan = paint(36);
const green = paint(32);
const blue = paint(34);
const yellow = paint(33);
const magenta = paint(35);
const red = paint(31);
const dim = paint("2;37");
const bold = paint(1);
const stripAnsi = (s: string) => s.replace(/\x1b\[[0-9;]*m/g, "");

// ── Tree model ───────────────────────────────────────────────────────
interface TNode {
  key: string | null; // null for the root
  value: unknown;
  depth: number;
  parent: TNode | null;
  expanded: boolean;
  children: TNode[] | null; // null = not built yet
  circular: number | null; // id of the ancestor this value aliases
}

const circularIds = new Map<object, number>();
let nextCircularId = 1;

const isContainer = (v: unknown): v is object =>
  v !== null && typeof v === "object";

function isExpandable(n: TNode): boolean {
  return n.circular === null && isContainer(n.value);
}

/** The chain of container values from a node up to the root (cycle detection). */
function chain(n: TNode): Set<object> {
  const seen = new Set<object>();
  for (let p: TNode | null = n; p; p = p.parent) {
    if (isContainer(p.value)) seen.add(p.value);
  }
  return seen;
}

function makeChild(parent: TNode, key: string, value: unknown, ancestors: Set<object>): TNode {
  let circular: number | null = null;
  if (isContainer(value) && ancestors.has(value)) {
    circular = circularIds.get(value) ?? nextCircularId++;
    circularIds.set(value, circular);
  }
  return {
    key,
    value,
    depth: parent.depth + 1,
    parent,
    expanded: false,
    children: null,
    circular,
  };
}

function ensureChildren(n: TNode): void {
  if (n.children !== null || !isContainer(n.value)) return;
  const v = n.value;
  const ancestors = chain(n);
  const kids: TNode[] = [];

  if (Array.isArray(v)) {
    v.forEach((item, i) => kids.push(makeChild(n, String(i), item, ancestors)));
  } else if (ArrayBuffer.isView(v)) {
    const view = v as unknown as ArrayLike<number>;
    for (let i = 0; i < view.length; i++) kids.push(makeChild(n, String(i), view[i], ancestors));
  } else if (v instanceof Map) {
    let i = 0;
    for (const [k, val] of v) {
      const key = typeof k === "object" || typeof k === "function" ? String(i) : String(k);
      kids.push(makeChild(n, key, val, ancestors));
      i++;
    }
  } else if (v instanceof Set) {
    let i = 0;
    for (const val of v) kids.push(makeChild(n, String(i++), val, ancestors));
  } else {
    const names = Object.getOwnPropertyNames(v);
    const symbols = Object.getOwnPropertySymbols(v);
    for (const k of names) kids.push(makeChild(n, k, (v as Record<string, unknown>)[k], ancestors));
    for (const s of symbols) kids.push(makeChild(n, String(s), (v as Record<symbol, unknown>)[s], ancestors));
  }
  n.children = kids;
}

// ── Rendering ────────────────────────────────────────────────────────
function escapeString(s: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .replace(/'/g, "\\'")
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
}

function fmtPrimitive(v: unknown): string {
  switch (typeof v) {
    case "string":
      return green(`'${escapeString(v)}'`);
    case "number":
      return blue(Object.is(v, -0) ? "-0" : String(v));
    case "bigint":
      return blue(`${v}n`);
    case "boolean":
      return yellow(String(v));
    case "undefined":
      return dim("undefined");
    case "symbol":
      return green(String(v));
    case "function":
      return dim(`[Function: ${(v as { name?: string }).name || "anonymous"}]`);
    default:
      return v === null ? dim("null") : "";
  }
}

/** One-line hint after a collapsed compound node, e.g. `{…}` or `[ 3 ]`. */
function header(n: TNode): string {
  const v = n.value;
  if (Array.isArray(v)) return `[ ${v.length} ]`;
  if (ArrayBuffer.isView(v)) {
    return `[${(v as object).constructor.name}: ${(v as unknown as ArrayLike<number>).length}]`;
  }
  if (v instanceof Map) return `Map(${v.size}) {…}`;
  if (v instanceof Set) return `Set(${v.size}) {…}`;
  return "{…}";
}

function renderRow(n: TNode): string {
  const indent = "  ".repeat(n.depth);
  const arrow = isExpandable(n) ? (n.expanded ? "▼ " : "▶ ") : "  ";
  const key = n.key === null ? "" : `${cyan(n.key)}: `;

  let val: string;
  if (n.circular !== null) {
    val = dim(`[Circular *${n.circular}]`);
  } else if (!isContainer(n.value)) {
    if (n.value instanceof Date) val = magenta(n.value.toISOString());
    else if (n.value instanceof RegExp) val = red(String(n.value));
    else val = fmtPrimitive(n.value);
  } else if (n.expanded) {
    val = n.children && n.children.length === 0 ? dim(Array.isArray(n.value) ? "[]" : "{}") : "";
  } else {
    val = dim(header(n));
  }
  return `${indent}${arrow}${key}${val}`;
}

function visibleRows(root: TNode): TNode[] {
  const out: TNode[] = [];
  const walk = (n: TNode) => {
    out.push(n);
    if (n.expanded && n.children) for (const ch of n.children) walk(ch);
  };
  walk(root);
  return out;
}

// ── Interactive session ──────────────────────────────────────────────
let active = false;
const pending: unknown[] = [];

/** Queue a value for inspection; sessions run one at a time. */
export function inspect(value: unknown): void {
  if (active) {
    pending.push(value);
    return;
  }
  if (!process.stdin.isTTY || !process.stdout.isTTY) {
    // Non-interactive fallback: static, pre-expanded dump (never blocks).
    console.log(staticDump(value));
    return;
  }
  active = true;
  runSession(value);
}

function runSession(value: unknown): void {
  const root: TNode = {
    key: null,
    value,
    depth: 0,
    parent: null,
    expanded: true,
    children: null,
    circular: null,
  };
  ensureChildren(root);
  root.expanded = true;

  let cursor = 0;
  let start = 0; // viewport scroll offset
  const { stdin, stdout } = process;

  const render = () => {
    const rows = visibleRows(root);
    cursor = Math.max(0, Math.min(cursor, rows.length - 1));
    const height = Math.max(3, (stdout.rows || 24) - 2);
    if (cursor < start) start = cursor;
    if (cursor >= start + height) start = cursor - height + 1;
    const width = stdout.columns || 80;

    let out = "\x1b[?25l\x1b[H\x1b[2J"; // hide cursor, home, clear
    out +=
      dim("── inspector ── ↑/↓ move · →/space expand · ← collapse · q quit ") +
      "\n";
    for (let i = start; i < Math.min(start + height, rows.length); i++) {
      const line = renderRow(rows[i]);
      if (i === cursor) {
        const pad = Math.max(0, width - stripAnsi(line).length);
        out += `\x1b[7m❯\x1b[0m` + bold(line) + " ".repeat(pad) + "\n";
      } else {
        out += " " + line + "\n";
      }
    }
    out += dim(`── ${rows.length} rows ──`);
    stdout.write(out);
  };

  const toggle = (n: TNode, want: boolean) => {
    if (!isExpandable(n)) return;
    ensureChildren(n);
    n.expanded = want;
  };

  const onKey = (buf: Buffer) => {
    const rows = visibleRows(root);
    const node = rows[cursor];
    const k = buf.toString();
    if (k === "\x03" || k === "q") return quit(); // ctrl-c / q
    switch (k) {
      case "\x1b[A": // up
      case "k":
        cursor--;
        break;
      case "\x1b[B": // down
      case "j":
        cursor++;
        break;
      case "\x1b[C": // right
      case " ":
      case "\r":
        if (node && isExpandable(node)) {
          toggle(node, true);
          cursor++; // step into the freshly revealed children
        }
        break;
      case "\x1b[D": // left
        if (node && node.expanded) {
          toggle(node, false);
        } else if (node && node.parent) {
          cursor = rows.indexOf(node.parent);
        }
        break;
    }
    render();
  };

  const restore = () => {
    stdin.removeListener("data", onKey);
    if (stdin.isTTY) stdin.setRawMode(false);
    stdin.pause();
    stdout.write("\x1b[?25h\x1b[0m\x1b[2J\x1b[H"); // show cursor, reset, clear
  };

  const quit = () => {
    restore();
    active = false;
    const next = pending.shift();
    if (next !== undefined) inspect(next);
  };

  stdin.setRawMode(true);
  stdin.resume();
  stdin.on("data", onKey);
  stdout.on("resize", render);
  render();
}

/** Non-TTY fallback: expand everything up to `maxDepth` and print once. */
function staticDump(value: unknown, maxDepth = 4): string {
  const root: TNode = {
    key: null,
    value,
    depth: 0,
    parent: null,
    expanded: true,
    children: null,
    circular: null,
  };
  const expandAll = (n: TNode) => {
    if (n.depth < maxDepth && isExpandable(n)) {
      ensureChildren(n);
      n.expanded = true;
      n.children!.forEach(expandAll);
    } else if (isExpandable(n)) {
      n.expanded = false;
    }
  };
  expandAll(root);
  return visibleRows(root).map(renderRow).join("\n");
}

// ── Demo ─────────────────────────────────────────────────────────────
const vm = new Vm();

// Guest values arrive marshalled to plain host JS — the same path pretty()
// uses — so the inspector needs no VM-specific code at all.
vm.exposeFunction("inspect", (value: unknown) => inspect(value));

const sample = `
  var user = {
    name: "ada",
    age: 36,
    active: true,
    tags: ["math", "engines"],
    address: { city: "London", zip: "NW1" },
    scores: [10, [20, [30]]]
  };
`;

console.log("=== Interactive Inspector Demo ===");
console.log("(in a TTY: navigate with arrow keys, q to quit each session)\n");

// Session 1: a guest VM object, marshalled across the NAPI boundary.
vm.run(sample + "inspect(user);");

// Session 2: a circular structure built host-side (cycles do not survive
// NAPI marshalling, see the note at the top of this file).
const ring: Record<string, unknown> = { name: "root" };
ring.self = ring;
ring.nested = { back: ring, list: [1, ring] };
inspect(ring);
