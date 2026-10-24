# Pointer Dangling Bag Lab

This fixture family keeps the C-like pointer hazard visible without turning the
examples into silent memory-corruption payloads.

- `examples/pointers/stress_bank_risk_ptr_bag_ok.n` allocates four heap cells,
  writes risk scores with compact `.write`, reads through `.read`, opens a
  compact `@p[..n]` region view, recovers a pointer through `.ptr`, stores
  dangling addresses in a small bag, frees them, and prints the resulting freed
  state.
- `examples/high_level/pointers/pointer_dangling_bag_compact_ok.n` records the
  high-level raw pointer/address/site authority boundary with `risk:`.
- `examples/pointers/negative/stress_bank_risk_ptr_bag_uninit_fail.n` records
  the uninitialized-pointer boundary.
- `examples/pointers/negative/stress_bank_risk_ptr_bag_uaf_write_fail.n`
  attempts compact `.write` after free and must stop with a stale-pointer
  diagnostic.
- `examples/pointers/negative/stress_bank_risk_ptr_bag_double_free_fail.n`
  attempts a second free and must stop with a dead-block diagnostic.
- `examples/pointers/negative/stress_bank_risk_ptr_bag_overflow_fail.n`
  attempts an out-of-bounds `.ptr` recovery from a compact region view before a
  write.
- `examples/backend/mct_llvm_pointer_bag_bytes.n` is the backend bytes witness:
  it uses `addrOf`, compact `.write` and compact `.read` so MCT LLVM emits real
  pointer stores and loads for the host target.

The runtime examples prove lifetime/provenance behavior. The backend example
proves material load/store lowering. The boundary is intentional: the checked
runtime must kill invalid writes before they become use-after-free, double-free,
or overflow corruption in a host process.
