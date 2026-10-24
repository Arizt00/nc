# Backend LLVM CI Multi-Host

Status: CI surface integrated for MCT LLVM toolchain smoke.

This document defines the retained CI path for the MCT LLVM backend work. It
does not change nC semantics, the native backend, machine, basal, primitive, or
the MCT LLVM lowering contract. It only makes the LLVM toolchain matrix
reproducible on hosted CI runners.

## Workflow

The workflow is:

```text
.github/workflows/mct-llvm-toolchain-multi-host.yml
```

It runs on:

- `ubuntu-24.04` for Linux x86_64;
- `ubuntu-24.04-arm` for Linux AArch64;
- `macos-14` for macOS AArch64;
- `windows-latest` for Windows x86_64.

Each row installs Rust stable with `rustfmt` and `clippy`, installs an LLVM
toolchain for the host, probes `llvm-as`, `opt`, `llc` and the LLD-family
linkers, runs source gates, then executes the retained MCT LLVM toolchain smoke.

## Source Gates

The CI workflow runs:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test -p ncc-cli mct_cli --lib --quiet
cargo test -p ncc-ir mct_llvm --all-targets --quiet
cargo clippy -p ncc-cli -p ncc-ir --all-targets -- -D warnings
```

The gate is intentionally scoped to MCT/LLVM crates for test speed, while still
checking the workspace for compilation and format compatibility.

## Smoke Wrapper

The retained wrapper is:

```text
tools/ncc/nc/src/verification/mct_smoke.n
```

Build `tools/ncc/nc/retirement.n` as `ncc-tools`; run `ncc-tools mct-smoke`.
It delegates the actual semantics to:

```sh
ncc mct llvm-toolchain-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-llvm-ci/<host>-<arch> \
  --all-targets \
  --opt-level 2 \
  --json
```

The wrapper writes:

- exact JSON evidence;
- stderr capture;
- a markdown summary;
- LLVM `.ll`, `.bc`, optimized `.bc`, object and linked artifacts emitted by
  the underlying command.

It exits non-zero unless every selected matrix row passes. The Windows hosted
runner may retain `missing_toolchain` audit evidence when `llvm-as`, `opt`,
`llc` or the LLD-family binaries are not actually materialized by the runner
image. That audit path is not counted as LLVM pass evidence.

## Artifact Retention

The workflow uploads:

```text
target/tmp/mct-llvm-ci/<host>-<arch>/**
```

The retained artifact contains the raw smoke report and every generated stage
artifact for review. CI artifacts are the external evidence for the multi-host
claim; local reports are only preparatory evidence.

## Truth Boundary

Closed:

- multi-host GitHub Actions workflow definition;
- host LLVM toolchain installation/probe steps;
- source gates for MCT LLVM;
- retained evidence wrapper;
- artifact upload path;
- local wrapper execution path.
- Windows missing-toolchain artifact retention when hosted runner LLVM binaries
  are unavailable.

Not claimed until the workflow is pushed and run:

- retained exact-head multi-host evidence;
- proof that hosted macOS, Windows, Linux x86_64 and Linux AArch64 all have
  compatible LLVM/LLD binaries on the selected runner images;
- release evidence based on remote GitHub artifacts.

If a hosted row lacks a required LLVM linker, the workflow must fail honestly.
It must not convert missing tools into pass evidence.
