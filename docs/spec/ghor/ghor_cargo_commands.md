# ghor Cargo-Compatible Command Surface

Status: closed for missing/incomplete Cargo-grade registry commands.

References:

- Cargo update: https://doc.rust-lang.org/cargo/commands/cargo-update.html
- Cargo vendor: https://doc.rust-lang.org/cargo/commands/cargo-vendor.html
- Cargo tree: https://doc.rust-lang.org/cargo/commands/cargo-tree.html
- Cargo metadata: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
- Cargo install: https://doc.rust-lang.org/cargo/commands/cargo-install.html
- Cargo package: https://doc.rust-lang.org/cargo/commands/cargo-package.html
- Cargo publish: https://doc.rust-lang.org/cargo/commands/cargo-publish.html
- Cargo owner: https://doc.rust-lang.org/cargo/commands/cargo-owner.html
- Cargo login/logout: https://doc.rust-lang.org/cargo/commands/cargo-login.html

This document closes the command gaps after registry protocol compatibility, resolver compatibility, workspace support, `.crate` packaging, and Cargo-style `.cargo/config.toml`.

## update

`ghor update` now runs the real resolver and writes `ghor.lock`.

Supported:

- full dependency update through resolver;
- `--dry-run`;
- `-p` / `--package` package specs in the update report;
- `--precise <version>` for selected packages;
- `--frozen` rejection;
- before/after lock hashes;
- resolver diagnostics.

The command does not edit manifest version requirements except for the in-memory `--precise` resolution path.

## vendor

`ghor vendor [path]` now vendors registry packages instead of only writing path markers.

Supported:

- real resolver input;
- registry package download;
- checksum verification;
- `.crate` extraction;
- `.cargo-checksum.json` per vendored package;
- `--versioned-dirs`;
- `--no-delete`;
- source replacement snippet output for `.cargo/config.toml`.

Path dependencies are not copied as registry sources; they are recorded as path markers.

## tree

`ghor tree` now uses the complete resolver plan.

Output includes:

- root dependencies;
- resolver metadata;
- resolved nodes;
- dependency edges;
- full resolve report.

## metadata

`ghor metadata` keeps the existing `ghor.metadata.v1` envelope and adds Cargo-compatible fields:

- `cargo_metadata_format_version`;
- `packages`;
- `workspace_members`;
- `workspace_default_members`;
- `target_directory`;
- `workspace_root`;
- `resolve`;
- nested `cargo_compatible`.

This gives machine-readable metadata compatible with Cargo-style tooling expectations while preserving ghor-specific fields.

## install

`ghor install` supports both local manifest install and registry install.

Registry install:

```bash
ghor install package_name --version 1.2.3 --registry mirror --root ~/.ghor
```

The registry path:

- selects the newest compatible non-yanked semantic version;
- downloads the `.crate` and its canonical package manifest;
- verifies archive SHA-256, manifest identity, archive name and package root;
- extracts source under `registry-src/<package>-<version>`;
- builds the package through native nC stage1 and installs the material executable
  under `<root>/bin`;
- retains `.nm`, object and build report under `<root>/artifacts/<package>-<version>`;
- writes `installed/<package>.json` using `ghor.install.receipt.v2`.

Local install accepts `--manifest-path`, `--manifest` or `--path`. It builds and
installs the same material artifacts, but the receipt does not claim ownership of
the caller's source directory.

Install and replacement are staged below `<root>/.transactions`. Every move is
verified and rolled back on failure. A successful upgrade removes only material
owned by the previous receipt. `ghor uninstall <package> [--version <version>]`
moves every owned path into a transaction before removal and preserves path
sources it does not own.

## package and publish

Already closed and retained:

- `ghor package --list`;
- `.crate` tar.gz archive;
- metadata/prepublish verification;
- `ghor publish --dry-run`;
- HTTP/Web API plus index mirror publish.

## release-check

`ghor release-check` is release-oriented and intentionally safer than invoking
`ghor package` blindly on a large workspace. The command runs:

- `ghor doctor --json`;
- `ghor check`;
- `ghor lock`;
- `ghor verify-lock`;
- package preflight with publish metadata diagnostics and selected file count.

By default it does not write a `.crate` archive. Archive creation is explicit:

```bash
ghor release-check --manifest-path ghor.toml --package --out target/ghor-release
```

This keeps release checks fast and auditable for monorepos while preserving the
real `.crate` writer in `ghor package` and explicit release archive mode.

## owner

`ghor owner` supports Cargo-style flag forms and legacy positional forms:

```bash
ghor owner crate --list
ghor owner crate team --add
ghor owner crate team --remove
ghor owner list crate
ghor owner add crate team
ghor owner remove crate team
```

Remote mutations require authentication through token, login, environment token, or credential provider. Anonymous remote `owner add/remove` fails with `E_GHOR_AUTH_001`.

## login / logout

`ghor login` can store a token supplied directly or obtained through the configured credential provider.

`ghor logout` removes the stored credential for the effective registry.

Both commands honor named registries from `.cargo/config.toml`.

## Tests

Coverage:

- `ghor_update_real_resolver_dry_run_and_precise_ok`
- `ghor_vendor_registry_extracts_package_and_config_ok`
- `ghor_tree_and_metadata_include_resolver_nodes_ok`
- `native_nc_stage1_parses_a_real_manifest` (registry/path material install,
  execution, reinstall and receipt-owned uninstall)
- `ghor_registry_http_web_api_yank_owner_login_ok`
- `ghor_login_logout_with_credential_provider_ok`
- `ghor_release_check_preflights_package_without_archiving_ok`
