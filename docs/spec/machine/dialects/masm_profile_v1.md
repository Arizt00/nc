# nC-masm-compat-v1

Status: closed.

This profile covers the MASM subset declared by nC: `PUBLIC`, `EXTERN`, `PROC/ENDP`, `.CODE/.DATA` section mapping, `DB/DW/DD/DQ`, full `DUP` expansion/lowering, `EQU`, `ALIGN`, `ORG` where target policy allows it, `BYTE/WORD/DWORD/QWORD PTR`, Intel memory addressing, local labels, comments, import, export, and import-back.

`DUP` is a closed deep feature in this profile. Supported forms include literal counts, `EQU` counts, CTFE counts, nested `DUP`, lists, byte strings, DB/DW/DD/DQ width handling, uninitialized `?` reserves, overflow diagnostics, invalid count diagnostics, malformed-parenthesis diagnostics, expansion limits, expansion reports, MachineIR lowering, object emission through data/BSS directives, export, and import-back.

The importer never keeps `DUP` as opaque text. Initialized forms lower to official nC data directives; reserve forms lower to official BSS reserve directives. Export may spell semantically equivalent reserves as MASM `DUP(?)`, and import-back must report lossless or semantic-equivalent behavior without silent loss.

`INVOKE`, `PROTO`, and high-level MASM runtime predicates are unsupported by design and must be rejected with diagnostics.
