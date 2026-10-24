# Backend real CLI contract - Priority 2

Status: implemented as the official hosted backend route for backend-lowerable `.n` programs.

This contract defines the user-facing command surface that separates the source-level runtime executor from the backend/native route.

## Execution modes

`ncc run file.n`

- Default mode: interpreted/runtime executor.
- Purpose: broad language coverage, runtime families, diagnostics and teaching examples.

`ncc run file.n --interpreted`

- Explicit spelling of the default runtime executor.
- It is rejected when combined with `--real`, `--native`, `--native-fast` or a native execution profile.

`ncc run file.n --real`

- Alias for the real backend/native-fast route.
- Equivalent intent to `--native-fast` and `--profile release-fast`.
- It parses, checks, lowers to backend object IR, links a hosted native artifact and runs it.
- If the program declares `foreign:` contracts and matching sidecar sources are available, the real route autolinks C/C++/Rust sidecars through the hosted FFI path.

`ncc run file.n --profile real`

- Profile alias for `release-fast`.
- Kept as ergonomic spelling, not as a separate backend tier.

## Build modes

`ncc build file.n out-exe`

- Official explicit-output build form.
- Parses, checks and lowers Basal root into `.ncc/lowering/<unit>.nm`, retains
  `.ncc/lowering/<unit>.backend-object.nbo`, emits `.ncc/obj/<unit>.obj`, invokes
  the configured linker and writes a manifest under `.ncc/run`.

`ncc build file.n`

- Standard C-like build form.
- If the output path is omitted, the CLI writes to `.ncc/bin/<source-stem>.exe`
  and still retains `.nm`, `.obj` and manifest evidence.

`ncc build file.n --real`

- Compatibility compact build form for real backend output.
- If the output path is omitted, the CLI writes to `target/nc/bin/<source-stem>` on Unix and `target/nc/bin/<source-stem>.exe` on Windows.
- The main symbol is resolved from the AST, including `ref main ->> ...;` entries.

`ncc build file.n --interpreted`

- Invalid by design. Build is backend-only; interpreted execution belongs to `ncc run file.n --interpreted`.

## Current boundary

This phase does not claim that every runtime family is lowered to standalone machine syscalls. Runtime OS services from Priority 1 are real in `ncc run`'s runtime executor. `--real` is the native backend route for backend-lowerable programs: scalars, arrays, pointers, structs, calls, the existing native-fast corpus and hosted FFI sidecars declared through `foreign:`.

The next backend/runtime phase should connect selected OS runtime calls to native ABI calls or nC FFI lowering without changing this CLI surface.

## Verification

Required checks for this contract:

- `cargo test -p ncc-cli --test native_fast_path -- --nocapture`
- `cargo test -p ncc-cli ncc_execution_profile_routes_release_fast_to_native -- --nocapture`
- `ncc run examples/main_family_v2_0_0/main_ref_int_ok.n --real --show-return`
- `ncc build examples/main_family_v2_0_0/main_ref_int_ok.n --real`
