# Basal .nbb Format

Basal Fase 5 defines `.nbb` as a binary container named `NBB0`. It is not a JSON or text projection. The file starts with a fixed header, followed by a section table and binary sections for strings, root records, site records, bind records, lowering records, and pending evidence.

The writer is fed by the closed Fase 4 pipeline:

`.nb -> AST -> root/site/bind tables -> BasalIR -> verified lowering -> .nbb`

Machine evidence remains outside this format closure. Fase 5 stores a pending evidence section so later phases can integrate evidence without implying a final seal.

Basal Fase 6 keeps `NBB0` and upgrades the evidence payload in place. Pending artifacts keep `EVPN`; integrated artifacts use `EVID` plus a `SEAL` record. The header remains 64 bytes.

Required sections:

- `string_table`
- `root`
- `site`
- `bind`
- `lowering`
- `pending_evidence`

Optional compatible section added by Fase 5.5:

- `executable_surface`

This section keeps NBB0 magic, version, header size, and required section semantics unchanged.

Verification status values:

- `structurally_valid`
- `structurally_invalid`
- `unsupported_version`
- `checksum_mismatch`
- `section_out_of_bounds`
- `alignment_error`
- `invalid_magic`
