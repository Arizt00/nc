# Basal .nbb Verifier

Basal Fase 6 adds seal verification on top of structural NBB0 verification. `verify-nbb` may accept pending evidence for structural workflows; `verify-seal` requires integrated evidence and a valid seal record.

`ncc basal verify-nbb <file.nbb> --json` validates:

- magic and version
- endian marker and header size
- total file size
- structural checksum
- section table bounds
- section offsets, sizes, alignment, overlap, and hashes
- required record sections
- root/site/bind/lowering record parseability
- pending evidence state
- cross-section references for local site binds

Corrupt artifacts fail with official `E_NBB_*` diagnostics.
