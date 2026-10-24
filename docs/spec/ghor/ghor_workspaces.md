# ghor Workspaces

Status: Cargo-grade workspace surface closed for ghor registry/orchestration.

Source reference: Cargo Workspaces, https://doc.rust-lang.org/cargo/reference/workspaces.html

## Scope

`ghor` workspaces model the Cargo workspace contract for nC orchestration:

- workspace root manifests with `[workspace]`;
- virtual manifests with `[workspace]` and no `[package]`;
- `members`, `exclude`, and `default-members`;
- inheritance from `workspace.package`;
- inheritance from `workspace.dependencies`;
- inheritance from `workspace.lints`;
- one shared `ghor.lock` in the workspace root;
- one shared `target/ghor` directory in the workspace root;
- workspace metadata and member actions through CLI.

This does not change nC language semantics. It only makes project orchestration behave like Cargo where workspace structure, selection, inheritance, lockfiles, and target directories are concerned.

## Root And Virtual Manifests

A root workspace manifest is a `ghor.toml` or compatibility `nc.toml` that declares `[workspace]`.

If it also declares `[package]`, the root package remains a package and can participate in the workspace.

If it declares `[workspace]` without `[package]`, it is a virtual manifest. Virtual manifests must declare:

```toml
[workspace]
members = ["crates/*"]
resolver = "3"
```

`resolver` is required for virtual manifests because there is no package edition to infer resolver behavior from.

## Members, Exclude, And Default Members

`members` accepts explicit package directories and simple glob patterns:

```toml
[workspace]
members = ["crates/*", "tools/ghor-helper"]
exclude = ["crates/experimental"]
default-members = ["crates/app"]
resolver = "3"
```

Rules:

- member paths are resolved relative to the workspace root;
- `exclude` removes matched paths from the effective member list;
- `default-members` selects the members used by `ghor workspace check/build/test/graph`;
- if a virtual workspace has no `default-members`, all members are used by default;
- path dependencies inside the workspace root are reported as effective members when metadata is requested, but explicit `default-members` are not widened by those path dependencies.

## workspace.package

Members can inherit package metadata by declaring `{key}.workspace = true`.

Root:

```toml
[workspace.package]
version = "1.2.3"
edition = "2026"
license = "MIT"
```

Member:

```toml
[package]
name = "workspace_app"
version.workspace = true
edition.workspace = true
license.workspace = true
kind = "bin"
```

The effective manifest reports inherited values in `package`.

## workspace.dependencies

Workspace dependencies are declared once and inherited by members:

```toml
[workspace.dependencies]
shared_dep = { path = "crates/shared_dep", version = "0.5.0", features = ["std"] }
```

Member:

```toml
[dependencies]
shared_dep.workspace = true
```

Rules:

- `workspace.dependencies` cannot declare `optional = true`;
- member features are additive with workspace dependency features;
- member dependency kind and target context are preserved;
- inherited dependencies report `workspace_inherited = true`;
- inherited path dependencies participate in resolution and lockfile generation.

## workspace.lints

Workspace lints can be inherited by package manifests:

```toml
[workspace.lints.rust]
unsafe_code = "deny"
```

Member:

```toml
[lints]
workspace = true
```

The effective manifest reports inherited lints with their namespace, such as `rust.unsafe_code = "deny"`.

## Shared Lockfile

All workspace members share:

```text
<workspace-root>/ghor.lock
```

Running `ghor lock --manifest-path crates/app/ghor.toml` writes the lockfile to the workspace root, not to the member package directory.

The lock embeds the resolver plan, including workspace member dependencies, inherited workspace dependencies, overrides, alternate registries, source replacement, and feature groups.

## Shared Target Directory

All workspace artifacts use:

```text
<workspace-root>/target/ghor/
```

This includes:

- build artifacts;
- docs;
- package output when no explicit `--out` is given;
- registry cache;
- registry publish staging;
- ghor artifact records;
- conformance/seal outputs.

Member builds do not create `crates/member/target/ghor` unless an explicit output path requests that behavior.

## CLI

Metadata:

```text
ghor workspace metadata --manifest-path ghor.toml --json
```

Member actions:

```text
ghor workspace check --manifest-path ghor.toml --json
ghor workspace build --manifest-path ghor.toml --json
ghor workspace test --manifest-path ghor.toml --json
ghor workspace graph --manifest-path ghor.toml --json
```

Member manifest inspection:

```text
ghor manifest --manifest-path crates/app/ghor.toml --json
```

Shared lock:

```text
ghor lock --manifest-path crates/app/ghor.toml --json
ghor verify-lock --manifest-path crates/app/ghor.toml --json
```

## Diagnostics

- `E_GHOR_WORKSPACE_RESOLVER_001`: virtual workspace manifest is missing `workspace.resolver`;
- `E_GHOR_WORKSPACE_MEMBERS_001`: virtual workspace manifest has no members;
- `E_GHOR_WORKSPACE_DEPENDENCY_001`: workspace dependency is invalid, including optional workspace dependencies;
- `E_GHOR_WORKSPACE_MEMBER_001`: a selected workspace member failed an action;
- `E_GHOR_WORKSPACE_001`: unknown workspace action.

## Closure

This closes the Cargo-grade workspace point for:

- workspace root/virtual manifests;
- member expansion and exclusion;
- default member selection;
- package/dependency/lint inheritance;
- shared lockfile;
- shared target directory;
- workspace metadata;
- workspace member command execution;
- resolver participation across members.

Future work, if requested, can add Cargo's full package selection flags such as `-p`/`--package`; the current closure covers the requested workspace protocol surface without changing ghor language or registry semantics.
