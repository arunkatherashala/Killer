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

console.log("============================");
console.log("KILLER LANGUAGE SHOWCASE");
console.log("============================");
console.log("");
console.log("--- Basic Types ---");
let age = 25;
let score = 99.5;
console.log(age);
console.log(score);