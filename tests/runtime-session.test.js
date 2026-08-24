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

// `exposeFunction()` predates the declarative shape model and its `typeName` /
// `returns` are documented as plain `string`. They are display text: Rust keeps
// them verbatim for the hover signature and degrades anything it cannot
// interpret to `unknown`, so validating them against the declarative vocabulary
// would break callers for no gain.
test("legacy host-function metadata accepts descriptive type names", () => {
  const vm = new Vm();
  const session = new VmSession({ vm });
  session.attach(vm);

  for (const returns of ["User", "UserId", "Store", "Result<User>", "string | null"]) {
    session.exposeFunction(`get_${returns.replace(/\W/g, "")}`, () => {}, {
      params: [{ name: "id", typeName: "UserId" }],
      returns,
    });
  }
  const snapshot = session.snapshot();
  expect(snapshot.functions.map((fn) => fn.returns)).toEqual([
    "User", "UserId", "Store", "Result<User>", "string | null",
  ]);
  expect(snapshot.functions[0].params[0].typeName).toBe("UserId");

  // Still bounded: the leniency is about vocabulary, not size.
  expect(() => session.exposeFunction("huge", () => {}, { returns: "x".repeat(257) })).toThrow();
  expect(() => session.exposeFunction("ctrl", () => {}, { returns: "a\u0000b" })).toThrow();
});

// registerGlobal() shapes are interpreted rather than displayed, so they stay
// strict — a name the LSP cannot resolve is a mistake worth reporting early.
test("registerGlobal stays strict about unknown type names", () => {
  const session = new VmSession({ vm: new Vm() });
  session.attach(session.vm);
  expect(() => session.registerGlobal("api", "User")).toThrow(/unsupported legacy shape string/);
  expect(() => session.registerGlobal("api", "Result<User>")).toThrow();
  session.registerGlobal("fine", "string[]");
  expect(session.snapshot().globals[0].shape).toBe("string[]");
});

// Mirrors `legacy_string_shapes_share_the_structured_depth_limit` in
// src/lang/metadata.rs. When the two disagreed, registerGlobal() reported
// success and the Rust LSP then rejected the entire globals collection,
// silently dropping the snapshot the session had just published.
test("legacy string shapes enforce the same depth limit as structured shapes", () => {
  const session = new VmSession({ vm: new Vm() });
  session.attach(session.vm);
  const MAX_SHAPE_DEPTH = 8;

  const structured = (wrappers) => {
    let shape = { kind: "string" };
    for (let i = 0; i < wrappers; i += 1) shape = { kind: "array", items: shape };
    return shape;
  };

  for (let wrappers = 0; wrappers <= MAX_SHAPE_DEPTH; wrappers += 1) {
    session.registerGlobal("legacy", "string" + "[]".repeat(wrappers));
    session.registerGlobal("structured", structured(wrappers));
  }
  for (const wrappers of [MAX_SHAPE_DEPTH + 1, 64, 5000]) {
    expect(() => session.registerGlobal("legacy", "string" + "[]".repeat(wrappers))).toThrow(
      /exceeds maximum depth/,
    );
  }
  for (const wrappers of [MAX_SHAPE_DEPTH + 1, 64]) {
    expect(() => session.registerGlobal("structured", structured(wrappers))).toThrow(
      /exceeds maximum depth/,
    );
  }

  // Promise wrappers count toward the same budget as array wrappers.
  expect(() =>
    session.registerGlobal("mixed", "Promise<".repeat(9) + "string" + ">".repeat(9)),
  ).toThrow(/exceeds maximum depth/);
});

// A field that no branch recurses into is a field whose limits are never
// checked, so each kind accepts only the fields it actually interprets.
test("shape kinds reject fields they do not interpret", () => {
  const session = new VmSession({ vm: new Vm() });
  session.attach(session.vm);

  expect(() => session.registerGlobal("a", { kind: "string", properties: { x: { kind: "number" } } }))
    .toThrow(/invalid fields: properties/);
  expect(() => session.registerGlobal("b", { kind: "number", params: [], async: true }))
    .toThrow(/invalid fields: async, params/);
  expect(() => session.registerGlobal("c", { kind: "object", returns: { kind: "string" } }))
    .toThrow(/invalid fields: returns/);
  expect(() => session.registerGlobal("d", { kind: "array", params: [] }))
    .toThrow(/invalid fields: params/);
  expect(() => session.registerGlobal("e", { kind: "function", items: { kind: "string" } }))
    .toThrow(/invalid fields: items/);

  // The fields each kind does interpret still work.
  session.registerGlobal("ok1", { kind: "string", documentation: "fine" });
  session.registerGlobal("ok2", { kind: "array", items: { kind: "string" } });
  session.registerGlobal("ok3", { kind: "promise", value: { kind: "string" } });
  session.registerGlobal("ok4", {
    kind: "function",
    params: [{ name: "a", type: { kind: "string" } }],
    returns: { kind: "number" },
    async: true,
  });
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
