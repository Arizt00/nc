# Backend LLVM/MCT F6: multi-host LLVM toolchain matrix

Status: closed for the ncc LLVM-equivalent toolchain smoke surface.

F6 adds an external LLVM toolchain matrix around the MCT LLVM adapter. The
compiler now has a public smoke path that writes textual LLVM IR and, when the
host provides the required tools, drives:

1. `llvm-as` from `.ll` to `.bc`;
2. `opt` using the MCT-owned pass pipeline;
3. `llc` from optimized bitcode to object bytes;
4. the matching LLD-family linker for the object format.

The command is:

```sh
ncc mct llvm-toolchain-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-llvm-f6 \
  --all-targets \
  --opt-level 2 \
  --json
```

For environments that intentionally lack LLVM stage tools, use:

```sh
ncc mct llvm-toolchain-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-llvm-f6 \
  --all-targets \
  --opt-level 2 \
  --json \
  --allow-missing-tools
```

`--allow-missing-tools` does not turn missing evidence into success. It only
allows the audit command to exit after reporting `missing_toolchain` rows.

## Matrix

The closed F6 matrix covers the official external LLVM lanes:

- `nC.sysv64` -> x86_64 ELF through `ld.lld`;
- `nC.win64` -> x86_64 COFF through `lld-link`;
- `nC.aapcs64` -> AArch64 Mach-O through `ld64.lld`;
- `nC.portable` -> wasm32 WASI through `wasm-ld`.

Each row records:

- profile id;
- target triple;
- object format;
- emitted `.ll`;
- `.bc`;
- optimized `.bc`;
- object path;
- linked artifact path;
- per-stage command, arguments, stdout, stderr and status.

## Tool Discovery

F6 searches unversioned and versioned tools:

- `llvm-as`, `llvm-as-20`, `llvm-as-19`, `llvm-as-18`, `llvm-as-17`, `llvm-as-16`, `llvm-as-15`;
- `opt`, `opt-20`, `opt-19`, `opt-18`, `opt-17`, `opt-16`, `opt-15`;
- `llc`, `llc-20`, `llc-19`, `llc-18`, `llc-17`, `llc-16`, `llc-15`;
- `ld.lld`, `lld-link`, `ld64.lld`, `wasm-ld` with the same versioned suffix policy.

## Truth Boundary

Closed:

- public `llvm-toolchain-smoke` CLI;
- matrix manifest in `ncc mct manifest --json`;
- honest missing-tool classification;
- `llvm-as -> opt -> llc -> lld` command construction;
- per-row JSON evidence.

Not claimed:

- permanent CI evidence for every OS/architecture combination;
- full LLVM/GCC/rustc backend parity;
- every possible LLVM target triple;
- every LLD flavor option needed by nonofficial platforms.
