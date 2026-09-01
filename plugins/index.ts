/**
 * A capability-based plugin system for `napi-vm`.
 *
 * The VM stays sealed: plugins never see `require`, `process`, `node:fs`,
 * `Bun` or `Deno`. They see `napi:fs` and `napi:path` — small host modules
 * whose every privileged call is checked against the plugin's manifest *and*
 * the host policy before it touches the filesystem.
 *
 * ```ts
 * import { PluginHost } from "napi-vm/plugins";
 *
 * const host = new PluginHost({
 *   policy: { fs: { absoluteRead: false, absoluteWrite: false } },
 * });
 *
 * host.load("./examples/plugins/example-plugin");
 * host.reload("example-plugin");
 * host.unload("example-plugin");
 * ```
 */

export {
  PermissionDeniedError,
  PluginLoadError,
  PluginManifestError,
  ResourceLimitError,
} from "./errors";

export {
  parseManifest,
  validateManifest,
  SUPPORTED_API_VERSION,
  type FsPermission,
  type PluginManifest,
} from "./manifest";

export {
  compilePattern,
  escapesRoot,
  matchRule,
  normalizeSegments,
  toPosix,
  isAbsoluteGuestPath,
  type PathRule,
  type PathRuleKind,
} from "./path-rules";

export {
  compileFsPermission,
  compilePermissions,
  compilePolicy,
  defaultPolicy,
  FsPermissionChecker,
  type CompiledFsPermissions,
  type CompiledPermissions,
  type FsAccessMode,
  type PluginHostPolicy,
  type ResolvedPath,
} from "./permissions";

export {
  createNodeFileSystem,
  DEFAULT_MAX_FILE_BYTES,
  type HostFileSystem,
  type NodeFileSystemOptions,
} from "./host-filesystem";

export {
  installFsCapability,
  uninstallFsCapability,
  FS_GLOBALS,
  FS_MODULE_NAME,
} from "./filesystem-capability";

export {
  installPathCapability,
  uninstallPathCapability,
  PATH_GLOBALS,
  PATH_MODULE_NAME,
} from "./path-capability";

export {
  installCryptoCapability,
  uninstallCryptoCapability,
  CRYPTO_GLOBALS,
  CRYPTO_MODULE_NAME,
  MAX_RANDOM_BYTES,
} from "./crypto-capability";

export {
  installTimersCapability,
  uninstallTimersCapability,
  TIMERS_GLOBALS,
  TIMERS_MODULE_NAME,
  type TimersCapabilityOptions,
} from "./timers-capability";

export {
  checkFetchOrigin,
  compileFetchPermission,
  installFetchCapability,
  uninstallFetchCapability,
  DEFAULT_MAX_RESPONSE_BYTES,
  FETCH_GLOBALS,
  FETCH_MODULE_NAME,
  type CompiledFetchPermissions,
  type FetchCapabilityOptions,
  type FetchPermission,
  type FetchPolicy,
  type FetchTransport,
} from "./fetch-capability";

export {
  bootstrapSource,
  describePlugin,
  pluginModuleName,
  uninstallLifecycle,
  LIFECYCLE_GLOBALS,
  PLUGIN_MODULE_PREFIX,
  type PluginContext,
  type PluginShape,
  type UnloadContext,
  type UnloadReason,
} from "./lifecycle";

export {
  PluginHost,
  MANIFEST_FILENAME,
  type LoadedPlugin,
  type PluginHostOptions,
} from "./plugin-host";
