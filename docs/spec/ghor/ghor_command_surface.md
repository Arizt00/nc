# ghor Command Surface

The Pre-Fase 17 command surface is closed as an installable binary surface.

## Project Commands

`new`, `init`, `check`, `build`, `run`, `test`, `bench`, `clean`, `fmt`, `lint`, and `doc` operate on the project manifest and source graph. `new` and `init` create both `ghor.toml` and compatible `nc.toml`.

## Universal Commands

- `target list --json`: emits the known target matrix from native nC stage1.
- `target info <triple> --json`: emits one validated target entry from native nC stage1.
- `profile --json`: lists manifest profiles plus default debug/release/test/bench profiles in native nC stage1.
- `features --json`: lists manifest features and selected features in native nC stage1.
- `cross --target <triple> --json`: emits a native nC cross-build plan and observed host toolchain matrix.
- `native-deps --json`: normalizes nested and inline native dependencies, validates material artifacts and checks SHA-256 contracts in native nC stage1.
- `ffi-manifest --json`: asks the compiler for each source FFI link manifest and aggregates link arguments, runtime paths, artifacts and diagnostics in native nC stage1.
- `link --json`: emits a native nC link plan over the validated graph, native dependencies and compiler-owned FFI contracts.
- `artifact --out <path> --json`: writes a native nC artifact manifest with graph, manifest, native-boundary and FFI hashes.
- `fetch --json`: validates local dependencies and checksum policy without using unapproved network access.
- `graph --json`: walks the six project source roots and emits the material dependency graph in native nC stage1.
- `report summary --json`: emits manifest, target, toolchain, and command summary from native nC stage1.
- `report commands --json`: accounts for all 77 command contracts; phase 26 records 57 complete stage1 commands, 16 partial routes and no pending routes.
- `bin --json`: resolves the native ghor installation bin directory from `GHOR_HOME` or the host user home.

## Release Commands

`package`, `evidence`, `seal`, `conformance`, and `release-check` write structured release artifacts. Evidence records machine and basal command hashes and host toolchain observations.

## Delegation Commands

`machine`, `basal`, `runtime`, `compiler`, and `primitive` are dispatched by the native nC stage1 executable. The adapter resolves `NC_GHOR_NCC`, then `NCC`, and finally `ncc`; it constructs the child argument vector explicitly, starts `ncc` without a shell, inherits the terminal and returns the exact child exit status. `machine` and `basal` default to `doctor` when no action is supplied.

`cargo-exec`, `build-script`, `proc-macro`, `interop`, and `external-compat`
are dispatched by `foreign_execution.n`. Stage1 starts tools with typed
`spawnIn(program, argv, cwd)` or captured direct process execution. Cargo and
C/C++ compilers remain explicit foreign tools; the orchestrator and evidence
model are native nC.
