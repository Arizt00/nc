# Machine External COFF Compatibility

COFF validation uses `dumpbin`, `llvm-readobj`, `llvm-objdump`, `file`, and hash tools when available.

The profile records headers, sections, symbols, relocations, and unwind visibility as secondary compatibility evidence.
