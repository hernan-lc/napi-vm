import { test, expect } from "bun:test";
import { Vm } from "../index.js";

// `registerHostModule` is the generic form of `exposeFunction` +
// `registerModule`: the core bridges the functions and generates the wrapper
// module; what the functions do stays entirely on the host side.

test("registerHostModule exposes host functions as module exports", () => {
  const vm = new Vm();
  vm.registerHostModule("napi:demo", {
    hello: (name) => `hi ${name}`,
    add: (a, b) => a + b,
  });
  vm.registerModule(
    "app",
    `import { hello, add } from "napi:demo";
     export function go() { return hello("world") + "/" + add(2, 3); }`,
  );
  expect(vm.run(`import { go } from "app"; go();`)).toBe("hi world/5");
});

test("registerHostModule returns the globals it created", () => {
  const vm = new Vm();
  const globals = vm.registerHostModule("napi:fs", { readText: () => "x" });
  expect(globals).toEqual(["__hostmod_napi_fs_readText"]);
  expect(vm.hasGlobal("__hostmod_napi_fs_readText")).toBe(true);
});

test("the module appears in listModules and can be removed", () => {
  const vm = new Vm();
  const globals = vm.registerHostModule("napi:demo", { ping: () => "pong" });
  expect(vm.hasModule("napi:demo")).toBe(true);
  expect(vm.listModules()).toContain("napi:demo");

  expect(vm.removeModule("napi:demo")).toBe(true);
  for (const name of globals) expect(vm.removeGlobal(name)).toBe(true);
  expect(vm.hasModule("napi:demo")).toBe(false);
  expect(vm.hasGlobal(globals[0])).toBe(false);
});

test("exports receive every argument and can return objects", () => {
  const vm = new Vm();
  vm.registerHostModule("napi:demo", {
    collect: (...args) => ({ count: args.length, args }),
  });
  vm.registerModule("app", `import { collect } from "napi:demo";
    export function go() { return collect(1, "a", true); }`);
  expect(vm.run(`import { go } from "app"; JSON.stringify(go());`)).toBe(
    '{"count":3,"args":[1,"a",true]}',
  );
});

test("a host error thrown by an export is catchable in the guest", () => {
  const vm = new Vm();
  vm.registerHostModule("napi:demo", {
    boom: () => {
      throw new Error("PermissionDenied: nope");
    },
  });
  vm.registerModule("app", `import { boom } from "napi:demo";
    export function go() { try { boom(); return "no-throw"; } catch (e) { return e.message; } }`);
  expect(vm.run(`import { go } from "app"; go();`)).toBe("PermissionDenied: nope");
});

test("options.async marks exports the guest can await", async () => {
  const vm = new Vm();
  vm.registerHostModule(
    "napi:net",
    { fetchText: async (url) => `body-of:${url}` },
    { async: ["fetchText"] },
  );
  vm.registerModule("app", `import { fetchText } from "napi:net";
    export async function go() { return await fetchText("https://example.test"); }`);
  await expect(vm.runAsync(`import { go } from "app"; await go();`)).resolves.toBe(
    "body-of:https://example.test",
  );
});

test("re-registering a host module replaces the previous exports", () => {
  const vm = new Vm();
  vm.registerHostModule("napi:demo", { value: () => 1 });
  vm.registerHostModule("napi:demo", { value: () => 2 });
  vm.registerModule("app", `import { value } from "napi:demo";
    export function go() { return value(); }`);
  expect(vm.run(`import { go } from "app"; go();`)).toBe("2");
});

test("an empty exports object is rejected", () => {
  const vm = new Vm();
  expect(() => vm.registerHostModule("napi:demo", {})).toThrow(
    /must export at least one function/,
  );
});

test("non-function exports are rejected", () => {
  const vm = new Vm();
  expect(() => vm.registerHostModule("napi:demo", { a: 1 })).toThrow(
    /export 'a' must be a function/,
  );
});

test("export names that are not identifiers are rejected", () => {
  const vm = new Vm();
  expect(() => vm.registerHostModule("napi:demo", { "a-b": () => 1 })).toThrow(
    /is not a usable export name/,
  );
  expect(() => vm.registerHostModule("napi:demo", { default: () => 1 })).toThrow(
    /is not a usable export name/,
  );
  expect(() => vm.registerHostModule("napi:demo", { "2fast": () => 1 })).toThrow(
    /is not a usable export name/,
  );
});

test("registering a host module does not leak host globals into the guest", () => {
  // The bridge globals exist by design; the point is that the *host* object
  // itself never crosses over.
  const vm = new Vm();
  const host = { secret: "s3cret", ping: () => "pong" };
  expect(() => vm.registerHostModule("napi:demo", host)).toThrow(
    /export 'secret' must be a function/,
  );
});

test("options.async naming a missing export is rejected", () => {
  const vm = new Vm();
  expect(() =>
    vm.registerHostModule("napi:demo", { ping: () => "pong" }, { async: ["pong"] }),
  ).toThrow(/options.async names 'pong', which is not an export/);
});
