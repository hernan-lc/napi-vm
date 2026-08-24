import type { Vm } from "../../index";
import { VmSession } from "../../runtime/session.cjs";

const IPC_GLOBALS = Object.freeze({
  invoke: "__ipcInvoke",
  invokeAsync: "__ipcInvokeAsync",
  send: "__ipcSend",
  list: "__ipcList",
});

export interface IpcCommandInfo {
  params?: Array<{ name: string; typeName: string }>;
  returns?: string;
  documentation?: string;
  async?: boolean;
}

export type IpcHandler = (payload: unknown) => unknown;

type Listener = (payload: unknown) => void;

/**
 * Small in-process IPC bridge for the playground VM.
 *
 * Host commands are called from the VM with `ipc.invoke(command, payload)`.
 * VM events are delivered to host listeners with `ipc.send(event, payload)`.
 */
export class VmIpc {
  private vm: Vm | null = null;
  private session: VmSession | null = null;
  private commands = new Map<string, { handler: IpcHandler; info: IpcCommandInfo }>();
  private listeners = new Map<string, Set<Listener>>();

  attach(vm: Vm, session?: VmSession): void {
    this.detach();
    this.vm = vm;
    this.session = session || null;

    this.expose(IPC_GLOBALS.invoke, (command: unknown, payload: unknown) =>
      this.invoke(String(command), payload), {
      params: [
        { name: "command", typeName: "string" },
        { name: "payload", typeName: "unknown" },
      ],
      returns: "unknown",
      documentation: "Invokes a registered host command synchronously.",
    });
    this.exposeAsync(IPC_GLOBALS.invokeAsync, async (command: unknown, payload: unknown) =>
      this.invokeAsync(String(command), payload), {
      params: [
        { name: "command", typeName: "string" },
        { name: "payload", typeName: "unknown" },
      ],
      returns: "unknown",
      documentation: "Invokes a registered host command asynchronously.",
    });
    this.expose(IPC_GLOBALS.send, (event: unknown, payload: unknown) => {
      this.emit(String(event), payload);
    }, {
      params: [
        { name: "event", typeName: "string" },
        { name: "payload", typeName: "unknown" },
      ],
      returns: "void",
      documentation: "Sends a one-way event from the VM to the host.",
    });
    this.expose(IPC_GLOBALS.list, () => this.listCommands(), {
      params: [],
      returns: "string[]",
      documentation: "Lists registered host command names.",
    });

    vm.run(`
      var ipc = {
        invoke: function(command, payload) {
          return ${IPC_GLOBALS.invoke}(command, payload);
        },
        invokeAsync: function(command, payload) {
          return ${IPC_GLOBALS.invokeAsync}(command, payload);
        },
        send: function(event, payload) {
          return ${IPC_GLOBALS.send}(event, payload);
        },
        commands: function() {
          return ${IPC_GLOBALS.list}();
        }
      };
    `);
  }

  detach(): void {
    const vm = this.vm;
    if (!vm) return;
    for (const name of Object.values(IPC_GLOBALS)) {
      try {
        if (!vm.hasGlobal(name)) continue;
        if (this.session?.vm === vm) this.session.removeGlobal(name);
        else vm.removeGlobal(name);
      } catch (error) {
        // runAsync owns the VM until its worker completes. Teardown must still
        // release the host-side references and let the worker observe bridge
        // shutdown instead of throwing from a SIGINT handler or reload task.
        if (!isBusyVmError(error)) throw error;
      }
    }
    this.vm = null;
    this.session = null;
  }

  handle(name: string, handler: IpcHandler, info: IpcCommandInfo = {}): () => void {
    if (!name.trim()) throw new Error("IPC command name cannot be empty");
    if (this.commands.has(name)) throw new Error(`IPC command already registered: ${name}`);
    this.commands.set(name, { handler, info });
    return () => this.removeHandler(name);
  }

  handleAsync(name: string, handler: IpcHandler, info: IpcCommandInfo = {}): () => void {
    return this.handle(name, handler, { ...info, async: true });
  }

  removeHandler(name: string): boolean {
    return this.commands.delete(name);
  }

  listCommands(): string[] {
    return [...this.commands.keys()].sort();
  }

  on(event: string, listener: Listener): () => void {
    if (!this.listeners.has(event)) this.listeners.set(event, new Set());
    this.listeners.get(event)!.add(listener);
    return () => this.off(event, listener);
  }

  off(event: string, listener?: Listener): void {
    if (!listener) {
      this.listeners.delete(event);
      return;
    }
    this.listeners.get(event)?.delete(listener);
  }

  invoke(name: string, payload?: unknown): unknown {
    const command = this.commands.get(name);
    if (!command) throw new Error(`Unknown IPC command: ${name}`);
    if (command.info.async) throw new Error(`IPC command requires invokeAsync: ${name}`);
    return command.handler(payload);
  }

  async invokeAsync(name: string, payload?: unknown): Promise<unknown> {
    const command = this.commands.get(name);
    if (!command) throw new Error(`Unknown IPC command: ${name}`);
    return command.handler(payload);
  }

  private expose(name: string, handler: (...args: unknown[]) => unknown, info: IpcCommandInfo): void {
    if (this.session) this.session.exposeFunction(name, handler, info);
    else this.vm?.exposeFunction(name, handler);
  }

  private exposeAsync(name: string, handler: (...args: unknown[]) => Promise<unknown>, info: IpcCommandInfo): void {
    if (this.session) this.session.exposeAsyncFunction(name, handler, info);
    else this.vm?.exposeAsyncFunction(name, handler);
  }

  private emit(event: string, payload: unknown): void {
    for (const listener of [...(this.listeners.get(event) || [])]) listener(payload);
  }
}

function isBusyVmError(error: unknown): boolean {
  return /VM is busy/i.test(error instanceof Error ? error.message : String(error));
}
