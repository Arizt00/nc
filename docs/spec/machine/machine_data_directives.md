# Machine Data Directives

Phase 10 closes native data directives:

- `db`, `dw`, `dd`, `dq`
- `ascii`
- `asciz`

Values are compile-time expressions, width checked, endian-aware, and emitted into object `.data` or WASM custom data. Overflow reports `E_MACHINE_DATA_OVERFLOW_001`.

Data labels are emitted as object symbols so verify-object, disasm, and roundtrip can preserve the semantic surface.
