# ghor Security Hardening

Status: closed.

`ghor` hardens registry and package flows without changing nC language semantics.
The scope is the package-manager surface: credentials, registry mutation, package
integrity, auditability, yank policy and moderation policy.

## Token Redaction

Registry tokens are accepted through `--token`, login credentials, environment
tokens and configured credential providers. Command JSON output and hardening
reports never include raw token values. Remote registry mutations emit hashes or
status fields instead of secrets.

## Credential Permissions

`ghor login` stores credentials under `GHOR_HOME/credentials.json`. On Unix-like
hosts the credentials directory is written with mode `0700` and the credentials
file with mode `0600`. If an existing credentials file is group-readable or
world-readable, `ghor` ignores it for auth resolution, so an insecure file cannot
unlock remote mutation.

## Audit Log

Registry governance, moderation and yank mutations append to
`registry-audit.json`. Audit events include action, package, version, reason,
timestamp and actor hash. The actor is a hash of the token identity, never the
raw token.

## Checksums And Corruption

Published packages record SHA-256 checksums in the Cargo-style index entry and
package metadata. Fetch and vendor paths verify the expected checksum before the
package is accepted. A checksum mismatch fails with `E_GHOR_CHECKSUM_001`; the
corrupt package is rejected rather than cached as valid.

## Yank And Moderation Policy

Remote yank, owner and moderation mutations require authentication. Filesystem
registries remain usable for local development, but the productive hardening gate
requires audit, governance and moderation state for remote release registries.
`ghor yank --undo` records an audited unyank transition instead of silently
rewriting package state.

## Diagnostics

- `E_GHOR_AUTH_001`: remote mutation requires authentication.
- `E_GHOR_CHECKSUM_001`: fetched package checksum does not match the index.
- `E_GHOR_REGISTRY_001`: registry hardening found a release blocker.

## Validation Surface

The hardening is visible in:

- `ghor doctor --json` under `security_hardening`.
- `ghor conformance --json` under `security_hardening`.
- `ghor registry hardening --json` through token, audit, checksum, governance,
  moderation and CDN checks.
