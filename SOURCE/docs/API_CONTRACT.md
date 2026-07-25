# Killer VM Public API Contract

**Version**: 2.1.0  
**Last Updated**: March 12, 2026

## Overview

The Killer VM public API is organized by **Stability Levels** to help users understand which APIs are safe to depend on and which may change.

## Stability Levels

### ✅ Stable (Guaranteed Backward Compatible)

APIs marked as **Stable** maintain backward compatibility within the same major version (e.g., 2.x → 2.y).

**Core VM Functions (v2.0.0+)**
- `vm::new()` - Create new virtual machine instance
- `vm::execute()` - Execute bytecode in virtual machine
- `vm::reset()` - Clear all state from virtual machine

**Compiler Functions (v2.0.0+)**
- `compiler::compile()` - Compile source code to bytecode
- `compiler::optimize()` - Optimize compiled bytecode

**Parser Functions (v2.0.0+)**
- `parser::parse()` - Parse source code to AST

**Exception Handling (v2.0.0+)**
- `exception::try_catch()` - Execute code with exception handling
- `exception::throw()` - Throw exception with message

**Generators (v2.0.0+)**
- `generator::create()` - Create generator object
- `generator::next()` - Get next value from generator

**Interactive REPL (v2.1.0+) - NEW**
- `repl::start()` - Start interactive REPL session
- `repl::eval_line()` - Evaluate single line in REPL

**Interactive Debugger (v2.1.0+) - NEW**
- `debugger::start()` - Start interactive debugger session
- `debugger::set_breakpoint()` - Set breakpoint at line number
- `debugger::step()` - Step to next instruction

**Version Management (v2.1.0+) - NEW**
- `version::get_version()` - Get current Killer version
- `version::check_compatibility()` - Check version compatibility

### ⚠️ Unstable (May Change Without Notice)

APIs marked as **Unstable** are experimental and may change in any release (including patch releases).

**Type Specialization (v2.0.0+)**
- `specialization::infer_type()` - Infer type of expression *(may change algorithm)*
- `specialization::generate_code()` - Generate specialized code *(experimental)*

> **Recommendation**: Use with caution in production. Pin specific versions if you depend on these APIs.

### ❌ Deprecated (Scheduled for Removal)

No APIs are currently deprecated in v2.1.0.

Deprecated items will be marked with:
- Replacement API name (if applicable)
- Migration instructions
- Removal version (e.g., "Will be removed in v3.0.0")

## API Compatibility Matrix

| API Function | v2.0 | v2.1 | Stability | Notes |
|--------------|------|------|-----------|-------|
| vm::new | ✓ | ✓ | Stable | Core VM |
| vm::execute | ✓ | ✓ | Stable | Core VM |
| vm::reset | ✓ | ✓ | Stable | Core VM |
| compiler::compile | ✓ | ✓ | Stable | Core |
| compiler::optimize | ✓ | ✓ | Stable | Core |
| parser::parse | ✓ | ✓ | Stable | Core |
| exception::try_catch | ✓ | ✓ | Stable | Core |
| exception::throw | ✓ | ✓ | Stable | Core |
| generator::create | ✓ | ✓ | Stable | Core |
| generator::next | ✓ | ✓ | Stable | Core |
| repl::start | - | ✓ | Stable | New v2.1 |
| repl::eval_line | - | ✓ | Stable | New v2.1 |
| debugger::start | - | ✓ | Stable | New v2.1 |
| debugger::set_breakpoint | - | ✓ | Stable | New v2.1 |
| debugger::step | - | ✓ | Stable | New v2.1 |
| version::get_version | - | ✓ | Stable | New v2.1 |
| version::check_compatibility | - | ✓ | Stable | New v2.1 |
| specialization::infer_type | ✓ | ✓ | Unstable | Experimental |
| specialization::generate_code | ✓ | ✓ | Unstable | Experimental |

## API Statistics (v2.1.0)

```
Total Functions: 19
├─ Stable: 17 (89.5%) ✓
├─ Unstable: 2 (10.5%) ⚠️
└─ Deprecated: 0 (0.0%)
```

## Backward Compatibility Guarantees

### Within Major Version (2.x)
- **Guaranteed**: Stable APIs will not be removed or have breaking changes
- **Guaranteed**: New Stable APIs may be added
- **Guaranteed**: Unstable APIs may change or be removed
- **Guaranteed**: Deprecated APIs will show warnings and migration path

### Across Major Versions (2.x → 3.x)
- **Allowed**: Breaking changes to Unstable APIs
- **Allowed**: Removal of Deprecated APIs (with 1 major version notice)
- **Allowed**: Changes to public API structure
- **Not Allowed**: Removal of Stable APIs without deprecation period

## API Usage by Stability

### Recommended: Using Stable APIs

```rust
use killer_native::api::create_default_api_contract;

// Create and verify API contract
let api_contract = create_default_api_contract();

// Check if API is stable before using
if api_contract.is_stable("vm::execute") {
    println!("✓ Safe to use in production");
}

// Get API metadata
if let Some(api_func) = api_contract.get("vm::execute") {
    println!("Introduced: {}", api_func.introduced);
    println!("Description: {}", api_func.description);
}
```

### Caution: Using Unstable APIs

```rust
// Mark code that uses unstable APIs
let api_contract = create_default_api_contract();

if !api_contract.is_stable("specialization::infer_type") {
    eprintln!("⚠️  Warning: Using unstable API - may change");
    eprintln!("    Consider pinning to specific version");
}
```

## Migration Guide for Version 2.0 → 2.1

**No breaking changes!** All v2.0 code continues to work in v2.1.

### New Features Available in v2.1
- Interactive REPL: Use `repl::start()` for interactive shell
- Interactive Debugger: Use `debugger::start()` with breakpoints
- Version API: Use `version::*` for version checking

### Recommended Adoption Path

1. **Update to v2.1.0** - No code changes required
2. **Try new features** - Use REPL and Debugger for development
3. **Add version checks** - Use `version::check_compatibility()` for version gates
4. **Monitor unstable APIs** - Plan migration if using `specialization::*`

## Backward Compatibility Layer

The Killer VM provides automatic compatibility through:

1. **API Aliases** - Old API names resolve to new implementations
2. **Migration Paths** - Version-to-version migration information
3. **Extension Support** - Safe to add new Stable APIs anytime

### Example: API Aliases

```rust
use killer_native::api::BackwardCompatibility;

let mut compat = BackwardCompatibility::new();

// Map old name to new implementation
compat.add_alias("old_vm_run", "vm::execute");

// Old code continues to work
let canonical = compat.resolve_alias("old_vm_run");
assert_eq!(canonical, "vm::execute");
```

## API Contract Validation

Generate an API compatibility report:

```rust
use killer_native::api::create_default_api_contract;

let contract = create_default_api_contract();
println!("{}", contract.compatibility_report());
```

Output:
```
=== Killer API Contract v2.1.0 ===
Total Functions: 19
Stable: 17 (89.5%)
Unstable: 2 (10.5%)
Deprecated: 0 (0.0%)
```

## Version Requirements

### Minimum Compatibility
- **Killer v2.0.0+** for stable APIs
- **Killer v2.1.0+** for REPL, Debugger, Version APIs
- **Killer v3.0.0** when released will allow breaking changes

### Feature Gates

Use version checks to conditionally use newer APIs:

```rust
use killer_native::version;

if version::feature_available("debugger", "2.1.0") {
    // Use debugger features
    debugger::start();
}
```

## FAQ

### Q: Can I use Unstable APIs in production?
**A**: You can, but with the understanding that they may change. Pin to specific versions and plan for migration.

### Q: What happens when an API is deprecated?
**A**: You'll see a warning message with:
- What's being deprecated
- What to use instead
- When it will be removed
- Migration instructions

### Q: How do I check which APIs are available?
**A**: Use the API contract:
```rust
let contract = create_default_api_contract();
println!("{}", contract.compatibility_report());
```

### Q: Is v2.0 code compatible with v2.1?
**A**: Yes! 100% backward compatible. No code changes required.

### Q: What if I need v2.0 behavior in v2.1+?
**A**: Use version checks:
```rust
if version::check_compatibility("2.0.0").is_ok() {
    // v2.0+ code
}
```

## Reporting API Issues

If you find an API that:
- Doesn't match its documented stability level
- Has breaking changes within a major version  
- Is missing from this documentation

Please report it with:
1. API name and full path
2. Your Killer version: `killer-native --version`
3. Expected vs actual behavior
4. Reproduction steps

---

**Last Updated**: March 12, 2026  
**Next Review**: June 12, 2026 (for v2.2.0 release)
