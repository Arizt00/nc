# ghor .crate Packaging And Publish

Status: closed for native nC stage1 package construction and package/publish protocol.

Source references:

- Cargo publish: https://doc.rust-lang.org/cargo/commands/cargo-publish.html
- Cargo package: https://doc.rust-lang.org/beta/cargo/commands/cargo-package.html
- crates.io publishing reference: https://doc.rust-lang.org/cargo/reference/publishing.html
- Cargo manifest metadata: https://doc.rust-lang.org/cargo/reference/manifest.html

## .crate Format

`ghor package` writes a Cargo-compatible `.crate` archive:

```text
target/ghor/package/<name>-<version>.crate
```

The file is a deterministic gzip stream containing DEFLATE stored blocks and a USTAR
archive. `tools/ghor/nc/src/artifact.n` writes every header, checksum, padding block and
footer directly through nC byte operations. Package construction does not invoke Rust,
`tar`, `gzip` or a private stage0 adapter. Its root directory is:

```text
<name>-<version>/
```

The package root includes:

- normalized `ghor.toml`;
- original `ghor.toml.orig`;
- `ghor.lock`, generated when missing unless `--exclude-lockfile` is used;
- `.cargo_vcs_info.json`;
- selected source/docs/metadata files.

`manifest.json` next to the archive records:

- archive path and checksum;
- package root;
- selected file list;
- crates.io metadata;
- verification status;
- manifest hash.

## Include And Exclude

`[package] include` and `exclude` are honored with Cargo-like behavior.

If `include` is present, it is authoritative:

```toml
[package]
include = ["ghor.toml", "src/**", "README.md", "LICENSE"]
```

If `include` is absent, all package-root files are considered except:

- package-root `target/`;
- `.git/`;
- nested packages containing `ghor.toml`, `nc.toml`, or `Cargo.toml`;
- hidden files when the package is outside Git.

`exclude` applies only when `include` is not specified:

```toml
[package]
exclude = ["public/assets/*", "videos/*"]
```

## Prepublish Verification

`ghor package` and `ghor publish` run preflight checks before writing or uploading:

- manifest diagnostics must not contain errors;
- `package.publish = false` forbids publish while retaining local `package`;
- non-dev path dependencies must also declare `version`;
- `readme` and `license-file` paths must exist when declared;
- package file selection must not be empty;
- archive verification decodes the native GZIP/DEFLATE stream and validates CRC32,
  USTAR checksums, root, member count and generated manifest unless `--no-verify` is used.

The report separates `source_members` from `source_checks`. Structural archive
verification never claims that source code was compiled. Compilation belongs to
`ghor check` and release verification.

Supported flags:

```text
ghor package --manifest-path ghor.toml --out target/ghor/package
ghor package --manifest-path ghor.toml --list --json
ghor package --manifest-path ghor.toml --no-verify
ghor package --manifest-path ghor.toml --allow-dirty
ghor package --manifest-path ghor.toml --exclude-lockfile
ghor publish --manifest-path ghor.toml --registry ./registry
ghor publish --manifest-path ghor.toml --dry-run --registry ./registry --json
```

## crates.io Metadata

The publish metadata follows the Cargo Web API shape:

- `name`;
- `vers`;
- `deps`;
- `features`;
- `authors`;
- `description`;
- `documentation`;
- `homepage`;
- `readme`;
- `readme_file`;
- `keywords`;
- `categories`;
- `license`;
- `license_file`;
- `repository`;
- `links`;
- `rust_version`.

Missing human-facing metadata is reported as warning unless `--no-metadata` is used. Invalid metadata that Cargo/crates.io would reject is a hard error.

## Registry Publish

`ghor publish` uploads the `.crate` file through the Cargo Web API body format when the registry exposes HTTP API:

```text
u32 metadata_len
metadata_json
u32 crate_len
crate_bytes
```

Writable registries also receive:

```text
packages/<name>/<version>/<name>-<version>.crate
packages/<name>/<version>/manifest.json
<cargo package index path>
index.json
```

The package JSON-lines index entry includes the SHA-256 checksum of the `.crate` bytes and carries `ghor_metadata` for registry inspection.

## Closure

This closes:

- `.crate` archive format;
- Cargo-like include/exclude selection;
- package file listing;
- prepublish verification;
- crates.io metadata transport;
- local/HTTP registry publish using `.crate`;
- fetch/cache paths for `.crate`;
- compatibility reports for registry metadata.

The Rust stage0 `package` command remains available only through the explicit stage0
compatibility route. Normal delegation sends `package` and the package half of
`publish` to the nC stage1 executable.
