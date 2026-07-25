# Killer Language v4.2 - Workspace Structure

## Quick Navigation

### Core Files
- **Cargo.toml** - Project manifest
- **QUICK_START_REFERENCE.md** - Entry point documentation

### Folders

#### source/
Contains all Killer language source code (Rust implementation)
- 534+ modules
- Compiler, parser, VM, stdlib, AI framework

#### tests/
All test files organized by type
- functional/ - Feature/functionality tests
- regression/ - Regression test suite
- syntax/ - Syntax and language tests
- showcase/ - Example/showcase files

#### docs/
- current/ - Active documentation for v4.2
- archive/
  - v1.0-docs/ - Historical v1.0 documentation
  - phases-1-35/ - Phase completion reports
  - research/ - Research and experimental docs
  - migration/ - Version migration guides
  - submissions/ - Archive submissions
  - exploration/ - Exploratory work and experiments

#### _LOGS/
- tracking/ - CSV tracking files and metrics
- test_results/ - Test execution reports
- build_logs/ - Build output logs
- performance/ - Performance benchmarks

#### production/
Deployment artifacts and binaries
- killer.exe - Standalone binary
- deployment guides

## Development Workflow

1. **Source Code**: See SOURCE/
2. **Add Tests**: tests/functional/ or tests/regression/
3. **Documentation**: Update docs/current/ only
4. **Build**: Run Cargo commands (see Cargo.toml)
5. **Logs**: Check _LOGS/ for build/test results

## Version Information

**Current Version**: v4.2 (March 20, 2026)
**Last Updated**: 2026-03-20
**Cleanup Status**: Phase 4/5 complete

## Support

For questions about workspace organization, see docs/current/
