# V81: native alignment above 16 bytes

English edition of the retained V81 engineering record. The results below describe that milestone, not a new reconstruction performed during publication.

## Resolution

The native x86-64 profile was closed for Windows/Win64 and Ubuntu/WSL/SysV. All powers of two from 32 bytes to 1 MiB were executed for global and local storage. COFF section alignment is representable up to 8192 bytes. Direct PE and ELF admit larger alignments within this profile. Unrepresentable requests are rejected, not truncated.

The new implementation, builders and verifiers are written in nC. This milestone introduced no Rust, C, C++ or Python sources. Stage0 remains an AST oracle for the corpus, not the builder of B or C.

## Changes

| nC source | Material change |
| --- | --- |
| `native_body.n` | Aligned stack intervals, temporaries, arguments, returns, rsp restoration and page probing |
| `mct.n` | Maximum and per-symbol alignment; range, power-of-two and offset validation when reading materials |
| `mct_object.n` | Computed ELF sh_addralign and COFF characteristics, replacing the fixed value of 16 |
| `mct_link.n` | Data/BSS alignment relative to the final virtual address |
| `mct_image_elf.n` | Final section alignment published in ELF |
| `content_cache.n` | V81 identity preventing restoration of earlier emitter materials |

`align(N)` syntax is unchanged. Parser, AST, semantics and layout already accepted these requests; the backend now consumes their material contract. No ISA was added: the work uses existing x86-64 instructions.

## Reconstruction

Four final reconstructions were completed: B2 and C2 on each host. Each starts in a new directory and rebuilds 74 units with `project objects --no-incremental`, without LTO or retained project objects. Each generation also rebuilds 36 native providers on Windows or 35 on Linux and their distributable libraries.

| Host | B2 | C2 | Compiler identity |
| --- | --- | --- | --- |
| Windows | 572485 ms | 540828 ms | Identical SHA-256 |
| Linux/WSL | 475769 ms | 474393 ms | Identical SHA-256 |

Windows: `4ff2300d30fbe99a4c541481e0fab9df284d77c32d1df082084440b9718854c9`.
Linux: `e3d6fc744f0768d19a3f44ca93eddcb90cc3f89caee536f68f8f5ce486eae1e7`.
These are observed build times, not generated-code performance benchmarks against C.

## Execution and contracts

- Sixteen alignments: 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288 and 1048576 bytes.
- Initialized data, BSS, padding, pointer relocations and addresses observed in real processes.
- Overaligned locals, arrays, nested structs, unions and packed aggregates.
- Direct and indirect calls, aggregate returns, register pressure, scalar arguments preceding aggregates, nesting and recursion.
- A 1 MiB aggregate argument and return executed with explicit stack reserves on both hosts.
- Win64 by-reference passing and a realigned SysV argument area.
- COFF with LLD/MSVC and ELF with GNU ld. MSVC requires `/ALIGN:8192` for the 8192-byte COFF fixture; rejection without that option is retained.
- Explicit 16 MiB stack reserve for large Windows local-storage tests. The emitter probes pages without introducing CRT `__chkstk`.
- Real `RtlLookupFunctionEntry` and `RtlVirtualUnwind` APIs applied to linked COFF metadata and a test context. This does not claim a real exception at every prologue instruction.
- An independent x86-64 byte oracle written in nC and executed through RW -> RX memory, testing overaligned arguments in both directions.
- 310 layout checks across ten profiles per generation and host. These are layout calculations, not execution on ten architectures.
- 147 complete programs and 1472 checks per corpus run; four final runs. Windows executes 1176 commands per run and Linux 1323, including runner chmod commands.
- 38 runtime contracts per generation and host with exact stdout and exit codes, alongside V80 aggregate, signature and import regressions.

Warm-cache restoration recovers one AST and two materials in the overaligned project. Three corrupt materials per host are rejected even after their SHA-256 entries are recomputed in the cache manifest. The golden reader rejects an object copy with altered data.

## Evidence

The `docs/evidence/frontend81-r1` manifest maps logical paths to SHA-256-addressed payloads. Native equivalence reports record every compared byte and the first difference without normalization. Eight additional cross-host object/image pairs were checked. ELF images emitted on Windows ran in WSL; PE images emitted on Linux ran on Windows. An image altered at byte 31 is rejected and never executed.

Final runs and retries are identified separately in `NCC_MODULES_V81_ATTEMPTS.md`. Failed attempts are not rewritten as successes. The public source export does not include the full private evidence payload archive.

## Scope

This profile closure does not promote global Stage3, remove Stage0, replace the installed toolchain or certify physical execution of other ISAs. The inherited Git index was retained; clean reconstruction refers to generation sources, objects and caches, not a clean development commit.

Stack reserves and system limits remain explicit. Ownership, lifetime tracking and raw-pointer checks are not added. See `NCC_MODULES_V81_ABI_LAYOUT.md` for format and ABI limits and `NCC_MODULES_V81_REPRO.md` for commands.
