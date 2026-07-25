# Module System and Import Documentation

## Overview

Killer's module system enables code organization, code reuse, and package management. The system supports:
- Local module imports
- Standard library modules  
- Package manager (KPM) integration
- Selective imports (import specific symbols)
- Module caching

## Basic Usage

### Importing Entire Module
```killer
import "json"

// Access module contents
obj = json.parse("{\"x\": 1}")
```

### Selective Imports
```killer
import { parse, stringify } from "json"

// Use directly without prefix
obj = parse("{\"x\": 1}")
```

### Aliasing
```killer
import "collections" as coll

arr = coll.map([1,2,3], def(x) { x * 2 })
```

### Standard Library
```killer
import "io"           // File I/O
import "json"         // JSON parsing/serialization
import "collections"  // Array/dict utilities
import "math"         // Mathematical functions
import "string"       // String manipulation
```

## Module Resolution

Killer resolves modules in this order:

1. **Relative path** (as written): `./my-module.killer`
2. **With .killer extension** (auto-added): `my-module` → `my-module.killer`
3. **In packages directory**: `my-package` → `packages/my-package.killer`
4. **In stdlib**: Built-in modules (json, math, string, etc.)

Example resolution:
```killer
import "json"
// Tries:
// 1. json
// 2. json.killer
// 3. packages/json
// 4. packages/json.killer
// 5. stdlib/json.killer  (✓ found)
```

## Creating Modules

### Simple Module
Create `math_utils.killer`:
```killer
// math_utils.killer
def square(x) {
  return x * x
}

def cube(x) {
  return x * x * x
}

export square, cube
```

Use it:
```killer
import { square, cube } from "math_utils"

print(square(5))  // 25
print(cube(3))    // 27
```

### Module with Initialization
```killer
// config.killer
let api_url = "https://api.example.com"
let timeout = 30

def get_endpoint(path) {
  return api_url + path
}

export get_endpoint, timeout, api_url
```

### Package Modules
Directory structure:
```
my-package/
  ├── package.json    # KPM manifest
  ├── index.killer    # Main module
  ├── utils.killer
  └── types.killer
```

Import with:
```killer
import "my-package"  // Loads my-package/index.killer
```

## Export Patterns

### Named Exports
```killer
export func1, func2, CONSTANT
```

### Re-export
```killer
import { helper } from "other-module"
export helper  // Re-export for consumers
```

### Default Export (Module Interface)
```killer
// Define main functionality
def process(data) {
  return transform(data)
}

export process
```

## Module Caching

Modules are compiled and cached automatically. Once loaded, subsequent imports reuse the cached module:

```killer
import "utils"  // Loads and compiles
import "utils"  // Uses cached version (no recompile)
```

To force reload (for development):
```killer
// Not yet supported; restart interpreter
```

## Standard Library Modules

All stdlib modules are pre-compiled and available:

| Module | Purpose | Key Functions |
|--------|---------|---|
| `io` | File and console I/O | `read_file`, `write_file`, `read_line` |
| `json` | JSON parsing/serialization | `parse`, `stringify` |
| `collections` | Array and dict utilities | `map`, `filter`, `reduce`, `sort` |
| `math` | Math functions | `sqrt`, `sin`, `cos`, `random` |
| `string` | String utilities | `uppercase`, `split`, `replace` |

## Package Manager Integration

### KPM-Installed Packages
When installing with KPM:
```bash
kpm install my-package --dest ./kpm_packages
```

Import in Killer:
```killer
import "my-package"  // Looks in kpm_packages/ automatically
```

### Publishing Packages
Create `manifest.json`:
```json
{
  "name": "my-utils",
  "version": "1.0.0",
  "files": ["index.killer", "utils.killer", "README.md"]
}
```

Publish:
```bash
kpm publish manifest.json --out .
# Creates my-utils-1.0.0.tar.gz
```

Install from remote:
```bash
kpm install https://registry.example.com/my-utils-1.0.0.tar.gz
```

## Circular Dependencies

Killer allows circular imports but caution is needed:

```killer
// a.killer
import "b"
def process() { return b.transform() }

// b.killer  
import "a"
def transform() { return "data" }
```

Resolution: Modules load in declaration order. Ensure circular dependencies don't prevent initialization.

## Namespacing

Use object literals for namespace organization:

```killer
// math-utils.killer
let Utils = {
  square: def(x) { x * x },
  cube: def(x) { x * x * x },
  distance: def(x, y) { (x*x + y*y) ^ 0.5 }
}

export Utils
```

Usage:
```killer
import { Utils } from "math-utils"
print(Utils.square(5))
```

## Type Hints (Documentation)

Document module interfaces with comments:

```killer
// file-system.killer

// read_file(path: string) -> string
// Reads entire file contents into memory
def read_file(path) {
  // implementation
}

// write_file(path: string, content: string) -> void
// Writes content to file, creating if needed
def write_file(path, content) {
  // implementation
}

export read_file, write_file
```

## Error Handling in Modules

Handle import errors gracefully:

```killer
try {
  import "optional-feature"
  has_feature = true
} catch e {
  print("Optional feature not available:", e)
  has_feature = false
}

if has_feature {
  use_feature()
}
```

## Performance Tips

1. **Import only what you need**: Selective imports reduce memory
   ```killer
   import { map, filter } from "collections"  // Better than import "collections"
   ```

2. **Import at module level**: Not in loops
   ```killer
   import "json"
   for i in range(1000) {
     obj = json.parse(data)  // ✓ Good
   }
   ```

3. **Use module-level caching**: Store computed values
   ```killer
   // utils.killer
   let cache = {}
   def expensive_fn(x) {
     if cache[x] == null {
       cache[x] = compute(x)
     }
     return cache[x]
   }
   ```

## Future Enhancements

Planned features:
- Type checking system (static/optional)
- Module versioning (semver)
- Namespace packages (`@scope/package`)
- Dynamic imports (`import_dynamic(name)`)
- Tree-shaking (remove unused exports)
- Lazy loading (load on-demand)

---

## See Also
- [Language Reference](./language-reference.md)
- [Standard Library](../stdlib/README.md)
- [KPM Package Manager](../tools/kpm/)
