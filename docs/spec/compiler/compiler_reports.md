# Compiler Reports

This document defines the release-industrial reporting surface for `[@compiler]`.

## Commands

| Command | Output | Purpose |
|---|---|---|
| `ncc compiler doctor --json` | JSON | Global compiler health, phases, diagnostics, type catalog and industrial surfaces. |
| `ncc compiler diagnostics --json` | JSON | Stable compiler diagnostic catalog. |
| `ncc compiler report <file> --json` | JSON | Aggregated source, pipeline, type, contract, operator and diagnostics report. |
| `ncc compiler report <file> --md` | Markdown | Human-readable form of the same compiler report. |
| `ncc compiler types <file> --json` | JSON | Type catalog alignment and unknown-type diagnostics. |
| `ncc compiler type-contracts <file> --json` | JSON | Nominal `type{}` contracts and their clauses. |
| `ncc compiler ffi <file> --json` | JSON | FFI imports, symbols, ABI/layout/repr contracts and diagnostics. |
| `ncc compiler machine-interop <file> --target <triple> --json` | JSON | High-level `.n` to `[@machine]` ABI parity: native symbols, call ABI, structs, pointers, string descriptors, runtime symbols and aggregate returns. |
| `ncc compiler arrays <file> --json` | JSON | Array forms, index sites, array diagnostics and runtime calls. |
| `ncc compiler bounds <file> --json` | JSON | Bounds and index-contract diagnostics. |
| `ncc compiler memory-contracts <file> --json` | JSON | Memory, region, pointer and movement contract diagnostics. |
| `ncc compiler pointer-audit <file> --json` | JSON | Optional read-only raw-pointer hazard audit; never changes compilation or generated code. |
| `ncc compiler self-check --json` | JSON | Internal compiler self-check for key diagnostics and placeholder absence. |

## Required Fields

Reports must include the active source path and enough structured data to audit:

- pipeline status;
- source contexts;
- imports;
- runtime calls;
- type catalog alignment;
- type contracts;
- FFI contracts;
- array and bounds diagnostics;
- condition placeholder risk;
- diagnostics catalog size;
- strict-mode behavior where requested.

## Markdown Contract

Markdown reports are evidence artifacts. They must be non-empty, deterministic enough for review, and must not replace the JSON report as the machine-readable source of truth.

## Release Evidence

F.1 compiler closure stores per-point evidence under `reports/compiler_closure/` and the aggregate evidence in:

- `reports/compiler_closure/compiler_industrial_evidence.json`
- `reports/compiler_closure/compiler_industrial_evidence.md`
- `reports/compiler_closure/compiler_doctor_industrial.json`
- `reports/compiler_closure/compiler_doctor_industrial.md`

## Non-Goals

Compiler reports do not silently run missing runtime imports, do not hide parser/sema failures, and do not use machine or primitive lowering as a workaround for high-level compiler evidence.
