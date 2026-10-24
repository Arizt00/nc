# nC V81 - Native bootstrap kit

This prerelease provides an experimental native compiler for Windows and Linux x86-64. It includes the matching runtime provider, hello and arithmetic examples, licenses and SHA-256 checksums. The first program requires no Rust, Cargo or LLVM installation.

## Download and run

Select `nc-bootstrap-v81-windows-x86_64.zip` or `nc-bootstrap-v81-linux-x86_64.tar.gz`. Compare the downloaded archive with `SHA256SUMS.txt` before extraction.

Windows PowerShell:

```powershell
.\ncc.exe self-test
.\ncc.exe compiler emit-executable hello.n --target x86_64-pc-windows-msvc --out hello.exe
.\hello.exe
```

Linux or WSL:

```bash
chmod +x ncc
./ncc self-test
./ncc compiler emit-executable hello.n --target x86_64-unknown-linux-gnu --out hello
chmod +x hello
LD_LIBRARY_PATH="$PWD${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./hello
```

Expected output: `Hello, nC public beta`. Exit code: `0`. Linux was tested on Ubuntu 24.04 under WSL; kit imports require glibc 2.34 symbols or later. Keep the included runtime beside the compiler.

## Documentation

- [Website and library](https://nc-language-arizt00.rarepanda1024.chatgpt.site).
- Language Guide PDF: 9 pages.
- Compiler Architecture and Bootstrap PDF: 9 pages.
- [Installation and source builds](https://github.com/Arizt00/nc/blob/main/docs/install.md).
- [Publication validation](https://github.com/Arizt00/nc/blob/main/docs/public/validacion.md).

The website, manuals, download instructions and contribution templates are in English. Previously published chapter URLs redirect to their English equivalents.

## Source correction

The `ncc-ast` string-catalog test expected 26 exports after `charFromCode(int) -> str` had already been added. The corrected test compares import-lowering symbols with the typed API catalog, rejects duplicates and explicitly requires `charFromCode`. All 118 `ncc-ast` tests pass. The string fixture also passes direct `ncc` analysis, IR generation and execution; invalid release calls are rejected.

This revision updates documentation, package instructions and a source regression test. The kit compiler and runtime retain their V81 B2 binary identities. Updated archive and PDF checksums are published in `SHA256SUMS.txt`; the original release tag remains unchanged.

## Scope

V81 exposes a partial bootstrap CLI. Tested routes include `self-test`, `compiler sema`, `compiler emit-ir`, `compiler emit-object` and `compiler emit-executable`. `--help`, `check` and `machine` can return `E_BOOTSTRAP_STAGE0_REQUIRED_001` with exit code 69.

V81 evidence records byte-identical B/C compilers within each host. Stage0 is retained and global Stage3 promotion remains open. The kit does not replace an existing installation or certify other ISAs. Memory remains manual/raw.

The [validation record](https://github.com/Arizt00/nc/blob/main/docs/public/validacion.md) distinguishes retained reconstruction evidence from newly executed tests and records outstanding failures. No global pass is implied by the AST correction.

Report issues with compiler and provider identity, host, command, minimal source and expected result.