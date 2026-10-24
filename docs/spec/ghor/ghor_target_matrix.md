# ghor Target Matrix

`ghor` knows the following target triples:

- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `aarch64-unknown-linux-gnu`
- `riscv64-unknown-elf`
- `wasm32-unknown-unknown`

The host triple is marked `host_available`. Declared non-host targets are marked `declared_cross_target`. Known but undeclared targets are marked `known_supported_target`.

Cross-target planning is allowed without pretending that a cross compiler exists. Actual cross execution requires the corresponding host toolchain.
