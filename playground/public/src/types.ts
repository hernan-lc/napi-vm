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
