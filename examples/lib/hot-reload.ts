import { readFileSync, watch, readdirSync, type FSWatcher } from "node:fs";
import { join, basename } from "node:path";
import { Vm } from "../../index";
import { VmSession } from "../../runtime/session.cjs";
import { VmEventBus } from "./vm-event-bus";

export interface ModuleEntry {
  name: string;
  file: string;
  status: "active" | "error";
  error?: string;
}

export interface HotReloadOptions {
  modulesDir: string;
  /** Called after a VM is created, before any module is evaluated. */
  onBeforeLoad?: (vm: Vm, session?: VmSession) => void;
  /** Called after every successful reload with the fresh VM + bus. */
  onReload?: (vm: Vm, bus: VmEventBus, session?: VmSession) => void;
  /** Optional live runtime channel consumed by the workspace LSP. */
  runtime?: VmSession;
  /** Debounce window in ms (default 100). */
  debounceMs?: number;
}

/**
 * Manages the full hot-reload lifecycle for a VM + event bus pair.
 *
 * Teardown order on each reload:
 *   1. bus.detach()          – removes the VM-side `emit` binding
 *   2. vm.removeModule(…)    – drops every registered module
 *   3. vm.removeGlobal(…)    – drops every exposed host function
 *   4. rebuild VM, re-register modules, bus.attach(newVm)
 *   5. re-expose host functions, re-run bootstrap code
 *
 * Host-side listeners registered via `bus.on(…)` survive across reloads
 * because they live on the bus, not in the VM. The VM only ever sees a
 * single `emit` global that is replaced atomically on each cycle, so
 * there is never a duplicate-listener window.
 */
export class HotReloader {
  readonly registry = new Map<string, ModuleEntry>();
  readonly bus = new VmEventBus();

  private vm: Vm | null = null;
  private watcher: FSWatcher | null = null;
  private debounceTimer: ReturnType<typeof setTimeout> | null = null;
  private moduleSources = new Map<string, string>();
  private runtime: VmSession | null;
  private stopped = false;
  private opts: Omit<HotReloadOptions, "onBeforeLoad" | "onReload" | "debounceMs"> & {
    onBeforeLoad: NonNullable<HotReloadOptions["onBeforeLoad"]>;
    onReload: NonNullable<HotReloadOptions["onReload"]>;
    debounceMs: number;
  };

  constructor(opts: HotReloadOptions) {
    this.runtime = opts.runtime || null;
    this.opts = {
      onBeforeLoad: () => {},
      onReload: () => {},
      debounceMs: 100,
      ...opts,
    };
  }

  get currentVm(): Vm | null {
    return this.vm;
  }

  // ── lifecycle ──────────────────────────────────────────────────────

  /** Build the initial VM, register modules, attach the bus. */
  start(): Vm {
    this.stopped = false;
    this.runtime?.start();
    const vm = this.buildVm();
    this.vm = vm;
    this.bus.attach(vm);
    this.opts.onReload(vm, this.bus, this.runtime || undefined);
    return vm;
  }

  /** Tear down everything and stop watching. */
  stop(): void {
    this.stopped = true;
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = null;
    }
    const watcher = this.watcher;
    this.watcher = null;
    watcher?.close();
    try {
      this.teardown();
    } finally {
      this.runtime?.stop();
    }
  }

  /** Begin watching the modules directory for changes. */
  watch(): void {
    if (this.watcher) return;
    this.watcher = watch(this.opts.modulesDir, (_event, filename) => {
      if (!this.stopped && filename && filename.endsWith(".js")) {
        this.scheduleReload(filename);
      }
    });
    this.watcher.on("error", (error) => {
      if (!this.stopped) {
        console.error(`[hot-reload] watcher error: ${error.message}`);
      }
    });
  }

  // ── internals ──────────────────────────────────────────────────────

  private scheduleReload(file: string): void {
    if (this.stopped) return;
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    this.debounceTimer = setTimeout(() => {
      this.debounceTimer = null;
      this.reload(file);
    }, this.opts.debounceMs);
  }

  private reload(changedFile: string): void {
    if (this.stopped) return;
    console.log(`[hot-reload] change detected: ${changedFile}`);
    try {
      this.teardown();
      const vm = this.buildVm();
      this.vm = vm;
      this.bus.attach(vm);
      this.opts.onReload(vm, this.bus, this.runtime || undefined);
      console.log("[hot-reload] rebuild complete\n");
    } catch (error) {
      console.error(`[hot-reload] rebuild failed: ${error instanceof Error ? error.message : error}`);
    }
  }

  /** Remove all modules, globals, and the bus binding from the current VM. */
  private teardown(): void {
    const vm = this.vm;
    this.runtime?.detach();
    this.bus.detach();
    if (!vm) return;
    this.vm = null;
    let moduleNames: string[] = [];
    try {
      moduleNames = vm.listModules();
    } catch (error) {
      if (!isBusyVmError(error)) throw error;
    }
    for (const name of moduleNames) {
      try {
        vm.removeModule(name);
      } catch (error) {
        if (!isBusyVmError(error)) throw error;
      }
    }
    // Remove exposed host functions (tracked by the caller via onReload).
  }

  private buildVm(): Vm {
    const vm = new Vm();
    const sources = this.readModules();
    this.moduleSources = sources;

    // registerModule evaluates top-level code immediately. Attach the live
    // runtime metadata and host bridge first so modules may safely use IPC at
    // module scope (for example, `ipc.commands()` in ipc.js).
    this.runtime?.attach(vm, {
      modules: [...sources.entries()].map(([name, source]) => ({ name, source })),
    });
    this.opts.onBeforeLoad(vm, this.runtime || undefined);

    // Deterministic load order: utils first (other modules import it).
    const sorted = [...sources.entries()].sort(([a], [b]) => {
      if (a === "utils") return -1;
      if (b === "utils") return 1;
      return 0;
    });

    this.registry.clear();
    for (const [name, source] of sorted) {
      this.loadModule(vm, name, source);
    }
    return vm;
  }

  private loadModule(vm: Vm, name: string, source: string): void {
    try {
      vm.registerModule(name, source);
      this.registry.set(name, { name, file: `${name}.js`, status: "active" });
      console.log(`  [OK] ${name} loaded`);
    } catch (err: any) {
      this.registry.set(name, {
        name,
        file: `${name}.js`,
        status: "error",
        error: err.message || String(err),
      });
      console.log(`  [ERROR] ${name}: ${err.message || err}`);
    }
  }

  private readModules(): Map<string, string> {
    const files = readdirSync(this.opts.modulesDir).filter((f) =>
      f.endsWith(".js")
    );
    const sources = new Map<string, string>();
    for (const file of files) {
      const name = basename(file, ".js");
      sources.set(name, readFileSync(join(this.opts.modulesDir, file), "utf-8"));
    }
    return sources;
  }
}

function isBusyVmError(error: unknown): boolean {
  return /VM is busy/i.test(error instanceof Error ? error.message : String(error));
}
