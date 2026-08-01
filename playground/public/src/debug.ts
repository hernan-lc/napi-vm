const DEBUG_SCOPE = "completion";

function isDebugEnabled(): boolean {
  if (import.meta.env.DEV) return true;
  try {
    const params = new URLSearchParams(window.location.search);
    return params.get("debug") === DEBUG_SCOPE || params.get("debug") === "1";
  } catch {
    return false;
  }
}

export function debugLog(event: string, details?: unknown): void {
  if (!isDebugEnabled()) return;
  if (details === undefined) {
    console.debug(`[napi-vm:${DEBUG_SCOPE}] ${event}`);
  } else {
    console.debug(`[napi-vm:${DEBUG_SCOPE}] ${event}`, details);
  }
}
