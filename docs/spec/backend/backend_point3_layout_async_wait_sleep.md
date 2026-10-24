# Backend Point 3 - Aggregate layout and C-level wait/sleep lowering

Status: implemented for the backend boundary.

This point closes the backend-owned part of aggregate layout and blocking
time-wait lowering. Runtime schedulers, cooperative task registries and async
workers remain runtime-owned APIs; the backend contract here is that their ABI
symbols stay visible and that C-level blocking sleeps lower to real host calls
when the target has an OS ABI.

## Closed

- Named `type:` structs, enums and tagged unions keep one stable
  `TypeAbiLayoutIR` descriptor with schema, kind, qualified name, size,
  alignment, field count, field offsets, field sizes, field alignments and
  fingerprint.
- MCT LLVM emits the same layout facts as executable comments:
  `ncc-type-abi` and `ncc-type-abi-field`.
- Nested structs use C-like source-order natural alignment. Each field offset is
  aligned to the field alignment and the final aggregate size is rounded to the
  largest field alignment.
- Fixed arrays of structs lower to `[N x %struct.X]`; indexed field access uses
  the struct layout metadata instead of byte-string guessing.
- `vector<Struct>` and dynamic aggregate arrays use a descriptor shape with
  backing data pointer, length and capacity. Field access computes:

```text
element_addr = data + index * sizeof(Struct)
field_addr   = element_addr + field_offset
```

- `packets[i].tag` and `packets.get(i).tag` lower through the same stride and
  field-offset path.
- Backend ABI planning keeps aggregate by-value, indirect, stack and hidden-sret
  decisions target-aware for SysV64 and Win64.
- `sleep`, `sleepMs`, `sleepSec`, `threadSleep` and `yieldRuntime` are
  recognized by the MCT backend as C-level blocking wait/sleep operations.
- Windows targets lower blocking sleep to `Sleep(i32)` and the PE64 internal
  linker maps the runtime surface to the `KERNEL32.dll` import thunk.
- Hosted POSIX targets lower blocking sleep to `usleep(i32)`.
- Sleep durations are clamped before the host call: negative values become zero
  and oversized values are bounded to the maximum accepted by the selected call
  lane.
- Freestanding or non-hosted targets do not fake a blocking OS sleep; the MCT
  value is `false` when no hosted sleep ABI exists.
- Standard-build runtime import accounting recognizes sleep, wait/task/async and
  event wait symbols as materializable runtime imports when they appear at the
  object boundary.

## Backend truth boundary

This point does not move the full async scheduler into the backend. A C compiler
does not implement an event loop in instruction selection; it emits calls with a
clear ABI. nC follows the same split:

- blocking sleep/wait that maps to the host C/OS ABI is backend materialized;
- async handles, tasks, futures, cancellation and event loop behavior stay in
  the runtime catalog and runtime implementation;
- when those runtime calls cross into machine/linker, they are preserved as
  direct ABI symbols instead of hidden interpreter-only actions.

## Evidence

- `crates/ncc-ir/src/lib.rs`
  - `mct_llvm_lowers_struct_layout_and_field_access`
  - `mct_llvm_calculates_nested_forward_struct_layout`
  - `mct_llvm_lowers_nested_struct_field_access`
  - `mct_llvm_lowers_vector_struct_field_access_by_index_and_get`
  - `mct_llvm_lowers_vector_struct_nested_field_access_by_index_and_get`
  - `mct_llvm_lowers_posix_sleep_family_to_c_usleep`
  - `mct_llvm_lowers_win64_sleep_family_to_kernel32_sleep`
- `crates/ncc-cli/tests/native_fast_path.rs`
  - native-fast struct field read/write smoke
  - nested struct field read/write smoke
  - struct array field read/write smoke
  - vector struct push and field access smoke
- `crates/ncc-cli/src/commands/machine.rs`
  - PE64 runtime sleep thunk through real `Sleep` import

## Validation command

```text
cargo test -p ncc-ir mct_llvm_lowers_ -- --nocapture
```

On this workspace the command was run through the local rustup cargo path and
passed 23 filtered IR backend tests.
