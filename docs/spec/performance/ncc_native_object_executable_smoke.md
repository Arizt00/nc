# ncc Native Object/Executable Smoke

Scope: native-fast artifact evidence.

This smoke closes the gap between `ncc run --native-fast` and real host artifact validation. The compiler must be able to emit a native ELF object, pass that object to the host linker, produce a native executable, run that executable directly, and compare both exit code and stdout.

Required path:

1. Parse/check/lower a native-fast `.n` fixture.
2. Run `ncc emit-elf <file.n> <out.o>`.
3. Verify the emitted bytes start with ELF magic.
4. Link the object with a tiny host C launcher through `cc`.
5. Verify the linked artifact starts with ELF magic.
6. Execute the linked binary directly, outside `ncc run`.
7. Assert exit code `0`.
8. Assert stdout is exactly `return=42\n`.
9. Assert stderr is empty.

Fixture:

- `examples/performance/native_fast/register_alloc_v2_liveness.n`

The fixture returns `42` through `nativeRegisterAllocationV2Perf.boot`. The launcher binds that symbol using an explicit asm symbol name and prints the returned value. This proves the object symbol, relocation, linker, executable loader, and native calling path are all live.

Boundary:

This smoke validates the hosted x86-64 ELF path available in the current environment. It does not claim every target object format or every linker flavor in the same test.
