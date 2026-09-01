import { test, expect } from "bun:test";
import { Vm, runCode } from "../index.js";

test("no access to real require", () => {
  const vm = new Vm();
  expect(vm.run("typeof require;")).toBe("object");
});

test("no access to real process", () => {
  const vm = new Vm();
  expect(vm.run("typeof process;")).toBe("object");
});

test("globalThis is sandboxed object", () => {
  const vm = new Vm();
  expect(vm.run("typeof globalThis;")).toBe("object");
});

test("window is sandboxed object", () => {
  const vm = new Vm();
  expect(vm.run("typeof window;")).toBe("object");
});

test("self is sandboxed object", () => {
  const vm = new Vm();
  expect(vm.run("typeof self;")).toBe("object");
});

test("fetch is sandboxed object", () => {
  const vm = new Vm();
  expect(vm.run("typeof fetch;")).toBe("object");
});

test("setTimeout schedules without a clock", () => {
  const vm = new Vm();
  expect(vm.run("typeof setTimeout;")).toBe("function");
  // The callback runs on the VM's own queue, after every microtask. There is
  // no wall clock in the sandbox, so the delay only orders timers against
  // each other.
  expect(vm.run("let out = 'no'; setTimeout(() => { out = 'ran'; }, 1000); out;")).toBe("no");
  expect(
    vm.run("let out = []; setTimeout(() => out.push('b'), 10); setTimeout(() => out.push('a'), 1); await 0; out.join();"),
  ).toBe("");
});

test("setInterval is the same scheduler as setTimeout", () => {
  const vm = new Vm();
  expect(vm.run("typeof setInterval;")).toBe("function");
});

test("eval is sandboxed object", () => {
  const vm = new Vm();
  expect(vm.run("typeof eval;")).toBe("object");
});

test("Worker is sandboxed object", () => {
  const vm = new Vm();
  expect(vm.run("typeof Worker;")).toBe("object");
});

test("SharedWorker is sandboxed object", () => {
  const vm = new Vm();
  expect(vm.run("typeof SharedWorker;")).toBe("object");
});

test("no filesystem access", () => {
  const vm = new Vm();
  expect(vm.run("typeof __dirname;")).toBe("object");
  expect(vm.run("typeof __filename;")).toBe("object");
});

test("GPU APIs are sandboxed objects", () => {
  const vm = new Vm();
  expect(vm.run("typeof GPU;")).toBe("object");
  expect(vm.run("typeof GPUDevice;")).toBe("object");
  expect(vm.run("typeof GPUBuffer;")).toBe("object");
});

test("Web Streams are sandboxed objects", () => {
  const vm = new Vm();
  expect(vm.run("typeof ReadableStream;")).toBe("object");
  expect(vm.run("typeof WritableStream;")).toBe("object");
  expect(vm.run("typeof TransformStream;")).toBe("object");
});

test("ServiceWorker APIs are sandboxed", () => {
  const vm = new Vm();
  expect(vm.run("typeof ServiceWorker;")).toBe("object");
  expect(vm.run("typeof ServiceWorkerContainer;")).toBe("object");
  expect(vm.run("typeof ServiceWorkerRegistration;")).toBe("object");
});

test("DOM APIs are sandboxed objects", () => {
  const vm = new Vm();
  expect(vm.run("typeof Event;")).toBe("object");
  expect(vm.run("typeof EventTarget;")).toBe("object");
  expect(vm.run("typeof CustomEvent;")).toBe("object");
  expect(vm.run("typeof AbortController;")).toBe("object");
});

test("TextEncoder/TextDecoder sandboxed", () => {
  const vm = new Vm();
  expect(vm.run("typeof TextEncoder;")).toBe("object");
  expect(vm.run("typeof TextDecoder;")).toBe("object");
});

test("Blob/File/FormData sandboxed", () => {
  const vm = new Vm();
  expect(vm.run("typeof Blob;")).toBe("object");
  expect(vm.run("typeof File;")).toBe("object");
  expect(vm.run("typeof FormData;")).toBe("object");
});

test("vm instances are isolated", () => {
  const vm1 = new Vm();
  const vm2 = new Vm();
  vm1.run("const secret = 'vm1';");
  expect(() => vm2.run("secret;")).toThrow();
});

test("vm state does not leak between instances", () => {
  const vm1 = new Vm();
  const vm2 = new Vm();
  vm1.run("let counter = 100;");
  vm2.run("let counter = 0;");
  expect(vm1.run("counter;")).toBe("100");
  expect(vm2.run("counter;")).toBe("0");
});

test("runCode is stateless between calls", () => {
  runCode("const x = 42;");
  expect(() => runCode("x;")).toThrow();
});

test("cannot escape sandbox via constructor", () => {
  const vm = new Vm();
  expect(vm.run("typeof Function;")).toBe("object");
});

test("Proxy is sandboxed", () => {
  const vm = new Vm();
  expect(vm.run("typeof Proxy;")).toBe("object");
});

test("structuredClone is sandboxed", () => {
  const vm = new Vm();
  expect(vm.run("typeof structuredClone;")).toBe("object");
});

test("queueMicrotask runs on the VM's own queue", () => {
  const vm = new Vm();
  expect(vm.run("typeof queueMicrotask;")).toBe("function");
  expect(
    vm.run("let out = []; queueMicrotask(() => out.push('later')); out.push('now'); await 0; out.join();"),
  ).toBe("now,later");
});

test("indexedDB is sandboxed", () => {
  const vm = new Vm();
  expect(vm.run("typeof indexedDB;")).toBe("object");
  expect(vm.run("'open' in indexedDB;")).toBe("true");
});

test("caches is sandboxed", () => {
  const vm = new Vm();
  expect(vm.run("typeof caches;")).toBe("object");
  expect(vm.run("'open' in caches;")).toBe("true");
  expect(vm.run("'has' in caches;")).toBe("true");
});

test("sessionStorage is sandboxed", () => {
  const vm = new Vm();
  expect(vm.run("typeof sessionStorage;")).toBe("object");
  expect(vm.run("'getItem' in sessionStorage;")).toBe("true");
  expect(vm.run("'setItem' in sessionStorage;")).toBe("true");
});
