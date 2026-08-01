const start = Date.now();
let total = 0;

for (let i = 0; i < 1000; i++) {
  total += i;
}

console.log("sum", total);
console.log("elapsed", Date.now() - start, "ms");
total;
