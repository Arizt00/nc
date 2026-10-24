# Basal Machine Evidence

Basal Phase 6 integrates real machine evidence into NBB0 artifacts. The evidence is produced by the closed `[@machine]` lane and records command status plus hashes for machine doctor, native doctor, object emission, verify-object, disasm, and roundtrip.

Required evidence fields:

- `machine_target`
- `object_format`
- `asm_complete_total=true`
- `lowering_hash`
- `executable_surface_hash`
- `artifact_hash`
- `verify_object_hash`
- `disasm_hash`
- `roundtrip_hash`
- `source_span_mapping_hash`

The evidence record is binary NBB0 section data, not JSON text stored as a binary substitute.

