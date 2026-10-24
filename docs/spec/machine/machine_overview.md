# Machine Overview

`[@machine]` is the nC mode for declaring, validating, encoding, and emitting machine-level programs under explicit target, ABI, register, stack, symbol, relocation, object, verifier, and seal contracts.

It is not free-form assembly. Assembly can say what to emit; `[@machine]` must also prove target, ABI, operands, clobbers, flags, stack, symbols, relocations, encoding, and object format.

Current conformance is profile-scoped machine closure. See
[Machine Source Of Truth](machine_source_of_truth.md).

Closed:

| Surface | Status |
|---|---|
| `.nm` classification | closed |
| `ncc machine` CLI | closed |
| x86_64 register/operand/instruction model | closed for declared profile |
| SysV64/Win64 ABI reports | closed |
| AAPCS64 ABI contract | closed for declared profile |
| x86_64 encoding profile | closed |
| ELF64/COFF/Mach-O/WASM object emission | closed |
| Debug/unwind, disasm, roundtrip, cross-arch parity | closed for declared profiles |
| Macro/directive, linker, external compatibility and corpus phases | closed for declared profiles |
| high-level `fn:` / `fa:` rejection | closed |

Not claimed: arbitrary ISA/dialect/tool behavior outside declared profiles.
Updated retained multi-host CI artifacts remain pending until the workflow runs.
