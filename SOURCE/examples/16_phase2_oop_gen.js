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

class Animal {
  constructor(name) {
    this.name = name;
  }

  speak() {
    return (this.name + " makes sound");
  }

  getName() {
    return this.name;
  }

}

class Dog extends Animal {
  constructor(name, breed) {
    this.breed = breed;
    this.name = name;
  }

  speak() {
    return (this.name + " says woof!");
  }

  getBreed() {
    return this.breed;
  }

  static getSpecies() {
    return "Canis lupus";
  }

  static compareBreeds(breed1, breed2) {
    return (breed1 === breed2);
  }

}

let dog = new Dog("Buddy", "Golden Retriever");
console.log(dog.speak());
console.log(dog.getName());
console.log(dog.getBreed());
console.log(Dog.getSpecies());
console.log(Dog.compareBreeds("Labrador", "Labrador"));
console.log(Dog.compareBreeds("Poodle", "Dachshund"));
class Rectangle {
  constructor(width, height) {
    this.width = width;
    this.height = height;
  }

  get area() {
    return (this.width * this.height);
  }

  get perimeter() {
    return (2 * (this.width + this.height));
  }

}

let rect = new Rectangle(5, 10);
console.log(rect.area);
console.log(rect.perimeter);
class Vehicle {
  constructor(type) {
    this.type = type;
  }

  describe() {
    return ("Vehicle type: " + this.type);
  }

  static getMaxSpeed() {
    return 200;
  }

}

class Car extends Vehicle {
  constructor(type, doors) {
    this.type = type;
    this.doors = doors;
  }

  describe() {
    return ((("Car with " + this.doors) + " doors, type: ") + this.type);
  }

  static getMaxSpeed() {
    return 300;
  }

}

let car = new Car("Sedan", 4);
console.log(car.describe());
console.log(Car.getMaxSpeed());