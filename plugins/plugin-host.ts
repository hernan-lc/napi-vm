/**
 * `PluginHost` — manifest → permissions → VM → lifecycle.
 *
 * The host owns every decision the guest is not allowed to make: what the
 * manifest may request, where `"./"` points, which capabilities exist in the
 * VM, and when the VM is thrown away and rebuilt.
 */

import * as nodePath from "node:path";

import { Vm } from "../index";

import { PermissionDeniedError, PluginLoadError } from "./errors";
import {
  installFsCapability,
  uninstallFsCapability,
} from "./filesystem-capability";
import { createNodeFileSystem, type HostFileSystem } from "./host-filesystem";
import {
  bootstrapSource,
  describePlugin,
  pluginModuleName,
  uninstallLifecycle,
  type PluginContext,
  type UnloadReason,
} from "./lifecycle";
import { parseManifest, type PluginManifest } from "./manifest";
import {
  compilePermissions,
  compilePolicy,
  defaultPolicy,
  FsPermissionChecker,
  type CompiledPermissions,
  type PluginHostPolicy,
} from "./permissions";
import {
  installPathCapability,
  uninstallPathCapability,
} from "./path-capability";

export const MANIFEST_FILENAME = "plugin.json";

export interface LoadedPlugin {
  manifest: PluginManifest;
  /** Canonical plugin root. Host-side only — never handed to the guest. */
  root: string;
  vm: Vm;
  permissions: CompiledPermissions;
  status: "loaded" | "error";
  error?: Error;
  /** Whatever the last `onLoad` / `onReload` returned. */
  loadResult?: unknown;
}

export interface PluginHostOptions {
  policy?: PluginHostPolicy;
  /** Swap in a Bun/Deno/Rust-backed filesystem. Defaults to `node:fs`. */
  fs?: HostFileSystem;
}

interface PreparedPlugin {
  manifest: PluginManifest;
  root: string;
  entrySource: string;
  permissions: CompiledPermissions;
  checker: FsPermissionChecker;
}

export class PluginHost {
  private readonly policy: PluginHostPolicy;
  private readonly compiledPolicy: ReturnType<typeof compilePolicy>;
  private readonly fs: HostFileSystem;
  private readonly plugins = new Map<string, LoadedPlugin>();

  constructor(options: PluginHostOptions = {}) {
    this.policy = options.policy ?? defaultPolicy();
    this.compiledPolicy = compilePolicy(this.policy);
    this.fs = options.fs ?? createNodeFileSystem();
  }

  /** Load a plugin directory and run `onLoad`. */
  load(pluginDirectory: string): LoadedPlugin {
    const prepared = this.prepare(pluginDirectory);
    const { name } = prepared.manifest;

    const existing = this.plugins.get(name);
    if (existing && existing.status === "loaded") {
      throw new PluginLoadError(`plugin "${name}" is already loaded`);
    }

    const plugin = this.instantiate(prepared);
    this.plugins.set(name, plugin);
    plugin.loadResult = this.invoke(
      plugin,
      "__plugin_onLoad",
      [this.context(prepared.manifest)],
      "onLoad",
    );
    return plugin;
  }

  /**
   * Rebuild a plugin from disk in a *fresh* VM.
   *
   * The old instance gets `onUnload({ reason: "reload" })`; whatever
   * serializable value it returns is handed to the new instance's `onReload`.
   */
  reload(name: string): LoadedPlugin {
    const current = this.plugins.get(name);
    if (!current) throw new PluginLoadError(`plugin "${name}" is not loaded`);

    let previousState: unknown;
    if (current.status === "loaded") {
      previousState = this.callUnload(current, "reload");
    }
    this.dispose(current);
    this.plugins.delete(name);

    const prepared = this.prepare(current.root);
    if (prepared.manifest.name !== name) {
      throw new PluginLoadError(
        `plugin directory now declares "${prepared.manifest.name}", expected "${name}"`,
      );
    }

    const plugin = this.instantiate(prepared);
    this.plugins.set(name, plugin);
    plugin.loadResult = this.invoke(
      plugin,
      "__plugin_onReload",
      [this.context(prepared.manifest), previousState ?? null],
      "onReload",
    );
    return plugin;
  }

  /** Run `onUnload`, tear the VM down and forget the plugin. */
  unload(name: string): unknown {
    const plugin = this.plugins.get(name);
    if (!plugin) throw new PluginLoadError(`plugin "${name}" is not loaded`);

    let state: unknown;
    if (plugin.status === "loaded") {
      state = this.callUnload(plugin, "unload");
    }
    this.dispose(plugin);
    this.plugins.delete(name);
    return state;
  }

  get(name: string): LoadedPlugin | undefined {
    return this.plugins.get(name);
  }

  list(): LoadedPlugin[] {
    return [...this.plugins.values()];
  }

  /** Unload every plugin, ignoring individual failures. */
  unloadAll(): void {
    for (const name of [...this.plugins.keys()]) {
      try {
        this.unload(name);
      } catch {
        this.plugins.delete(name);
      }
    }
  }

  // ── internals ──────────────────────────────────────────────────────

  /** Read + validate the manifest and entry file; compile permissions. */
  private prepare(pluginDirectory: string): PreparedPlugin {
    const resolvedRoot = nodePath.resolve(pluginDirectory);
    const root = this.fs.realpath(resolvedRoot);
    if (root === null) {
      throw new PluginLoadError(`plugin directory not found: ${pluginDirectory}`);
    }

    const manifestPath = nodePath.join(root, MANIFEST_FILENAME);
    if (!this.fs.exists(manifestPath)) {
      throw new PluginLoadError(`missing ${MANIFEST_FILENAME} in ${pluginDirectory}`);
    }
    const manifest = parseManifest(this.fs.readText(manifestPath));
    const permissions = compilePermissions(manifest);

    // The entry file is read by the host, not by the plugin, so it is not
    // subject to `permissions.fs` — but it must still live inside the root,
    // symlinks included.
    const entryNative = nodePath.resolve(
      root,
      manifest.entry.split("/").join(nodePath.sep),
    );
    const entryReal = this.fs.realpath(entryNative);
    if (entryReal === null) {
      throw new PluginLoadError(`entry file not found: ${manifest.entry}`);
    }
    if (entryReal !== root && !entryReal.startsWith(root + nodePath.sep)) {
      throw new PluginLoadError("entry must be a path inside the plugin directory");
    }
    const entrySource = this.fs.readText(entryReal);

    const checker = new FsPermissionChecker(
      root,
      permissions.fs,
      this.compiledPolicy,
      this.fs,
    );

    return { manifest, root, entrySource, permissions, checker };
  }

  /** Build a VM, install capabilities and create the plugin instance. */
  private instantiate(prepared: PreparedPlugin): LoadedPlugin {
    const { manifest, root, entrySource, permissions, checker } = prepared;
    const vm = new Vm();
    const plugin: LoadedPlugin = {
      manifest,
      root,
      vm,
      permissions,
      status: "loaded",
    };

    try {
      installFsCapability(vm, { checker, fs: this.fs });
      if (permissions.path) installPathCapability(vm);

      const moduleName = pluginModuleName(manifest.name);
      vm.registerModule(moduleName, entrySource);
      vm.run(bootstrapSource(moduleName));

      const shape = describePlugin(vm);
      if (!shape.hasInstance) {
        throw new PluginLoadError(
          `plugin "${manifest.name}" must default-export an object or a class`,
        );
      }
    } catch (error) {
      plugin.status = "error";
      plugin.error = asError(error);
      this.dispose(plugin);
      this.plugins.set(manifest.name, plugin);
      throw wrapLoadError(manifest.name, "instantiate", error);
    }

    return plugin;
  }

  private context(manifest: PluginManifest): PluginContext {
    return { name: manifest.name, version: manifest.version };
  }

  private callUnload(plugin: LoadedPlugin, reason: UnloadReason): unknown {
    return this.invoke(
      plugin,
      "__plugin_onUnload",
      [{ ...this.context(plugin.manifest), reason }],
      "onUnload",
    );
  }

  /** Call a lifecycle wrapper, recording failures on the plugin entry. */
  private invoke(
    plugin: LoadedPlugin,
    fn: string,
    args: unknown[],
    hook: string,
  ): unknown {
    try {
      return plugin.vm.callFunction(fn, args);
    } catch (error) {
      plugin.status = "error";
      plugin.error = asError(error);
      throw wrapLoadError(plugin.manifest.name, hook, error);
    }
  }

  /** Detach every host capability and drop the module graph. */
  private dispose(plugin: LoadedPlugin): void {
    const { vm } = plugin;
    try {
      uninstallLifecycle(vm, pluginModuleName(plugin.manifest.name));
      uninstallPathCapability(vm);
      uninstallFsCapability(vm);
    } catch {
      // A half-built VM is being thrown away anyway.
    }
  }
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

function wrapLoadError(name: string, hook: string, error: unknown): Error {
  if (error instanceof PermissionDeniedError) return error;
  const cause = asError(error);
  if (error instanceof PluginLoadError) return error;
  return new PluginLoadError(`plugin "${name}" failed in ${hook}: ${cause.message}`, {
    cause,
  });
}
