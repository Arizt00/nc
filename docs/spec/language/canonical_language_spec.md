# Canonical Language Specification

Status: executable authority.

La forma fuente oficial esta definida en `canonical_compact_syntax_v3.md` y
reflejada directamente en `CANONICAL_GRAMMAR`. Las grafias anteriores son solo
entradas de compatibilidad para `nfmt`.

The canonical manifest has schema `nc.language.canonical.v1`. It is assembled
by `canonical_language_spec(pointer_width)` from structured compiler data and
is the source used to render grammar, type catalogs, runtime imports,
diagnostics and capability status. Historical prose cannot override it.

## Commands

```text
ncc language-spec --format json
ncc language-spec --format markdown
ncc language-spec --format ebnf
ncc language-spec --pointer-width 4 --format json
ncc language-spec --generate docs/generated/language
ncc language-spec --check docs/generated/language
```

`--generate` writes deterministic JSON, Markdown and EBNF. `--check` compares
all three byte for byte and reports `E_SPEC_DRIFT_001` on missing, stale or
manually edited output.

## Structured catalogs

- Grammar productions describe official program, import, container, type,
  statement, propagation and binary-layout surfaces.
- Scalar types are derived from the executable scalar type contract for the
  requested pointer width.
- Generic families describe fixed/dynamic arrays, collections, pointers,
  regions, algebraic values, functions, records and unions.
- Imports are copied from the runtime API family catalog used by resolution.
- Diagnostics and capabilities expose stable identifiers and owning phases.
- The diagnostics catalog is generated at build time from every canonical
  `E_*`, `W_*` and `I_*` code present in exact-head Rust sources. Curated
  descriptions enrich known codes without excluding the remainder.

The JSON manifest is suitable for IDEs, documentation generators, conformance
tests and release drift gates. Consumers should validate `schema`, `version`
and `pointer_width_bytes` before using target-dependent sizes.

## Authority rule

Changes that affect syntax, types, imports, diagnostics or declared capability
must update the structured manifest and regenerate its artifacts in the same
change. A release check fails when generated output differs from the manifest.

`.github/workflows/ncc-language-contract.yml` enforces this byte-for-byte gate
on Linux, Windows and macOS, records the exact commit and toolchains, runs the
parser/sema/IR/execution contracts and retains the resulting evidence.
