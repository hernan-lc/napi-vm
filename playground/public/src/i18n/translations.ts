export type Locale = "en" | "es" | "pt";

export interface Translations {
  run: string;
  reset: string;
  clear: string;
  loopLimit: string;
  console: string;
  entries: string;
  entry: string;
  emptyConsole: string;
  loadingWasm: string;
  ready: string;
  error: string;
  runHint: string;
  completeHint: string;
  vmReady: string;
  vmFailed: string;
  theme: string;
  language: string;
  alert: string;
  diagWarning: string;
}

const en: Translations = {
  run: "Run",
  reset: "Reset",
  clear: "Clear",
  loopLimit: "loop limit",
  console: "console",
  entries: "entries",
  entry: "entry",
  emptyConsole: "Run code to see output here",
  loadingWasm: "loading wasm…",
  ready: "ready",
  error: "error",
  runHint: "Ctrl/⌘+↵ run",
  completeHint: "Ctrl+Space complete",
  vmReady: "WASM VM ready — running entirely in your browser",
  vmFailed: "could not initialise the WASM VM:",
  theme: "theme",
  language: "language",
  alert: "alert",
  diagWarning: "⚠",
};

const es: Translations = {
  run: "Ejecutar",
  reset: "Reiniciar",
  clear: "Limpiar",
  loopLimit: "límite de bucle",
  console: "consola",
  entries: "entradas",
  entry: "entrada",
  emptyConsole: "Ejecuta código para ver la salida aquí",
  loadingWasm: "cargando wasm…",
  ready: "listo",
  error: "error",
  runHint: "Ctrl/⌘+↵ ejecutar",
  completeHint: "Ctrl+Espacio completar",
  vmReady: "WASM VM lista — ejecutándose completamente en tu navegador",
  vmFailed: "no se pudo inicializar la WASM VM:",
  theme: "tema",
  language: "idioma",
  alert: "alerta",
  diagWarning: "⚠",
};

const pt: Translations = {
  run: "Executar",
  reset: "Reiniciar",
  clear: "Limpar",
  loopLimit: "limite de loop",
  console: "console",
  entries: "entradas",
  entry: "entrada",
  emptyConsole: "Execute código para ver a saída aqui",
  loadingWasm: "carregando wasm…",
  ready: "pronto",
  error: "erro",
  runHint: "Ctrl/⌘+↵ executar",
  completeHint: "Ctrl+Espaço completar",
  vmReady: "WASM VM pronta — rodando inteiramente no seu navegador",
  vmFailed: "não foi possível inicializar a WASM VM:",
  theme: "tema",
  language: "idioma",
  alert: "alerta",
  diagWarning: "⚠",
};

export const LOCALES: Record<Locale, Translations> = { en, es, pt };

export const LOCALE_LABELS: Record<Locale, string> = {
  en: "English",
  es: "Español",
  pt: "Português",
};
