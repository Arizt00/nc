# Runtime import: `use <text:unicode>;`

## Contract

This explicit import exposes the Unicode 17.0.0 text provider. The provider and
its generated lookup tables are nC source compiled in the same linked AST and
MCT/LLVM module as the caller. It does not compile or link a C implementation.

## Surface

| Function | Signature | Ownership |
|---|---|---|
| `isValidUtf8` | `(str) -> bool` | borrows input |
| `normalizeUnicode` | `(str) -> str` | returns owned NFC buffer |
| `normalizeNfc` | `(str) -> str` | returns owned buffer |
| `normalizeNfd` | `(str) -> str` | returns owned buffer |
| `normalizeNfkc` | `(str) -> str` | returns owned buffer |
| `normalizeNfkd` | `(str) -> str` | returns owned buffer |
| `stripInvalidUnicode` | `(str) -> str` | returns owned buffer |
| `unicodeMode` | `() -> str` | returns static storage |
| `supportsGrapheme` | `() -> bool` | no allocation |
| `graphemeCount` | `(str) -> int` | borrows input |
| `graphemeSlice` | `(str, int, int) -> str` | returns owned buffer |
| `unicodeRelease` | `(str) -> bool` | releases an owned result |

Owned results use explicit raw virtual-memory storage. The caller must invoke
`unicodeRelease` exactly when desired; no destructor, garbage collector or C
allocator is inserted. Invalid pointers retain the language's raw behavior.

## Semantics

- UTF-8 validation rejects overlong forms, surrogates, truncated sequences and
  values above `U+10FFFF`.
- `stripInvalidUnicode` removes invalid bytes and preserves valid sequences.
- Normalization implements NFC, NFD, NFKC and NFKD, including canonical order,
  composition exclusions and algorithmic Hangul processing.
- Grapheme operations implement extended clusters, including Indic conjuncts,
  emoji ZWJ sequences and regional-indicator pairing.

## Generated data

`tools/unicode_codegen.rs` consumes the retained official UCD files under
`runtime/unicode/17.0.0` and generates `runtime/nc/unicode_tables.n`. The tool
also validates every official normalization and grapheme-break test before it
writes the provider tables.

Official references:

- https://www.unicode.org/reports/tr15/
- https://www.unicode.org/reports/tr29/
- https://www.unicode.org/Public/17.0.0/ucd/

## Evidence

```text
rustc --edition=2021 tools/unicode_codegen.rs -O -o target/unicode_codegen.exe
target/unicode_codegen.exe .
cargo test -p ncc-ast stoud_text_unicode_is_manual_nc_and_caller_owned
cargo test -p ncc-sema --test compact_text_unicode
cargo test -p ncc-ir stoud_text_unicode_lowers_every_export_to_linked_nc_definitions
cargo test -p ncc-cli nc_unicode_provider_is_generated_and_linked_without_c_runtime
ncc compiler build-native examples/runtime/families/unicode_basic.n \
  --out-dir target/evidence/imports/stoud-text-unicode-20260802-final \
  --target x86_64-pc-windows-msvc --profile release-fast
```

The conformance result is retained in
`runtime/unicode/17.0.0/CONFORMANCE.json`.
