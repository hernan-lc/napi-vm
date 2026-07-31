export interface RunResult {
  ok: boolean;
  value: string;
  error: string;
  logs: LogEntry[];
}

export interface LogEntry {
  level: string;
  text: string;
}

export interface CompletionItem {
  label: string;
  kind: string;
  detail?: string;
}

export interface Diagnostic {
  line: number;
  col: number;
  message: string;
  severity: string;
}

export interface HostOptions {
  loopLimit: number;
  onAlert: (msg: string) => void;
}

export interface ModuleDef {
  name: string;
  source: string;
}

export interface ConsoleView {
  addLine(cls: string, html: string): void;
  sys(text: string): void;
  clear(): void;
  renderResult(r: RunResult, ms: number): void;
}

export interface CompletionController {
  isOpen(): boolean;
  close(): void;
  request(force: boolean): void;
  move(delta: number): void;
  accept(): void;
}

export interface DiagnosticsController {
  refresh(): void;
}
