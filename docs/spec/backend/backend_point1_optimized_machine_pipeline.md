# Backend Point 1: Optimized IR To Machine Object/Exe

Status: closed for the normal `ncc build` userland surface.

The standard build path for `.n` programs is now:

```text
.n
  -> parser + sema + main-entry validation
  -> optimized IR report
  -> Machine .nm lowering bridge
  -> Machine BackendObjectIR
  -> native program .obj bytes
  -> Machine .entry.obj bytes
  -> .exe
```

The implementation point is `run_nbuild_command` in the CLI. It now uses
`emit_optimized_backend_object_ir_from_ast_with_profile`, not the older direct
non-optimized object helper, before object serialization.

## Retained Artifacts

For `ncc build app.n --out-dir .ncc --json`, the manifest
`nc.compiler.standard_native_build.v1` records:

- `.ncc/lowering/app.optimizer.json`
- `.ncc/lowering/app.nm`
- `.ncc/lowering/app.backend-object.nbo`
- `.ncc/code/app.launcher.n`
- `.ncc/obj/app.obj`
- `.ncc/obj/app.entry.obj`
- `.ncc/bin/app.exe`
- `.ncc/bin/app.ps1`
- `.ncc/bin/app.bat`
- `.ncc/run/app.native-build.json`

`*.optimizer.json` is the optimizer evidence bundle. It includes AST, SSA,
loop, alias, register allocation, CPU tuning, instruction-selection, ABI/layout
and release zero-cost counters.

`*.backend-object.nbo` is textual Machine BackendObjectIR evidence. The
program `.obj` is emitted from that object IR as real target bytes for the
selected object format.

## Link Routes

Win64 COFF without `--linker` uses the internal PE64 linker route:

```text
program .obj + Machine .entry.obj -> native PE32+ .exe
```

Other hosted targets use the configured or discovered host linker:

```text
program .obj + Machine .entry.obj -> hosted executable
```

If a high-level program still contains runtime-heavy constructs not supported
by direct native object emission, the command may take the explicit LLVM/MCT
fallback. That fallback is native execution, but it is not counted as the
closed Point 1 direct path.

## Verification

Targeted local evidence:

```powershell
cargo check -q -p ncc-cli
cargo test -q -p ncc-cli --test native_fast_path `
  ncc_nbuild_windows_pe_executes_string_concat_without_shadow_space_clobber `
  -- --nocapture
```

The Windows test builds a normal `.n`, retains the optimizer and backend object
artifacts, emits PE64 through the internal machine linker and executes the
resulting `.exe` outside the IDE.

## Boundary

Closed here:

- normal `.n` build uses optimized IR before machine object emission;
- artifacts are retained and hashed in the manifest;
- Machine entry bridge is checked by the machine verifier and emitted to
  `.entry.obj`;
- Win64 PE64 direct path produces an executable without a host linker.

Not claimed here:

- every possible runtime import is direct-machine materialized;
- every target has an internal OS executable linker;
- LLVM/MCT fallback is removed.
