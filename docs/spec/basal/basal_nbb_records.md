# Basal .nbb Records

Record sections preserve the verified Fase 4 lowering result:

- Root records store root id, root kind, target profile, and normalized metadata.
- Site records store site id, kind, size, alignment, permissions, owner root, target section, and lowering status.
- Bind records store source, destination, kind, local resolution, machine resolution, and relocation intent.
- Lowering records store target triple, machine profile, object format, lowered sections, relocations, and machine symbol references, including external ABI symbol references planned from Basal `foreign:` contracts.
- Pending evidence records store the explicit Fase 6 boundary.
- Executable surface records store const/routine/raw/verify/probe/report/mem summaries introduced by Fase 5.5 and Basal foreign binding summaries for C/C++/Rust/Zig/Go/ASM contracts. Foreign roots also carry a `foreign_ffi_manifest` surface record with `schema=ghor.ffi_manifest.v1`, the foreign entry count, and `ffi_link_manifest_hash`.
- Integrated evidence records may include an optional `EXTR` block before the final `SEAL` record. `EXTR` stores one `external_artifact` record per Basal `foreign:` contract: language, symbol, ABI, artifact kind, artifact hash, linked executable hash, and link status. The block is optional so older NBB0 files without foreign contracts remain readable.

The reader reconstructs these records into structured JSON for `read-nbb`,
`inspect-nbb`, and `nbb-info`, including an `ffi_link_manifest` summary derived
from the surface records.
