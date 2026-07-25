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

let person = {"name": "Alice", "age": 30, "city": "New York"};
console.log("Name:", person["name"]);
console.log("Age:", person["age"]);
console.log("City:", person["city"]);
person["age"] = 31;
person["job"] = "Engineer";
console.log("Updated age:", person["age"]);
console.log("New field:", person["job"]);
let empty = {};
console.log("Empty dict:", empty);
let data = {"count": 5, "active": true, "label": "test"};
console.log("Count:", data["count"]);
console.log("Active:", data["active"]);
console.log("Label:", data["label"]);
console.log("--- Dictionaries work! ---");