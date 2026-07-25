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

let name = "Hello World";
console.log(name.length);
console.log(name.toUpperCase());
console.log(name.toLowerCase());
console.log(name.charAt(0));
console.log(name.charAt(6));
console.log(name.substring(0, 5));
console.log(name.replace("World", "Killer"));
let text = "apple,banana,cherry";
let fruits = text.split(",");
for (let fruit of fruits) {
  console.log(fruit);
}

console.log("---");
let arr = [1, 2, 3];
console.log(arr.length);
console.log(arr.length);
let popped = arr.pop();
console.log(popped);
console.log(arr.length);
console.log("---");
let dict = {"name": "Alice", "age": 30, "city": "NYC"};
console.log(dict.length);
let keys = Object.keys(dict);
for (let k of keys) {
  console.log(k);
}
