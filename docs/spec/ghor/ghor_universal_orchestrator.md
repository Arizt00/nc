# ghor Universal Build Orchestrator

`ghor` is the official nC build orchestrator binary. It is intentionally rooted in `[@machine]`: it can plan, validate, package, seal, and report across nC projects while recording machine and basal closure evidence.

`ghor` is Cargo-like in command shape, not a clone of every Cargo network behavior. Remote registry, external compiler, and cross-host evidence must be observed from real tools before it is recorded as available.

## Scope

- Native language: nC.
- Baseical manifest: `ghor.toml`.
- Compatibility manifest: `nc.toml`.
- Lockfile: `ghor.lock`.
- Foreign adapters: C, C++, ASM, Rust, Java, Python, JavaScript, TypeScript.
- Target matrix: Linux, Windows, macOS, aarch64, RISC-V, and wasm profiles declared by ghor.
- Machine dependency: requires machine doctor plus native doctor with `asm_complete_total=true`.
- Basal dependency: requires `basal_closed=true` when release evidence is requested.

## Evidence Policy

`ghor` never records unobserved external parity as evidence. Missing optional host tools are reported as `missing_optional_tool` or `requires_external_toolchain`. Build and link planning can be complete while external cross compilation remains environment-bound.

## Core Pipeline

1. Read `ghor.toml` or compatible `nc.toml`.
2. Validate package, targets, profiles, language adapters, and dependency checksums.
3. Build a source graph over `src`, `machine`, `basal`, `primitive`, `tests`, and `examples`.
4. Run nC checks for `.n` source files.
5. Record build, artifact, lock, evidence, conformance, and seal reports.
6. Delegate machine, basal, runtime, compiler, and primitive commands through `ncc`.

## Command Families

- Project: `new`, `init`, `check`, `build`, `run`, `test`, `bench`, `clean`.
- Formatting and docs: `fmt`, `lint`, `doc`.
- Package management: `add`, `remove`, `resolve`, `fetch`, `lock`, `verify-lock`, `vendor`, `tree`.
- Release: `artifact`, `package`, `publish`, `install`, `uninstall`, `evidence`, `seal`, `conformance`, `release-check`.
- Universal surface: `target`, `profile`, `features`, `cross`, `link`, `report`, `graph`, `metadata`.
- Toolchain: `toolchain`, `verify-toolchain`, `sync-toolchain`.
- Delegation: `machine`, `basal`, `runtime`, `compiler`, `primitive`.

## Non-Goals For Pre-Fase 17

Pre-Fase 17 does not implement a new language phase. It closes the ghor binary/orchestrator surface and records honest toolchain evidence so Fase 17 can open from a stable executable base.
