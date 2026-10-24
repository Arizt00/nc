# Machine VEX Table Backbone

Fase 1A defines the table backbone used to close x86_64 VEX support entry by entry. It does not close full VEX support and it does not start AVX2.

Each VEX instruction has a `VexInstructionDef` with mnemonic, aliases, family, extension, VEX kind, opcode map, opcode, prefix fields, operand forms, register classes, memory and immediate width, feature gates, flags, profile restrictions, encoding status, verifier status, decoder metadata status, tests status, docs status and final status.

The table keeps the Phase 1 manifest total at 104 entries. Current status remains 0 closed complete instructions, 7 partial entries, 92 deferred entries and 5 unsupported-by-design entries.

The CLI exposes the table through:

- `ncc machine vex manifest --json`
- `ncc machine vex table --json`
- `ncc machine vex coverage --json`
- `ncc machine vex diagnostics --json`

The backbone is closed when every manifest entry maps to a table definition and every unsupported-by-design entry has an explicit rejection diagnostic.
