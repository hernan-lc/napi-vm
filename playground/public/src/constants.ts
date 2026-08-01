export const UI = {
  productName: "napi-vm",
  brandSubtitle: "playground",
  editorScope: "global",
  wasmLabel: "WASM",
} as const;

export const WORKSPACE = {
  storageKey: "napi-vm-playground-files",
  defaultFileId: "playground",
  fileExtension: ".js",
} as const;

export const EDITOR_KEYS = {
  run: "Enter",
  completion: "Space",
  tab: "Tab",
  escape: "Escape",
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
  home: "Home",
  end: "End",
} as const;

export const COMPLETION = {
  modulePrefix: "@playground/",
  memberSeparator: ".",
  maxItems: 50,
  requestDelayMs: 90,
} as const;

export const COMPLETION_KIND_LETTERS: Record<string, string> = {
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

export const EDITOR = {
  diagnosticDelayMs: 240,
  tabIndent: "  ",
  lineBreak: "\n",
} as const;

export const RESIZER = {
  storageKey: "napi-vm-panel-ratio",
  dividerHeight: 7,
  defaultRatio: 0.55,
  minRatio: 0.2,
  maxRatio: 0.8,
} as const;

export const LOOP_LIMIT_OPTIONS = [
  { value: 100_000, label: "100K" },
  { value: 1_000_000, label: "1M" },
  { value: 5_000_000, label: "5M" },
  { value: 20_000_000, label: "20M" },
  { value: 100_000_000, label: "100M" },
] as const;
