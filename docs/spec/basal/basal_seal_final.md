# Basal Final Seal

The final basal seal is SHA-256 over deterministic seal inputs:

- NBB0 magic, version, and header size.
- Structural checksum.
- Root, site, bind, lowering, executable surface, and evidence hashes.
- Machine evidence hash.
- Seal policy version.

Host paths and command strings are kept as report metadata and excluded from deterministic seal inputs unless a profile explicitly chooses otherwise.

