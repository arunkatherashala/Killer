# Killer Standard Library

The Killer standard library provides essential modules for common tasks.

## Available Modules

### `io` - File and Console I/O
- `read_file(path)` - Read file contents
- `write_file(path, content)` - Write to file
- `read_line()` - Read line from stdin
- `read_lines(path)` - Read all lines from file
- `open(path, mode)` - Open file handle
- `close(handle)` - Close file handle
- `write(handle, content)` - Write to handle
- `read(handle, bytes)` - Read from handle

### `json` - JSON Parsing and Serialization
- `parse(json_string)` - Parse JSON string
- `stringify(value)` - Serialize value to JSON
- `parse_file(path)` - Parse JSON from file
- `write_file(path, value)` - Write JSON to file
- `keys(obj)`, `values(obj)`, `entries(obj)` - Object inspection
- `clone(obj)`, `deep_clone(obj)` - Cloning utilities
- `merge(obj1, obj2)` - Merge objects

### `collections` - Array and Dictionary Utilities
- **Array**: `push`, `pop`, `shift`, `unshift`, `map`, `filter`, `reduce`, `find`, `includes`
- **Array**: `index_of`, `reverse`, `slice`, `join`, `sort`, `unique`, `flatten`
- **Dictionary**: `set`, `get`, `remove`, `has_key`, `clear`
- **Utility**: `range(start, end)` - Generate number ranges

### `math` - Mathematical Functions
- **Constants**: `PI`, `E`, `TAU`
- **Basic**: `abs`, `sign`, `floor`, `ceil`, `round`, `sqrt`, `pow`
- **Transcendental**: `exp`, `log`, `log10`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`
- **Utilities**: `min`, `max`, `clamp`, `lerp`, `gcd`, `lcm`, `factorial`
- **Random**: `random()`, `random_range(min, max)`, `random_int(min, max)`

### `string` - String Manipulation
- **Case**: `uppercase`, `lowercase`, `capitalize`
- **Trim**: `trim`, `ltrim`, `rtrim`
- **Search**: `index_of`, `last_index_of`, `starts_with`, `ends_with`, `contains`
- **Transform**: `split`, `substring`, `char_at`, `replace`, `replace_all`, `reverse`
- **Format**: `pad_start`, `pad_end`, `format(template, values)`
- **Convert**: `to_number`, `to_string`

## Usage

To use a module in your Killer code:

```killer
// Import all exports from a module
import "json" as json

// Or specific functions
import { parse, stringify } from "json"

// Use the module
obj = json.parse("{\"key\": \"value\"}")
```

## Implementation Status

| Module | Status | Notes |
|--------|--------|-------|
| `io` | Alpha | Basic file I/O; async variant planned |
| `json` | Alpha | Parse/stringify; schema validation TBD |
| `collections` | Beta | Full array/dict API; performance optimized |
| `math` | Beta | All standard functions; GPU accelerated variants TBD |
| `string` | Beta | Comprehensive string toolkit; Unicode handling TBD |

## Future Modules

- `crypto` - Hashing, encryption, digital signatures
- `datetime` - Date and time utilities
- `http` - HTTP client library
- `fs` - Advanced file system operations
- `path` - Path manipulation utilities
- `encoding` - Base64, URL encoding, etc.
- `async` - Promises, async/await
- `debug` - Logging, tracing, profiling
