# Runtime Import Closure Table

`ncc imports runtime-table --json` is the single truth table for the runnable
state of every compact standard import, `use::auth` and `use::dom` item.

Every import item must have exactly one `closure_status`:

- `closed_real_backing_bytes`: the import has MCT activation bytes and `ncc run`
  can call a real runtime or OS-backed symbol.
- `closed_native_backend`: the import has MCT activation bytes and `ncc run`
  can lower it through the language/native backend intrinsic.
- `closed_interpreter_runtime`: the import has MCT activation bytes and is
  supported by the current runtime executor.
- `catalog_only_with_diagnostic`: the item is reserved in the catalog, but must
  not be treated as runnable support. It carries
  `E_USE_BACKING_DEFERRED_001`.

This prevents the old failure mode where an item looked supported because it
was listed in the import facade, but then failed later during `ncc run`.

## Commands

```text
ncc imports runtime-table --json
ncc imports runtime-table examples/runtime/runtime_real_fs_append_fsync.n --json
```

Passing a source file filters the table to the imports actually used by that
file.

## Contract

Closed:

- Every listed import has exactly one of the four official statuses.
- Non-runnable imports are visible as `catalog_only_with_diagnostic`.
- Runnable imports set `can_run_without_runtime_import_failure` to `true`.
- Catalog-only imports set `can_run_without_runtime_import_failure` to `false`.
- FS, TCP, process and thread imports are classified as
  `closed_real_backing_bytes`.
- Pointer, heap, region, array, list and result imports are classified as
  `closed_native_backend`.
- Input, output, render, random, time and sleep imports are classified as
  `closed_interpreter_runtime`.

Not claimed:

- A catalog-only import does not become runnable merely because it has an
  activation thunk.
- Import activation bytes do not replace the implementation of the runtime
  operation itself.
