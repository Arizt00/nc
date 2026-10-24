# Compiler Memory Contracts

Status: canonical for raw pointers; bounded aggregate views keep an independent contract.

- `p<T>` is the compact spelling of canonical `ptr<T>`. Both denote a raw typed machine address.
- Raw pointers carry no mandatory ownership, lifetime, bounds, generation, provenance, nullability, or alignment state.
- Null, uninitialized values, misalignment, aliasing, out-of-range access, use-after-free, double-free, dangling values, and address overflow compile without `risk` or `silentPointer` authority.
- Those operations have C-like undefined behavior. Acceptance never guarantees a stable value, crash, trap, or repeatable result.
- `ptrOffset<T>` scales by `sizeof(T)` and wraps at target pointer width. LLVM lowering uses non-`inbounds` GEP and no `nsw`/`nuw` promise for raw address arithmetic.
- `alloc(n)` means `n` raw bytes and reserves its range before a binder is assigned.
- Arrays, vectors, slices, buffers, sites, and `memRegion<T>` preserve their length/span proofs and bounds diagnostics.
- `regionFromPtr<T>` checks that the requested bounded view fits its owner; `regionPtrAt<T>` checks index and generation.
- `ncc compiler pointer-audit <file.n>` enables the optional read-only pointer-state auditor. It does not modify AST, IR, optimization, ABI, object bytes, or executable bytes. `--strict` only changes the tool exit status.
- `risk: rawPtr/rawAddr/rawSite/silentPointer` remains parseable as legacy evidence and is a semantic no-op.
- `ncc compiler bounds`, `memory-contracts`, `pointer-industrial`, and `pointer-lowering` expose bounded-view and lowering evidence.
- IR carries target-independent pointer width, stride, load/store, address-add, region base/end and generation intent; target legalizers select existing machine instructions.
