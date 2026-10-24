# Basal .nbb Sections

Each section table entry is 64 bytes and records:

- section kind
- flags
- alignment
- offset
- size
- SHA-256 hash of the section payload

The verifier rejects unknown required layout, out-of-bounds sections, overlapping sections, invalid alignment, and section hash mismatches. Section payloads are binary records keyed through the string table.

Basal Fase 6 keeps section kind `6` as the evidence section. Its payload is either pending evidence (`EVPN`) or integrated machine evidence (`EVID`) followed by the final seal record (`SEAL`).
