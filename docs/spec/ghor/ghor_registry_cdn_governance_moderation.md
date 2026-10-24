# ghor Registry CDN / Governance / Moderation

Status: closed in native nC stage1.

This document closes the registry social/edge surface that was previously a
boundary for `ghor`. The implementation is a real `ghor` registry capability,
not a claim that `ghor` operates the public crates.io service.

## CDN

`ghor cdn build --registry <registry> --out <dir> --json` builds a
content-addressed registry edge cache from an existing registry. The cache
contains a `cdn-manifest.json`, registry `config.json` when available,
package-compatible paths under `packages/`, and official package blobs under
`blobs/sha256/<checksum>.crate`.

The CDN never invents package bytes. It reads package entries from the Cargo
JSON-lines index, downloads package archives from the registry, and records
their SHA-256 hashes in the manifest.

`status` and `inspect` read the retained manifest. `purge --sha256 <hash>`
removes exactly one content-addressed blob while leaving the package-compatible
origin path intact. Filesystem, HTTP, sparse HTTP and materialized Git reads use
the shared stage1 transport.

## Governance

`ghor governance --registry <registry> --init --token <token> --json` creates a
registry governance policy. `--role <user:role>` records role assignments. The
policy stores roles, enforcement flags and audit records.

Governance mutation requires explicit auth. Read-only inspection remains
available where the registry transport permits it. Git registry mutation is
rejected until the caller performs an explicit VCS transaction.

## Moderation

`ghor moderation <package> --version <version> --action <action> --reason
<reason> --registry <registry> --token <token> --json` records package
moderation state.

Actions:

- `review`: record review state;
- `block` / `quarantine`: set index `yanked=true` and record status;
- `approve` / `restore`: clear `yanked` and record status.

Moderation changes are auditable and keep package checksums intact.
Mutations update `moderation.json`, `index.json` and the Cargo JSON-lines shard
as one administrative transaction. Bearer tokens are never emitted; audit
records retain a SHA-256 actor identity instead.

## Native route

`tools/ghor/nc/src/registry_administration.n` owns these commands. It delegates
atomic JSON, package-index and audit writes to `registry_mutation.n`. The
launcher advertises `nc-stage1-when-available`; the native executable reports
`executor: nc-stage1` and propagates exit status 2 for contract failures.

## Compatibility Boundary

`ghor` provides CDN/governance/moderation mechanics for nC registries. It does
not clone crates.io's account database, human moderation process, legal policy,
or CDN provider contracts.
