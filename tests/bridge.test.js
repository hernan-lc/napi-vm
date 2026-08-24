import { test, expect } from "bun:test";
import { Vm } from "../index.js";

// --- Node -> VM: setGlobal -------------------------------------------------

test("setGlobal exposes a number as a bare global", () => {
  const vm = new Vm();
  vm.setGlobal("answer", 42);
  expect(vm.run("answer;")).toBe("42");
});

test("setGlobal exposes a string", () => {
  const vm = new Vm();
  vm.setGlobal("greeting", "hello");
  expect(vm.run("greeting + '!';")).toBe("hello!");
});

test("setGlobal exposes a boolean and null", () => {
  const vm = new Vm();
  vm.setGlobal("flag", true);
  vm.setGlobal("nothing", null);
  expect(vm.run("flag;")).toBe("true");
  expect(vm.run("nothing;")).toBe("null");
});

test("setGlobal exposes a structured object", () => {
  const vm = new Vm();
  vm.setGlobal("config", { n: 42, name: "vm", nested: { deep: 7 } });
  expect(vm.run("config.n;")).toBe("42");
  expect(vm.run("config.name;")).toBe("vm");
  expect(vm.run("config.nested.deep;")).toBe("7");
});

test("setGlobal exposes an array usable from the VM", () => {
  const vm = new Vm();
  vm.setGlobal("nums", [1, 2, 3, 4]);
  expect(vm.run("nums.length;")).toBe("4");
  expect(vm.run("nums[2];")).toBe("3");
  expect(vm.run("let s = 0; for (const x of nums) { s += x; } s;")).toBe("10");
});

// --- Node -> VM: exposeFunction (VM calls a Node function) -----------------

test("exposeFunction: VM calls a Node function with number args", () => {
  const vm = new Vm();
  vm.exposeFunction("add", (a, b) => a + b);
  expect(vm.run("add(2, 3);")).toBe("5");
  expect(vm.run("typeof add;")).toBe("function");
});

test("exposeFunction: string args and return", () => {
  const vm = new Vm();
  vm.exposeFunction("greet", (name) => "hi " + name);
  expect(vm.run("greet('bob');")).toBe("hi bob");
});

test("exposeFunction: Node function receives a VM array", () => {
  const vm = new Vm();
  vm.exposeFunction("sum", (arr) => arr.reduce((a, b) => a + b, 0));
  expect(vm.run("sum([1, 2, 3, 4]);")).toBe("10");
});

test("exposeFunction: Node function receives a VM object", () => {
  const vm = new Vm();
  vm.exposeFunction("area", (rect) => rect.w * rect.h);
  expect(vm.run("area({ w: 3, h: 4 });")).toBe("12");
});

test("exposeFunction: Node function returns an object to the VM", () => {
  const vm = new Vm();
  vm.exposeFunction("makePoint", () => ({ x: 1, y: 2 }));
  expect(vm.run("const p = makePoint(); p.x + p.y;")).toBe("3");
});

test("exposeFunction: a thrown JS error is catchable inside the VM", () => {
  const vm = new Vm();
  vm.exposeFunction("boom", () => {
    throw new Error("nope");
  });
  expect(vm.run("try { boom(); 'no-throw'; } catch (e) { 'caught:' + e.message; }")).toBe(
    "caught:nope",
  );
});

test("exposeFunction rejects a non-function", () => {
  const vm = new Vm();
  expect(() => vm.exposeFunction("bad", 123)).toThrow(/must be a function/);
});

// --- VM -> Node: callFunction (Node calls a VM-defined function) -----------

// Arguments are passed as a single array (napi-rs maps the Rust `Vec<Unknown>`
// parameter to one JS array argument).

test("callFunction invokes a VM function and returns a number", () => {
  const vm = new Vm();
  vm.run("function mul(a, b) { return a * b; }");
  expect(vm.callFunction("mul", [6, 7])).toBe(42);
});

test("callFunction returns a structured object", () => {
  const vm = new Vm();
  vm.run("function point(x, y) { return { x: x, y: y }; }");
  const p = vm.callFunction("point", [5, 6]);
  expect(p).toEqual({ x: 5, y: 6 });
});

test("callFunction returns an array", () => {
  const vm = new Vm();
  vm.run("function range(n) { const a = []; for (let i = 0; i < n; i++) { a.push(i); } return a; }");
  expect(vm.callFunction("range", [4])).toEqual([0, 1, 2, 3]);
});

test("callFunction returns a string", () => {
  const vm = new Vm();
  vm.run("function shout(s) { return s + '!'; }");
  expect(vm.callFunction("shout", ["hey"])).toBe("hey!");
});

test("callFunction can call an exposed Node function indirectly", () => {
  const vm = new Vm();
  vm.exposeFunction("double", (n) => n * 2);
  vm.run("function useDouble(x) { return double(x) + 1; }");
  expect(vm.callFunction("useDouble", [10])).toBe(21);
});

test("callFunction on an undefined name throws", () => {
  const vm = new Vm();
  expect(() => vm.callFunction("doesNotExist", [])).toThrow(/not defined/);
});

// --- window / globalThis / self alias the global scope ---------------------

test("the global aliases are objects and all identical", () => {
  const vm = new Vm();
  expect(vm.run("typeof window;")).toBe("object");
  expect(vm.run("typeof globalThis;")).toBe("object");
  expect(vm.run("window === globalThis;")).toBe("true");
  expect(vm.run("window === self;")).toBe("true");
  expect(vm.run("globalThis === self;")).toBe("true");
  expect(vm.run("window.window === window;")).toBe("true");
});

test("setGlobal is reachable via window and globalThis", () => {
  const vm = new Vm();
  vm.setGlobal("cfg", { n: 1 });
  expect(vm.run("window.cfg.n;")).toBe("1");
  expect(vm.run("globalThis.cfg.n;")).toBe("1");
  expect(vm.run("self.cfg.n;")).toBe("1");
});

test("exposeFunction is callable via window", () => {
  const vm = new Vm();
  vm.exposeFunction("triple", (n) => n * 3);
  expect(vm.run("window.triple(5);")).toBe("15");
  expect(vm.run("globalThis.triple(4);")).toBe("12");
});

test("guest object keys are own data properties at the N-API boundary", () => {
  const payload = { own: 1 };
  Object.defineProperty(payload, "__proto__", {
    value: { polluted: true },
    enumerable: true,
    writable: true,
    configurable: true,
  });
  const vm = new Vm();
  vm.setGlobal("payload", payload);
  expect(vm.run("payload.own;" )).toBe("1");
  expect(vm.run("payload.__proto__.polluted;" )).toBe("true");
  expect(vm.run("payload.polluted;" )).toBe("undefined");
});

test("runAsync dispatches an ordinary exposed function on Node's main thread", async () => {
  const vm = new Vm();
  vm.exposeFunction("add", (a, b) => a + b);
  await expect(vm.runAsync("add(1, 2);" )).resolves.toBe("3");
});

test("runAsync rejects overlapping operations on one VM", async () => {
  const vm = new Vm();
  vm.exposeAsyncFunction("wait", () => new Promise((resolve) => setTimeout(() => resolve(1), 10)));
  const running = vm.runAsync("await wait();" );
  expect(() => vm.runAsync("1;" )).toThrow(/busy/i);
  await expect(running).resolves.toBe("1");
});

test("runAsync owns the VM state until a dropped VM's worker finishes", async () => {
  let vm = new Vm();
  const running = vm.runAsync("let a = [0]; for (let i = 0; i < 10000; i++) { a = [a]; } 'done';" );
  vm = null;
  await expect(running).resolves.toBe("done");
});

test("async host arguments are deep-copied before the VM continues", async () => {
  const vm = new Vm();
  vm.setGlobal("obj", { n: 1 });
  vm.exposeAsyncFunction(
    "read",
    (value) => new Promise((resolve) => setTimeout(() => resolve(value.n), 10)),
  );
  await expect(
    vm.runAsync("let value = obj; let pending = read(value); value.n = 2; await pending;" ),
  ).resolves.toBe("1");
});

test("host thenables settle only once", async () => {
  const vm = new Vm();
  vm.exposeAsyncFunction("badThenable", () => ({
    then(resolve, reject) {
      resolve(7);
      resolve(8);
      reject(new Error("late rejection"));
    },
  }));
  await expect(vm.runAsync("await badThenable();" )).resolves.toBe("7");
});

test("writing via window defines a real global", () => {
  const vm = new Vm();
  expect(vm.run("window.foo = 99; foo;")).toBe("99");
  expect(vm.run("globalThis.bar = 'x'; window.bar;")).toBe("x");
});

test("top-level declarations are visible on window", () => {
  const vm = new Vm();
  expect(vm.run("function baz() { return 7; } window.baz();")).toBe("7");
  expect(vm.run("window.Math.PI;")).toBe("3.141592653589793");
});
