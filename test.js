const { compileTs, compileJs } = require('./index.js');

const tsSource = `
interface User {
  name: string;
  age: number;
}

function greet(user: User): string {
  return \`Hello, \${user.name}! You are \${user.age} years old.\`;
}

const user: User = { name: "Alice", age: 30 };
console.log(greet(user));
`;

console.log("=== TypeScript Compilation ===");
const jsOutput = compileTs(tsSource);
console.log(jsOutput);

console.log("\n=== JavaScript Compilation ===");
const jsSource = `const x = 10; const y = 20; console.log(x + y);`;
const jsOutput2 = compileJs(jsSource);
console.log(jsOutput2);
