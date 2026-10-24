# ghor Rust Integration

Rust adapters are declared with `primary = "rust"` or `adapters = ["rust"]` and `[language_adapter.rust]`.

`ghor` records Rust dependency sections and host `cargo` / `rustc` availability in the toolchain matrix. Actual Rust compilation uses the real Cargo toolchain when required by a project.

## Cargo-Backed Execution

`ghor` provides three Rust execution evidence commands:

- `ghor cargo-exec --manifest-path Cargo.toml --json`
- `ghor build-script --manifest-path Cargo.toml --json`
- `ghor proc-macro --manifest-path Cargo.toml --json`

Each command invokes real `cargo check` against the selected `Cargo.toml` and
captures:

- exit status;
- stdout/stderr SHA-256 hashes;
- whether `build.rs` or `package.build` is declared;
- whether `[lib] proc-macro = true` is declared;
- target directory used for the Cargo execution;
- mode-specific evidence such as build-script warnings or proc-macro
  validation through a consumer check.

This closes Rust build-script/proc-macro execution as an evidence surface
without pretending that nC reimplements Cargo or `rustc`.

## C ABI Staticlib Bridge

`ghor interop --json` closes the Rust side of the C/C++/Rust bridge:

- it creates an isolated Rust crate with `crate-type = ["staticlib"]`;
- it exports `ghor_rust_add` and `ghor_rust_pair_sum` as `extern "C"`;
- it uses `#[repr(C)]` for the shared record layout;
- it builds the crate with real `cargo build --release`;
- it links a C executable against the produced static library;
- it runs that executable and records artifact hashes.

Missing Cargo or Rust toolchains are reported as environment blockers. ghor
does not synthesize Rust evidence and does not replace `rustc`.

See `docs/spec/ghor/ghor_c_cpp_rust_interop.md`.
