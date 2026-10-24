# Machine External ELF64 Compatibility

ELF64 validation uses tools such as `readelf`, `llvm-readobj`, `objdump`, `llvm-objdump`, `file`, and hash tools when available.

The profile checks that emitted ELF objects are parseable by external readers while nC `verify-object` remains the internal authority.
