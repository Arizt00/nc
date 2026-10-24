# ghor Cargo-Parity Industrial Closure

Status: closed for the declared nC package-orchestration surface.

`ghor` is not a social clone of crates.io and does not replicate Cargo's whole public marketplace. It is closed as an industrial, Cargo-compatible package manager/orchestrator for nC projects: registry protocol, resolver behavior, workspace behavior, package format, config, cache, core commands, metadata, vendor, and CLI parity.

## 1. Lockfile Compatibility

`ghor.lock` is now a Cargo.lock-style TOML primary lockfile:

- generated text starts with `version = 4`.
- packages are emitted as `[[package]]` entries.
- package entries contain name, version, source, checksum, and dependencies where applicable.
- registry packages use `registry+...` sources.
- git packages use `git+...#resolved_head` sources.
- path packages omit source like Cargo path packages.

The nC-specific rich lock metadata is preserved in the sidecar:

```text
ghor.lock.meta.json
```

The sidecar keeps the `ghor.lock.v1` JSON contract, including machine/basal/runtime capability metadata, language adapters, dependency groups, resolver report, and the exact TOML text hash.

## 2. Resolver Compatibility

The resolver supports SemVer requirements and the Cargo resolver families declared by the manifest or CLI:

- resolver v1/v2/v3 selection.
- normal, dev, build, and target-specific dependency kinds.
- target activation rules.
- workspace feature grouping and unification.
- `[patch]`, `[replace]`, alternate registries, and source replacement.
- `ghor update -p <pkg> --precise <version>`.
- yanked package rejection.
- transitive dependency expansion from Cargo Registry Index JSON-lines entries.

Dev dependencies remain inactive unless explicitly requested through test/all-targets/include-dev paths.

## 3. Git Dependencies

Git dependencies are resolved through a real checkout cache under `GHOR_HOME/git-deps-cache`.

Supported selectors:

- `git = "..."`
- `branch = "..."`
- `tag = "..."`
- `rev = "..."`

The resolver performs clone/fetch/checkout, records the exact `resolved_head`, reads `ghor.toml`/`nc.toml` from the checkout when present, and emits a source string of the form:

```text
git+<url>?rev=<rev>#<resolved_head>
```

Git dependencies do not require registry checksums because the lock records the exact git head instead.

## 4. Registry Protocol

The registry implementation is compatible with Cargo Registry Index shape:

- root `config.json`.
- per-package index paths.
- JSON-lines entries with `name`, `vers`, `deps`, `features`, `cksum`, `yanked`, `rust_version`, and `v`.
- filesystem registry.
- sparse+HTTP(S) registry.
- registry git index clone/fetch/read.
- HTTP cache with ETag, Last-Modified, and 304 Not Modified.
- Web API publish/yank/owner/login/logout through configured API endpoints.

`ghor publish` sends Cargo-compatible publish bodies for HTTP registries: metadata length, metadata JSON, crate length, crate bytes.

## 5. Cargo-Grade Workspaces

Workspace behavior includes:

- root and virtual manifests.
- `members`, `exclude`, and `default-members`.
- `workspace.package`.
- `workspace.dependencies`.
- `workspace.lints`.
- shared lockfile at workspace root.
- shared target directory at `target/ghor`.
- workspace metadata.
- `ghor check/build/test --workspace`.
- `ghor workspace check/build/test/graph`.

The command path now avoids treating virtual workspace roots as an `unnamed` package when `--workspace` is requested.

## 6. .crate Package Format

`ghor package` produces Cargo-compatible `.crate` tar.gz archives with:

- include/exclude rules.
- normalized `ghor.toml`.
- original manifest copy as `ghor.toml.orig`.
- `ghor.lock` as Cargo.lock-style TOML.
- `ghor.lock.meta.json` as nC metadata sidecar.
- `.cargo_vcs_info.json`.
- publish preflight validation.
- README, license, categories, keywords, repository, documentation, homepage metadata.
- `package --list`.
- `publish --dry-run`.

The package is verified before publish and rejected when metadata is invalid or publishing is disabled.

## 7. .cargo/config.toml Equivalent

The effective configuration reader supports:

- named registries.
- default registry.
- sparse/git protocol hints.
- credential providers and aliases.
- token persistence.
- proxy, timeout, retry, cainfo, proxy-cainfo, check-revoke.
- git proxy/cainfo/ssl-verify.
- offline mode through CLI, env, or `[net].offline`.
- source replacement.

Remote registry access is blocked when offline mode is active.

## 8. Vendor, Metadata, Build/Test/Check

Vendor:

- resolves registry packages.
- downloads verified `.crate` bytes.
- extracts package source.
- writes Cargo-shaped `.cargo-checksum.json`.
- includes per-file SHA-256 hashes and package checksum.
- emits source replacement config.

Metadata:

- emits `cargo_metadata_format_version = 1`.
- includes package fields expected by Cargo-style consumers.
- exposes workspace members, target directory, workspace root, resolve nodes, deps, dep kinds, and features.

Build/test/check:

- `check`, `build`, and `test` operate on a single manifest or `--workspace`.
- build writes deterministic build plan artifacts under the shared target directory.
- test checks project tests/examples through the local nC checker.

## 9. CLI Parity

The declared command surface includes the Cargo-like commands needed by nC release orchestration:

```text
check build test package publish install update tree metadata vendor
owner login logout yank registry workspace lock verify-lock fetch
```

Additional nC-specific commands remain available for language orchestration, machine/basal/runtime/compiler/primitive passthrough, evidence, seal, conformance, and release-check.

The help surface is Cargo-like: `ghor`, `ghor --help`, `ghor help <command>`,
`ghor <command> --help`, `ghor <command> help`, and `ghor --list` all produce
human-readable command guidance instead of the old single-line usage output.

## 10. Exact Cargo Project Compatibility

`ghor` now accepts `Cargo.toml` as an input manifest beside `ghor.toml` and
`nc.toml`. This is intentionally compatibility-oriented: a real Cargo project
can be inspected and resolved without renaming its manifest.

For `Cargo.toml` manifests, `ghor metadata` emits Cargo-shaped fields:

- Cargo-like `path+file://...#name@version` package ids.
- `cargo_metadata_format_version = 1`.
- package name, version, edition, license, rust_version and metadata.
- target records for libraries, binaries, proc-macro libraries and build
  scripts.
- dependency fields expected by Cargo consumers: `req`,
  `uses_default_features`, `features`, `rename`, `kind`, `target` and source.

Cargo's `cargo metadata --format-version 1 --no-deps` is used as a reference in
the regression suite for the Cargo-compatible subset that `ghor` claims.

## 11. SemVer, Features and Target cfg Edge Cases

The Cargo-grade resolver path covers:

- caret, tilde, wildcard, exact and comparator requirements.
- feature sets with `default-features = false`.
- resolver families v1, v2 and v3.
- target-specific dependency activation.
- `cfg(unix)`, `cfg(windows)`, `cfg(not(...))`, `cfg(any(...))` and
  `cfg(target_os = "...")` forms for the declared target matrix.

The target cfg implementation is deliberately conservative: a cfg form that
cannot be understood does not become silently active for every target.

## 12. Remote Registry and Hardened Auth Boundary

`ghor` supports filesystem, sparse+HTTP(S), HTTP(S) and git registry indexes.
For remote registries it uses Cargo Registry Index shape and Cargo Web API
routes for publish/yank/owner/login/logout.

Authentication is hardened by explicit token lookup and redaction:

- CLI token.
- environment token.
- named registry token.
- `.cargo/config.toml` token.
- configured credential provider.
- local ghor credentials.

No token is printed in human reports. Offline mode blocks remote registry
access. A real crates.io credential is not required or embedded by the test
suite; protocol compatibility is tested with controlled HTTP registries and the
same request shapes.

## 13. Source Replacement and Git Dependency Edge Cases

Cargo-style source replacement is supported through effective `.cargo/config.toml`
sources:

- `[source.crates-io] replace-with = "..."`
- named directory replacements.
- named local-registry replacements.
- named registry replacements.

Git dependencies support branch, tag and rev selectors, and they record the
exact resolved commit head. A git dependency can expose `Cargo.toml`,
`ghor.toml` or `nc.toml`; discovery follows the same manifest order as local
projects.

## 14. Build Scripts and Proc-Macro Execution

`ghor` now closes Rust build-script and proc-macro execution as a Cargo-backed
compatibility surface. It does not reimplement `rustc` or Cargo's compiler
pipeline; instead, it invokes the real host Cargo toolchain through
`cargo check`, captures stdout/stderr hashes, exit status and declared
build-script/proc-macro evidence, and reports the result through `ghor
build-script`, `ghor proc-macro` and `ghor cargo-exec`.

The metadata surface remains Cargo-compatible:

- `[lib] proc-macro = true` emits a `proc-macro` target.
- `build = "..."` or `build.rs` emits a `custom-build` target.
- target names, crate types, source paths, edition and doc/test/doctest flags
  follow the Cargo metadata subset.

The execution path is real: Cargo runs the build script when present, proc macro
crates are compiled/loaded by Cargo in a consumer check, and `ghor` records the
evidence without pretending that nC owns Rust's compiler internals.

## 15. Fuzzing and Registry Corpus

The regression suite includes a generated manifest/registry corpus that combines
SemVer requirements, feature forms and registry entries. This is not a public
crates.io mirror, but it gives a repeatable corpus for the resolver and index
reader without network flakiness.

The long-running CI workflow is the intended place for expanding that corpus
over time while preserving deterministic local tests.

## 16. Multi-Host CI

The Cargo-grade `ghor` closure has a dedicated multi-host workflow:

```text
.github/workflows/ghor-cargo-grade-release-industrial.yml
```

It runs the targeted `ghor_orchestrator` suite on Linux, macOS and Windows,
with rustfmt/clippy/check gates. The workflow can be invoked manually and is
also wired to pull requests and pushes that touch ghor, registry docs or
workflow files.

## 17. Backward Compatibility Policy

The compatibility policy is Cargo-like but nC-scoped:

- Existing `ghor.toml` and `nc.toml` manifests remain valid.
- Existing `use` of `ghor.lock.meta.json` remains valid for nC-rich metadata.
- Cargo-compatible `Cargo.toml` ingestion is additive.
- New diagnostics prefer warnings or explicit unsupported-by-design statuses
  over silent behavior changes.
- Lockfile changes must preserve a readable migration path and must not silently
  discard package identity, source or checksum information.
- Registry/index/cache formats must remain readable by the current release or
  produce a official diagnostic.

See also [ghor_backward_compatibility.md](ghor_backward_compatibility.md).

## Validation

Primary regression suite:

```text
cargo test -p ncc-cli --test ghor_orchestrator
```

Key tests:

- `ghor_lock_exports_cargo_compatible_lockfile_ok`
- `ghor_git_dependency_exact_head_ok`
- `ghor_registry_sparse_http_cache_etag_304_ok`
- `ghor_registry_git_index_fetch_ok`
- `ghor_update_real_resolver_dry_run_and_precise_ok`
- `ghor_vendor_registry_extracts_package_and_config_ok`
- `ghor_tree_and_metadata_include_resolver_nodes_ok`
- `ghor_workspace_cli_check_build_test_workspace_ok`
- `ghor_registry_http_web_api_yank_owner_login_ok`
- `ghor_cargo_toml_real_project_metadata_matches_cargo_subset_ok`
- `ghor_cargo_toml_discovery_semver_and_target_cfg_ok`
- `ghor_cargo_toml_source_replacement_uses_local_mirror_ok`
- `ghor_manifest_registry_corpus_fuzz_smoke_ok`

## Remaining Boundaries

- ghor implements a registry CDN/cache, governance policy and moderation state;
  it does not claim to be the public crates.io service or inherit crates.io's
  real-world account database.
- ghor executes Rust build-script/proc-macro checks through the real Cargo
  toolchain; it does not replace Cargo or `rustc` as Rust's official compiler
  pipeline.
- ghor does not replace external host compilers.
- ghor does not silently accept unverified registry bytes.
- ghor keeps `ghor.lock` as the Cargo.lock-style TOML primary lockfile and preserves nC-rich lock metadata in `ghor.lock.meta.json`.
