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
  explorer: string;
  workspace: string;
  discardVmState: string;
  clearConsole: string;
  fileName: string;
  editorLanguage: string;
  encoding: string;
  indentation: string;
  autocomplete: string;
  runShortcut: string;
  editorAriaLabel: string;
  problems: string;
  noProblems: string;
  filterOutput: string;
  copyAll: string;
  expand: string;
  collapse: string;
  noMatchingEntries: string;
  failedModule: string;
  vmReset: string;
  files: string;
  newFile: string;
  renameFile: string;
  deleteFile: string;
  deleteFileConfirm: string;
  closeTab: string;
  untitledFile: string;
  cannotDeleteLastFile: string;
  readOnlyFile: string;
  openImportHint: string;
  cancel: string;
  ok: string;
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
  explorer: "EXPLORER",
  workspace: "PLAYGROUND",
  discardVmState: "Discard all VM state",
  clearConsole: "Clear the console",
  fileName: "playground.js",
  editorLanguage: "JavaScript",
  encoding: "UTF-8",
  indentation: "Spaces: 2",
  autocomplete: "autocomplete",
  runShortcut: "run",
  editorAriaLabel: "JavaScript source editor",
  problems: "problem",
  noProblems: "No problems",
  filterOutput: "filter output…",
  copyAll: "Copy all",
  expand: "Expand",
  collapse: "Collapse",
  noMatchingEntries: "No matching entries",
  failedModule: "failed to register module",
  vmReset: "VM state reset",
  files: "FILES",
  newFile: "New file",
  renameFile: "Rename file",
  deleteFile: "Delete file",
  deleteFileConfirm: "Delete this file? Its contents will be removed from this browser workspace.",
  closeTab: "Close tab",
  untitledFile: "untitled.js",
  cannotDeleteLastFile: "The workspace must keep one file.",
  readOnlyFile: "Read-only example module",
  openImportHint: "Ctrl/⌘+click an import to open it",
  cancel: "Cancel",
  ok: "OK",
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
  explorer: "EXPLORADOR",
  workspace: "PLAYGROUND",
  discardVmState: "Descartar todo el estado de la VM",
  clearConsole: "Limpiar la consola",
  fileName: "playground.js",
  editorLanguage: "JavaScript",
  encoding: "UTF-8",
  indentation: "Espacios: 2",
  autocomplete: "autocompletar",
  runShortcut: "ejecutar",
  editorAriaLabel: "Editor de código JavaScript",
  problems: "problema",
  noProblems: "Sin problemas",
  filterOutput: "filtrar salida…",
  copyAll: "Copiar todo",
  expand: "Expandir",
  collapse: "Contraer",
  noMatchingEntries: "No hay entradas coincidentes",
  failedModule: "no se pudo registrar el módulo",
  vmReset: "Estado de la VM reiniciado",
  files: "ARCHIVOS",
  newFile: "Nuevo archivo",
  renameFile: "Renombrar archivo",
  deleteFile: "Eliminar archivo",
  deleteFileConfirm: "¿Eliminar este archivo? Su contenido se quitará de este espacio de trabajo del navegador.",
  closeTab: "Cerrar pestaña",
  untitledFile: "sin-título.js",
  cannotDeleteLastFile: "El espacio de trabajo debe conservar un archivo.",
  readOnlyFile: "Módulo de ejemplo de solo lectura",
  openImportHint: "Ctrl/⌘+clic en un import para abrirlo",
  cancel: "Cancelar",
  ok: "Aceptar",
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
  explorer: "EXPLORADOR",
  workspace: "PLAYGROUND",
  discardVmState: "Descartar todo o estado da VM",
  clearConsole: "Limpar o console",
  fileName: "playground.js",
  editorLanguage: "JavaScript",
  encoding: "UTF-8",
  indentation: "Espaços: 2",
  autocomplete: "autocompletar",
  runShortcut: "executar",
  editorAriaLabel: "Editor de código JavaScript",
  problems: "problema",
  noProblems: "Sem problemas",
  filterOutput: "filtrar saída…",
  copyAll: "Copiar tudo",
  expand: "Expandir",
  collapse: "Recolher",
  noMatchingEntries: "Nenhuma entrada correspondente",
  failedModule: "não foi possível registrar o módulo",
  vmReset: "Estado da VM reiniciado",
  files: "ARQUIVOS",
  newFile: "Novo arquivo",
  renameFile: "Renomear arquivo",
  deleteFile: "Excluir arquivo",
  deleteFileConfirm: "Excluir este arquivo? O conteúdo será removido deste espaço de trabalho do navegador.",
  closeTab: "Fechar aba",
  untitledFile: "sem-título.js",
  cannotDeleteLastFile: "O espaço de trabalho precisa manter um arquivo.",
  readOnlyFile: "Módulo de exemplo somente leitura",
  openImportHint: "Ctrl/⌘+clique em um import para abrir",
  cancel: "Cancelar",
  ok: "OK",
};

export const LOCALES: Record<Locale, Translations> = { en, es, pt };

export const LOCALE_LABELS: Record<Locale, string> = {
  en: "English",
  es: "Español",
  pt: "Português",
};
