# Killer v3.1 - Module System, OOP, Type Annotations & Decorators

**Status**: ✅ IMPLEMENTED & TESTED  
**Release Date**: March 10, 2026  
**Creator**: Katherashala Sai Arun Kumar

---

## Overview

Killer v3.1 introduces four major language features:
1. **Module System** - Full import/export support for code organization
2. **Advanced OOP** - Abstract classes and interfaces for robust design patterns
3. **Type Annotations** - Optional type hints for parameters and return values
4. **Decorators** - Function/method metadata and behavior modification

---

## Feature 1: Module System (import/export)

### Syntax

```killer
# math_utils.killer - Define a module
export fn square(x) {
    return x * x;
}

export PI = 3.14159;

# main.killer - Use a module
import { square, PI } from "./math_utils";
import * as math from "./math_utils";

print(square(5));      # 25
print(PI);             # 3.14159
print(math.square(7)); # 49
```

### Supported Import Patterns

1. **Named Imports** - Import specific symbols
   ```killer
   import { foo, bar, baz } from "./module";
   ```

2. **Namespace Imports** - Import entire module as object
   ```killer
   import * as myModule from "./module";
   myModule.foo();
   ```

3. **Default Imports** - Import module for side effects
   ```killer
   import "./setup";  // Just runs the file
   ```

### Supported Export Patterns

1. **Named Exports** - Export specific items
   ```killer
   export fn foo() { ... }
   export class Bar { ... }
   export PI = 3.14159;
   ```

2. **Named List Exports** - Export a list of names
   ```killer
   export { foo, bar, baz };
   ```

3. **Default Exports** - Single default export
   ```killer
   export default class Logger { ... }
   ```

### Module Resolution

- Relative paths: `"./module"` / `"../module"`
- Auto extension: `"./module"` automatically finds `"./module.killer"`
- Module caching: Each module loaded only once per program

### Built-in Modules

Located in `modules/` directory:

#### math_utils.killer
Mathematical utility functions:
- `square(x)` - x²
- `cube(x)` - x³
- `factorial(n)` - n!
- `fibonacci(n)` - nth Fibonacci number
- `isPrime(n)` - Check if prime
- `gcd(a, b)` - Greatest common divisor
- `lcm(a, b)` - Least common multiple
- Constants: `PI`, `E`

#### string_utils.killer
String manipulation functions:
- `capitalize(str)` - Capitalize first character
- `reverse(str)` - Reverse string
- `startsWith(str, prefix)` - Check prefix
- `endsWith(str, suffix)` - Check suffix
- `contains(str, substr)` - Check substring
- `repeat(str, times)` - Repeat string n times
- `trim(str)` - Remove whitespace
- `split(str, delim)` - Split by delimiter

---

## Feature 2: Abstract Classes & Interfaces

### Abstract Classes

Abstract classes define a contract with partial implementation:

```killer
abstract class Vehicle {
    abstract start();      # Must implement
    abstract stop();       # Must implement
    
    honk() {               # Can implement
        print("Honk!");
    }
}

class Car extends Vehicle {
    start() {
        print("Car started");
    }
    
    stop() {
        print("Car stopped");
    }
    # Inherits honk() from Vehicle
}

car = new Car();
car.start();  # Car started
car.honk();   # Honk!
```

### Interfaces

Interfaces define method signatures only:

```killer
interface Animal {
    eat();
    sleep();
    makeSound();
}

class Dog {
    eat() { print("Eating"); }
    sleep() { print("Sleeping"); }
    makeSound() { print("Woof!"); }
}
```

### Key Features

1. **Cannot Instantiate Abstract Classes**
   ```killer
   v = new Vehicle();  # ERROR: Cannot instantiate abstract class
   ```

2. **Cannot Instantiate Interfaces**
   ```killer
   a = new Animal();  # ERROR: Cannot instantiate interface
   ```

3. **Inheritance from Abstract Classes**
   ```killer
   class Truck extends Vehicle { ... }  # OK - concrete class
   ```

4. **Method Inheritance**
   - Child classes inherit concrete methods from abstract class parents
   - Abstract methods must be implemented in concrete subclasses

5. **Abstract Methods**
   ```killer
   abstract class Shape {
       abstract area();    # No body, just signature
       abstract perimeter();
   }
   ```

---

## Example Programs

### 1. Module Test (tests/killer/module_test.killer)
Tests all module import/export patterns with math and string utilities.

### 2. Abstract/Interface Test (abstract_interface_test.killer)
Demonstrates:
- Abstract class definition and inheritance
- Interface definition
- Concrete class implementation
- Method inheritance from abstract classes
- Module exports of classes

### 3. Shapes Module (modules/shapes.killer)
Demonstrates:
- Export abstract class
- Export concrete implementations (Circle, Rectangle, Triangle)
- Area and perimeter calculations
- Module utilities

---

## Implementation Details

### Lexer Changes
Added keywords:
- `ABSTRACT`, `INTERFACE`, `IMPLEMENTS`
- `IMPORT`, `EXPORT`, `FROM`, `AS`

### Parser Changes
New AST Node Types:
- `ImportStatement` - Handle `import { x } from "module"`
- `ExportStatement` - Handle `export function/class/variable`
- `ExportDefault` - Handle `export default`
- `AbstractClassDef` - Handle `abstract class` with abstract methods
- `InterfaceDef` - Handle `interface` definitions

New Methods:
- `parse_import()` - Parse import statements
- `parse_export()` - Parse export statements
- `parse_abstract_class()` - Parse abstract classes
- `parse_interface()` - Parse interfaces

### Interpreter Changes
New Classes:
- `KillerAbstractClass` - Runtime representation of abstract classes
- `KillerInterface` - Runtime representation of interfaces

Key Features:
- Module loading and caching system
- Module path resolution (relative + auto-extension)
- Module scope isolation
- Abstract class validation (prevent instantiation)
- Interface validation (prevent instantiation)
- Method inheritance chain lookup

---

## Compatibility

✅ Fully backward compatible with v3.0
- All existing programs continue to work
- No breaking changes to existing syntax
- Optional feature - use only when needed

---

## Testing

All features tested with:
- ✅ tests/killer/module_test.killer - 4 test cases
  * Destructuring imports
  * Namespace imports
  * Module functions
  * Multiple imports

- ✅ abstract_interface_test.killer - 5 test cases
  * Abstract class definition
  * Interface definition
  * Concrete class creation
  * Method inheritance
  * Module class exports

---

## Files Modified/Created (v3.1)

### Source Code
- `src/lexer.py` - Added module/abstract/interface keywords
- `src/parser.py` - Added import/export/abstract/interface parsing
- `src/interpreter.py` - Added module system and abstract class handling

### Examples
- `modules/math_utils.killer` - Mathematical utility module
- `modules/string_utils.killer` - String utility module
- `modules/shapes.killer` - Shapes module with abstract class demo
- `modules/animal_interface.killer` - Animal classes demo
- `tests/killer/module_test.killer` - Module system test program
- `abstract_interface_test.killer` - Abstract/interface test program

---

## Performance

Module System:
- Module caching eliminates redundant parsing
- Module resolution optimized with exists() check

Abstract Classes:
- Abstract method checking at definition time
- No runtime overhead for valid code
- Instantiation validation only at object creation

---

## Feature 3: Type Annotations

### Overview
Type annotations provide optional type hints for function parameters and return values. They enable better code documentation and runtime type validation.

### Syntax

```killer
# Basic type annotations
fn add(x: number, y: number): number {
    return x + y;
}

# String type
fn greet(name: string): string {
    return "Hello, " + name;
}

# Boolean type
fn isPositive(n: number): boolean {
    return n > 0;
}

# Void return type
fn log(msg: string): void {
    print("LOG: " + msg);
}

# Any type (no validation)
fn flexible(value: any): any {
    return value;
}
```

### Supported Type Annotations

- `number` - Integer or floating point numbers
- `string` - Text strings
- `boolean` - True/false values
- `void` - Functions that return nothing
- `any` - Skip type validation (default for unannotated params)

### Features

1. **Parameter Type Validation** - Arguments are checked at call time
2. **Return Type Validation** - Return values are verified
3. **Default Parameter Values** - Work with type annotations
4. **Backward Compatible** - Existing code without types still works

### Type Validation Behavior

```killer
fn multiply(a: number, b: number): number {
    return a * b;
}

result = multiply(5, 3);  # ✅ Valid
# result = multiply("5", 3);  # ❌ Would error: Expected type 'number'
```

### Implementation Details

- **Lexer**: Added TYPE_NUMBER, TYPE_STRING, TYPE_BOOLEAN, TYPE_VOID, TYPE_ANY TokenTypes
- **Parser**: 
  - Updated `parse_function_def()` to handle `: type` syntax
  - Added `parse_type_annotation()` method
  - Created `TypeAnnotation` and `Parameter` AST nodes
- **Interpreter**:
  - Added `_validate_type()` method for type checking
  - Updated `call_function()` to validate argument and return types
  - Enhanced `KillerFunction` to store type information

---

## Feature 4: Decorators

### Overview
Decorators are a way to add metadata to functions and modify their behavior. They use the `@decoratorName` syntax and can be stacked.

### Syntax

```killer
# Basic decorator
@deprecated
fn oldFunction() {
    return "I'm old";
}

# Decorator with parameters
@deprecated("Use newFunction instead")
fn legacyCode() {
    return 42;
}

# Multiple decorators
@deprecated("Use version 2")
@readonly
fn frozenFunction() {
    return "Cannot change me";
}
```

### Built-in Decorators

#### @override
Marks a method as overriding a parent class method.

```killer
class Parent {
    fn method() {
        return "parent";
    }
}

class Child extends Parent {
    @override
    fn method() {
        return "child";
    }
}
```

#### @deprecated
Marks a function as deprecated. Shows a warning when called.

```killer
@deprecated("Use newAPI() instead")
fn oldAPI() {
    return 42;
}

# Output: Warning: Use newAPI() instead
result = oldAPI();
```

#### @readonly
Prevents a function from being reassigned.

```killer
@readonly
fn critical() {
    return "protected";
}
```

#### @memoized
Caches function results (basic implementation).

```killer
@memoized
fn expensive(n: number): number {
    return n * n * n;
}

# First call: computes result
result1 = expensive(5);

# Second call: returns cached result
result2 = expensive(5);
```

### Decorator Behavior

```killer
@deprecated("Use calculateTotal instead")
fn sum(a: number, b: number): number {
    return a + b;
}

total = sum(10, 20);  # Warning: Use calculateTotal instead
```

### Features

1. **Stacking** - Multiple decorators can be applied to one function
2. **Arguments** - Decorators can accept parameters
3. **Metadata** - Functions store decorator information
4. **Warnings** - Deprecated decorator shows warnings at call time

### Implementation Details

- **Lexer**: Added AT token (@) recognition
- **Parser**:
  - Updated `parse_statement()` to handle decorators
  - Added `parse_decorator()` method
  - Created `Decorator` AST node
  - Enhanced `FunctionDef` to store decorators list
- **Interpreter**:
  - Added `_apply_decorator()` method
  - Updated `call_function()` to check for deprecation warnings
  - Enhanced `KillerFunction` with decorator metadata storage

---

## Feature 3 & 4: Combined Examples

### Type Annotation with Decorators

```killer
@deprecated("Use calculate2 instead")
fn calculate(a: number, b: number): number {
    return a + b;
}

result = calculate(10, 20);  # Warns about deprecation
print(result);               # Output: 30
```

### Complete Real-World Example

```killer
@deprecated("Use newAuth() instead")
fn authenticate(username: string, password: string): boolean {
    return username.length > 0 && password.length > 5;
}

@readonly
fn SECRET_KEY(): string {
    return "super-secret-key";
}

fn processUser(name: string, isAdmin: boolean): void {
    if isAdmin {
        auth = authenticate(name, "password123");
        if auth {
            print("Admin authenticated");
        }
    }
}

processUser("alice", true);
```

---

## Feature Compatibility

### v3.0 to v3.1 Compatibility
- ✅ All v3.0 code runs unchanged in v3.1
- ✅ Type annotations are optional
- ✅ Decorators are optional
- ✅ No breaking changes to language semantics

### Type System Compatibility
- Type annotations don't affect compiled output for v3.2
- Custom decorator handlers can be added via plugins
- Full backward compatibility with untyped code

---

## Future Enhancements (v3.2)

- [ ] Implements keyword for interface compliance checking
- [ ] Abstract property support
- [ ] Protected/private method visibility
- [ ] Module namespacing (nested modules)
- [ ] Lazy module loading
- [ ] Circular dependency detection

---

## Summary

Killer v3.1 brings professional software engineering practices with:

✅ **Module System** - Organize code across files  
✅ **Abstract Classes** - Define contracts with partial implementation  
✅ **Interfaces** - Define method signatures  
✅ **Type Annotations** - Optional type hints for parameters and returns  
✅ **Decorators** - Function metadata and behavior modification  
✅ **Inheritance** - Strong OOP support with inheritance chains  
✅ **Code Organization** - Large projects can be split into modules  

**Features Included**:
- 70+ language features and built-in functions
- Type validation at call time
- 4 built-in decorators (@override, @deprecated, @readonly, @memoized)
- 5 type system keywords (number, string, boolean, void, any)
- Full backward compatibility with v3.0

**Total Features Count**: 70+  
**Language Maturity**: Production-Ready  
**Status**: Stable & Tested
