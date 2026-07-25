# Killer Language - Feature Roadmap
## v3.0 - v5.0+ Development Path

### Current Status

| Version | Status | Focus |
|---------|--------|-------|
| **v2.0** | ✅ Complete | Python Interpreter (all core features) |
| **v2.5** | ✅ Complete | Self-Hosted Interpreter (1650+ lines pure Killer) |
| **v3.0** | ⏳ In Progress | Standalone Executable (Phase 2 - C Compilation) |
| **v3.1** | 📋 Planned | Module System & Ecosystem |
| **v4.0** | 📋 Planned | Advanced OOP & Type System |
| **v5.0** | 📋 Planned | Async/Await & Advanced Features |

---

## v3.0: Standalone Executable (Phase 2)
**Timeline**: March 8-22, 2026  
**Goal**: Zero Python dependency - Killer compiles to native binary

### Core Features (Implement)
- ✅ Lexer.killer - Tokenization engine
- ✅ Parser.killer - AST builder
- ✅ Interpreter.killer - Execution engine
- ⏳ Code Generator - AST to C code
- ⏳ Runtime Library - C runtime support
- ⏳ Bootstrap Compiler - Killer → C → executable
- ⏳ Installer Scripts - Windows/macOS/Linux

### Deliverables
```
killer.exe               [Windows executable]
killer                   [macOS/Linux executable]
killer-standalone-installer.bat  [Windows installer]
killer-standalone-installer.sh   [Unix installer]
STANDALONE_INSTALLER_GUIDE.md    [User guide]
```

### Success Criteria
- [x] All 16 examples run on standalone binary (offline test)
- [ ] Killer compiles to C code successfully
- [ ] killer.exe / killer binary created and tested
- [ ] Zero Python dependency in final executable
- [ ] File size < 5MB after optimization
- [ ] Works on Windows 10/11, macOS, Linux

---

## v3.1: Module System & Basic Ecosystem
**Timeline**: March 23 - April 6, 2026 (2 weeks after v3.0)  
**Goal**: Enable code reuse and project organization

### 1. Module System (Import/Export)

#### Syntax
```killer
# lib.killer - Library file
export fn helper(x) {
    return x * 2
}

export class Math {
    fn add(a, b) {
        return a + b
    }
}

# main.killer - Using the library
import { helper, Math } from "lib.killer"

result = helper(5)
calc = Math.add(10, 3)
```

#### Implementation
- **Parser**: Handle `import` and `export` statements
- **Resolver**: Track exported symbols
- **Loader**: Load external `.killer` files
- **Scoping**: Isolated module scope with explicit exports

#### Key Files
```
lib/
  ├── math.killer
  ├── string.killer
  ├── array.killer
  └── io.killer

main.killer   [imports from lib/]
```

---

### 2. Package Manager (Basic)

#### CLI Commands
```bash
killer init              # Create new project
killer add math          # Add package to project
killer install           # Install dependencies
killer remove math       # Remove package
killer list              # List installed packages
```

#### Package Manager Structure
```
.killer/
  ├── killer.toml        [Package manifest]
  ├── packages/          [Installed packages]
  │   ├── math/
  │   ├── string/
  │   └── ...
  └── lock.killer        [Dependency lock file]
```

#### Example killer.toml
```toml
[package]
name = "my-project"
version = "1.0.0"
author = "Your Name"
license = "MIT"

[dependencies]
math = "1.0.0"
string = "2.1.0"
```

---

### 3. Standard Library Modules (Core Set)

#### math.killer
```killer
export const PI = 3.14159
export const E = 2.71828

export fn sqrt(x) { return x ** 0.5 }
export fn pow(x, n) { return x ** n }
export fn abs(x) { return x < 0 ? -x : x }
export fn sin(x) { ... }
export fn cos(x) { ... }
export fn tan(x) { ... }
export fn random() { ... }
```

#### string.killer
```killer
export fn toUpperCase(s) { ... }
export fn toLowerCase(s) { ... }
export fn trim(s) { ... }
export fn split(s, delim) { ... }
export fn replace(s, find, replace) { ... }
export fn capitalize(s) { ... }
export fn reverse(s) { ... }
export fn repeat(s, n) { ... }
```

#### array.killer
```killer
export fn map(arr, fn) { ... }
export fn filter(arr, fn) { ... }
export fn reduce(arr, fn, init) { ... }
export fn find(arr, fn) { ... }
export fn some(arr, fn) { ... }
export fn every(arr, fn) { ... }
export fn reverse(arr) { ... }
export fn sort(arr, fn) { ... }
export fn unique(arr) { ... }
export fn flatten(arr) { ... }
export fn zip(arr1, arr2) { ... }
```

#### io.killer (File I/O)
```killer
export fn readFile(path) { ... }
export fn writeFile(path, content) { ... }
export fn appendFile(path, content) { ... }
export fn deleteFile(path) { ... }
export fn listFiles(dir) { ... }
export fn mkdir(dir) { ... }
export fn exists(path) { ... }
```

#### http.killer (Network - optional)
```killer
export fn get(url) { ... }
export fn post(url, data) { ... }
export fn put(url, data) { ... }
export fn delete(url) { ... }
```

#### json.killer (Serialization)
```killer
export fn stringify(obj) { ... }
export fn parse(str) { ... }
```

---

### 4. REPL (Interactive Shell)

#### Usage
```bash
$ killer
Killer v3.1 Interactive Shell
Type 'exit' to quit, 'help' for commands

> x = 10
undefined
> y = 20
undefined
> x + y
30
> fn add(a, b) { return a + b }
undefined
> add(5, 3)
8
> exit
$
```

#### Features
- Command history (↑/↓ arrows)
- Auto-completion (Tab)
- Help system
- Multi-line input (for functions/classes)
- Pretty-print output

---

### 5. Build Tool (Basic)

#### killer build command
```bash
killer build                    # Compile project
killer build --output app.exe   # Custom output name
killer build --optimize         # Optimize for size
killer build --debug            # Include debug symbols
```

#### killer.toml build config
```toml
[build]
entry = "main.killer"
output = "app.exe"
optimize = true
strip = true

[build.platforms]
windows = true
macos = true
linux = true
```

---

## v4.0: Advanced OOP & Type System
**Timeline**: April 7 - May 5, 2026 (4 weeks)  
**Goal**: Professional-grade object-oriented programming

### 1. Abstract Classes & Interfaces

#### Abstract Classes
```killer
abstract class Animal {
    abstract fn speak()
    
    fn move() {
        print("Moving...")
    }
}

class Dog extends Animal {
    fn speak() {
        print("Woof!")
    }
}

# Error: Cannot instantiate abstract class
# dog = new Animal()
```

#### Interfaces
```killer
interface Drawable {
    fn draw()
    fn erase()
}

class Circle implements Drawable {
    fn draw() { print("Drawing circle") }
    fn erase() { print("Erasing circle") }
}
```

#### Implementation
- Type checking: Warn if abstract methods not implemented
- Runtime validation: Block instantiation of abstract classes
- Interface compliance: Check method signatures match

---

### 2. Mixins & Traits

#### Mixin Definition
```killer
mixin Serializable {
    fn toJSON() {
        # Automatically serialize all properties
        result = {}
        for (key in self) {
            result[key] = self[key]
        }
        return result
    }
    
    fn toString() {
        return JSON.stringify(self.toJSON())
    }
}

class User with Serializable {
    fn constructor(name, email) {
        self.name = name
        self.email = email
    }
}

user = new User("Alice", "alice@example.com")
print(user.toString())  # Uses mixin method
```

#### Benefits
- Code reuse without inheritance
- Multiple mixins per class
- Method composition
- Avoids diamond problem

---

### 3. Advanced Generics

#### Generic Classes
```killer
class Box<T> {
    fn constructor(value) {
        self.value = value
    }
    
    fn getValue() {
        return self.value
    }
    
    fn setValue(value) {
        self.value = value
    }
}

# Usage
intBox = new Box<number>(42)
strBox = new Box<string>("Hello")
```

#### Generic Functions
```killer
fn identity<T>(x: T): T {
    return x
}

fn swap<T>(a: T, b: T): [T, T] {
    return [b, a]
}
```

#### Generic Constraints
```killer
fn sum<T extends number[]>(arr: T): number {
    result = 0
    for (x in arr) {
        result = result + x
    }
    return result
}
```

---

### 4. Type Annotations (Optional)

#### Gradual Typing
```killer
# With type annotations
fn add(a: number, b: number): number {
    return a + b
}

# Without - still works
fn multiply(x, y) {
    return x * y
}

# Class properties
class Person {
    name: string
    age: number
    email: string
    
    fn constructor(name: string, age: number) {
        self.name = name
        self.age = age
    }
}
```

#### Type Checking
- Optional: Enable with flag `killer --strict`
- Runtime assertions: Check types at execution
- IDE support: Enable autocomplete and error checking

---

### 5. Property Decorators

#### Decorator Syntax
```killer
@readonly
class Config {
    setting1 = "value"
    setting2 = 42
}

@deprecated("Use newFunction instead")
fn oldFunction() { ... }

class User {
    @validate(isEmail)
    email: string
    
    @validate(isAge)
    age: number
}
```

#### Built-in Decorators
- `@readonly` - Prevent property modification
- `@deprecated(message)` - Mark as obsolete
- `@validate(fn)` - Add validation
- `@memoize` - Cache function results
- `@throttle(ms)` - Limit call frequency
- `@debounce(ms)` - Delay execution

---

## v5.0: Async & Advanced Features
**Timeline**: May 6 - June 2, 2026 (4 weeks)  
**Goal**: Modern programming paradigms

### 1. Async/Await

#### Promise Support
```killer
# Function returning promise
async fn fetchData(url) {
    # This would normally use HTTP library
    result = await http.get(url)
    return result
}

async fn main() {
    try {
        data = await fetchData("https://api.example.com/data")
        print(data)
    } catch (err) {
        print("Error: ", err)
    }
}
```

#### Promise Chaining
```killer
fetchData(url)
    .then(fn(data) {
        print("Received:", data)
    })
    .catch(fn(err) {
        print("Error:", err)
    })
    .finally(fn() {
        print("Done")
    })
```

#### Implementation
- Promise object: Pending → Resolved/Rejected
- Async dispatcher: Event loop handling
- Error propagation: Catch promise rejections

---

### 2. Pattern Matching

#### Match Expressions
```killer
fn classify(value) {
    match (value) {
        case 0 => print("Zero")
        case 1..10 => print("Small number")
        case 10..100 => print("Medium number")
        case _ => print("Large number")
    }
}

# Destructuring in pattern
fn processArray(arr) {
    match (arr) {
        case [] => print("Empty array")
        case [x] => print("Single element:", x)
        case [x, y] => print("Two elements:", x, y)
        case [x, ...rest] => print("First:", x, "Rest:", rest)
    }
}
```

#### Guard Clauses
```killer
fn validate(user) {
    match (user) {
        case {age} if age < 18 => print("Minor")
        case {age} if age >= 18 && age < 65 => print("Adult")
        case {age} if age >= 65 => print("Senior")
        case _ => print("Invalid")
    }
}
```

---

### 3. Metaclasses & Reflection

#### Reflection API
```killer
class MyClass {
    fn method1() {}
    fn method2(x, y) {}
}

obj = new MyClass()

# Get class info
methods = obj.__class__.getMethods()
properties = obj.__class__.getProperties()
superClass = obj.__class__.getSuperClass()

# Introspection
for (method in methods) {
    print("Method:", method.name, "params:", method.params.length)
}
```

#### Dynamic Behavior
```killer
# Add method at runtime
MyClass.addMethod("newMethod", fn() {
    return "Created dynamically"
})

# Modify property behavior
MyClass.defineProperty("computed", {
    get: fn() { return self.x + self.y },
    set: fn(value) { self.x = value }
})
```

---

### 4. Decorators for Methods

#### Method Decorators
```killer
class Database {
    @cached
    fn getUser(id: number) {
        # This method's result will be cached
        return this.query("SELECT * FROM users WHERE id = " + id)
    }
    
    @logged
    fn saveUser(user) {
        # This method call will be logged
        return this.insert("users", user)
    }
    
    @throttle(1000)
    fn syncData() {
        # This method can only be called once per second
        return this.sync()
    }
}
```

#### Creating Custom Decorators
```killer
fn benchmark(fn) {
    return fn(...args) {
        start = Date.now()
        result = fn(...args)
        elapsed = Date.now() - start
        print("Execution time:", elapsed, "ms")
        return result
    }
}
```

---

### 5. Error Handling Enhancements

#### Custom Error Types
```killer
class ValidationError extends Error {
    fn constructor(message, field) {
        super(message)
        self.field = field
    }
}

try {
    if (email.length == 0) {
        throw new ValidationError("Email is required", "email")
    }
} catch (err) {
    if (err instanceof ValidationError) {
        print("Validation failed on field:", err.field)
    }
}
```

#### Stack Traces
```killer
try {
    fn1()
} catch (err) {
    print("Error:", err.message)
    print("Stack trace:")
    for (frame in err.stack) {
        print("  at", frame.function, "in", frame.file, ":", frame.line)
    }
}
```

---

### 6. Additional Features

#### Set & Map Collections
```killer
# Set - unique values
mySet = new Set([1, 2, 3, 2, 1])
print(mySet.size)  # 3
mySet.add(4)
mySet.has(2)  # true

# Map - key-value pairs
myMap = new Map()
myMap.set("name", "Alice")
myMap.get("name")  # "Alice"
myMap.delete("name")
```

#### Symbol Type
```killer
const id = Symbol("id")
const user = {
    [id]: 12345,
    name: "Bob"
}

# Symbols can't be accessed via normal property access
user[id]  # 12345
for (key in user) {
    # Symbols won't appear here
    print(key)  # Just "name"
}
```

#### Proxy Objects
```killer
target = {x: 10}

handler = {
    get: fn(obj, prop) {
        print("Getting:", prop)
        return obj[prop]
    },
    set: fn(obj, prop, value) {
        print("Setting ", prop, "to", value)
        obj[prop] = value
    }
}

proxy = new Proxy(target, handler)
proxy.x  # Logs "Getting: x", returns 10
proxy.x = 20  # Logs "Setting x to 20"
```

---

## Feature Comparison Table

| Feature | v3.0 | v3.1 | v4.0 | v5.0 |
|---------|------|------|------|------|
| Core language | ✅ | ✅ | ✅ | ✅ |
| Classes & OOP | ✅ | ✅ | ✅ | ✅ |
| Module system | ❌ | ✅ | ✅ | ✅ |
| Package manager | ❌ | ✅ | ✅ | ✅ |
| REPL | ❌ | ✅ | ✅ | ✅ |
| Abstract classes | ❌ | ❌ | ✅ | ✅ |
| Generics | ❌ | ❌ | ✅ | ✅ |
| Mixins | ❌ | ❌ | ✅ | ✅ |
| Type annotations | ❌ | ❌ | ✅ | ✅ |
| Decorators | ❌ | ❌ | ✅ | ✅ |
| Async/await | ❌ | ❌ | ❌ | ✅ |
| Pattern matching | ❌ | ❌ | ❌ | ✅ |
| Reflection | ❌ | ❌ | ❌ | ✅ |
| Set/Map | ❌ | ❌ | ❌ | ✅ |
| Symbols | ❌ | ❌ | ❌ | ✅ |

---

## Implementation Priority

### Phase 2 (Current - v3.0)
```
Week 1: C Code Generator + Runtime Library
Week 2: Compiler Pipeline + Bootstrap
Week 3: Testing + Optimization
```

### Phase 3 (v3.1 - Module System)
```
Week 1-2: Module system (import/export)
Week 3: Standard library modules
Week 4: Package manager basics
Week 5: REPL implementation
Week 6: Build tool
```

### Phase 4 (v4.0 - Advanced OOP)
```
Week 1-2: Abstract classes & interfaces
Week 3: Mixins & traits
Week 4: Generics system
Week 5: Type annotations
Week 6: Decorators
```

### Phase 5 (v5.0 - Async & Advanced)
```
Week 1-2: Async/await implementation
Week 3: Pattern matching
Week 4: Reflection API
Week 5: Metaclasses
Week 6: Set/Map/Symbol collections
```

---

## Resource Requirements

| Phase | Developer | Months | Size |
|-------|-----------|--------|------|
| v3.0 (Phase 2) | 1 | 0.5 | ~500 lines C |
| v3.1 | 1-2 | 1 | ~1000 lines Killer |
| v4.0 | 1-2 | 1 | ~1500 lines Killer |
| v5.0 | 1-2 | 1 | ~2000 lines Killer |
| **Total** | 1-2 | 3.5 | ~5000 lines |

---

## Comparison with Other Languages

| Feature | Python | JavaScript | Killer (Planned) |
|---------|--------|-----------|-----------------|
| Module system | ✅ | ✅ | v3.1 |
| Async/await | ✅ | ✅ | v5.0 |
| Decorators | ✅ | ⏳ | v4.0 |
| Type annotations | ✅ | ⏳ | v4.0 |
| Pattern matching | ⏳ | ❌ | v5.0 |
| Generics | ❌ | ✅ | v4.0 |
| Mixins | ❌ | ✅ | v4.0 |
| Performance | Slow | Medium | Fast ✅ |
| Learning curve | Easy | Medium | Easy ✅ |
| Standalone exe | Hard | Possible | Easy ✅ |

---

## Community Feedback Integration

As v3.0+ is released, community requests will shape priorities:
- User contributions to standard library
- Feature requests via GitHub Issues
- Performance improvement feedback
- Ecosystem tool development

---

**Last Updated**: March 8, 2026  
**Status**: Roadmap confirmed, Phase 2 in progress  
**Next Milestone**: v3.0 Release (March 22, 2026)
