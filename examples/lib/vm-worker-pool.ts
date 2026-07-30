/**
 * vm-worker-pool.ts — Non-blocking VM execution via Worker threads.
 *
 * Usage:
 *   const pool = new VmWorkerPool();
 *   const result = await pool.run("heavyFib(32)");
 *   pool.terminate();
 *
 * The main event loop stays free while VM work runs in a worker.
 */

import { Worker } from "worker_threads";
import { join } from "node:path";

interface PendingCall {
  id: number;
  resolve: (value: string) => void;
  reject: (reason: Error) => void;
}

export interface VmWorkerPoolOptions {
  /** Path to directory containing module .js files to pre-register. */
  modulesDir?: string;
  /** Number of workers (default 1). */
  size?: number;
}

export class VmWorkerPool {
  private worker: Worker;
  private nextId = 1;
  private pending = new Map<number, PendingCall>();
  private ready: Promise<void>;
  private terminated = false;

  constructor(opts: VmWorkerPoolOptions = {}) {
    this.worker = new Worker(join(import.meta.dirname, "vm-worker.ts"), {
      workerData: { modulesDir: opts.modulesDir },
    });

    this.worker.on("message", (msg: { id: number; result?: string; error?: string }) => {
      // The -1 "ready" message signals the worker has booted.
      if (msg.id === -1) {
        this.readyResolve?.();
        return;
      }

      const p = this.pending.get(msg.id);
      if (!p) return;
      this.pending.delete(msg.id);

      if (msg.error) {
        p.reject(new Error(msg.error));
      } else {
        p.resolve(msg.result!);
      }
    });

    this.worker.on("error", (err) => {
      this.readyReject?.(err);
      for (const p of this.pending.values()) {
        p.reject(err);
      }
      this.pending.clear();
    });

    // Expose a promise that resolves when the worker signals "ready".
    this.ready = new Promise<void>((resolve, reject) => {
      this.readyResolve = resolve;
      this.readyReject = reject;
    });
  }

  private readyResolve?: () => void;
  private readyReject?: (err: Error) => void;

  /**
   * Run code in the VM worker. Returns a promise with the stringified result.
   * The main event loop is NOT blocked.
   */
  async run(code: string, modules?: Record<string, string>): Promise<string> {
    if (this.terminated) throw new Error("Worker pool is terminated");
    await this.ready;

    const id = this.nextId++;
    return new Promise<string>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.worker.postMessage({ id, code, modules });
    });
  }

  /** Terminate the worker. */
  terminate(): void {
    if (this.terminated) return;
    this.terminated = true;
    this.worker.terminate();
    for (const p of this.pending.values()) {
      p.reject(new Error("Worker terminated"));
    }
    this.pending.clear();
  }
}
