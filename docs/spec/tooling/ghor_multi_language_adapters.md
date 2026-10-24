# ghor Multi-Language Adapters

`ghor` is the nC project forge. nC is its native language. C, C++, Rust, Java, Python, JavaScript and TypeScript are explicit foreign adapters declared in `nc.toml`.

The official manifest surface is:

```toml
[languages]
primary = "nC"
adapters = ["c", "cpp", "rust", "java", "python", "typescript"]

[language_adapter.c]
toolchain = "clang"
standard = "c17"

[language_adapter.cpp]
toolchain = "clang++"
standard = "c++20"

[language_adapter.rust]
toolchain = "stable"
cargo = true

[language_adapter.java]
toolchain = "javac"
build = "gradle"

[language_adapter.python]
version = "3.12"
venv = true
test = "pytest"

[language_adapter.typescript]
runtime = "node"
package_manager = "pnpm"
compiler = "tsc"
```

Adapters do not become native dependencies of `ghor`. They are explicit project capabilities. `ghor` records them in metadata, graph, lockfile language metadata, package evidence and audit surfaces.

Required diagnostics:

- `E_GHOR_LANGUAGE_UNSUPPORTED_001`
- `E_GHOR_LANGUAGE_ADAPTER_MISSING_001`
- `E_GHOR_HOST_TOOLCHAIN_MISSING_001`
- `E_GHOR_LANGUAGE_TEMPLATE_001`
- `E_GHOR_FOREIGN_BUILD_FAIL_001`
- `E_GHOR_FOREIGN_ARTIFACT_001`

The initial adapter scaffolds are created by `ghor new --lang <language>`. Foreign adapter execution is delegated to the declared host toolchain only when the project asks for that adapter.
