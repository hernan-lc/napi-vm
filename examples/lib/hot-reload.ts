import { readFileSync, watch, readdirSync, type FSWatcher } from "node:fs";
import { join, basename } from "node:path";
import { Vm } from "../../index";
import { VmEventBus } from "./vm-event-bus";

export interface ModuleEntry {
  name: string;
  file: string;
  status: "active" | "error";
  error?: string;
}

export interface HotReloadOptions {
  modulesDir: string;
  /** Optional static validation before a module reaches the VM. */
  validate?: (source: string, name: string) => string[];
  /** Called after every successful reload with the fresh VM + bus. */
  onReload?: (vm: Vm, bus: VmEventBus) => void;
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
  private opts: Required<HotReloadOptions>;

  constructor(opts: HotReloadOptions) {
    this.opts = {
      validate: () => [],
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
    const vm = this.buildVm();
    this.vm = vm;
    this.bus.attach(vm);
    this.opts.onReload(vm, this.bus);
    return vm;
  }

  /** Tear down everything and stop watching. */
  stop(): void {
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    this.watcher?.close();
    this.watcher = null;
    this.teardown();
  }

  /** Begin watching the modules directory for changes. */
  watch(): void {
    this.watcher = watch(this.opts.modulesDir, (_event, filename) => {
      if (filename && filename.endsWith(".js")) {
        this.scheduleReload(filename);
      }
    });
  }

  // ── internals ──────────────────────────────────────────────────────

  private scheduleReload(file: string): void {
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    this.debounceTimer = setTimeout(() => this.reload(file), this.opts.debounceMs);
  }

  private reload(changedFile: string): void {
    console.log(`[hot-reload] change detected: ${changedFile}`);
    this.teardown();
    const vm = this.buildVm();
    this.vm = vm;
    this.bus.attach(vm);
    this.opts.onReload(vm, this.bus);
    console.log("[hot-reload] rebuild complete\n");
  }

  /** Remove all modules, globals, and the bus binding from the current VM. */
  private teardown(): void {
    const vm = this.vm;
    if (!vm) return;
    this.bus.detach();
    for (const name of vm.listModules()) {
      vm.removeModule(name);
    }
    // Remove exposed host functions (tracked by the caller via onReload).
    this.vm = null;
  }

  private buildVm(): Vm {
    const vm = new Vm();
    const sources = this.readModules();

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
    const errors = this.opts.validate(source, name);
    if (errors.length > 0) {
      this.registry.set(name, {
        name,
        file: `${name}.js`,
        status: "error",
        error: errors.join("; "),
      });
      console.log(`  [ERROR] ${name}: ${errors.join("; ")}`);
      return;
    }
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
