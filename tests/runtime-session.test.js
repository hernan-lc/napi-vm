import { test, expect } from "bun:test";
import { Vm } from "../index.js";
import { VmIpc } from "../examples/lib/vm-ipc.ts";
import { VmSession } from "../runtime/session.cjs";

const objectShape = (properties) => ({ kind: "object", properties });

test("VmSession.registerGlobal publishes and replaces generic metadata", () => {
  const vm = new Vm();
  const session = new VmSession({ vm });
  session.attach(vm);
  session.registerGlobal("custom", objectShape({ ping: { kind: "function", returns: { kind: "string" } } }));

  expect(session.snapshot().globals).toEqual([
    {
      name: "custom",
      shape: objectShape({ ping: { kind: "function", returns: { kind: "string" } } }),
    },
  ]);

  session.registerGlobal("custom", objectShape({ version: { kind: "number" } }));
  expect(session.snapshot().globals).toEqual([
    { name: "custom", shape: objectShape({ version: { kind: "number" } }) },
  ]);
});

test("VmSession attach and detach clear obsolete globals", () => {
  const first = new Vm();
  const replacement = new Vm();
  const session = new VmSession({ vm: first });
  session.attach(first);
  session.registerGlobal("oldApi", objectShape({ old: { kind: "boolean" } }));
  expect(session.snapshot().globals).toHaveLength(1);

  session.attach(replacement);
  expect(session.snapshot().globals).toEqual([]);
  session.registerGlobal("newApi", objectShape({ fresh: { kind: "null" } }));
  session.detach();
  expect(session.snapshot().globals).toEqual([]);
});

test("VmSession rejects malformed and oversized metadata", () => {
  const session = new VmSession({ vm: new Vm() });
  session.attach(session.vm);
  expect(() => session.registerGlobal("bad", { kind: "banana" })).toThrow();
  expect(() => session.registerGlobal("tooDeep", {
    kind: "array",
    items: { kind: "array", items: { kind: "array", items: { kind: "array", items: { kind: "array", items: { kind: "array", items: { kind: "array", items: { kind: "array", items: { kind: "array", items: { kind: "string" } } } } } } } } },
  })).toThrow();
  expect(() => session.registerGlobal("tooMany", objectShape(
    Object.fromEntries(Array.from({ length: 257 }, (_, i) => [`p${i}`, { kind: "unknown" }])),
  ))).toThrow();
  expect(() => session.registerGlobal("tooManyParams", {
    kind: "function",
    params: Array.from({ length: 65 }, (_, i) => ({ name: `p${i}`, type: { kind: "unknown" } })),
  })).toThrow();
  expect(() => session.registerGlobal("tooMuchDocs", {
    kind: "string",
    documentation: "x".repeat(16 * 1024 + 1),
  })).toThrow();

  const bounded = new VmSession({ vm: new Vm() });
  bounded.attach(bounded.vm);
  for (let i = 0; i < 256; i += 1) bounded.registerGlobal(`g${i}`, { kind: "unknown" });
  expect(() => bounded.registerGlobal("g256", { kind: "unknown" })).toThrow();
});

test("legacy host-function type strings remain accepted", () => {
  const vm = new Vm();
  const session = new VmSession({ vm });
  session.attach(vm);
  session.exposeFunction("hostObject", () => ({}), {
    params: [{ name: "value", typeName: "object" }],
    returns: "object",
  });
  expect(session.snapshot().functions[0].returns).toBe("object");
});

test("VmIpc publishes only public ipc metadata and removes the facade on detach", () => {
  const vm = new Vm();
  const session = new VmSession({ vm });
  session.attach(vm);
  const ipc = new VmIpc();
  ipc.attach(vm, session);

  const snapshot = session.snapshot();
  expect(snapshot.globals.map((global) => global.name)).toEqual(["ipc"]);
  expect(snapshot.functions.some((fn) => fn.name.startsWith("__ipc"))).toBe(false);
  expect(snapshot.globals[0].shape.properties.invokeAsync.async).toBe(true);

  ipc.detach();
  expect(vm.hasGlobal("ipc")).toBe(false);
  expect(session.snapshot().globals).toEqual([]);
});
