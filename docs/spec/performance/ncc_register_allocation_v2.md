# ncc Register Allocation V2

Scope: native-fast scalar backend.

Register Allocation V2 replaces the previous stack-first scalar lowering for local leaf blocks with a local linear-scan allocator. The allocator computes definition/use intervals for scalar locals, keeps address-free integer, boolean, pointer, and region-sized locals resident in caller-saved registers, and uses stack slots only when a value is not eligible, escapes by address, crosses a call/control-flow boundary, or exceeds available register pressure.

Register pool:

- `r8`
- `r9`
- `r10`
- `r11`
- `rsi`
- `rdi`

`rcx` and `rdx` are intentionally excluded from resident allocation because the current x86-64 scalar encoder uses `cl` for variable shifts and `rdx:rax` for division/remainder. This keeps local allocation fast without silently corrupting values during shift/division lowering.

Closed behavior:

- Local liveness intervals are computed from scalar projections, scalar expression statements, and returns.
- Address-taken locals are not made register resident.
- Arrays and explicit stack-backed regions remain stack-backed.
- Eligible locals are inserted into the scalar symbol table as `LocalRegister`.
- Register-resident projections compile directly into the assigned register.
- Symbol reads from register locals become register moves only when needed.
- Scratch allocation avoids resident registers and active scratch registers.
- Binary expressions, compares, pointer reads/writes, pointer offset/diff, region pointer addressing, and array address calculation use scratch registers when safe.
- Stack fallback remains available for calls, structured control flow, address escapes, arrays, and pressure spills.
- No callee-saved register push/pop is introduced by this allocator; it stays inside caller-saved register policy.

Spill policy:

When all V2 registers are busy for an interval, the local is classified as spilled and remains stack-backed through the existing scalar stack path. This keeps the first V2 release conservative and predictable: register pressure falls back to known-correct stack lowering instead of inventing partial stores.

Evidence:

- IR unit test `emit_backend_register_allocation_v2_keeps_leaf_locals_resident`.
- CLI integration test `ncc_run_native_fast_executes_register_allocation_v2_fixture`.
- Fixture `examples/performance/native_fast/register_alloc_v2_liveness.n`.
- Native execution returns `42` through `--native-fast`.

Boundary:

This is local block allocation, not global SSA allocation. Calls and structured control-flow blocks continue to use the proven stack path until global liveness, phi handling, and call-clobber modeling are added.
