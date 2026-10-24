# ncc public beta

This beta surface is intentionally smaller than the development tree.

Public releases are generated through `ncc-release-v43 prepare-public-beta-layout`.
Build the native family using [the V43 build instructions](NATIVE_TOOLS_V43_COMMANDS.md).
The generator copies only the files declared in
`tools/release/public_beta_manifest.json` and excludes private archives, local
cache state, generated reports and transient build output.

Final `.zip` and `.tar.gz` archives are created by
`ncc-release-v43 package-public-beta` from `dist/public-beta/ncc` only. The
archive root is `ncc/`, so extracting either artifact does not expose the
development repository layout.

## First run

The first runnable example in the public package is:

```bash
ncc run examples/00_hello/hello.n
```

The official end-to-end validation command for the extracted package is:

```bash
ghor public-beta
```

That command runs:

```bash
cargo build -p ncc-cli
cargo test --workspace
ncc run examples/00_hello/hello.n
ncc project sync examples/08_project
ncc project run examples/08_project
```

## Closed for beta

- Public package generation and verification.
- Curated public examples under numbered folders.
- `examples/00_hello/hello.n` as the first smoke run.
- `examples/08_project` as the public multi-file project bridge example.
- Public fixture corpus for `cargo test --workspace`.
- Release binaries for Windows GNU and Linux GNU.
- Final source archives under `dist/public-beta/artifacts`.
- SHA256 evidence in `dist/release-bin/public-beta/SHA256SUMS.txt`.
- Release metadata in `dist/release-bin/public-beta/release-manifest.json`.
- `ncc --build-hash` on the generated Windows and Linux `ncc` binaries.

## Experimental or not yet complete

| Surface | Status | Honest limit |
| --- | --- | --- |
| `native-fast` | Experimental | It is not the universal default for all high-level programs. Some programs still need interpreter or LLVM/MCT-backed routes. |
| Full high-level backend | Experimental | Normal `.n` syntax is supported in the compiler, but not every high-level construct lowers to standalone native bytes yet. |
| Runtime deep OS APIs | Partial | Supported beta calls execute, but full FS/TCP/process/thread/memory/error parity is still being expanded. |
| Strings and dynamic collections in native paths | Partial | Work continues on native string storage, concat/render helpers, lists, vectors, dynarrays and tensors. |
| Pointers and heap in native paths | Partial | Pointer semantics exist, but complete compact pointer lowering and heap helper parity are still being closed. |
| MCT | Active contract | MCT is the bytes contract, not proof that every high-level feature already has complete native lowering. |
| `[@machine]` and basal | Public experimental surface | Typed machine/basal work is exposed, but not all OS/kernel-level workflows are production-stable. |
| `primitive[@]` | Public experimental surface | Primitive material paths exist, but target/device coverage and operational evidence are still expanding. |
| Project bridge physical linker | Partial | Logical project bridge works; complete C-style object/linker behavior is still on the roadmap. |
| macOS binaries | Not generated | No macOS/Darwin runner or validated cross-linker is available in this workspace. |

## Release binaries

Generated release binaries are placed under:

```text
dist/release-bin/public-beta
```

Current beta targets:

- `x86_64-pc-windows-gnu`
- `x86_64-unknown-linux-gnu`

macOS should only be announced after a real Darwin build and smoke test exists.

## Notes for contributors

The development repository may still contain historical names used by tests or
internal compiler notes. The public beta package exposes clean user-facing names
instead.

Do not mark a compiler surface as closed only because syntax parses. A surface is
closed only when parser, AST, sema, IR, tests and documentation agree.
