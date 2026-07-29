const { Vm, runCode } = require('./index.js');

console.log("=== Basic JS Execution ===");
console.log(runCode("const x = 42; x * 2;"));

console.log("\n=== Functions ===");
console.log(runCode("function add(a, b) { return a + b; } add(3, 4);"));

console.log("\n=== Arrow Functions ===");
console.log(runCode("const f = (x) => x * x; f(5);"));

console.log("\n=== Objects ===");
console.log(runCode("const obj = { name: 'Alice', age: 30 }; obj.name;"));

console.log("\n=== Arrays ===");
console.log(runCode("const arr = [1, 2, 3]; arr.length;"));

console.log("\n=== Loops ===");
console.log(runCode("let sum = 0; for (let i = 0; i < 10; i++) { sum += i; } sum;"));

console.log("\n=== Closures ===");
console.log(runCode("function counter() { let n = 0; return () => ++n; } const c = counter(); c(); c(); c();"));

console.log("\n=== import.meta ===");
const vm = new Vm();
console.log("main (default):", vm.run("import.meta.main;"));
vm.setImportMetaMain(true);
console.log("main (set):", vm.run("import.meta.main;"));
console.log("url:", vm.run("import.meta.url;"));

console.log("\n=== Try/Catch ===");
console.log(runCode("try { throw 'oops'; } catch(e) { 'caught: ' + e; }"));

console.log("\n=== Ternary ===");
console.log(runCode("true ? 'yes' : 'no';"));

console.log("\n=== While Loop ===");
console.log(runCode("let i = 0; while (i < 3) { i++; } i;"));

console.log("\n=== For...of ===");
console.log(runCode("let sum = 0; for (const x of [1, 2, 3, 4]) { sum += x; } sum;"));

console.log("\n=== Switch ===");
console.log(runCode("let r = ''; switch(2) { case 1: r = 'one'; break; case 2: r = 'two'; break; default: r = 'other'; } r;"));

console.log("\n=== Postfix/Prefix ===");
console.log(runCode("let i = 0; i++;"));
console.log(runCode("let i = 0; ++i;"));
console.log(runCode("let i = 0; i++; i;"));
console.log(runCode("let i = 0; ++i; i;"));

console.log("\n=== Compound Assignment ===");
console.log(runCode("let x = 5; x += 3; x;"));
console.log(runCode("let x = 10; x -= 4; x;"));
console.log(runCode("let x = 3; x *= 2; x;"));

console.log("\n=== Nested Functions ===");
console.log(runCode("function outer() { function inner() { return 42; } return inner(); } outer();"));

console.log("\n=== Recursion ===");
console.log(runCode("function factorial(n) { return n <= 1 ? 1 : n * factorial(n-1); } factorial(5);"));

console.log("\n=== Builtins ===");
console.log(runCode("Math.PI;"));
console.log(runCode("Math.E;"));
console.log(runCode("typeof 42;"));
console.log(runCode("typeof 'hello';"));

console.log("\n=== String concat ===");
console.log(runCode("'Hello, ' + 'World!';"));
console.log(runCode("const name = 'Alice'; 'Hello, ' + name + '!';"));
