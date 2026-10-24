# Backend LLVM/MCT F5: target attributes

Status: closed for the ncc official LLVM target-attribute matrix.

F5 makes target-specific LLVM attributes explicit. Emitted MCT LLVM functions
now carry a function attribute group, and the module reports that group through
`MctLlvmTargetAttributesIR`.

The later all-attributes closure adds a separate LLVM `Attributes.td` catalog:
`MctLlvmAttributeCatalogEntryIR` records every current upstream LLVM attribute,
its IR spelling, attribute kind and legal function/parameter/return placement.
That catalog is exposed by `ncc mct llvm-attributes --json` and embedded in
`ncc mct manifest --json`.

F10 builds on this by exposing the ncc/MCT official target matrix as a first
class report. F5 owns per-target LLVM attributes; F10 owns the closed profile
rows and the explicit boundary that ncc/MCT does not claim the entire LLVM
microarchitecture/tuning universe.

## Closed Surface

- `MctLlvmFunctionAttributeIR`.
- `MctLlvmAttributeKindIR`.
- `MctLlvmAttributeCatalogEntryIR`.
- `MctLlvmTargetAttributesIR`.
- `MctLlvmModuleIR.target_attributes`.
- `mct_llvm_attribute_catalog()`.
- `mct_llvm_target_attributes_for_spec(spec)`.
- closed validation for emitted function attributes, including rejection of
  arbitrary third-party string attributes outside the known LLVM catalog.
- LLVM `attributes #0 = { ... }` emission.
- Function definitions using the target attribute group.
- Hosted `main` wrapper using the same attribute group.
- JSON reporting for:
  - target triple;
  - object format;
  - ABI;
  - debug model;
  - target CPU;
  - target features;
  - function attributes;
  - complete `Attributes.td` catalog;
  - rendered LLVM attribute group.

## Baseical Matrix

The F5 matrix covers the official targets already modeled by ncc:

- x86_64 SysV64 ELF;
- x86_64 Win64 COFF;
- AArch64 AAPCS64 Mach-O;
- AArch64 SysV64 ELF through the target spec matrix;
- wasm32 WASI;
- x86_64/aarch64 freestanding;
- x86_64 nC sovereign.

## Attribute Policy

Common emitted attributes include:

- `nounwind`;
- `"target-cpu"`;
- `"target-features"` when the target has a non-empty feature set;
- `"frame-pointer"="all"`;
- `"min-legal-vector-width"="0"`;
- `"stack-protector-buffer-size"="8"`;
- `uwtable` for hosted object formats.

The current x86_64 hosted baseline uses:

```llvm
attributes #0 = { nounwind "target-cpu"="x86-64" "frame-pointer"="all" "min-legal-vector-width"="0" "stack-protector-buffer-size"="8" "target-features"="+cx8,+fxsr,+mmx,+sse,+sse2,+x87" uwtable }
```

The complete LLVM attribute catalog follows this policy:

- enum attributes are known and forwarded only when nC semantics or an explicit
  lowering request makes them legal;
- integer/type/range attributes are marked `validated_parameterized` and must
  validate their payload before emission;
- ABI and pointer-contract attributes such as `byval`, `sret`, `nonnull`,
  `dereferenceable`, `signext` and `zeroext` are grouped under
  `abi_or_pointer_contract`;
- sanitizer/hardening attributes are grouped under
  `toolchain_sanitizer_or_hardening`;
- LLVM string-bool attributes from `Attributes.td` are known function
  attributes, not arbitrary unvalidated strings.
- ncc-owned metadata, such as CPU tuning profile identity, is reported through
  MCT fields and JSON reports instead of being smuggled into LLVM as
  third-party string attributes.

## Evidence

```sh
ncc mct emit-llvm examples/backend/mct_llvm_target_attributes.n \
  --out target/tmp/mct-llvm/mct_llvm_target_attributes.ll \
  --cc nC.sysv64 --opt-level 2 --json

ncc mct llvm-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-llvm/target-attrs-smoke \
  --cc nC.sysv64 --opt-level 2 --json

ncc mct llvm-exe-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-llvm/target-attrs-exe \
  --cc nC.sysv64 --opt-level 2 --json

ncc mct llvm-attributes --json
```

## Truth Boundary

Closed:

- deterministic target attribute selection for ncc official targets;
- complete current LLVM `Attributes.td` catalog coverage;
- function/parameter/return legality metadata for every catalog entry;
- policy classification for emission/forwarding/validation;
- rejection of arbitrary third-party string attributes outside the known LLVM
  catalog;
- real LLVM function attribute group emission;
- JSON reporting;
- Clang object/executable smoke for the SysV64 hosted lane.

Not claimed:

- the complete universe of LLVM target attributes for every CPU microarchitecture;
- global LLVM microarchitecture/tuning parity beyond the F10 official matrix;
- unconditional emission of every LLVM attribute;
- multi-host object/link smoke for every target row in this F5 change alone.
