# ghor

`ghor` is the Cargo-like orchestrator for nC projects and the universal project forge for explicit language adapters.

Base: `ghor` nace desde `[@machine]`.

`ghor` does not destructively replace `ncc`, `ncup`, `npkg`, `nbuild`, `nfmt`, or `nlint`. It coordinates them under one project workflow:

- project creation with `nc.toml`;
- manifest validation and metadata;
- build graph planning;
- check/build/run/test orchestration;
- lockfile generation and verification;
- package and local registry workflows;
- formatter, linter, docs, install, audit, evidence, seal, and release checks;
- native pass-through to `machine`, `basal`, `runtime`, `compiler`, and `primitive`;
- native nC project creation plus explicit C, C++, Rust, Java, Python, JavaScript and TypeScript adapter templates;
- `ghor lang new` language-forge scaffolds for nC and supported host adapters.

nC is the native language of `ghor`. Foreign languages are adapters declared in `[languages]` and `[language_adapter.*]`, never implicit replacements for nC.

The command is available both as `ghor` and as `ncc ghor`.

`ghor doctor` compares the repository toolchain with an installed `ncc` when available. If the installed toolchain reports stale basal or machine state while the repository reports `basal_closed=true`, it emits `E_GHOR_TOOLCHAIN_STALE_001`.
