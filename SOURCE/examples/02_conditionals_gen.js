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

let age = 20;
console.log("Testing conditionals...");
if ((age >= 18)) {
  console.log("You are an adult!");
}

let age = 10;
if ((age >= 18)) {
  console.log("Adult");
} else {
  console.log("Minor");
}

let num = 15;
if ((num > 20)) {
  console.log("Number is greater than 20");
} else {
  if ((num > 10)) {
    console.log("Number is between 10 and 20");
  } else {
    console.log("Number is 10 or less");
  }

}

console.log("--- End of Example 2 ---");