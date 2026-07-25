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

try {
  let x = (10 / 0);
  console.log("This should not print");
} catch (e) {
  let error = e.message;
  console.log("Caught error:");
  console.log(error);
}

console.log("---");
console.log("Program continues after error");
console.log("---");
try {
  let arr = [1, 2, 3];
  console.log(arr[10]);
} catch (e) {
  let err = e.message;
  console.log("Array error:");
  console.log(err);
}

console.log("---");
try {
  let y = (10 / 2);
  console.log("y =");
  console.log(y);
} catch (e) {
  console.log("This catch should not execute");
}

console.log("Done");