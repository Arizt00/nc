# ncc package

`ncc package` is the normal project workflow for nC packages. It provides a
Cargo-style surface without replacing the existing compiler stages.

## Files

- `ncc.package.json`: editable package manifest.
- `ncc.lock`: resolved dependency lock.
- `src/main.n`: default high-level entry source.
- `.ncc/bin`: executables.
- `.ncc/code`: project identity JSON and user-facing launchers.
- `.ncc/lowering`: retained `.nm` Machine bridges, `.backend-object.nbo`, IR
  and lowering reports.
- `.ncc/obj`: native object files.
- `.ncc/run`: build reports.
- `.ncc/cache`, `.ncc/incremental`, `.ncc/lowering`, `.ncc/ide`: project caches
  and IDE/build metadata.

`.ncp` is not a package manifest extension. It is reserved for
`primitive[@]` / physical hardware sources.

## Commands

```powershell
ncc package init demo --path demo
ncc package add stoud::io --path demo
ncc package lock --path demo
ncc package verify-lock --path demo
ncc package check --path demo
ncc package build --path demo --profile release-fast --cpu native
ncc package run --path demo
ncc package clean --path demo
```

`package build` delegates native lowering to `nbuild`. That keeps one backend
authority for the real artifact chain:

```text
.n root -> .ncc/lowering/*.nm -> .ncc/lowering/*.backend-object.nbo
        -> .ncc/obj/*.obj -> .ncc/bin/<program>
```

`--no-exe` is available for CI/package graph validation when a host linker is
not available. It still checks the manifest, source file, parser, sema and lock
evidence.

Build profile options accepted by `package build` are forwarded to `nbuild`:
`--profile`, `--cc`, `--cpu`, `--tune-profile`, `--linker` and `--entry`.
