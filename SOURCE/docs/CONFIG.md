# Killer Configuration System

## Overview

The Killer configuration system provides a `.killerrc` file that projects can use to customize linter and formatter settings. This ensures consistent code style across team members without requiring command-line flags.

**Status**: ✅ Stable (v2.1.0)
**Module**: `src/config.rs` (280+ lines)
**Format**: TOML (simple key=value)
**Tests**: 12 unit tests passing

## Quick Start

### Create a Project Configuration

1. Create a `.killerrc` file in your project root:

```bash
killer-native --init-config
```

Or create manually with default settings:

```toml
# .killerrc
[linter]
max_line_length = 100
check_naming = true
check_unused = true
check_security = true
check_performance = true
min_severity = 0
disabled_rules = []

[formatter]
indent_style = "spaces"
indent_size = 2
line_length = 100
trailing_comma = "multiline"
brace_style = "same-line"
spaces_around_operators = true
spaces_after_keywords = true
uppercase_keywords = false
max_blank_lines = 2
```

2. Customize for your project:

```toml
# .killerrc - Custom configuration
[linter]
max_line_length = 120        # Allow longer lines
check_naming = false         # Disable naming checks
disabled_rules = ["unused-variable"]  # Disable specific rules

[formatter]
indent_size = 4              # Use 4-space indentation
brace_style = "new-line"     # Opening brace on new line
```

3. Use with linter and formatter:

```bash
# Automatically loads .killerrc from project root
killer-native --lint myfile.killer
killer-native --format myfile.killer
```

## Configuration Options

### Linter Configuration `[linter]`

#### `max_line_length` (default: 100)
Maximum number of characters per line before a warning
```toml
[linter]
max_line_length = 120
```

#### `check_naming` (default: true)
Enable naming convention checks (snake_case, CamelCase, etc.)
```toml
[linter]
check_naming = true   # true, false, yes, no, 1, 0
```

#### `check_unused` (default: true)
Enable unused code detection (unused variables, functions, imports)
```toml
[linter]
check_unused = true
```

#### `check_security` (default: true)
Enable security issue detection (SQL injection, command injection risks)
```toml
[linter]
check_security = true
```

#### `check_performance` (default: true)
Enable performance pattern detection (inefficient loops, string concatenation)
```toml
[linter]
check_performance = true
```

#### `min_severity` (default: 0)
Minimum severity level to report (0=info, 1=warning, 2=error)
```toml
[linter]
min_severity = 1   # Only show warnings and errors (no info)
```

#### `disabled_rules` (default: [])
Comma-separated list of rule names to disable
```toml
[linter]
disabled_rules = "rule1, rule2, rule3"
```

### Formatter Configuration `[formatter]`

#### `indent_style` (default: "spaces")
Use spaces or tabs for indentation
```toml
[formatter]
indent_style = "spaces"   # or "tabs"
```

#### `indent_size` (default: 2)
Number of spaces/tabs per indentation level (1-8)
```toml
[formatter]
indent_size = 4
```

#### `line_length` (default: 100)
Target line length for formatting (60-200)
```toml
[formatter]
line_length = 100
```

#### `trailing_comma` (default: "multiline")
Trailing comma placement strategy
```toml
[formatter]
trailing_comma = "never"       # Remove trailing commas
trailing_comma = "always"      # Add trailing commas
trailing_comma = "multiline"   # Only in multi-line structures
```

#### `brace_style` (default: "same-line")
Opening brace placement
```toml
[formatter]
brace_style = "same-line"    # if (x) {
brace_style = "new-line"     # if (x)\n{
```

#### `spaces_around_operators` (default: true)
Add spaces around operators (=, +, -, *, /)
```toml
[formatter]
spaces_around_operators = true   # x = 5 + 3
spaces_around_operators = false  # x=5+3
```

#### `spaces_after_keywords` (default: true)
Add space after keywords (if, for, while, etc.)
```toml
[formatter]
spaces_after_keywords = true   # if (x) vs if(x)
```

#### `uppercase_keywords` (default: false)
Convert keywords to uppercase
```toml
[formatter]
uppercase_keywords = false  # if, for, while...
uppercase_keywords = true   # IF, FOR, WHILE...
```

#### `max_blank_lines` (default: 2)
Maximum consecutive blank lines
```toml
[formatter]
max_blank_lines = 1    # Allow only 1 blank line
max_blank_lines = 2    # Allow up to 2 blank lines
```

## Example Configurations

### Strict Style (Google-style)
```toml
[linter]
max_line_length = 80
check_naming = true
check_unused = true
check_security = true
check_performance = true
min_severity = 1        # Warnings and errors only

[formatter]
indent_style = "spaces"
indent_size = 2
line_length = 80
trailing_comma = "never"
brace_style = "same-line"
spaces_around_operators = true
spaces_after_keywords = true
max_blank_lines = 1
```

### Flexible Style (LLVM-style)
```toml
[linter]
max_line_length = 120
check_naming = true
check_unused = false    # Allow unused variables
check_security = true
check_performance = false
disabled_rules = "unused-variable, unused-function"

[formatter]
indent_style = "spaces"
indent_size = 4
line_length = 120
trailing_comma = "always"
brace_style = "new-line"
spaces_around_operators = true
spaces_after_keywords = true
```

### Python-style
```toml
[linter]
max_line_length = 88
check_naming = true
check_unused = true
check_security = true
check_performance = true

[formatter]
indent_style = "spaces"
indent_size = 4
line_length = 88
trailing_comma = "multiline"
brace_style = "same-line"
```

### Compact Style
```toml
[linter]
max_line_length = 100
check_naming = true
check_unused = true
check_security = true
check_performance = false

[formatter]
indent_style = "spaces"
indent_size = 2
line_length = 100
trailing_comma = "multiline"
```

## Configuration Discovery

The configuration system searches for `.killerrc` in the following order:

1. **Project Root** - `./.killerrc` (checked first)
2. **Parent Directories** - `../, ../../, ../../../` etc. (search upward)
3. **Home Directory** - `~/.killerrc` (fallback - future enhancement)
4. **Default Settings** - Built-in defaults if no file found

### Example

```
my_project/
├── .killerrc          ← Found here first
├── src/
│   ├── models.killer
│   └── utils.killer
├── tests/
│   └── test_*.killer
└── examples/
    └── demo.killer
```

When running `killer-native --lint src/models.killer`, the system searches:
1. `src/.killerrc` (not found)
2. `.killerrc` (found!) ← Uses this configuration
3. `../.killerrc` (not checked, already found)

## Usage Examples

### Format and Lint a File

```bash
# Format using project config
killer-native --format myfile.killer

# Lint using project config
killer-native --lint myfile.killer

# Both will automatically load .killerrc if present
```

### With CI/CD Pipeline

```yaml
# GitHub Actions example
name: Check Code Quality

on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install killer
        run: cargo install killer-native
      - name: Lint code
        run: |
          for file in **/*.killer; do
            killer-native --lint "$file"
          done
      - name: Format check
        run: |
          for file in **/*.killer; do
            killer-native --format "$file"
            # Check if file was modified
            if ! git diff --quiet "$file"; then
              echo "File $file needs formatting"
              exit 1
            fi
          done
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Format all modified .killer files
for file in $(git diff --cached --name-only | grep '\.killer$'); do
  killer-native --format "$file"
  killer-native --lint "$file" || exit 1
  git add "$file"
done
```

### Batch Processing

```bash
# Format all Killer files in project
for file in **/*.killer; do
  echo "Formatting: $file"
  killer-native --format "$file"
  echo "Linting: $file"
  killer-native --lint "$file"
done
```

## API Usage

### Load Configuration Programmatically

```rust
use killer_native::config::KillerConfig;
use std::path::Path;

// Load from path (searches upward)
let config = KillerConfig::load_from_path(Path::new("."))?;

// Load specific file
let config = KillerConfig::load_from_file(Path::new(".killerrc"))?;

// Use default if no file
let config = KillerConfig::default();

// Access settings
println!("Max line length: {}", config.linter.max_line_length);
println!("Indent size: {}", config.formatter.indent_size);
```

### Create Configuration File

```rust
use killer_native::config::KillerConfig;
use std::path::Path;

// Create default .killerrc in current directory
KillerConfig::create_default_file(Path::new(".killerrc"))?;
```

## TOML Format Reference

The `.killerrc` file uses a simplified TOML format:

```toml
# Comments start with #
# Section headers use [name]
# Key-value pairs: key = value

[linter]
# Numbers: no quotes needed
max_line_length = 100

# Strings: quoted with " or '
indent_style = "spaces"
indent_style = 'spaces'

# Booleans: true/false, yes/no, 1/0
check_naming = true
check_naming = yes
check_naming = 1

# Lists: comma-separated in quotes
disabled_rules = "rule1, rule2, rule3"

# Inline comments supported
max_line_length = 100  # Limit to 100 chars
```

## Validation Rules

The configuration system validates all settings:

| Key | Valid Values | Example |
|-----|--------------|---------|
| `max_line_length` | 40-200 | 100 |
| `indent_size` | 1-8 | 2, 4 |
| `line_length` | 60-200 | 100 |
| `indent_style` | "spaces", "tabs" | "spaces" |
| `trailing_comma` | "never", "always", "multiline" | "multiline" |
| `brace_style` | "same-line", "new-line" | "same-line" |
| Boolean options | true/false, yes/no, 1/0 | true |
| `min_severity` | 0-2 | 1 |

### Invalid Configuration Error

```toml
[formatter]
indent_style = "spaces or tabs"  # ❌ ERROR: Must be "spaces" or "tabs"
trailing_comma = "maybe"         # ❌ ERROR: Must be "never", "always", "multiline"
indent_size = 20                 # ⚠️ WARNING: Usually 1-8
```

## Troubleshooting

### Configuration Not Loading

**Symptom**: `.killerrc` exists but settings aren't applied

**Solution**: 
- Check file is in project root
- Verify no syntax errors
- Ensure file is named `.killerrc` (not `.killerrc.toml`)
- Check file permissions (readable)

### Settings Being Ignored

**Symptom**: Changed config but linter/formatter uses old settings

**Solution**:
- Config loads from nearest parent directory
- Check there's no `.killerrc` in a parent folder overriding yours
- Verify key names match exactly (case-sensitive)

### Invalid Configuration Values

**Symptom**: Error like "Invalid value: invalid_option"

**Solution**:
- Check valid values in table above
- Review example configurations
- Use `killer-native --init-config` to generate valid template

### Comments Causing Parse Errors

**Symptom**: Numbers with inline comments fail to parse

**Solution**:
- Already fixed! Inline comments are supported
- Example: `max_line_length = 100  # My comment`

## Migration Guide

### From Command-Line Flags to Config File

**Before** (command-line flags):
```bash
killer-native --lint file.killer --max-line-length 120
killer-native --format file.killer --indent-size 4
```

**After** (.killerrc):
```toml
[linter]
max_line_length = 120

[formatter]
indent_size = 4
```

```bash
killer-native --lint file.killer
killer-native --format file.killer
```

## Best Practices

1. **Commit `.killerrc` to version control**
   - Ensures all team members use same settings
   - Configuration is part of project definition

2. **Start with defaults**
   - Use `killer-native --init-config`
   - Customize only what needed

3. **Consistent team configuration**
   - Discuss style preferences once
   - Document decisions in `.killerrc`
   - No per-person configuration

4. **Use reasonable limits**
   - Max line length: 80-120 characters
   - Indent size: 2 or 4 spaces
   - Don't be too strict

5. **Regular updates**
   - Review configuration quarterly
   - Adjust as team develops conventions
   - Document changes in project guide

## Related Features

- **Linter** (`--lint`) - Code quality analysis with 50+ rules
- **Formatter** (`--format`) - Auto-formatting with 30+ rules
- **Version Manager** - Semantic versioning and deprecations
- **API Contract** - Public API stability guarantees

## Future Enhancements

- [x] Basic TOML parsing
- [x] Linter configuration
- [x] Formatter configuration
- [ ] Home directory fallback (~/.killerrc)
- [ ] Environment variable support
- [ ] IDE integration for config editing
- [ ] Config generation CLI (--init-config)
- [ ] Config validation CLI (--validate-config)
- [ ] Config migration tool (between versions)

## Version History

### v2.1.0 (Current)
- Initial configuration system
- 12 unit tests
- TOML format support
- Full linter/formatter integration

### Future (v2.2.0+)
- Config validation command
- Config generation command
- Home directory support
- Profile system (multiple configs)
- Environment variable overrides
