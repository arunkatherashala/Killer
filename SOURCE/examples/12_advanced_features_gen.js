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

console.log("=== ADVANCED LANGUAGE FEATURES ===");
console.log("");
console.log("Test 1: Increment/Decrement");
console.log(((((("-" + "-") + "-") + "-") + "-") + "-"));
let x = 5;
console.log(("x = " + x));
console.log(x++);
console.log(("x = " + x));
let val = ++x;
console.log(("++x = " + val));
console.log(x--);
let val2 = --x;
console.log(("--x = " + val2));
console.log("");
console.log("Test 2: Compound Assignments");
console.log(((((("-" + "-") + "-") + "-") + "-") + "-"));
let a = 10;
console.log(("a = " + a));
a += 5;
console.log(("a += 5 -> " + a));
a -= 3;
console.log(("a -= 3 -> " + a));
a *= 2;
console.log(("a *= 2 -> " + a));
a /= 4;
console.log(("a /= 4 -> " + a));
a %= 3;
console.log(("a %= 3 -> " + a));
console.log("");
console.log("Test 3: Ternary Operator");
console.log(((((("-" + "-") + "-") + "-") + "-") + "-"));
let age = 25;
let status = ((age >= 18) ? "Adult" : "Minor");
console.log(((("Age " + age) + " -> ") + status));
let score = 75;
let grade = ((score >= 90) ? "A" : ((score >= 80) ? "B" : ((score >= 70) ? "C" : "F")));
console.log(((("Score " + score) + " -> Grade ") + grade));
let result = ((0 > 1) ? "true" : "false");
console.log(("0 > 1 ? true : false -> " + result));
console.log("");
console.log("Test 4: Logical AND (&&)");
console.log(((((("-" + "-") + "-") + "-") + "-") + "-"));
let p = true;
let q = false;
let r = true;
console.log(("true && false = " + ((p && q) ? "true" : "false")));
console.log(("true && true = " + ((p && r) ? "true" : "false")));
console.log(("false && true = " + ((q && r) ? "true" : "false")));
if ((false && true)) {
  console.log("This should not print (short-circuit prevented)");
}

if ((true && true)) {
  console.log("This should print (both true)");
}

console.log("");
console.log("Test 5: Logical OR (||)");
console.log(((((("-" + "-") + "-") + "-") + "-") + "-"));
console.log(("true || false = " + ((p || q) ? "true" : "false")));
console.log(("false || false = " + ((q || q) ? "true" : "false")));
console.log(("false || true = " + ((q || r) ? "true" : "false")));
if ((true || false)) {
  console.log("This should print (one is true)");
}

if ((false || false)) {
  console.log("This should not print (both false)");
}

console.log("");
console.log("Test 6: Break Statement");
console.log(((((("-" + "-") + "-") + "-") + "-") + "-"));
let i = 0;
while ((i < 10)) {
  if ((i === 5)) {
    break;
  }

  console.log(i);
  i++;
}

console.log(("Broke out when i = " + i));
console.log("");
console.log("Test 7: Continue Statement");
console.log(((((("-" + "-") + "-") + "-") + "-") + "-"));
for (let j = 0; j < 10; j += 1) {
  if (((j === 3) || (j === 7))) {
    continue;
  }

  console.log(j);
}

console.log("");
console.log("Test 8: Combination of Features");
console.log(((((("-" + "-") + "-") + "-") + "-") + "-"));
let numbers = [1, 2, 3, 4, 5];
let sum = 0;
for (let num of numbers) {
  sum += num;
}

console.log(("Sum of [1,2,3,4,5]: " + sum));
let product = 1;
for (let n of numbers) {
  product *= n;
}

console.log(("Product: " + product));
console.log("");
console.log("=== ALL TESTS PASSED ===");