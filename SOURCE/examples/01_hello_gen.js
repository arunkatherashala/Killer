// Killer Language Runtime
function typeof__(value) {
  if (typeof value === 'boolean') return 'boolean';
  if (typeof value === 'number') return 'number';
  if (typeof value === 'string') return 'string';
  if (Array.isArray(value)) return 'array';
  if (value === null) return 'null';
  if (typeof value === 'object') return 'object';
  if (typeof value === 'function') return 'function';
  return 'object';
}

console.log("Welcome to Killer!");
let x = 10;
let y = 5;
console.log("x =", x);
console.log("y =", y);
console.log("x + y =", (x + y));
console.log("x - y =", (x - y));
console.log("x * y =", (x * y));
console.log("x / y =", (x / y));
console.log("x % y =", (x % y));
let greeting = "Hello ";
let name = "World";
console.log((greeting + name));
console.log("--- End of Example 1 ---");