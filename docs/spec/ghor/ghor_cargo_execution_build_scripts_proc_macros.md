# ghor Cargo Execution: Build Scripts and Proc Macros

Status: closed in native nC stage1.

`ghor` closes Rust build-script and proc-macro execution by delegating to the
real Cargo toolchain and recording deterministic evidence. This is the correct
industrial boundary: Rust execution remains Cargo/rustc authority, while `ghor`
owns orchestration, checks, reports and evidence.

The authority is `tools/ghor/nc/src/foreign_execution.n`. It invokes Cargo
through the native process-capture backend and reports `executor: nc-stage1`.

## Commands

- `ghor cargo-exec --manifest-path Cargo.toml --json`
- `ghor build-script --manifest-path Cargo.toml --json`
- `ghor proc-macro --manifest-path Cargo.toml --json`

Each command runs `cargo check` with a controlled target directory and emits a
JSON report containing exit code, status, stdout/stderr hashes, declared
build-script/proc-macro metadata and mode-specific evidence.

## Build Scripts

For crates with `build.rs` or `package.build`, `ghor build-script` proves that
the real Cargo build-script path executed by observing Cargo's check output and
capturing warning/output hashes.

## Proc Macros

For proc-macro crates or consumer fixtures, `ghor proc-macro` validates macro
expansion through real Cargo checking. `ghor` does not load macros itself and
does not fake expansion results.

## Boundary

`ghor` does not replace Cargo or `rustc`; it makes their execution part of the
nC release evidence surface.
