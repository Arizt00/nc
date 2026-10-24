# Backend LLVM/MCT: all LLVM attributes catalog

Status: closed for current upstream LLVM `Attributes.td` catalog coverage.

This document defines the ncc/MCT closure for LLVM attributes. The source of
truth is LLVM `llvm/include/llvm/IR/Attributes.td`, observed on 2026-06-15.
ncc records every current entry from that table in
`MctLlvmAttributeCatalogEntryIR`.

## Scope

Closed:

- `MctLlvmAttributeKindIR`.
- `MctLlvmAttributeCatalogEntryIR`.
- `MCT_LLVM_ATTRIBUTE_CATALOG`.
- `mct_llvm_attribute_catalog()`.
- `render_mct_llvm_attribute_kind()`.
- JSON report through `ncc mct llvm-attributes --json`.
- Embedded catalog in `ncc mct manifest --json`.
- Rejection of arbitrary third-party LLVM string function attributes outside
  the known LLVM catalog/target string allowlist.
- Tests that assert the catalog count and representative ABI, pointer,
  range, sanitizer and string-bool attributes.

## Catalog Fields

Each catalog row records:

- TableGen name.
- LLVM IR spelling.
- attribute kind: enum, int, type, string_bool, constant_range or
  constant_range_list.
- legal function position.
- legal parameter position.
- legal return position.
- ncc/MCT policy.

## Policies

`forwarded_when_requested` means the attribute is known and can be forwarded
when nC semantics, target lowering or an explicit verified request requires it.

`validated_parameterized` applies to attributes with integer, type or range
payloads. These require payload validation before emission.

`abi_or_pointer_contract` applies to attributes that affect ABI, pointer
provenance, return values, argument passing or low-level memory contracts.

`toolchain_sanitizer_or_hardening` applies to sanitizer and hardening
attributes that must be gated by toolchain support and build mode.

`coroutine_toolchain_forward` applies to LLVM coroutine-specific attributes.

`known_string_function_attribute` applies to string-bool attributes listed by
LLVM itself. This does not open arbitrary string attributes; a string attribute
must be cataloged or part of the closed target/codegen allowlist before it can
enter an emitted LLVM function attribute group.

## Truth Boundary

This closes catalog coverage, placement metadata and policy classification.
It does not mean that every LLVM attribute is emitted on every function. ncc
still emits attributes according to nC semantics, ABI, target profile, optimizer
mode and explicit lowering requests.

The target CPU universe and scheduler itinerary universe remain separate
concerns from this catalog closure. Third-party string attributes are rejected
until ncc gives them an explicit catalog entry and policy.
