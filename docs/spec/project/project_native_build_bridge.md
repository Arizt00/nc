# Native build over `bridge.json` and `use <project:...>`

## Public command

The normal native build authority accepts a project mother directly:

```text
ncc project sync <project-dir>
ncc build <project-dir>/main.n [output-executable] --json
```

`ncc build` walks from the requested source toward its ancestors. When it finds
`bridge.json` and the requested source is the manifest `main`, it loads the
validated link order, reads every mother/child source identity, materializes the
project-qualified calls in one linked AST, and only then runs semantic analysis
and native lowering.

Building a child source directly does not silently select the entire project.
The formal bridge is selected only through its declared mother.

## Single native authority

The linked AST enters the same `nbuild` pipeline used by a standalone `.n`:

1. linked semantic validation;
2. optimized backend object emission;
3. retained typed `[@machine]` entry bridge;
4. target-native object serialization;
5. internal PE/ELF/Mach-O executable linking when the target admits it;
6. launcher and native-build manifest generation.

This route does not call `ncc project link` and does not generate that command's
C runtime shim. Project composition is resolved before lowering, so UI/runtime
imports remain ordinary backend-visible imports and are materialized by the
existing native linker/runtime authority.

The native-build JSON contains `project_linkage` with schema
`ncc.project.native_build_input.v1`. It records the project root, bridge path,
link order, edge list, unit count, and the explicit truth marker
`runtime_c_shim_used: false`.

## Implicit compact workspace

If no ancestor `bridge.json` exists but the source contains
`use stoud.project { unit };`, `ncc build` uses the existing implicit sibling resolver.
It searches the source directory, its `src` subtree, and `NCC_PROJECT_PATH`,
writes `.ncc/code/*.json` identities, and sends the resulting linked AST to the
same native build authority. The manifest reports
`source_mode: implicit-use-project`.

## Integrity and failure policy

A formal project build does not guess around stale metadata. Source hashes,
`.ncc/code/*.json`, bridge imports/dependencies, and `link_order` must agree.
After changing a mother or child, run `ncc project sync <project-dir>` before the
next build. Diagnostics from those checks retain the existing `E_PROJECT_*`
codes.

The LLVM/MCT fallback also receives the already linked AST. It never reloads
only the mother source and therefore cannot discard child units during fallback.
