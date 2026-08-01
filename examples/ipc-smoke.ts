import assert from "node:assert/strict";
import { Vm } from "../index";
import { VmIpc } from "./lib/vm-ipc";

const vm = new Vm();
const ipc = new VmIpc();
const events: Array<{ name: string; payload: unknown }> = [];

ipc.handle("math.add", (payload: any) => payload.left + payload.right, {
  params: [{ name: "payload", typeName: "object" }],
  returns: "number",
  documentation: "Adds two values sent through IPC.",
});
ipc.handleAsync("math.doubleAsync", async (payload: any) => payload.value * 2, {
  params: [{ name: "payload", typeName: "object" }],
  returns: "number",
});
ipc.on("result", (payload) => events.push({ name: "result", payload }));
ipc.attach(vm);

assert.equal(vm.run('ipc.invoke("math.add", { left: 20, right: 22 });'), "42");
assert.equal(vm.run("ipc.commands().length;"), "2");
vm.run('ipc.send("result", { ok: true });');
assert.deepEqual(events, [{ name: "result", payload: { ok: true } }]);

const asyncResult = await vm.runAsync(
  "async function main() { return await ipc.invokeAsync(\"math.doubleAsync\", { value: 21 }); } main();",
);
assert.equal(asyncResult, "42");

assert.throws(() => vm.run('ipc.invoke("missing", {});'), /Unknown IPC command/);
ipc.detach();
assert.equal(vm.hasGlobal("__ipcInvoke"), false);
console.log("VmIpc smoke test passed");
