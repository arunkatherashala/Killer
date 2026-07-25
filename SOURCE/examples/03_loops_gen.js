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

console.log("While loop counting from 0 to 4:");
let count = 0;
while ((count < 5)) {
  console.log(count);
  let count = (count + 1);
}

console.log("---");
console.log("Countdown from 5:");
let num = 5;
while ((num > 0)) {
  console.log(num);
  let num = (num - 1);
}

console.log("---");
console.log("Calculating 5 factorial:");
let n = 5;
let result = 1;
while ((n > 0)) {
  let result = (result * n);
  let n = (n - 1);
}

console.log("5! =", result);
console.log("--- End of Example 3 ---");