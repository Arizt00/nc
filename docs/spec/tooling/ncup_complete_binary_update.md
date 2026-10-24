# ncup complete binary update

Status: implemented

## Command contract

Running `ncup` without arguments is equivalent to:

```text
ncup update all
```

The command rebuilds and installs the complete command suite from the resolved
nC workspace:

```text
ncc nclsp nc-sys-lsp ghor nfmt nlint nbuild nide npkg ncup ncup-init nwal
```

It also rebuilds and installs the required native nC companion
`ghor-stage1`. The public command count is twelve; the atomic update set
contains thirteen executable artifacts.

`ncup update all --dry-run --json` emits the complete plan without building or
writing files. `ncup --help` only prints help. The historical `ncup update`
command keeps its stable meaning: update the effective active toolchain.

## Workspace resolution

The source workspace is resolved in this order:

1. `--workspace <path>`;
2. current directory or an ancestor;
3. `NC_SOURCE_ROOT` or `NCC_SOURCE_ROOT`;
4. the workspace recorded when `ncup` was compiled.

The selected directory must contain `crates/ncc-cli/Cargo.toml`.

## Installation guarantees

- Every expected command and native companion must exist and be non-empty
  before commit.
- The freshly built `ghor-stage1` must pass its native self-test before the
  transaction can begin.
- Cargo installations use `--locked`, so every host respects the workspace
  dependency lockfile.
- Files are copied to a transaction staging directory first.
- Installed bytes are checked with SHA-256.
- A failed commit restores the previous binary set.
- The ten Primitive `.nctarget` contracts and required linker resources are
  installed below `<prefix>/share/nC`; target checks therefore do not depend on
  the source directory recorded by the compiler build.
- Linux and macOS replace the running executable through normal inode-safe
  installation.
- Windows updates the other eleven commands and `ghor-stage1.exe` immediately and replaces the running
  `ncup.exe` from a hidden helper after its process exits.
- Windows `.cmd` shims are regenerated for all twelve commands.
- The managed Windows toolchain itself contains native `*.exe` artifacts;
  `doctor`, `which`, synchronization and integrity verification all resolve the
  same platform-specific filename. Unix toolchains retain extensionless shims.
- `ncc doctor bins` resolves a quoted native executable behind a `.cmd` or
  `.bat` wrapper before comparing hashes, so a valid launcher is not reported
  as a stale binary.
- On Windows, all user WSL distributions except `docker-desktop` are refreshed;
  `--distro <name>` limits this to one distribution.
- Every refreshed WSL distribution receives the same twelve commands and the
  matching `ghor-stage1` native companion in
  `$HOME/.cargo/bin`, `$HOME/.local/bin` and `/usr/local/bin`. This prevents an
  older executable earlier in `PATH` from exposing a stale language catalog.
- The Unix installer accepts `--source-dir <dir>` to transactionally mirror one
  already-built suite into another binary root without compiling it again.
- With its default destination, the standalone Unix update script also mirrors
  an active user-owned command directory (`$HOME/.cargo/bin` or `$HOME/bin`) in
  the same SHA-256 transaction when it precedes the primary destination in
  `PATH`; an explicit `--bin-dir` remains isolated. It never overwrites
  package-manager or system directories such as `/usr/bin`.
- A Windows host without WSL still updates its native suite.
- Unix verification supports `sha256sum`, macOS `shasum`, or OpenSSL SHA-256.

Each successful destination writes its own `.ncup-last-update.json` receipt.
The receipt reports twelve public commands, one companion and thirteen managed
executable artifacts.
