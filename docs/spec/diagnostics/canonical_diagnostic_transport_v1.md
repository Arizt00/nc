# Canonical diagnostic transport v1

## Purpose

The canonical diagnostic has two lossless stream representations:

- JSONL uses one `nc.diagnostic.jsonl.v1` object per UTF-8 line.
- `.ndiag` uses the binary `nc.diagnostic.ndiag.v1` container.

Both carry the same severity, domain, code, phase, source identity, span,
expected/found/help values, flags and cause linkage. JSONL also exposes the
exact `NCDI` record and arena as hexadecimal strings.

## `.ndiag` file header

All integers are little-endian. The 32-byte header is:

| Offset | Bytes | Field |
| ---: | ---: | --- |
| 0 | 8 | magic `NDIAG\0\0\0` |
| 8 | 2 | version, currently 1 |
| 10 | 2 | header bytes, 32 |
| 12 | 2 | `NCDI` record bytes, 128 |
| 14 | 2 | frame-header bytes, 16 |
| 16 | 4 | flags, currently zero |
| 20 | 4 | frame count |
| 24 | 4 | payload bytes |
| 28 | 4 | FNV-1a 32 checksum of the complete payload |

## Frame

Every frame is aligned to eight bytes and contains:

| Offset | Bytes | Field |
| ---: | ---: | --- |
| 0 | 4 | magic `NDFR` |
| 4 | 4 | total frame bytes, including padding |
| 8 | 4 | UTF-8 arena bytes |
| 12 | 4 | FNV-1a 32 checksum of record plus arena |
| 16 | 128 | canonical `NCDI` v2 record |
| 144 | variable | UTF-8 arena |
| end | 0..7 | zero padding to eight-byte alignment |

The decoder rejects trailing bytes, truncation, invalid sizes or alignment,
unknown severity/domain values, bad UTF-8 references, non-zero padding,
checksum mismatches and a `stable_id` that does not equal FNV-1a 64 of `code`.

## CLI

```text
ncc check source.n --diagnostic-format terminal
ncc check source.n --diagnostic-format jsonl
ncc check source.n --diagnostic-format ndiag --diagnostic-out source.ndiag
nide diagnostics source.n --format jsonl --out source.jsonl
nide diagnostics source.n --format ndiag --out source.ndiag
nide diagnostics source.n --format terminal
```

`--diagnostic-format terminal|jsonl|ndiag` and `--diagnostic-out <path>` are
global `ncc` options. They may appear before or after the command, but options
following `--` belong to the executed program. Without `--diagnostic-out`,
terminal diagnostics use stderr and JSONL/NDIAG use stdout. A file sink never
contains ANSI styling.

An `.ndiag` output extension selects binary format when `--format` is omitted.
Binary stdout is supported without an appended newline.

The process boundary carries `CliFailure`, an ordered list of canonical
diagnostics plus source text for terminal excerpts. Lexer, parser and sema
failures from `check`, `run`, `run-fn`, `emit-ast` and `emit-ir` reach that
boundary without first being rendered. Legacy command families pass through a
compatibility adapter until their internal error signatures are migrated.

The public command boundaries for `nfmt`, `nlint`, `nide`, `nclsp`, `npkg`,
`package` and `ghor` also return `CliFailure`. Their phases are respectively
rooted in `tooling`, `ghor` or `package`, producing ABI domains 12 and 13.
`nfmt`, `nlint` and the complete `package` call graph no longer have private
legacy entry helpers. Private adapters remain in the other listed families
while their individual producers are converted.

The legacy bridge recognizes both bare codes (`E_*`, `W_*`, `NCC-*`) and
bracketed codes such as `[NCC-CLI-0001]`. Brackets are presentation syntax and
are never part of the stored code or its stable identity.

`actionable_command_error` constructs `CommandDiagnosticError` directly. Its
canonical payload stores code `NCC-CLI-0001`, an owning command phase, message,
help and optional file separately. Its compatibility `Display` retains the old
bracketed line for internal callers, but the process boundary downcasts the
typed value and never parses that display string.

## External causes

Filesystem, JSON, subprocess and backend failures use typed parent/cause pairs.
The parent is an error with `cause_index = 1` and `cause_count = 1`; the second
record is a `note` containing the underlying provider error. Both records keep
the owning phase and optional path. JSON failures additionally retain the
one-based line and column supplied by the JSON parser.

| Kind | Parent | Cause |
| --- | --- | --- |
| filesystem | `E_CLI_FILESYSTEM_001` | `N_CLI_FILESYSTEM_CAUSE_001` |
| JSON | `E_CLI_JSON_001` | `N_CLI_JSON_CAUSE_001` |
| subprocess | `E_CLI_SUBPROCESS_001` | `N_CLI_SUBPROCESS_CAUSE_001` |
| backend | `E_CLI_BACKEND_001` | `N_CLI_BACKEND_CAUSE_001` |

`nfmt`, `nlint` and `package` now have direct `CliFailure` call graphs and no
private `*_legacy` entry wrapper. Every package filesystem, JSON, contract and
process operation is typed. The MCT zero-function rejection and ghor's central
external-tool launcher also construct typed causes before crossing their
remaining compatibility boundaries.

## LSP and terminal

`nc-sys-lsp` seals every publication with the canonical ABI metadata before
mapping it to LSP. LSP ranges are zero-based; the stored source span remains
one-based. `data` preserves the canonical fields and `source` is `ncc`.

Terminal output is a presentation of the same record. ANSI mode changes only
styling. `NO_COLOR` and truthy `NC_NO_COLOR` disable colors; `CLICOLOR_FORCE`
can force them for a non-interactive stream.

Normal `ncc check` and `ncc run` lexer, parser and sema failures use this same
renderer; Machine, Basal and ghor summaries do as well.
