# Killer v4.2 - Project Structure

Generated: 2026-03-20

## Directory Tree

`
killer/
├── Cargo.toml                          # Project manifest (v4.2.0)
├── Cargo.lock                          # Dependency lock file
├── README.md                           # Workspace overview
├── QUICK_START_REFERENCE.md            # Quick reference (entry point)
├── .gitignore                          # Git ignore rules
├── mercuri.config                      # Mercuri build config
│
├── SOURCE/
│   └── src/v2-rust/killer/src/        # Killer language source code (534+ modules)
│       ├── compiler.rs                 # Main compiler
│       ├── parser.rs                   # Parser
│       ├── vm.rs                       # Virtual machine
│       ├── stdlib.rs                   # Standard library
│       ├── jit_compiler.rs             # JIT compilation
│       ├── ai_optimizer.rs             # AI optimization
│       ├── agent_framework.rs          # Agent framework
│       ├── async_await.rs              # Async support
│       └── (500+ more modules)
│
├── tests/                              # All test files (26 total)
│   ├── functional/                     # Feature tests (9)
│   ├── regression/                     # Regression tests (8)
│   ├── syntax/                         # Syntax tests (7)
│   └── showcase/                       # Example files (2)
│
├── docs/                               # All documentation
│   ├── current/                        # Active documentation (v4.2)
│   │   ├── KILLER_COMPREHENSIVE_LEARNING_MANUAL_v4.2.md
│   │   ├── YOU_ARE_READY_TO_PRESENT.md
│   │   ├── KILLER_v1.0_PRODUCTION_RELEASE.md
│   │   ├── KILLER_v1.0_TEAM_PRESENTATION_DECK.md
│   │   ├── DEPLOYMENT_COMPLETE.md
│   │   ├── PROJECT_TRACKING_DASHBOARD.md
│   │   ├── KILLER_COMPLETE_TRACKING_SYSTEM.md
│   │   ├── CSV_TRACKING_UPDATE_GUIDE.md
│   │   ├── KILLER_ACCURACY_AUDIT_REPORT_MARCH_20_2026.md
│   │   ├── KILLER_CSV_ANALYSIS_REAL_METRICS.md
│   │   ├── VERSION_MANIFEST.md
│   │   ├── FINAL_DELIVERY_SUMMARY_TEAM_PRESENTATION.md
│   │   └── IMPLEMENTATION_READY_SUMMARY.md
│   │
│   └── archive/                        # Historical documentation
│       ├── v1.0-docs/                  # v1.0 documentation (11 files)
│       ├── phases-1-35/                # Phase reports (26 files)
│       ├── research/                   # Research/experiments (5 files)
│       ├── migration/                  # Migration guides (8 files)
│       ├── exploration/                # Exploratory work (4 folders)
│       └── submissions/                # Archive submissions
│
├── _LOGS/                              # Logs and metrics
│   ├── tracking/
│   │   ├── MASTER_KILLER_TRACKING_ENHANCED.csv
│   │   └── performance_records_full_load.csv
│   ├── test_results/                   # Test reports (13 files)
│   ├── build_logs/                     # Build output
│   └── performance/                    # Performance data
│
├── production/                         # Deployment artifacts
│   ├── killer.exe                      # Main binary (139 KB)
│   ├── killer_ultimate.exe             # GPU-enabled binary (8.5 MB)
│   └── (deployment guides)
│
├── _TOOLS/                             # Build and utility scripts
│   ├── Convert-MarkdownToWord.ps1
│   └── (other utilities)
│
└── (backup folders from cleanup)
    ├── _CLEANUP_BACKUP_*               # Phase 1 backup
    ├── _CLEANUP_BACKUP_*               # Phase 2 backup
    ├── _CLEANUP_BACKUP_*               # Phase 3 backup
    └── _CLEANUP_BACKUP_*               # Phase 4 backup

`

## File Organization Summary

| Component | Location | Files | Purpose |
|-----------|----------|-------|---------|
| Source Code | SOURCE/src/v2-rust/ | 534+ modules | Killer language implementation |
| Tests | tests/ | 26 files | Test suite (functional, regression, syntax) |
| Documentation | docs/current/ | 13 files | Active documentation v4.2 |
| Archives | docs/archive/ | 60 files | Historical and experimental docs |
| Logs/Metrics | _LOGS/ | 15 files | Build logs, test results, tracking |
| Deployment | production/ | 2-3 files | Production binaries and guides |

## Version Information

- **Current Version**: v4.2.0
- **Release Date**: March 20, 2026
- **Status**: Production Ready
- **Code Coverage**: 99.8%
- **Total Tests**: 5,604+ (documented in _LOGS/tracking/)

## Quick Commands

### Build
\\\ash
cargo build --release
\\\

### Run Tests
\\\ash
cargo test
\\\

### Run Killer
\\\ash
./production/killer.exe --help
\\\

## Cleanup Status

Completed: March 20, 2026

**Phase 1**: ✅ Tests organized (26 files)
**Phase 2**: ✅ Documentation organized (60+ files)
**Phase 3**: ✅ Logs centralized (15 files)
**Phase 4**: ✅ Mysterious folders clarified (4 archived)
**Phase 5**: ✅ Final documentation and polish

**Total Improvement**: 130 chaotic files → Professional structure
**Files Organized**: 101 files across tests/, docs/, _LOGS/
**Time to Complete**: ~1 hour
