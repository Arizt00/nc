# Compiler Native C-Like Build

Status: closed for retained native artifact generation through the normal build
surface and the explicit LLVM/MCT diagnostic lane.

`ncc build <file.n|file.nm>` is the standard user-facing C-like build surface
for normal `.n` programs and hand-written Machine `.nm` programs. It no longer
requires a compiler-only subcommand to retain the material artifact chain:

- parse and semantic check `.n`;
- lower through the optimized backend route:
  AST optimizer, whole-program/LTO/PGO accounting, SSA/mem2reg,
  const-prop, CSE/GVN, LICM, loop optimizer, alias analysis, register
  allocation evidence and instruction-selection evidence;
- retain `.ncc/lowering/<unit>.optimizer.json`;
- materialize `.ncc/root/<unit>.nb` as the canonical Basal address origin;
- encode and verify `.ncc/root/<unit>.nbb` with the `NBB0` binary contract;
- retain a canonical Machine `.nm` bridge under `.ncc/lowering`;
- retain `.ncc/lowering/<unit>.backend-object.nbo` after the readable Machine
  bridge;
- retain a user-facing `launcher.n` under `.ncc/code`;
- emit a native `.obj` under `.ncc/obj`;
- emit a Machine entry `.entry.obj` under `.ncc/obj` when the executable needs
  a hosted or PE entry bridge;
- link a hosted `.exe` under `.ncc/bin` when no explicit output is supplied;
- emit `.ps1` and `.bat` host launchers under `.ncc/bin`;
- write a JSON manifest with hashes, target, entry, pipeline and artifact
  evidence under `.ncc/run`.

For `.nm` inputs, `ncc build` validates Machine directly, infers the target from
the `target:` block unless `--target <triple>` is supplied, emits the native
object under `.ncc/obj`, links the OS-loader executable internally, and writes a
`nc.compiler.standard_machine_build.v1` manifest.

Handwritten `.nm` inputs remain authoritative and are not copied into
`.ncc/lowering`. The `root/.nb` and `root/.nbb` pair belongs to high-level `.n`
lowering; it is therefore generated only when an `.n` source is built.

`ncc run <file.n> --real` and `ncc run <file.n> --native-fast` share the same
material contract for executable native runs. When the selected profile lowers
to real process bytes, the command executes the native image and retains:

- `.ncc/lowering/<unit>.nm`;
- `.ncc/lowering/<unit>.backend-object.nbo`;
- `.ncc/code/<unit>.launcher.n`;
- `.ncc/obj/<unit>.obj`;
- `.ncc/obj/<unit>.entry.obj` when a hosted entry bridge is needed;
- `.ncc/bin/<unit>.exe`;
- `.ncc/bin/<unit>.ps1`;
- `.ncc/bin/<unit>.bat`;
- `.ncc/run/<unit>.native-run.json`.

The hosted entry bridge is generated as Machine `.nm` and emitted as a native
object. The normal run/build surface does not generate a C `launcher.c`.
`launcher.n` is retained as the nC-level launcher for the produced executable;
the `.nm` bridge remains the byte-producing Machine artifact.

The standard `.n` build manifest now records:

- `paths.optimized_ir_report`
- `paths.basal_root_source`
- `paths.basal_root_binary`
- `paths.machine_backend_object_ir`
- `truth_boundary.object_is_optimized_backend_output = true`
- `truth_boundary.optimized_ir_report_retained = true`
- `truth_boundary.machine_backend_object_ir_retained = true`
- `speed_contract.complete_artifact_set = [".nb", ".nbb", ".optimizer.json",
  ".backend-object.nbo", ".nm", ".obj", ".entry.obj", ".exe"]`

The corresponding lineage schema is
`nc.lowering.source-root-machine-nbo.v2`. It hashes the input `.n`, generated
Basal source, verified Basal binary, Machine source, backend object, native
object and final executable. Editing any material layer requires rebuilding the
whole content-addressed chain; the compiler never silently combines stale
`.nb`, `.nbb`, `.nm` or `.nbo` files.

If a high-level runtime-heavy `.n` program cannot be materialized by the direct
Machine object route yet, `run --real` falls forward to the LLVM/MCT native
lane, retains `.ll`, `.obj`, `.exe` and a `native-run.json` route of
`mct-llvm-build-native-fallback`, then executes the produced system executable.
This fallback is still native execution and does not use the interpreter.

The generated executable is released for external execution after link: `ncc`
does not keep an open process/file handle on it, and Windows read-only metadata
is cleared if present. The retained `.ps1` and `.bat` files are recognized by
`ncc check` and `ncc run` as host launcher scripts.

For `--cc nC.win64`, the normal build surface now closes the Windows PE lane
without requiring a configured host linker. `ncc build <file.n> --cc nC.win64`
emits the high-level COFF object, retains the `.nm` bridge, emits a bridge COFF
entry object, and links a native PE32+ executable internally through Phase 12.
The PE writer emits `.reloc`, `IMAGE_REL_BASED_DIR64` entries for final `abs64`
slots, and ASLR flags `DYNAMIC_BASE` plus `HIGH_ENTROPY_VA`.
Passing `--linker <bin>` still selects the explicit host-linker compatibility
route.

`ncc compiler build-native <file.n>` remains as the explicit LLVM/MCT lane. It
adds `.ll` retention and uses `clang` for target-object materialization:

- parse and semantic check `.n`;
- lower through MCT to LLVM IR;
- retain a Machine `.nm` bridge under `.ncc/lowering`;
- retain `launcher.n`, `.ps1` and `.bat` launchers;
- compile a native `.obj` with `clang -c`;
- for Win64 COFF, emit the `.nm` bridge as `.entry.obj` and link PE32+
  internally through Machine Phase 12, including runtime/UI thunks and loader
  imports;
- for non-Windows targets, retain the existing `clang` host-link route;
- write a JSON manifest with hashes, target, optimizer and stage evidence.

The LLVM fallback selected automatically by `ncc build` follows that same
finalization rule. A Linux or WSL host therefore needs `clang` to produce the
Win64 COFF object, but it does not need MSVC, `link.exe`, MinGW CRT libraries or
a cross-host `ld.bfd` invocation to obtain the PE executable. The manifest
records `link.route = mct-llvm-coff-phase12-pe64`, the Machine entry object,
link map, full Phase 12 report, and a truth boundary that distinguishes the
Clang-produced object from the internally produced executable.

Default performance profile:

- `--profile release-fast`;
- LLVM/MCT optimization level `O3`;
- no interpreter fallback;
- failed object/link/run stages are reported as failures.

Boundary:

The generated `.obj` is real Clang target output in the LLVM/MCT lane. The
generated `.exe` is either an internal Phase 12 loader artifact for supported
final targets such as Win64 PE32+, or a host-toolchain artifact on the retained
non-Windows route. The
retained `.nm` is a valid Machine bridge that records the native entry handoff
to `main`; on Win64 it also owns process termination through `ExitProcess`. It
is not yet a full
instruction-for-instruction Machine lowering of every possible high-level `.n`
construct.

Standard example:

```sh
ncc build examples/10_mct/print_bytes.n --out-dir .ncc --json
```

Win64 PE example:

```sh
ncc build app.n --out-dir .ncc --cc nC.win64 --json
```

Machine source example:

```sh
ncc build app.nm --out-dir .ncc --json
```

Native run example:

```sh
ncc run app.n --real --show-return
```

Explicit LLVM/MCT example:

```sh
ncc compiler build-native examples/10_mct/print_bytes.n \
  --out-dir .ncc \
  --profile release-fast \
  -O3 \
  --run \
  --json
```
