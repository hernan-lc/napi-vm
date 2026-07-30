import { Vm } from "../index";
import { readFileSync, watch, readdirSync } from "node:fs";
import { join, basename } from "node:path";

const MODULES_DIR = join(import.meta.dir, "callbacks", "modules");

interface CallbackEntry {
  name: string;
  file: string;
  lastModified: number;
  status: "active" | "error" | "stale";
  error?: string;
}

const registry = new Map<string, CallbackEntry>();

const BLOCKED_PATTERNS = [
  /\bwhile\s*\(\s*true\s*\)/,
  /\bfor\s*\(\s*;\s*;\s*\)/,
  /\beval\s*\(/,
  /\bFunction\s*\(/,
  /\bsetTimeout\s*\(/,
  /\bsetInterval\s*\(/,
  /\bimportScripts\s*\(/,
  /\bprocess\s*\./,
  /\brequire\s*\(/,
  /\b__dirname\b/,
  /\b__filename\b/,
];

function validateCode(source: string, moduleName: string): string[] {
  const errors: string[] = [];
  const lines = source.split("\n");

  for (let i = 0; i < BLOCKED_PATTERNS.length; i++) {
    const pattern = BLOCKED_PATTERNS[i];
    for (let j = 0; j < lines.length; j++) {
      if (pattern.test(lines[j])) {
        errors.push(
          `Line ${j + 1}: blocked pattern '${pattern.source}' found`
        );
      }
    }
  }

  if (source.length === 0) {
    errors.push("Module source is empty");
  }

  if (!source.includes("export")) {
    errors.push("Module must export at least one symbol");
  }

  return errors;
}

function readModules(): Map<string, string> {
  const files = readdirSync(MODULES_DIR).filter((f) => f.endsWith(".js"));
  const sources = new Map<string, string>();
  for (const file of files) {
    const name = basename(file, ".js");
    const source = readFileSync(join(MODULES_DIR, file), "utf-8");
    sources.set(name, source);
  }
  return sources;
}

function loadModule(vm: Vm, name: string, source: string): CallbackEntry {
  const errors = validateCode(source, name);
  if (errors.length > 0) {
    const entry: CallbackEntry = {
      name,
      file: `${name}.js`,
      lastModified: Date.now(),
      status: "error",
      error: errors.join("; "),
    };
    registry.set(name, entry);
    console.log(`  [ERROR] ${name}: ${entry.error}`);
    return entry;
  }

  try {
    vm.registerModule(name, source);
    const entry: CallbackEntry = {
      name,
      file: `${name}.js`,
      lastModified: Date.now(),
      status: "active",
    };
    registry.set(name, entry);
    console.log(`  [OK] ${name} loaded`);
    return entry;
  } catch (err: any) {
    const entry: CallbackEntry = {
      name,
      file: `${name}.js`,
      lastModified: Date.now(),
      status: "error",
      error: err.message || String(err),
    };
    registry.set(name, entry);
    console.log(`  [ERROR] ${name}: ${entry.error}`);
    return entry;
  }
}

function createVmWithCallbacks(): Vm {
  const vm = new Vm();
  const sources = readModules();

  const sorted = [...sources.entries()].sort(([a], [b]) => {
    if (a === "utils") return -1;
    if (b === "utils") return 1;
    return 0;
  });

  for (const [name, source] of sorted) {
    loadModule(vm, name, source);
  }

  vm.exposeFunction("hostLog", (...args: any[]) => {
    console.log("[host]", ...args);
  });

  vm.exposeFunction("hostNow", () => Date.now());

  vm.exposeFunction("hostJson", (value: unknown) => JSON.stringify(value));

  vm.run(`
    import { greet, farewell, announce } from "greet";
    import { add, multiply, factorial, fib, clampValue } from "math";
    import { capitalize, reverse, repeat, slugify, wordCount } from "transform";

    function dispatch(name, args) {
      if (name === "greet") return greet(args[0]);
      if (name === "farewell") return farewell(args[0]);
      if (name === "announce") return announce(args[0], args[1]);
      if (name === "add") return add(args[0], args[1]);
      if (name === "multiply") return multiply(args[0], args[1]);
      if (name === "factorial") return factorial(args[0]);
      if (name === "fib") return fib(args[0]);
      if (name === "clampValue") return clampValue(args[0], args[1], args[2]);
      if (name === "capitalize") return capitalize(args[0]);
      if (name === "reverse") return reverse(args[0]);
      if (name === "repeat") return repeat(args[0], args[1]);
      if (name === "slugify") return slugify(args[0]);
      if (name === "wordCount") return wordCount(args[0]);
      throw new Error("Unknown callback: " + name);
    }

    function dispatchToJson(name, args) {
      const result = dispatch(name, args);
      return JSON.stringify({ ok: true, callback: name, result: result });
    }
  `);

  return vm;
}

function runAllCallbacks(vm: Vm): void {
  console.log("--- Running All Callbacks ---\n");

  const calls = [
    { name: "greet", args: ["Alice"] },
    { name: "farewell", args: ["Bob"] },
    { name: "announce", args: ["Server is starting", "system"] },
    { name: "add", args: [10, 20] },
    { name: "multiply", args: [6, 7] },
    { name: "factorial", args: [5] },
    { name: "fib", args: [10] },
    { name: "clampValue", args: [150, 0, 100] },
    { name: "capitalize", args: ["hello world"] },
    { name: "reverse", args: ["abcdef"] },
    { name: "repeat", args: ["ha", 3] },
    { name: "slugify", args: ["Hello World!  --  Foo Bar"] },
    { name: "wordCount", args: ["  the quick  brown fox  "] },
  ];

  for (const call of calls) {
    const argsStr = JSON.stringify(call.args);
    const result = vm.run(`dispatchToJson("${call.name}", ${argsStr})`);
    console.log(`  ${call.name}(${call.args.join(", ")}) => ${result}`);
  }
  console.log("");
}

function testDispatch(vm: Vm): void {
  console.log("--- Dispatch Test ---\n");

  const calls = [
    { name: "greet", args: ["Charlie"] },
    { name: "add", args: [100, 200] },
    { name: "factorial", args: [7] },
    { name: "slugify", args: ["Hot Reload Is Working!"] },
    { name: "fib", args: [15] },
    { name: "wordCount", args: ["hello world foo bar baz"] },
  ];

  for (const call of calls) {
    const argsStr = JSON.stringify(call.args);
    const result = vm.run(`dispatchToJson("${call.name}", ${argsStr})`);
    console.log(`  ${call.name}(${call.args.join(", ")}) => ${result}`);
  }
  console.log("");
}

function printRegistry(): void {
  console.log("--- Callback Registry ---\n");
  for (const [name, entry] of registry) {
    const status = entry.status === "active" ? "+" : "x";
    const err = entry.error ? ` (${entry.error})` : "";
    console.log(`  [${status}] ${name} => ${entry.file}${err}`);
  }
  console.log("");
}

function setupHotReload(): void {
  console.log("--- Hot Reload Watcher ---\n");
  console.log(`Watching: ${MODULES_DIR}\n`);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  const reload = (changedFile: string) => {
    if (!changedFile.endsWith(".js")) return;

    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      console.log(`[hot-reload] Change detected: ${changedFile}`);
      console.log("[hot-reload] Rebuilding VM...\n");

      const vm = createVmWithCallbacks();
      runAllCallbacks(vm);
      testDispatch(vm);
      printRegistry();
      console.log("Waiting for changes...\n");
    }, 100);
  };

  watch(MODULES_DIR, (eventType, filename) => {
    if (filename) reload(filename);
  });

  console.log("Hot reload active. Edit a module file to see changes.");
  console.log("Press Ctrl+C to stop.\n");
}

console.log("=== Node-VM Callback System ===\n");
const vm = createVmWithCallbacks();
runAllCallbacks(vm);
testDispatch(vm);
printRegistry();
setupHotReload();
