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

let numbers = [1, 2, 3, 4, 5];
let names = ["Alice", "Bob", "Charlie"];
console.log("First number:", numbers[0]);
console.log("Last name:", names[2]);
numbers[0] = 10;
names[1] = "Robert";
console.log("Modified array:", numbers);
console.log("Modified names:", names);
console.log("Loop through numbers:");
let i = 0;
while ((i < 5)) {
  console.log("  ", numbers[i]);
  let i = (i + 1);
}

let mixed = [1, "hello", true, 3.14];
console.log("Mixed array:", mixed);
console.log("--- Arrays work! ---");