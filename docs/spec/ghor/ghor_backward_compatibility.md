# ghor Backward Compatibility Policy

Status: closed for the Cargo-grade nC orchestrator surface.

`ghor` follows a Cargo-like compatibility posture without pretending to be the
whole crates.io ecosystem. Compatibility means that existing nC package projects
remain readable, lockfiles remain auditable, registry bytes remain verifiable,
and Cargo-facing metadata is stable enough for external tooling.

## Stable Inputs

The following manifest names are supported:

- `ghor.toml`
- `nc.toml`
- `Cargo.toml`

`Cargo.toml` support is additive. It does not remove the nC manifest surface and
does not change the internal nC package contract.

## Stable Lock Contract

`ghor.lock` is the primary Cargo.lock-style TOML lockfile. It preserves package
identity, version, source, checksum and dependencies.

`ghor.lock.meta.json` is the nC-rich sidecar. It preserves:

- resolver report;
- target/capability metadata;
- machine/basal/runtime/compiler/primitive orchestration metadata;
- hash of the TOML lock text;
- compatibility metadata needed by nC release tooling.

Future lock changes must keep older lockfiles readable or fail with a official
diagnostic that explains the required migration.

## Stable Registry Contract

The registry/index contract is Cargo Registry Index compatible:

- root `config.json`;
- per-package JSON-lines files;
- package archive checksums;
- `yanked` status;
- `rust_version`;
- feature metadata;
- sparse HTTP conditional cache metadata.

Unknown or unsupported registry fields are preserved when possible and ignored
only when they are non-authoritative metadata. Fields that affect identity,
source, checksum, yanked status or dependency resolution cannot be ignored
silently.

## Stable Metadata Contract

`ghor metadata --json` retains the `ghor.metadata.v1` envelope and exposes
Cargo-compatible fields inside it:

- `cargo_metadata_format_version = 1`;
- `packages`;
- `workspace_members`;
- `target_directory`;
- `workspace_root`;
- `resolve`;
- `cargo_compatible`.

For `Cargo.toml` inputs, package ids, dependency fields and target records are
kept close to Cargo's metadata shape.

## Diagnostics Policy

Compatibility changes must prefer explicit diagnostics to silent behavior:

- unsupported registry protocol: diagnostic;
- missing checksum for nC registry package: diagnostic;
- invalid source replacement: diagnostic;
- unknown target cfg that cannot be mapped: inactive or diagnostic, never
  silently active for all targets;
- remote access under offline mode: diagnostic;
- auth-required remote mutation without credentials: diagnostic.

## Boundaries

`ghor` now provides registry CDN/cache, governance policy, moderation state, and
Cargo-backed build-script/proc-macro execution checks. These are compatibility
surfaces owned by `ghor`; they do not imply that `ghor` is the public
crates.io service or Rust's official package manager.

`ghor` still does not promise:

- crates.io's real public account database, moderation staff workflow, CDN
  provider contracts or marketplace governance process;
- a replacement for Cargo/rustc internals; Rust build scripts and proc macros
  are executed through the real Cargo toolchain and recorded as evidence;
- replacement of Cargo as Rust's official package manager;
- replacement of external C, C++, Rust, Java or other host compilers.

Those boundaries keep `ghor` precise: it is a Cargo-compatible nC orchestrator
with registry/social/execution evidence surfaces, not an undisclosed clone of
every external ecosystem institution.
