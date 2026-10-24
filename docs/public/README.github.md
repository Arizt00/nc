<p align="center"><img src="sites/nc-language/dist/mark.svg" width="82" alt="nC"></p>

# nC

**Typed continuity between the program, memory and machine.**

nC is an experimental language that integrates functions, foundational contracts, machine instructions and material primitives. Its compiler, **ncc**, is in native bootstrap. The design requires types, control flow and representation decisions to remain coherent through analysis, lowering and linking.

[![Status: bootstrap V81](https://img.shields.io/badge/status-bootstrap_V81-244c3b)](docs/public/estado.md)
[![License: MIT or Apache 2.0](https://img.shields.io/badge/license-MIT_or_Apache_2.0-e4edb1)](LICENSE)

[**Website and library**](https://nc-language-arizt00.rarepanda1024.chatgpt.site) · [**Native downloads**](https://github.com/Arizt00/nc/releases/tag/v0.1.0-bootstrap.81) · [**Language guide**](docs/public/origen-y-guia.md) · [**Architecture**](docs/public/arquitectura-y-bootstrap.md) · [**Contribute**](CONTRIBUTING.md)

## Why it exists

Layout, alignment, address validity and calling conventions affect observable program behavior. nC addresses these decisions within one semantic model. The root `[@]`, `[@machine]`, `primitive[@]`, runtime and linker participate in that contract.

Memory is manual/raw. Type checking does not add ownership, automatic release or implicit pointer validation. The [language guide](docs/public/origen-y-guia.md#why-nc-exists) explains the design.

## Run a program

Download your platform's kit from [Releases](https://github.com/Arizt00/nc/releases/tag/v0.1.0-bootstrap.81), verify its SHA-256 digest and extract it. The kit includes the compiler, runtime and `hello.n`. This workflow requires no Rust, Cargo or LLVM installation.

```nc
use <io:output>;

helloCompacto {
    to main {
        println("Hello, nC public beta");
    }

    fa:
        return true;
}
```

On Windows x86-64, run PowerShell from the extracted directory:

```powershell
.\ncc.exe self-test
.\ncc.exe compiler emit-executable hello.n --target x86_64-pc-windows-msvc --out hello.exe
.\hello.exe
```

On Linux x86-64 or WSL:

```bash
chmod +x ncc
./ncc self-test
./ncc compiler emit-executable hello.n --target x86_64-unknown-linux-gnu --out hello
chmod +x hello
LD_LIBRARY_PATH="$PWD${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./hello
```

Expected output: `Hello, nC public beta`. Exit code: `0`.

The Linux kit was tested on Ubuntu 24.04 under WSL and requires glibc 2.34 symbols or later. Keep its provider beside the compiler. The native CLI is partial: `--help`, `check` and `machine` still require Stage0. Read [installation](docs/install.md) before using other routes.

## Verified scope

| Area | Scope of this edition |
| --- | --- |
| First use | Isolated kits, self-test, hello and arithmetic executed on Windows and Linux/WSL x86-64 |
| V81 reconstruction | Byte-identical B2/C2 within each host; 74 units per reconstruction in the dossier |
| V81 evidence | 147 corpus programs and 38 runtime contracts per generation and host |
| Memory and ABI | Native alignments from 32 bytes to 1 MiB in the documented profile, subject to format limits |
| Open work | Global Stage3 promotion, full native CLI and physical execution of other ISAs |

V81 reconstruction results belong to the [technical dossier](docs/NCC_MODULES_V81_DOSSIER.md). [Publication validation](docs/public/validacion.md) records subsequent checks separately. This is an experimental prerelease.

## Documentation

- [Language guide](docs/public/origen-y-guia.md) · [PDF](docs/public/origen-y-guia.pdf).
- [Compiler architecture and bootstrap](docs/public/arquitectura-y-bootstrap.md) · [PDF](docs/public/arquitectura-y-bootstrap.pdf).
- [Syntax](docs/syntax.md), [runtime](docs/runtime.md), [machine](docs/machine.md), [primitives](docs/primitive.md) and [FFI](docs/ffi.md).
- [V81 reproduction](docs/NCC_MODULES_V81_REPRO.md), [ABI and layout](docs/NCC_MODULES_V81_ABI_LAYOUT.md) and [current limits](docs/public/estado.md).
- [Roadmap](ROADMAP.md) and [contribution guide](CONTRIBUTING.md).

## Working from source

The [V81 reconstruction recipe](docs/NCC_MODULES_V81_REPRO.md) identifies the seed, B/C generations, providers and artifact comparisons. The [build contract](bootstrap/frontend-v81-build.json) fixes its inputs.

Stage0 remains available for development and validation. Building it requires Rust/Cargo, a verified native seed and an archiver. The [installation guide](docs/install.md) documents that route separately. Invoke the built `ncc` executable directly for language checks.

| Directory | Contents |
| --- | --- |
| `tools/ncc/nc/src` | Compiler written in nC |
| `crates` | Retained Stage0 implementation and tools |
| `runtime/nc` | Runtime and providers |
| `examples` | Examples by subject |
| `bootstrap` | Build contracts and recipes |
| `docs` | Guides and technical references |
| `sites/nc-language/dist` | Static website |

## Contributing

A minimal failing program is useful evidence. Include the compiler digest, host, command, provider and expected result. [Issue templates](https://github.com/Arizt00/nc/issues/new/choose) identify the required details.

Language proposals must explain continuity through parser, AST, semantics and IR. Start with a reproducible behavior, a diagnostic or a documentation correction.

## License

The code is available under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your choice. Third-party notices retain their own terms.
