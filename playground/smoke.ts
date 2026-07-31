// Temporary headless smoke test: drives the browser `WasmVm` under Bun to prove
// the wasm executes correctly (run, console capture, exposed fns, modules, and
// the shared language services). Not part of the shipped playground.
import { readFileSync } from "node:fs";
import { join } from "node:path";
// @ts-expect-error - wasm-bindgen glue has no bundled types for this import style
import init, { WasmVm } from "./pkg/napi_vm.js";

const bytes = readFileSync(join(import.meta.dir, "pkg", "napi_vm_bg.wasm"));
await init(bytes);

let failures = 0;
function check(name: string, cond: boolean, extra = "") {
  if (cond) console.log(`  ok   ${name}`);
  else {
    failures++;
    console.log(`  FAIL ${name} ${extra}`);
  }
}

const vm = new WasmVm();
vm.set_loop_limit(5_000_000);

// 1. Basic run + result value.
let r = vm.run("1 + 2");
check("run returns ok", r.ok === true, JSON.stringify(r));
check("run value is 3", r.value === "3", `got ${JSON.stringify(r.value)}`);

// 2. Console capture.
r = vm.run('console.log("hello", 42); console.warn("careful"); "done"');
check("two logs captured", (r.logs?.length ?? 0) === 2, JSON.stringify(r.logs));
check("log text formatted", r.logs?.[0]?.text === "hello 42", JSON.stringify(r.logs?.[0]));
check("warn level", r.logs?.[1]?.level === "warn", JSON.stringify(r.logs?.[1]));
// Nested strings stay quoted for readability.
r = vm.run('console.log({ name: "alice" }); 1');
check("nested string quoted", r.logs?.[0]?.text === '{ name: "alice" }', JSON.stringify(r.logs?.[0]));

// 3. Error path.
r = vm.run("throw new Error('boom')");
check("throw is not ok", r.ok === false);
check("error message present", String(r.error).includes("boom"), String(r.error));

// 4. Loop cap.
r = vm.run("while (true) {}");
check("infinite loop caught", r.ok === false && String(r.error).includes("loop"), String(r.error));

// 5. Exposed function (host callback) + its completion.
let alerted = "";
vm.expose_function("alert", (m: string) => {
  alerted = String(m);
  return undefined;
});
r = vm.run('alert("hi from vm")');
check("exposed fn called", alerted === "hi from vm", `alerted=${alerted}`);
let comp = vm.complete("al", 2);
check(
  "exposed fn completes",
  comp.some((c: any) => c.label === "alert" && c.kind === "exposed"),
  JSON.stringify(comp),
);

// 6. Registered module + namespace-export completion.
vm.register_module("utils", "export function double(x){return x*2} export const VERSION='1.0';");
r = vm.run('import * as u from "utils"; u.double(21)');
check("module import runs", r.ok === true && r.value === "42", JSON.stringify(r));
comp = vm.complete('import * as u from "utils";\nu.', 'import * as u from "utils";\nu.'.length);
check(
  "namespace exports complete",
  comp.some((c: any) => c.label === "double") && comp.some((c: any) => c.label === "VERSION"),
  JSON.stringify(comp),
);

// 7. Member completion on a builtin.
comp = vm.complete("Math.fl", "Math.fl".length);
check("Math.floor completes", comp.some((c: any) => c.label === "floor"), JSON.stringify(comp));

// 8. Identifier completion offers globals + scope decls.
const src = "const counter = 1;\nfunction bump() {}\n";
comp = vm.complete(src, src.length);
check(
  "scope decls complete",
  comp.some((c: any) => c.label === "counter") && comp.some((c: any) => c.label === "bump"),
);
comp = vm.complete("Ma", 2);
check("global completes", comp.some((c: any) => c.label === "Math"));

// 9. Diagnostics.
let diag = vm.diagnose("const x = [1, 2;");
check("unbalanced diagnosed", (diag?.length ?? 0) > 0, JSON.stringify(diag));
diag = vm.diagnose("const x = [1, 2];");
check("balanced clean", (diag?.length ?? 0) === 0, JSON.stringify(diag));

// 10. Symbols.
const syms = vm.symbols("function f(){} class C{} const x = 1;");
check(
  "symbols found",
  syms.some((s: any) => s.name === "f") &&
    syms.some((s: any) => s.name === "C") &&
    syms.some((s: any) => s.name === "x"),
  JSON.stringify(syms),
);

// 11. Reset clears state. (`to_string` renders strings unquoted, so the
// `typeof` result arrives as the bare text `undefined`.)
vm.reset();
r = vm.run("typeof counter");
check("reset clears globals", r.ok === true && r.value === "undefined", JSON.stringify(r));

console.log(failures === 0 ? "\nALL PASSED" : `\n${failures} FAILURE(S)`);
process.exit(failures === 0 ? 0 : 1);
