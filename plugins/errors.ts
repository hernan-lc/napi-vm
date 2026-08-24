/**
 * Error types shared by the plugin host.
 *
 * `PermissionDeniedError` is the only one that crosses into the VM. The
 * interpreter carries `name` across, so the guest sees
 * `e.name === "PermissionDenied"` with the bare detail as `e.message`, and a
 * host-side `callFunction` sees them rejoined as `"PermissionDenied: …"`.
 *
 * Messages must never contain host filesystem paths — a guest may only see the
 * path string it passed in itself.
 */

/** A path denied by host policy or by the plugin's manifest permissions. */
export class PermissionDeniedError extends Error {
  override readonly name = "PermissionDenied";

  constructor(detail: string) {
    super(detail);
  }
}

/** A malformed or unsupported `plugin.json`. */
export class PluginManifestError extends Error {
  override readonly name = "PluginManifestError";

  constructor(detail: string) {
    super(`PluginManifestError: ${detail}`);
  }
}

/** A failure while loading, reloading or unloading a plugin. */
export class PluginLoadError extends Error {
  override readonly name = "PluginLoadError";

  constructor(detail: string, options?: { cause?: unknown }) {
    super(`PluginLoadError: ${detail}`, options);
  }
}
