# Canonical machine diagnostic ABI v2

## Authority

The material implementation is `runtime/nc/diagnostic.n`. It writes a fixed
128-byte record with raw `wr<T>` operations. Text is stored in a caller-owned
UTF-8 arena and records contain offsets, never process pointers.

The structured layout is
`docs/spec/diagnostics/canonical_machine_diagnostic_abi_v2.json`.

## Continuity

The same record crosses primitive, basal, machine, lexer, parser, sema, IR,
MCT, linker, runtime, kernel, high-level, IDE and ghor boundaries. A layer may
add `expected`, `found`, `help` or causes. It must preserve code, stable id,
phase, source identity and the primary span.

Recoverable runtime failures remain normal `Result` values until an explicit
reporting boundary converts them into this record. Raw operations do not gain
bounds, lifetime, null, overflow or ownership checks from this ABI.

## Human rendering

```text
error[E_TYPE_CONTRACT_SECTION_001] parser.type
 --> gestor.n:5:13
5 | nombre >> str;
  |             ^
 = invalid type contract section
```

Paths belong to the emitted occurrence. They are not stored in catalog
descriptions, so an installed compiler never refers to its source checkout.

## Machine representation

Bytes 0..23 form the fixed header. Seven `text_ref` fields occupy bytes
24..79. A `text_ref` is a little-endian `u32` arena offset followed by a `u32`
byte count. The source span occupies bytes 80..95, cause linkage 96..103 and
bytes 104..127 are reserved and zero.

`stable_id` is FNV-1a 64 over the diagnostic code. It is a compact identity for
machine transport; the full textual code remains authoritative.

## Stage0 boundary

Lexer and parser failures happen before user machine code exists. The current
Rust Stage0 frontend therefore creates the same binary record mechanically and
renders it. Runtime, kernel and `.nm` code can write the record directly. This
does not claim that a parser error executed inside the failed program.

NIDE keeps the compatible `nide.diagnostics.v1` envelope and adds
`diagnostic_abi`, `stable_id`, `machine_record_hex` and `machine_arena_hex`.
It no longer replaces a real `E_*` code with a generic IDE code.

## Producer integration

MCT, Machine, Basal and ghor store `CanonicalDiagnostic` directly behind thin
compatibility constructors. Their terminal summaries and JSON reports consume
the same typed severity, domain, stable id and machine packet. The 67 former
MCT struct literals now call `MctDiagnosticIR::new`; that type contains only
the canonical record. `MctDiagnosticSeverityIR` remains a source-compatible
constructor enum and is immediately normalized to the ABI severity.

LLVM, ABI, object, linker and optimizer codes select a stable MCT subphase
before packet encoding.

All structured CLI entries use `canonical_diagnostic_to_json`. The entry keeps
the familiar `code`, `message` and `severity` keys and adds the complete ABI
fields plus exact record and arena hex. Basal additionally preserves its byte
offset span under `span`.

Streaming and persistence use the lossless JSONL and `.ndiag` formats defined
in `canonical_diagnostic_transport_v1.md`. Terminal and LSP are presentation
adapters over that same canonical record.

## Required validation

- ABI unit tests assert every fixed offset and the 128-byte encoding.
- CLI tests assert code, subphase, file, line, column and caret.
- NIDE tests assert the real code and exact 256-character record hex.
- MCT, Machine, Basal and ghor tests assert domain, phase, severity and the
  `NCDI` v2/128-byte record header.
- `runtime/nc/diagnostic.n` builds through `.n -> .nm -> .obj -> .exe`.
- `examples/machine/diagnostic_record_v2_x86_64.nm` emits an autocontained
  ELF64 object/executable and validates the header through native loads/stores.
