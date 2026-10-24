# Backend MCT Baseical Target Matrix F10

Status: closed for the ncc/MCT official matrix.

F10 closes the official target matrix used by ncc/MCT. This is the matrix that
the compiler owns directly when routing modal containers through the native and
LLVM-backed lanes. It is intentionally smaller than the full LLVM target
attribute universe: ncc/MCT closes the targets it promises, not every CPU,
subtarget, feature string, tuning knob, sanitizer policy, PGO profile shape or
microarchitecture variant that LLVM can express.

## Closed Rows

The closed matrix is exposed by `mct_official_target_matrix()` in `ncc-ir` and
by `ncc mct manifest --json` under `official_target_matrix_f10`.

| Profile | Triple | ABI | Object | Runtime | Tier |
| --- | --- | --- | --- | --- | --- |
| `nC.sysv64` | `x86_64-unknown-linux-gnu` | `sysv64` | `elf64` | hosted | tier1 |
| `nC.win64` | `x86_64-pc-windows-msvc` | `win64` | `coff` | hosted | tier1 |
| `nC.aapcs64` | `aarch64-apple-darwin` | `aapcs64` | `mach-o` | hosted | tier1 |
| `nC.portable` | `wasm32-wasi` | `wasi` | `wasm-module` | portable | tier1 |
| `nC.sovereign` | `x86_64-unknown-sovereign-nc` | `nc-sovereign` | `freestanding-image` | sovereign | tier3 |

## LLVM Attribute Policy

Each row binds to the existing F5 LLVM target-attribute selector:

- x86_64 rows use `target-cpu="x86-64"` with the baseline feature set
  `+cx8,+fxsr,+mmx,+sse,+sse2,+x87`.
- AArch64 uses `target-cpu="generic"` with `+neon`.
- Wasm32 uses `target-cpu="generic"` with `+mutable-globals,+sign-ext`.
- Hosted object formats get unwind table attributes where appropriate.
- Freestanding/sovereign rows keep their no-host boundary explicit.

## Tuning Boundary

F10 does not claim complete LLVM global tuning parity. The matrix records:

- `full_llvm_microarch_universe = false`
- `full_llvm_tuning_parity = false`

That boundary is part of the contract. ncc/MCT can add future target rows by
RFC-style changes, but F10 only closes the profiles listed above.

## Validation Surface

The closure is enforced by tests in `ncc-ir` and `ncc-cli`:

- `mct_official_target_matrix_closes_f10_without_global_llvm_tuning_claim`
- `mct_contract_manifest_is_honest_about_external_backends`

The JSON report includes target triples, ABI/object/runtime/debug/linker models,
LLVM CPU/features, smoke status, tuning policy and diagnostics for every row.
