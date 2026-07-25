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

console.log("=== SWITCH/CASE STATEMENTS ===");
function grade(score) {
  switch (score) {
    case 90:
      return "A";
    case 80:
      return "B";
    case 70:
      return "C";
    default:
      return "F";
  }

}

console.log(grade(90));
console.log(grade(80));
console.log(grade(70));
console.log(grade(50));
console.log("=== DO-WHILE LOOPS ===");
let x = 0;
do {
  console.log(x);
  x++;
} while ((x < 3));

console.log("=== STRING METHODS ===");
let s = "  hello world  ";
console.log(s.trim());
let words = "apple,banana,cherry".split(",");
console.log(words);
let txt = "hello world";
console.log(txt.replace("world", "killer"));
console.log("=== ARRAY METHODS ===");
function double(x) {
  return (x * 2);
}

function add_acc(acc, x) {
  return (acc + x);
}

function compare_nums(a, b) {
  return (a - b);
}

let nums = [1, 2, 3];
let doubled = nums.map(double);
console.log(doubled);
let sum = [1, 2, 3, 4].reduce(add_acc, 0);
console.log(sum);
let arr = ["a", "b", "c"];
console.log(arr.join("-"));
let unsorted = [3, 1, 4, 1, 5];
console.log(unsorted);
let arr2 = [1, 2, 3];
console.log(arr2);
let arr3 = [1, 2, 3, 4, 5];
let removed = arr3.splice(1, 2, 10, 20);
console.log("Removed:");
console.log(removed);
console.log("After splice:");
console.log(arr3);
console.log("=== ARRAY.isArray ===");
console.log(Array.isArray([1, 2, 3]));
console.log(Array.isArray("not an array"));
console.log(Array.isArray(42));