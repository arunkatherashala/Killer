# Archived, never-compiled source files

These 155 `.rs` files were under `src/` but are not reachable from `src/lib.rs`, `src/main.rs` or any
`src/bin/*` (no `mod` declaration, no `#[path]`), so they were never compiled or tested. A review found
nothing unique in them: older copies of modules that exist in compiled form, generated "Problem 1..N"
templates and in-memory simulations. They were moved here with `git mv` (history is kept) instead of deleted.

Same relative paths as before, under `archive/uncompiled/`. To bring one back: `git mv` it to its old place
and add its `mod` line. The 36 files that looked worth wiring in (the `spark/` tree, `cluster_coordinator.rs`,
`effect_system.rs`, `trait_system.rs`, ...) stayed in `src/`.
