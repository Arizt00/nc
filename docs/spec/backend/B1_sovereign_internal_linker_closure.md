# nC - B1 Sovereign Internal Linker Closure

Status: Active Base (B1.10)
Version: v2.0.0-B1

## 1. Scope

This document freezes the sovereign internal linker stage for B1.10.

It closes:

- deterministic sovereign link execution for `build-sovereign`;
- internal relocation patching across linked sections;
- official entry-symbol resolution policy (default + override);
- CLI wiring for explicit sovereign entry selection.

## 2. Internal Link Entry Point

Implemented in `ncc-ir`:

- `link_image_sovereign(&BackendObjectIR, entry_symbol_override: Option<&str>) -> Result<Vec<u8>, String>`

Contract:

- requires lane `sovereign`;
- requires object format `freestanding-image`;
- enforces unresolved-symbol policy at backend stage;
- emits linked native image bytes with official header magic:
  - `NCSOVLNK`.

## 3. Deterministic Link Layout

Baseical constants:

- image base: `0x0010_0000`
- section alignment: `16`

Behavior:

- sections are placed in deterministic order from object sections;
- each section receives an aligned virtual address;
- symbol addresses are resolved from section base + symbol offset;
- image size is patched in header after serialization.

## 4. Relocation Coverage

B1.10 linker applies these relocation kinds:

- `CallNear32`
- `JumpNear32`
- `Abs64`

Baseical safety checks:

- relocation section must exist;
- relocation target symbol must resolve;
- offset bounds must fit patch width (`4` or `8` bytes);
- relative displacement and absolute result must fit numeric ranges.

## 5. Entry Symbol Resolution Policy

Resolution precedence:

1. explicit `--entry <symbol>` override;
2. global symbol `root.main`;
3. first global symbol ending in `.main`;
4. first global symbol;
5. first symbol in object;
6. hard error if object has no symbols.

Baseical diagnostics include:

- `sovereign entry symbol '<...>' not found in linked object symbol table`
- `sovereign link could not resolve default entry symbol; use --entry <symbol>`

## 6. CLI Integration

Implemented in `ncc-cli`:

- `build-sovereign <file.n> <out.ncsov> [--cc nC.sovereign] [--entry <symbol>]`

Behavior:

- defaults to sovereign profile when `--cc` is omitted;
- validates lane/artifact compatibility before link execution;
- routes to `link_image_sovereign(...)`;
- rejects unknown sovereign-only options with official diagnostics.

## 7. Baseical Diagnostics

Minimum diagnostics bound to B1.10 flow:

- `sovereign link execution requires lane 'sovereign' with object format 'freestanding-image'`
- `symbol unresolved at backend stage: <...>`
- `sovereign linker requires materialized native bytes for section '<...>' with relocations`
- `symbol '<...>' points to unknown section '<...>'`
- `relocation target section '<...>' is missing for sovereign link`
- `duplicate symbol '<...>' is not allowed in sovereign internal linker`
- `sovereign entry symbol '<...>' not found in linked object symbol table`

## 8. Conformance Evidence

Core tests:

- `backend_emit_ir_tests::link_image_sovereign_materializes_linked_header_and_entry_address`
- `backend_emit_ir_tests::link_image_sovereign_rejects_missing_entry_symbol_override`
- `backend_emit_cli_tests::parse_sovereign_command_options_defaults_to_sovereign_profile_without_cc`
- `backend_emit_cli_tests::parse_sovereign_command_options_accepts_entry_for_build_mode`
- `backend_emit_cli_tests::parse_sovereign_command_options_rejects_duplicate_entry`
- `backend_emit_cli_tests::parse_sovereign_command_options_rejects_unknown_option`
- `backend_emit_cli_tests::build_sovereign_with_sovereign_profile_materializes_linked_image`
- `backend_emit_cli_tests::build_sovereign_accepts_explicit_entry_symbol_override`
- `backend_emit_cli_tests::build_sovereign_rejects_unknown_entry_symbol`

Validation runs:

- `cargo test -p ncc-ir --lib -- --nocapture`
- `cargo test -p ncc-cli --lib -- --nocapture`
