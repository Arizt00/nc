# ghor Release Install Smoke

Status: closed.

`ghor install-smoke` validates that the installed release command surface is
present and runnable after a release build or local install.

## Command

```text
ghor install-smoke --json
ghor install-smoke --bin-dir /path/to/bin --json
```

The command checks these required binaries:

- `ghor`
- `ncc`
- `nclsp`
- `nfmt`
- `nlint`

Each binary is located through `PATH` by default, or through `--bin-dir` when a
release bundle directory must be validated directly. The smoke is
non-destructive: each binary is executed with `--help` and the report records
exit status, stdout/stderr hashes and byte counts.

## Failure Model

Missing binaries or failing smoke commands emit:

```text
E_GHOR_INSTALL_SMOKE_001
```

The command does not synthesize success from build artifacts. It validates the
actual installed paths visible to the release environment.

## Scope

This closes the release/install smoke for the nC command surface needed by ghor
and the editor/toolchain loop. It does not package installers, sign binaries or
modify PATH; those belong to distribution and packaging phases.
