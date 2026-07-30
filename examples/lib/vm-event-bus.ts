import type { Vm } from "../../index";

type Listener = (...args: unknown[]) => void;

/**
 * Host-side event bus that bridges VM events to Node listeners.
 *
 * The VM gets a single `emit(event, ...args)` global (via exposeFunction).
 * When VM code calls it, execution crosses the NAPI boundary synchronously
 * and dispatches to every listener registered for that event on the host.
 *
 * Because the VM is synchronous, `emit` blocks the VM until all listeners
 * return — this is the mechanism that lets you observe how blocking work
 * interacts with the JS event loop (see the demo in callback.ts).
 *
 * Hot-reload safety: call `detach()` before rebuilding the VM, then create
 * a fresh bus and `attach()` again. Listeners registered via `on` survive
 * across reloads (they live on the host, not in the VM); the VM-side
 * `emit` binding is the only thing that gets replaced.
 */
export class VmEventBus {
  private listeners = new Map<string, Set<Listener>>();
  private vm: Vm | null = null;
  private emitLog: Array<{ event: string; args: unknown[]; at: number }> = [];

  /** Wire the bus to a VM instance, exposing `emit` as a global. */
  attach(vm: Vm): void {
    this.vm = vm;
    // Remove any stale binding first (idempotent on a fresh VM).
    if (vm.hasGlobal("__vmEmit")) {
      vm.removeGlobal("__vmEmit");
    }
    vm.exposeFunction("__vmEmit", (event: unknown, ...args: unknown[]) => {
      this.dispatch(String(event), args);
    });
    // Install a thin wrapper so VM code uses `emit(event, ...args)`.
    // Uses spread (supported) instead of Function.prototype.apply (not implemented).
    vm.run(`function emit(...args) { return __vmEmit(...args); }`);
  }

  /** Detach from the current VM (call before hot-reload teardown). */
  detach(): void {
    if (this.vm && this.vm.hasGlobal("__vmEmit")) {
      this.vm.removeGlobal("__vmEmit");
    }
    this.vm = null;
  }

  /** Register a listener. Returns an unsubscribe function. */
  on(event: string, fn: Listener): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(fn);
    return () => this.off(event, fn);
  }

  /** Register a one-shot listener that removes itself after firing. */
  once(event: string, fn: Listener): () => void {
    const wrapper: Listener = (...args) => {
      this.off(event, wrapper);
      fn(...args);
    };
    return this.on(event, wrapper);
  }

  /** Remove a specific listener, or all listeners for an event. */
  off(event: string, fn?: Listener): void {
    if (!fn) {
      this.listeners.delete(event);
      return;
    }
    this.listeners.get(event)?.delete(fn);
  }

  /** Remove every listener across all events. */
  offAll(): void {
    this.listeners.clear();
  }

  /** Number of listeners registered for an event. */
  listenerCount(event: string): number {
    return this.listeners.get(event)?.size ?? 0;
  }

  /** All event names that have at least one listener. */
  eventNames(): string[] {
    return [...this.listeners.keys()].filter(
      (k) => (this.listeners.get(k)?.size ?? 0) > 0
    );
  }

  /** Recent emit history (useful for debugging / demos). */
  get log(): ReadonlyArray<{ event: string; args: unknown[]; at: number }> {
    return this.emitLog;
  }

  clearLog(): void {
    this.emitLog.length = 0;
  }

  private dispatch(event: string, args: unknown[]): void {
    this.emitLog.push({ event, args, at: Date.now() });
    const fns = this.listeners.get(event);
    if (!fns || fns.size === 0) return;
    // Snapshot so listeners can safely remove themselves during iteration.
    for (const fn of [...fns]) {
      fn(...args);
    }
  }
}
