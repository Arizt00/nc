# Machine Third-Party Object Matrix

`ncc machine third-party-objects --json` is the Phase 13 producer-side matrix for real third-party relocatable objects consumed by the Phase 12 linker.

The matrix covers objects produced outside NCC and without `.machine-link-meta.json` sidecars. Each successful row must be imported through the native object reader, report `link_unit_origin: native_object`, resolve symbols through `phase12_link_image`, and apply or retain the declared relocation contract.

Required producers:

| Producer | Format | Required relocation families |
|---|---|---|
| `clang` | ELF64 | `R_X86_64_PLT32`, `R_X86_64_64` |
| `gcc` | ELF64 | `R_X86_64_PLT32`, `R_X86_64_64` |
| GNU `as`/gas | ELF64 | `R_X86_64_PLT32`, `R_X86_64_GOTPCREL`, `R_X86_64_64` |
| `nasm` | ELF64 | x86_64 call/PC-relative relocation, `R_X86_64_64` |
| `clang` | COFF | `IMAGE_REL_AMD64_REL32`, `IMAGE_REL_AMD64_ADDR64` |
| MSVC `cl` | COFF | `IMAGE_REL_AMD64_REL32`, `IMAGE_REL_AMD64_ADDR64` |
| MASM `ml64` | COFF | `IMAGE_REL_AMD64_REL32`, `IMAGE_REL_AMD64_ADDR64` |
| Apple `clang` | Mach-O | `X86_64_RELOC_BRANCH`, `X86_64_RELOC_UNSIGNED` |

Missing tools are not counted as success. They are reported as `blocked_by_environment`. CI uses `--strict-available`, which means every host-applicable tool that is available in the runner must compile, import, resolve, and satisfy its relocation contract.

Retained CI evidence is written per lane to:

`reports/machine_closure/ci/<lane>/third_party_object_matrix.json`

Generated objects and linked artifacts are retained under:

`target/tmp/machine_third_party_objects/<lane>/`
