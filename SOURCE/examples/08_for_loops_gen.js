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

for (let i = 0; i < 5; i += 1) {
  console.log(i);
}

console.log("---");
let arr = [10, 20, 30, 40];
for (let x of arr) {
  console.log(x);
}

console.log("---");
for (let j = 0; j < 10; j += 2) {
  console.log(j);
}
