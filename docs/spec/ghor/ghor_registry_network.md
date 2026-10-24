# ghor Registry / Network

`ghor` closes registry and network behavior as an orchestration registry protocol for nC packages and as a Cargo Registry Index compatible surface. It is not the public crates.io service, but the protocol-level registry pieces are real: root `config.json`, Cargo-style per-package JSON-lines index files, sparse HTTP index reads, git registry index reads, checksum verification, conditional HTTP cache, content-addressed CDN/edge cache, registry governance policy, moderation state, and Cargo Web API publish/yank/owner/login semantics.

The effective Cargo-style configuration layer is documented separately in [ghor_cargo_config.md](ghor_cargo_config.md). That layer covers `.cargo/config.toml`, named registries, credential providers, offline mode, retry, proxy, certs, and git registry fetch config.

## Registry Transports

Supported transports:

- filesystem: `--registry ./registry`
- HTTP: `--registry http://host:port`
- HTTPS: `--registry https://host`
- sparse HTTP(S): `--registry sparse+http://host:port` or `sparse+https://host`
- git index: `--registry git+https://host/index.git` or `--registry <url> --registry-protocol git`

Remote HTTP(S) transport uses `curl` for TLS, redirects, authorization, GET downloads, PUT uploads, and Cargo Web API calls. The optional token source order is:

1. `--token <token>`;
2. `GHOR_REGISTRY_TOKEN`;
3. `CARGO_REGISTRIES_<NAME>_TOKEN` / `GHOR_REGISTRIES_<NAME>_TOKEN`;
4. `.cargo/config.toml` registry tokens;
5. Cargo-style credential providers and aliases;
6. `~/.ghor/credentials.json`.

`ghor login --registry <url> --token <token>` stores the token locally and does not print it. Following Cargo's model, login itself is an out-of-band token acquisition flow; `config.json` `api` determines the `<api>/me` URL that a user visits or a registry UI exposes.

## Native stage1 client and mutations

The native nC stage1 owns `search`, `info`, `login`, `logout`, and
`registry doctor|cargo-config|index|config` for material registry references.
Its credential precedence is deliberately inspectable:

1. `--token <token>`;
2. `GHOR_REGISTRY_TOKEN`;
3. `GHOR_HOME/credentials.json`.

Tokens are passed to `curl` as bearer headers and never copied into JSON
reports. Login persists a per-registry entry; logout removes that entry and
retains the credentials document. Unix storage uses directory mode `0700` and
file mode `0600`. Insecure credential permissions are rejected for reads.

The same executable owns `publish`, `owner` and `yank`. Filesystem registries
use locked atomic JSON replacement and recoverable backups. HTTP and sparse
registries require bearer authentication, execute Cargo Web API operations,
upload archives, manifests and indexes, update Cargo package shards and retain
`registry-audit.json`. Cargo publish uses little-endian metadata and archive
length framing.

The launcher retains stage0 when effective behavior depends exclusively on
`.cargo/config.toml`, credential-provider aliases, proxy, CA, or net policy.
This avoids claiming equivalence before those configuration providers migrate.
Package archive construction remains an explicit stage0 adapter boundary;
registry policy and mutation authority are nC stage1.
Windows transport finalization normalizes paths near `MAX_PATH` to extended
`\\?\` form and verifies byte counts before accepting a downloaded artifact.

On Linux, the native client receives the process environment through the
phase12 ELF startup contract. The internal `_start` materializes `argc`,
`argv`, and `envp` from the loader stack before calling the SysV entry. The
environment runtime reads that vector directly and does not depend on libc
environment lookup or C allocation functions.

## Registry Layout

Every registry exposes:

```text
config.json
index.json
<cargo package index path>
packages/<name>/<version>/<name>-<version>.crate
packages/<name>/<version>/manifest.json
```

Basal registries published by `ncc basal publish` use the same registry root
discipline, but they intentionally use the basal package shape:

```text
index.json
index/<safe_name>/versions.jsonl
packages/<safe_name>/<version>/<safe_name>-<version>.nbp.tar.gz
packages/<safe_name>/<version>/manifest.json
```

The basal aggregate index uses `schema = "basal.registry.index.v1"` and each
entry carries `root_id`, `safe_name`, `version`/`vers`, archive checksum,
`package_manifest`, `package_manifest_hash`, `ffi_link_manifest_hash`, source
hash and NBB hash. ghor consumes these entries only through
`[basal-dependencies]`; they are not treated as Cargo crates.

`config.json` follows the Cargo Registry Index shape:

```json
{
  "dl": "packages/{crate}/{version}/{crate}-{version}.crate",
  "api": "https://registry.example",
  "auth-required": false
}
```

The `index.json` file remains as a ghor compatibility aggregate for search and older local tooling. The authoritative package index is Cargo-style: one file per package, path-derived from the package name, lowercase, with one JSON object per version line.

Examples:

```text
1/a
2/ab
3/a/abc
ca/rg/cargo
re/gi/registry_pkg
```

Each JSON-lines entry carries Cargo-compatible fields:

```json
{
  "name": "pkg",
  "vers": "0.1.0",
  "deps": [],
  "cksum": "<sha256-of-crate-bytes>",
  "features": {},
  "yanked": false,
  "rust_version": "1.85",
  "v": 2,
  "ghor_archive": "packages/pkg/0.1.0/pkg-0.1.0.crate",
  "ghor_manifest": "packages/pkg/0.1.0/manifest.json",
  "ghor_package_manifest_hash": "<sha256-of-package-manifest-json>",
  "ghor_ffi_link_manifest_hash": "<sha256-of-ffi-link-manifest-json>"
}
```

## Publish

`ghor publish --manifest-path ghor.toml --registry <path-or-url>` runs the package writer, writes the package archive and package manifest, inserts or replaces the index entry for the same package/version, and writes `index.json`.

The package manifest is part of the publish contract. It contains ghor package
metadata, package verification, and the compiler FFI manifest under
`ffi_link_manifest`. Publish records both the package manifest SHA-256 and the
stable FFI manifest hash in local ghor metadata and Cargo-compatible index
metadata. A package whose FFI manifest is failed, missing, or checksum-invalid
is not publishable.

Filesystem publish writes directly to the registry tree. HTTP(S)/sparse publish first attempts the Cargo Web API:

- `PUT /api/v1/crates/new`
- body: little-endian `u32` metadata length, metadata JSON, little-endian `u32` crate length, crate bytes

Then ghor mirrors the package archive, manifest, Cargo package index file, and `index.json` into registries it directly controls:

- `PUT packages/<name>/<version>/<name>-<version>.crate`
- `PUT packages/<name>/<version>/manifest.json`
- `PUT <cargo package index path>`
- `PUT index.json`

## Fetch

`ghor fetch --manifest-path ghor.toml --registry <path-or-url>` resolves non-path dependencies from the registry index. For each dependency it:

1. fetches `config.json`;
2. reads the package JSON-lines index file for the dependency, falling back to `index.json` only for compatibility;
3. finds a non-yanked package entry by name and version;
4. builds the download URL from `config.json` `dl` markers or from `ghor_archive`;
5. downloads the archive bytes from filesystem, HTTP(S), sparse HTTP(S), or git index cache;
6. verifies the archive SHA-256 against the registry index `cksum`;
7. verifies the archive SHA-256 against the dependency checksum declared in `ghor.toml`;
8. reads `packages/<name>/<version>/manifest.json`;
9. verifies the package manifest hash when the index declares it;
10. validates the embedded `ffi_link_manifest` and its hash when declared;
11. writes the archive to `target/ghor/registry-cache/<name>/<version>/`.

Checksum mismatch is a hard failure. Missing package entries are hard failures.
Yanked entries are not selected. Corrupt package manifests, missing FFI
manifests, failed FFI manifests, and FFI manifest hash mismatches are hard
failures; ghor does not accept native/FFI registry state silently.

`ghor install` and `ghor vendor` reuse the same package manifest validation
before extracting or materializing packages. Their JSON outputs include
`package_manifest_validation` with the manifest hash and FFI hash that were
accepted.

For `source = "basal-registry"` dependencies, fetch reads the basal registry
index, selects the requested root/package by SemVer requirement, verifies the
`.nbp.tar.gz` archive checksum, verifies the declared dependency checksum,
loads and hashes `manifest.json`, requires `schema = "basal.package.v1"`,
requires `ghor_compatible = true`, requires NBB0 metadata, and validates the
embedded compiler FFI manifest summary. Fetch stores accepted archives under
`target/ghor/basal-registry-cache/<safe_name>/<version>/`. Vendor extracts the
archive and writes `.basal-checksum.json` containing file hashes and the accepted
basal validation report.

The nC stage1 path uses the same contract. The package manifest hash is the
SHA-256 of the exact published file bytes; the embedded FFI hash is the SHA-256
of canonical compact JSON. Resolution, fetch and vendor repeat these checks.
Vendor names the directory from `safe_name` and writes
`ghor.basal.vendor.checksum.v1` with `root_id`, source registry and the accepted
validation report.

Sparse HTTP(S) uses a Cargo-style conditional metadata cache:

- captures `ETag` and `Last-Modified`;
- sends `If-None-Match` or `If-Modified-Since`;
- accepts `304 Not Modified` and reuses the local cached body.

Git registry support clones/fetches the registry index into `~/.ghor/registry-git-cache/<hash>` and reads `config.json`, `index.json`, and package JSON-lines files from the clone.

## Search / Info / Yank / Owner

`ghor search`, `ghor info`, `ghor yank`, and `ghor owner` operate on the same registry metadata. `search` and `info` retain `index.json` compatibility while package-aware fetch uses Cargo JSON-lines entries.

## CDN / Edge Cache

`ghor cdn build --registry <registry> --out <dir>` materializes a real
content-addressed registry edge cache. The cache includes:

- `cdn-manifest.json` with package/version/checksum entries;
- `config.json` copied from the source registry when available;
- `blobs/sha256/<hash>.crate` for official content-addressed package bytes;
- Cargo package paths under `packages/<name>/<version>/...` for direct
  registry-compatible reads.

`ghor cdn status --out <dir> --json` reports manifest presence and entry count.
`ghor cdn purge --out <dir> --sha256 <hash> --json` removes one cached blob by
checksum. CDN output is deterministic for a fixed registry state and does not
invent package bytes: all blobs are read from the registry and verified by
SHA-256.

## Governance

`ghor governance --registry <registry> --init --token <token>` creates or
updates `governance.json`. The policy stores:

- policy version and registry authority;
- role records such as `admin`, `owner`, `publisher`, `moderator`, `auditor`;
- enforcement flags for publish, owner mutation and moderation auth;
- audit records with action, token presence and timestamp.

Mutation requires an explicit registry token. Reads remain available without a
token where the registry transport permits it. Git registries are read-only for
governance through `ghor`.

## Moderation

`ghor moderation <package> --version <version> --action <action> --reason
<reason> --registry <registry> --token <token>` records auditable moderation
state in `moderation.json` and updates the package index when the registry is
writable.

Supported actions:

- `review`: record review state without changing package yanked status;
- `block` / `quarantine`: mark the package version yanked and record the
  moderation status;
- `approve` / `restore`: clear yanked status and record the approved/restored
  status.

Moderation does not hide bytes or silently rewrite packages. It records a
policy decision, preserves the index history model, and relies on checksum
verification for package integrity.

`ghor yank <package> --vers <version>` calls the Cargo Web API when available:

- `DELETE /api/v1/crates/{crate}/{version}/yank`

Then ghor updates local writable metadata by setting `yanked=true` in both `index.json` and the package JSON-lines entry. A yanked package remains visible through `info` but is not eligible for `fetch`.

`ghor owner add/remove/list` calls the Cargo Web API when available:

- `GET /api/v1/crates/{crate}/owners`
- `PUT /api/v1/crates/{crate}/owners` with `{"users":["login"]}`
- `DELETE /api/v1/crates/{crate}/owners` with `{"users":["login"]}`

Owner state is registry metadata; it does not replace registry authentication policy.

## Diagnostics

Registry/network diagnostics include:

- `E_GHOR_REGISTRY_001`: malformed or unsupported registry operation;
- `E_GHOR_REGISTRY_API_001`: Cargo Web API endpoint missing or unusable;
- `E_GHOR_REGISTRY_GIT_001`: git registry clone/fetch/read failure;
- `E_GHOR_NETWORK_001`: remote transport failure;
- `E_GHOR_REGISTRY_RESOLUTION_001`: package/version not found in registry;
- `E_GHOR_REGISTRY_CHECKSUM_001`: registry index checksum does not match package bytes;
- `E_GHOR_CHECKSUM_001`: dependency checksum does not match package bytes;
- `E_GHOR_REGISTRY_MANIFEST_001`: registry package manifest missing, malformed, or hash-invalid;
- `E_GHOR_FFI_MANIFEST_001`: registry package FFI manifest missing, failed, or hash-invalid.
- `E_GHOR_BASAL_PACKAGE_001`: basal registry dependency missing, checksum-invalid,
  manifest-invalid, FFI-invalid, or NBB0-invalid.

## Closure Status

Registry/network is closed for ghor orchestration:

- local publish/search/info/yank/fetch: closed;
- Cargo-compatible `config.json`: closed;
- Cargo package index JSON-lines: closed;
- HTTP publish via Cargo Web API plus controlled PUT mirror: closed;
- HTTP search/info/fetch via GET: closed;
- sparse+HTTP(S) config/package-index cache with ETag, Last-Modified, and 304: closed;
- git registry index clone/fetch/read: closed;
- owner metadata mutation: closed;
- checksum verification: closed;
- package manifest hash validation: closed;
- compiler FFI manifest registry validation: closed;
- basal registry dependency resolution/fetch/vendor validation: closed;
- cache materialization: closed;
- credential storage: closed;
- Cargo-style `.cargo/config.toml` registry/network/auth configuration: closed;
- doctor/index/config/package-index inspection: closed.
