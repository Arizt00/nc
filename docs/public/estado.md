# nC status

Updated 24 October 2026. Technical reference: V81.

nC is in native bootstrap. The first distribution provides a partial native compiler for Windows and Linux/WSL x86-64. Retained Stage0 is still required for other development routes.

## Retained evidence

The V81 dossier records identical B2/C2 digests within each host, 74 units per reconstruction, 147 corpus programs and 38 runtime contracts per generation and host. Native alignment materialization covers powers of two from 32 bytes to 1 MiB in the documented profile. COFF section alignment is limited to 8192 bytes.

See the [dossier](../NCC_MODULES_V81_DOSSIER.md), [reproduction recipe](../NCC_MODULES_V81_REPRO.md) and [ABI record](../NCC_MODULES_V81_ABI_LAYOUT.md). Evidence applies to identified generations, not automatically to every later commit.

## Release limits

- The native kit supports tested `self-test`, `compiler sema`, `compiler emit-ir`, `compiler emit-object` and `compiler emit-executable` routes.
- `--help`, `compiler --help`, `check` and `machine` can return `E_BOOTSTRAP_STAGE0_REQUIRED_001` with exit code 69.
- Linux requires its shared provider. Tutorial executables need `LD_LIBRARY_PATH` pointing to that provider's directory.
- Memory semantics remain manual/raw. Compilation does not establish pointer safety or insert automatic cleanup.
- Global Stage3 promotion remains open. This release does not replace the installed toolchain or certify physical execution of other ISAs.
- No cross-language performance comparisons are claimed. Dossier timings describe observed builds.

[Publication validation](validacion.md) records subsequent checks separately. Broader support requires a reproducible case, environment, artifacts and result.
