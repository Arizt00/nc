# Machine External Compatibility Overview

Phase 13 validates artifacts emitted by `[@machine]` with external tools when those tools are present. The nC verifier, disassembler, roundtrip engine, and linker remain the authoritative contract.

Statuses are `compatible_ok`, `compatibility_warning`, `compatibility_fail`, `blocked_by_environment`, `unsupported_by_tool`, and `skipped_by_profile`.

Phase 14 corpus hardening is separate.
