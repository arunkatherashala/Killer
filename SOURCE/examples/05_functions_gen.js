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

function add(a, b) {
  return (a + b);
}

function greet(name) {
  console.log("Hello, ", name);
}

function factorial(n) {
  if ((n <= 1)) {
    return 1;
  } else {
    return (n * factorial((n - 1)));
  }

}

let result = add(5, 3);
console.log("5 + 3 =", result);
let fac5 = factorial(5);
console.log("5! =", fac5);
console.log("--- Functions work! ---");