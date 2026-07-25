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

console.log("=== Killer Calculator ===");
let a = 10;
let b = 3;
console.log("Numbers:", a, b);
let result = (a + b);
console.log("Addition:", a, "+", b, "=", result);
let result = (a - b);
console.log("Subtraction:", a, "-", b, "=", result);
let result = (a * b);
console.log("Multiplication:", a, "*", b, "=", result);
let result = (a / b);
console.log("Division:", a, "/", b, "=", result);
let result = (a % b);
console.log("Modulo:", a, "%", b, "=", result);
console.log("---");
console.log("Comparing numbers:");
if ((a > b)) {
  console.log(a, "is greater than", b);
}

if ((a === b)) {
  console.log(a, "equals", b);
} else {
  console.log(a, "does not equal", b);
}

console.log("---");
console.log("Sum from 1 to 5:");
let sum_result = 0;
let num = 1;
while ((num <= 5)) {
  let sum_result = (sum_result + num);
  let num = (num + 1);
}

console.log("Sum:", sum_result);
console.log("--- End of Example 4 ---");