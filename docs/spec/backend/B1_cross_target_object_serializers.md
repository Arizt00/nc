# nC - B1 Cross-Target Object Serializers

Status: Active Base (B1.12)
Version: v2.0.0-B1

## 1. Scope

This document closes the remaining B1 backend gap for object emission breadth:

- native COFF serializer in `ncc-ir`;
- native Mach-O serializer in `ncc-ir`;
- deterministic section/symbol/relocation serialization path without external conversion tools.

This closure is about typed backend serialization, not external linker behavior.

## 2. Native serializer entrypoints

Implemented in `crates/ncc-ir/src/lib.rs`:

- `emit_object_coff(&BackendObjectIR) -> Result<Vec<u8>, String>`
- `emit_object_macho64(&BackendObjectIR) -> Result<Vec<u8>, String>`

These are first-class backend emitters, parallel to:

- `emit_object_elf64(...)`
- `emit_module_wasm32(...)`
- `emit_image_sovereign(...)`

## 3. Serialization guarantees

For both COFF and Mach-O serializers:

- section payload materialization is deterministic (`bytes` when present, otherwise official line-join text payload);
- symbol inventory includes defined locals, defined globals, and unresolved imports (lane policy permitting);
- relocation records are emitted from typed relocation IR;
- unresolved symbol rejection follows lane policy (`BackendUnresolvedSymbolPolicyIR`) and is enforced at backend stage where required.

## 4. Lane and artifact boundary

Native COFF/Mach-O serializers reject incompatible lanes with official diagnostics:

- portable lane -> artifact unavailable, use wasm path;
- sovereign lane -> artifact unavailable, use sovereign path.

This keeps lane identity explicit and prevents cross-lane semantic drift.

## 5. CLI integration

`ncc-cli` now uses native serializer bytes as the primary emission path for:

- `emit-coff`
- `emit-macho`

No external conversion stage is required for default operation.

## 6. Conformance lock

Minimum conformance evidence includes:

- `backend_emit_ir_tests::emit_object_coff_materializes_x86_64_header_and_symbol_table`
- `backend_emit_ir_tests::emit_object_coff_materializes_arm64_machine_when_profile_is_aapcs64`
- `backend_emit_ir_tests::emit_object_macho64_materializes_header_and_load_commands`
- `backend_emit_cli_tests::emit_coff_with_native_serializer_materializes_x86_64_header`
- `backend_emit_cli_tests::emit_macho_with_native_serializer_materializes_magic_header`

## 7. Closure note

B1.12 closes cross-target deep object serialization for the active matrix:

- ELF64
- COFF
- Mach-O
- wasm-module
- sovereign freestanding image

With B1.12, B1 has no remaining open backend serializer gap in the declared active matrix.
