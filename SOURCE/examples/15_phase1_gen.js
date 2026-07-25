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

console.log("=== ARROW FUNCTIONS ===");
let add = ((x, y) => (x + y));
console.log(add(10, 5));
let square = (x => (x * x));
console.log(square(7));
let greet = (() => "Hello Arrow Function!");
console.log(greet());
let process = (name => {...});
console.log(process("Data"));
let makeArray = (() => [1, 2, 3]);
console.log(makeArray());
console.log("=== TEMPLATE LITERALS ===");
let greeting = "World";
let msg1 = `Hello, ${greeting}!`;
console.log(msg1);
let x = 5;
let y = 3;
let msg2 = `${x} + ${y} = ${(x + y)}`;
console.log(msg2);
function getName() {
  return "Killer";
}

let msg3 = `Welcome to ${getName()} language!`;
console.log(msg3);
console.log("=== DEFAULT PARAMETERS ===");
function greetUser(name = "Guest") {
  return ("Hello, " + name);
}

console.log(greetUser());
console.log(greetUser("Alice"));
function area(width = 5, height = 3) {
  return (width * height);
}

console.log(area());
console.log(area(4));
console.log(area(10, 8));
console.log("=== COMBINATIONS ===");
let formatter = (val => `Value: ${val}`);
console.log(formatter(42));
let adder = ((a, b) => (a + b));
let subtractor = ((a, b) => (a - b));
let multiplier = ((a, b) => (a * b));
console.log(adder(10, 5));
console.log(subtractor(10, 5));
console.log(multiplier(10, 5));