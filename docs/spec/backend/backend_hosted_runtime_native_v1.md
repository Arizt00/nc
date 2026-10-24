# Hosted runtime native ABI v1

## Purpose

Runtime calls used by normal `.n` programs are material backend calls. They do
not switch to the evaluator. MCT LLVM defines migrated families directly and
declares only the operating-system ABI they need. Legacy families may still
use a target object while their migration remains explicit in the manifest.

The JSON family follows the same rule. Its portable C ABI builds as ELF32,
ELF64, COFF x86_64/AArch64 and Mach-O x86_64/AArch64. The implementation uses
the target `size_t` width for allocation, string length and file I/O, so i386
does not leak stale high-register bits into length checks.

## Material operations

Filesystem and directory operations, process spawn/wait and thread spawn/join
are now LLVM/MCT providers and no longer exports of
`runtime_hosted_native.c`. Windows uses `CreateProcessA`, `SearchPathA`,
`CreateThread` and native handles. Executable lookup covers `.exe`, `.com`,
`.cmd` and `.bat`; scripts are launched explicitly through `cmd.exe /D /S /C`.
POSIX targets use `fork`, `execvp`, `waitpid` and the target pthread ABI.
Handles are raw allocated records consumed by wait/join.

## Result representation

`Result<T,E>` and `Option<T>` scalar payloads use one canonical tagged word:

```text
u64 bit 0      positive tag (Ok/Some)
u64 bits 1..63 signed scalar or pointer payload
```

Pattern matching, `isOk`, `isErr`, unwrap and payload bindings decode the same
layout. Function values passed to `threadSpawn` lower to real function pointers
and are called through the target ABI.

## Link contract

The retained native-build manifest records whether any legacy hosted object was
actually referenced. Process/thread-only programs record `referenced=false`,
`source=null` and `native_object=null`. PE64 is linked by Machine Phase 12.
Cross-Darwin builds use the internal Mach-O finalizer; Linux and native Darwin
use their system linker and pthread linkage when available.

The standard MCT route links every referenced runtime object in one operation:
machine entry, high-level body, JSON runtime and hosted runtime. A configured
external linker receives the same complete object list. Internal-linker
manifests derive runtime symbols from the final Phase 12 import report rather
than from a guessed catalog.

## Evidence

Execution tests cover process exit status, `.cmd` lookup, PowerShell quoting,
thread return value, `Result` matching and native stdout. Retained objects cover
COFF x86_64/i386, ELF x86_64/i386 and Mach-O AArch64. A successful object-only
cross build is not reported as loader execution. The Win32 provider is compiled
directly as COFF-i386 because `compiler build-native` does not yet admit a
canonical i686 Windows target profile.
