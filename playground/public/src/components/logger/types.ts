export type LogLevel = "log" | "info" | "warn" | "error" | "debug" | "dir" | "result" | "sys";

export const LOG_LEVELS = {
  log: "log",
  info: "info",
  warn: "warn",
  error: "error",
  debug: "debug",
  dir: "dir",
  result: "result",
  sys: "sys",
} as const satisfies Record<LogLevel, LogLevel>;

export interface LogEntry {
  id: number;
  level: LogLevel;
  text: string;
  timestamp: number;
  html?: string;
}

export interface LoggerFilter {
  log: boolean;
  info: boolean;
  warn: boolean;
  error: boolean;
  debug: boolean;
  dir: boolean;
  result: boolean;
  sys: boolean;
}

export const DEFAULT_FILTER: LoggerFilter = {
  log: true,
  info: true,
  warn: true,
  error: true,
  debug: true,
  dir: true,
  result: true,
  sys: true,
};

export const LEVEL_ICONS: Record<LogLevel, string> = {
  log: "›",
  info: "ℹ",
  warn: "⚠",
  error: "✕",
  debug: "◆",
  dir: "▸",
  result: "←",
  sys: "·",
};

export const LEVEL_LABELS: Record<LogLevel, string> = {
  log: "log",
  info: "info",
  warn: "warn",
  error: "error",
  debug: "debug",
  dir: "dir",
  result: "result",
  sys: "sys",
};
