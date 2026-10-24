# ELF64 Object Emission

`ncc machine emit-elf <file.nm> --out <file.o>` emits a native ELF64 relocatable object through the nC object serializer for x86_64/aarch64 and through the machine minimal ELF writer for riscv64 smoke artifacts.

Verification checks object magic, size, and hash through `ncc machine verify-object`.

Evidence fixtures: `examples/machine/object_elf64_minimal.nm`, `examples/machine/aarch64_add_two_aapcs64.nm`, and `examples/machine/riscv64_add_two_elf.nm`.
