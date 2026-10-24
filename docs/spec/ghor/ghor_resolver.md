# ghor Resolver

`ghor resolve` implements a Cargo-like dependency resolver for nC package orchestration. It is designed to match Cargo's registry protocol expectations while keeping ghor's language/runtime scope explicit.

## SemVer

The resolver parses and applies Cargo-style SemVer requirements:

- caret requirements: `1`, `1.2`, `1.2.3`, `^1.2.3`;
- zero-major caret behavior: `^0.2.3` stays below `0.3.0`;
- tilde requirements: `~1`, `~1.2`, `~1.2.3`;
- wildcard requirements: `*`, `1.*`, `1.2.*`, `x`;
- comparators: `=`, `>`, `>=`, `<`, `<=`;
- comma/space-separated comparator sets.

When multiple registry entries match, ghor chooses the highest non-yanked compatible version. Resolver v3 also prefers entries whose `rust_version` is compatible with the manifest `toolchain.rust_version` when such a compatible candidate exists.

## Dependency Kinds

The manifest supports:

- `[dependencies]`: normal dependencies;
- `[dev-dependencies]`: dev dependencies;
- `[build-dependencies]`: build dependencies;
- `[target.<triple>.dependencies]`;
- `[target.<triple>.dev-dependencies]`;
- `[target.<triple>.build-dependencies]`.

Dependency inline tables support:

```toml
pkg = {
  version = "1",
  checksum = "<sha256>",
  features = ["serde"],
  default-features = false,
  optional = false,
  registry = "mirror",
  package = "actual-package"
}
```

## Feature Resolvers

`--resolver 1`, `--resolver 2`, and `--resolver 3` are supported.

Resolver v1:

- unifies features across normal, build, dev, and target-specific dependencies;
- activates target-specific dependency features regardless of the selected target, matching the older broad unification model.

Resolver v2:

- separates build dependency features from normal dependency features;
- separates dev dependency features unless dev resolution is explicitly included;
- does not activate features for inactive target-specific dependencies.

Resolver v3:

- keeps resolver v2 feature behavior;
- adds MSRV-aware version preference using `toolchain.rust_version` and registry entry `rust_version`.

Feature expansion reads the registry package entry `features` map, activates `default` when `default-features` is true, and expands feature aliases recursively.

## Overrides

ghor supports Cargo-style override surfaces:

```toml
[patch.crates-io]
pkg = { path = "../pkg-patched", version = "9.0.0" }

[replace]
pkg = { path = "../pkg-replaced", version = "8.0.0" }
pkg:1.0.0 = { path = "../pkg-exact", version = "1.0.0" }
```

Patch and replace are reported in the resolve plan as explicit `override` values. Path overrides do not pretend to be registry artifacts.

## Alternate Registries And Source Replacement

Alternate registries can be declared in `ghor.toml`:

```toml
[registries]
mirror = "/srv/ghor-registry"

[dependencies]
pkg = { version = "1", checksum = "<sha256>", registry = "mirror" }
```

Source replacement can be declared either in `ghor.toml` or `.cargo/config.toml`:

```toml
[source.crates-io]
replace-with = "mirror"

[source.mirror]
local-registry = "/srv/ghor-registry"
```

Supported replacement endpoints:

- `registry`;
- `local-registry`;
- `directory`;
- `git`;
- `[registries.<name>] index = "..."`
- `[registries] name = "..."`.

The resolve output records the original source, effective source, replacement kind, and resolved registry transport.

The native nC stage1 resolver follows every `replace-with` edge until it reaches
a concrete `registry`, `index`, `local-registry`, `directory`, `path`, or `git`
source. It rejects cycles and missing terminal sources with
`E_GHOR_REGISTRY_RESOLUTION_001`. Relative references are resolved from the
manifest directory. Named `[registries.<name>]` entries and direct dependency
`registry` values share this path.

`[basal-dependencies]` use a distinct package identity. Stage1 selects by root
id, safe name or declared alias, filters yanked versions, validates the exact
package-manifest file hash, then validates `basal.package.v1`,
`ghor_compatible`, `NBB0` and the canonical FFI manifest hash. Basal packages
never enter the Cargo crate cache accidentally.

## Workspace Unification

`[workspace] members = [...]` are parsed and member manifests participate in the root resolution plan. Feature groups are keyed by package, selected version, and resolver scope, so workspace feature unification is visible and reproducible.

## CLI

```text
ghor resolve --manifest-path ghor.toml --registry ./registry --resolver 2 --json
ghor lock --manifest-path ghor.toml --registry ./registry --resolver 2
ghor verify-lock --manifest-path ghor.toml --registry ./registry --resolver 2
```

`ghor.lock` embeds the resolve plan so lock drift reflects dependency resolution, feature unification, and source selection.

## Diagnostics

- `E_GHOR_RESOLVER_SEMVER_001`: invalid SemVer requirement;
- `E_GHOR_RESOLVER_VERSION_001`: no non-yanked registry version satisfies a requirement;
- `E_GHOR_REGISTRY_RESOLUTION_001`: package/version cannot be found for fetch;
- `E_GHOR_CHECKSUM_001`: remote dependency lacks checksum or package checksum mismatch.

## Closure

The resolver is closed for:

- SemVer selection;
- feature resolver v1/v2/v3 behavior;
- dependency kinds;
- target-specific dependency activation;
- patch/replace overrides;
- alternate registries;
- source replacement;
- basal registries and basal lock identities;
- workspace feature grouping;
- lockfile evidence.
