# ghor Registry Productive Hardening

Status: closed.

This document defines the productive registry hardening surface for `ghor`.
The goal is to decide whether a registry is ready for real package release
traffic, not merely whether a local test registry can answer simple reads.

## Commands

- `ghor registry hardening --manifest-path ghor.toml --registry <registry> --token <token> --cdn-out <dir> --json`
- `ghor release-hardening --manifest-path ghor.toml --registry <registry> --token <token> --cdn-out <dir> --json`

`registry hardening` validates one registry. `release-hardening` combines the
normal `ghor doctor` gate with the registry hardening report.

## Checks

The hardening report includes structured `checks[]` entries:

- registry transport selected and readable;
- `curl` availability for remote HTTP/Sparse/Git registry operations;
- remote auth token availability, always redacted from JSON output;
- HTTPS or explicit localhost smoke allowance;
- readable `config.json`;
- satisfiable `auth-required` policy;
- readable registry index;
- package JSON-lines entry for the manifest package;
- archive SHA-256 matching package index `cksum`;
- governance policy presence and schema;
- moderation state presence and schema;
- content-addressed CDN manifest when `--cdn-out` is supplied.

Critical failures produce `E_GHOR_REGISTRY_HARDENING_001` or
`E_GHOR_RELEASE_HARDENING_001`. Warnings remain visible in the report and keep
the registry from being silently treated as fully release-ready.

## Security Rules

- Remote productive registry checks require a token.
- Tokens are never printed; reports use `available_redacted`.
- HTTP is only accepted for localhost smoke registries. Productive remote
  registries should use HTTPS.
- Package bytes are not trusted unless the archive hash matches the registry
  package index checksum.
- Governance and moderation state are required for remote productive release
  hardening.

## Boundary

This hardening closes `ghor` registry release readiness. It does not operate the
public crates.io service and does not replace external host compilers.
