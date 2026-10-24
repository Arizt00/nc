# ghor External Cargo Compatibility

Status: closed in native nC stage1.

This document defines the external compatibility gate between `ghor` and the
real host Cargo toolchain. The goal is to validate that Cargo-facing commands
produce compatible evidence for real `Cargo.toml` projects, while documenting
the differences that are accepted because `ghor` is an nC orchestrator.

## Command

```text
ghor external-compat --manifest-path Cargo.toml --json
```

`cargo-compat` is a compatibility alias for the same native stage1 authority.
An explicit manifest path that does not exist is rejected with
`E_GHOR_EXTERNAL_COMPAT_001` and exit code 2 before any foreign tool starts.

The command runs or computes the following comparisons:

- `cargo metadata --format-version 1 --no-deps` versus `ghor metadata`;
- `cargo tree` versus `ghor tree` JSON resolver evidence;
- `cargo package --list` versus `ghor package --list`;
- `cargo vendor` versus `ghor vendor`;
- `cargo update --dry-run` versus `ghor update --dry-run`.

## Project Shapes

The regression suite covers varied real Cargo projects:

- library plus binary target;
- package with `build.rs`;
- integration test targets under `tests/*.rs`;
- feature declarations;
- path dependency consumer;
- proc-macro crate.

## Accepted Difference Matrix

- Metadata: `ghor` wraps Cargo-compatible metadata under `ghor.metadata.v1`
  and adds nC machine/native fields.
- Tree: Cargo emits human text; `ghor` emits JSON resolver evidence.
- Package list: Cargo may include `Cargo.toml.orig` and VCS package details;
  `ghor` records nC package metadata and verification separately.
- Vendor: Cargo emits TOML source replacement text; `ghor` emits JSON plus
  Cargo-shaped checksum files.
- Update: Cargo updates `Cargo.lock`; `ghor` updates `ghor.lock` and
  `ghor.lock.meta.json`.
- External registry snapshot: Cargo projects can resolve crates.io through
  Cargo metadata, vendor and update evidence while `ghor` reports
  `E_GHOR_RESOLVER_VERSION_001` if no matching ghor registry snapshot is
  configured. This is accepted only inside `external-compat` as
  `accepted_registry_snapshot_gap`; it is not treated as invented ghor
  registry evidence.

Any difference outside that accepted matrix is treated as an external
compatibility error.

The comparison authority is `foreign_execution.n`; the recursive ghor side is
the same stage1 executable. Rust stage0 is not used unless
`NC_GHOR_FORCE_STAGE0=1` is selected explicitly.
