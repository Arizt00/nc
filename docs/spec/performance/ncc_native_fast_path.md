# ncc Native Fast Path

Status: closed for phase 2 high-level native execution routing.

This document defines the first performance closure point for `ncc`: making the
native backend route an explicit, tested, user-facing path for build and run
commands. It now also covers the normal high-level surface that is
backend-lowerable today: `to main`, `ref main`, scalar globals, local
initializers, fixed arrays, calls, control flow, string concatenation used by
native print/return paths, and hosted runtime helper calls declared for the
native route. It does not claim full C/C++/Rust optimizer parity on arbitrary
programs; that remains an optimizer, runtime, linker, benchmark and profiling
workstream.

## Goal

`ncc` must have a real route that parses, checks, lowers to backend object IR,
emits a native object and invokes the host linker. The route must be selectable
from the official CLI without relying on hidden flags.

## Profiles

The following execution profiles are accepted by `ncc build`, `ncc build-exe`,
`ncc nbuild` and `ncc run` where applicable:

- `debug`: source/runtime interpreter route.
- `instrumented`: source/runtime interpreter route with checks-oriented intent.
- `release`: native fast path.
- `release-fast`: primary native fast path.
- `release-small`: native fast path with size-oriented intent.
- `unsafe-fast`: native fast path with raw speed intent.

The profile names are route contracts at this phase. They do not yet imply a
full optimizer pipeline equivalent to `-O3`, LTO, PGO or Rust `--release`.
`ncc build` now retains the normal C-like artifact chain (`.nm`, `.obj`,
`.exe`, manifest) through the standard backend route. `ncc compiler
build-native` remains the explicit LLVM/MCT lane that selects and reports `O3`
by default for `release-fast`.

## CLI Contract

Baseical commands:

```text
ncc build <file.n> <out-exe> --profile release-fast
ncc build <file.n> --out-dir .ncc --json
ncc build-exe <file.n> <out-exe> --profile release-fast
ncc compiler build-native <file.n> --profile release-fast --out-dir .ncc --json
ncc run <file.n> --profile release-fast
ncc run <file.n> --native
ncc run <file.n> --native-fast
```

`--native-ffi` remains accepted as a compatibility alias for the same native
execution route.

## Native Route

The native fast path performs:

1. Source read.
2. Lexer/parser.
3. Semantic/type-contract validation.
4. Backend profile selection through `--cc`.
5. Backend object IR emission.
6. Native object serialization and retention as `.obj`.
7. Machine bridge retention as `.nm`.
8. Hosted process-entry launcher generation for `ncc run`.
9. Host linker invocation.
10. Executable materialization.
11. Manifest retention under `.ncc/run`.
12. Execution for `ncc run`.

High-level functions are not interpreted on this route when the program is
accepted by the native scalar builder. The entry body is emitted as native
function bytes, `stc` state is materialized in object data sections, and `to
main` is lowered to a physical entry function before linking.

The default backend profile remains `nC.sysv64`.

## Boundaries

This phase does not implement:

- New language semantics.
- New optimizer passes.
- Runtime removal for high-level constructs that still need OS/runtime services
  outside the declared native helper set.
- Heap-resizing vector/list descriptors; native-fast currently owns the local
  fixed-capacity descriptor shape used by backend-lowerable `vector<Struct>`
  push/index/field fixtures.
- Whole-program LTO.
- PGO.
- A blanket claim that every possible `.n` program matches C/C++/Rust codegen.

Those are later performance phases.

## Verification

The closure is backed by tests that:

- Parse execution profiles.
- Verify that `release-fast` routes to native execution.
- Build through a fake linker that writes a real output artifact.
- Run through a fake linker that writes a real executable and verifies the CLI
  reports `route=native-fast-path`.
- Run through the real host linker for the official minimal main fixture.
- Execute high-level `to main`, string/global-state, and global-array/call
  fixtures through `--native-fast --show-return` without interpreter fallback.
- Execute `vector<Struct>` push plus indexed/`.get` field access through
  `--native-fast --show-return` without LLVM/MCT fallback.
