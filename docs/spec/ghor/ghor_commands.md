# ghor Commands

This document describes the native nC stage1 surface and the retained Rust
stage0 recovery launcher. All 77 command contracts have a stage1 route or an
explicitly partial stage1 route. This does not claim compiler stage2
self-hosting. The split is emitted by `ghor report commands --json`.

`manifest` is the first command implemented by the native nC stage1. Ghor
discovers `ghor-stage1` beside its launcher; `NC_GHOR_STAGE1` can select a
different executable. Delegation is selective by supported option shape;
explicit recovery uses `NC_GHOR_FORCE_STAGE0=1`.

`metadata` is also implemented by stage1 and emits `ghor.metadata.v1` plus a
Cargo-compatible package/workspace view. `workspace metadata` is material in
nC: it discovers parent workspaces, expands `*` and `?`, applies exclusions,
selects default members and resolves workspace package/dependency/lint
inheritance. The orchestration actions `workspace check|build|test|graph`
remain in stage0 until their underlying command families migrate.

`resolve`, `lock`, and `verify-lock` have native stage1 routes for path
dependencies, filesystem registries, HTTP(S), sparse HTTP, Git registries and
Git dependencies. They support Cargo-style caret, tilde, wildcard and
comparator requirements, resolver feature groups, transitive metadata,
deterministic Cargo lock format v4 output and lock drift detection. Git
dependencies retain the requested ref and exact resolved HEAD in the lock
model.

`fetch`, `vendor`, and `update` use the same native nC resolver. Stage1 computes
SHA-256 itself, verifies package archives and canonical package manifests,
copies and rehashes cache artifacts, extracts `.crate` archives, writes
Cargo-compatible per-package checksums, and records deterministic lock
before/after hashes. HTTP uses bounded direct `curl` processes, redirects,
retries, bearer credentials and ETag caches; Git uses direct `git` processes
with exact checkout. Neither path invokes a shell. `--offline` requires cached
material, and `update -p/--package --precise` constrains the selected dependency
inside the resolver before lock generation.

`add`, `remove`, and `tree` are owned by native nC stage1. Manifest edits are
validated transactions: Ghor writes and parses a temporary manifest, retains
comments and unrelated sections, then replaces the source with a recoverable
backup. Re-adding a key replaces its logical TOML entry without duplication;
removing the final entry also removes an empty section created by Ghor. A
matching add/remove sequence restores the original bytes. `--dry-run` performs
the same parse validation without committing changes.

`add` accepts path, registry, and Git sources (`branch`, `tag`, or exact `rev`),
renames, features, optional/default-feature policy, normal/dev/build kinds, and
target-specific sections. Registry additions require an explicit SHA-256.
`tree` consumes the native resolver graph, includes root and transitive edges,
and honors target, registry, offline, and `--include-dev` selection. The
launcher delegates these commands to a discovered side-by-side stage1 binary.

`search`, `info`, `login`, `logout`, and
`registry doctor|cargo-config|index|config` are also material nC stage1
commands. Search and info filter the compatibility `index.json`; registry
inspection reports transport, protocol, index, and config without exposing
credentials. Login writes the selected bearer token to
`GHOR_HOME/credentials.json`; logout removes only that registry entry. On Unix,
the directory is mode `0700` and the file mode `0600`.

The launcher delegates this registry subset for explicit material
filesystem/HTTP/sparse/Git references, inline manifest registry configuration
and effective `.cargo/config.toml`/`.cargo/config` discovered by stage1. Native
nC also owns `publish`, `owner` and `yank`: it updates
the aggregate index and Cargo JSON-lines shard, invokes Cargo Web API, uploads
package material, records an audit model and reports a committed transaction
only when every required material target succeeds. Discovery, precedence,
named registries, source replacement, offline mode, proxy, timeout, retry, CA,
revocation and Git TLS policy are native nC. Execution/capture of an external
Cargo credential provider is native nC as well: stage1 resolves aliases,
executes the declared argv through `process:command`, captures stdout, checks
timeout/exit status and retains only redacted availability in reports.

The launcher delegates credential-provider configuration to stage1 and retains
`NC_GHOR_FORCE_STAGE0=1` as explicit recovery. `install` and `uninstall` are also
material native nC stage1 commands. Install builds a real package executable,
retains its `.nm`, object and build report, and commits a versioned ownership
receipt. Uninstall validates the optional version and removes only paths owned
by that receipt through a rollback-capable transaction. Cargo execution, build
scripts, proc macros, C/C++/Rust interop and external Cargo compatibility are
owned by `foreign_execution.n` in stage1.

The stage0 launcher retains the same command surface for recovery:

- `new`, `init`, `check`, `build`, `run`, `test`, `bench`, `clean`
- `fmt`, `lint`, `doc`
- `package`, `publish`, `registry`
- `update`, `fetch`, `tree`, `metadata`, `manifest`, `add`, `remove`
- `target`, `toolchain`, `doctor`, `report`, `evidence`
- `workspace`, `profile`, `features`, `cross`, `link`, `artifact`
- `vendor`, `lock`, `verify-lock`, `audit`, `sbom`, `license`, `supply-chain`
- `search`, `info`, `login`, `logout`, `owner`, `yank`

Commands emit JSON where structured output matters. Exit codes fail when validation or delegated commands fail.

Human help is closed as a Cargo-like surface:

- `ghor` and `ghor --help` print the package-manager overview, options, and command descriptions.
- `ghor help <command>` and `ghor <command> --help` print focused command help.
- `ghor <command> help` is treated as command help, so `ghor new help` no longer creates a package named `help`.
- `ghor --list` prints the official command list for shell tooling.
- Help text is lilac in interactive terminals. Set `GHOR_COLOR=never`,
  `NC_NO_COLOR=1`, or `NO_COLOR=1` for plain output; set `GHOR_COLOR=always`
  to force lilac output through pipes.

Registry commands are closed as a ghor protocol:

- `ghor publish --registry <path-or-url>` writes package archives, package manifests, and index entries.
- `ghor search <query> --registry <path-or-url>` reads the registry index.
- `ghor info <package> --registry <path-or-url>` lists package versions and hashes.
- `ghor fetch --registry <path-or-url>` downloads registry dependencies into `target/ghor/registry-cache` after SHA-256 verification.
- `ghor yank <package> --registry <path-or-url>` marks entries as yanked in the registry index.
- `ghor registry doctor --registry <path-or-url>` reports registry transport and reachability.
- `ghor registry index --registry <path-or-url>` emits the raw registry index.

Cargo-compatible command gaps are closed:

- `ghor update` resolves and writes `ghor.lock`, with `--dry-run`, `-p/--package`, and `--precise`.
- `ghor vendor [path]` vendors resolved registry packages with checksum verification and a Cargo source replacement snippet.
- Native stage1 `resolve`, `lock`, `verify-lock`, `fetch` and `vendor` honor
  chained `[source.*]` replacement, named `[registries.*]` entries and direct
  filesystem/HTTP/sparse/Git references.
- `[basal-dependencies]` resolve, lock, fetch and vendor through the native nC
  stage1. Archive, package-manifest, FFI and `NBB0` contracts are verified
  before bytes are cached or extracted.
- `ghor add` and `ghor remove` edit normal, dev, build, and target dependency
  sections through validated native nC transactions.
- `ghor tree` and `ghor metadata` are backed by the native resolver.
- `ghor install <package>` selects, authenticates, verifies, extracts and builds
  registry material; `ghor install --manifest-path <path>` builds a local project
  without taking ownership of its source tree.
- `ghor uninstall <package> [--version <version>]` consumes
  `ghor.install.receipt.v2` and transactionally removes the installed binary and
  owned artifacts, cache and registry source.
- `ghor owner` supports Cargo-style add/remove/list with auth required for remote mutations.
- `ghor login` and `ghor logout` honor named registries and credential providers.
