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

class Person {
  constructor(n, a) {
    this.name = n;
    this.age = a;
  }

  greet() {
    console.log("Hello, I am ");
    console.log(this.name);
  }

  getAge() {
    return this.age;
  }

}

let p = new Person("Alice", 30);
let age = p.getAge();
console.log("Age is: ");
console.log(age);
console.log("Direct access:");
console.log(p.name);
console.log(p.age);
p.age = 31;
console.log("Updated age:");
console.log(p.age);