# Canonical multi-target pointer contract

## Surface

`p<T[, space][, width]>` is the canonical raw pointer type. `ptr<T>` remains a compatible spelling. Spaces are `cpu`, `mmio`, `physical`, `dma`, `gpu`, and `firmware`. Width is `16`, `32`, `64`, `128`, or the target width when omitted.

The pointee may be data, a function, `member<Owner,Field>`, `array<T,N>`, `T[]`, or an aggregate. `cap<T>` is the explicit 128-bit capability spelling reserved for capability-aware targets.

## Semantics

- Arithmetic wraps at pointer width and emits no bounds, lifetime, ownership, provenance, alignment, or alias checks.
- Raw LLVM address arithmetic uses `getelementptr` without `inbounds` and does not add `nsw` or `nuw` promises.
- `ptrDiff<T>` returns target-sized `isize` and computes `(left - right) / sizeof(T)`.
- Equality and unsigned ordering compare addresses after the explicit width conversion required by the operands.
- `ptrCast`, `ptrAddr`, and `ptrFromAddr` preserve, extend, or truncate bits according to source width, destination width, and target architecture.
- MMIO reads and writes are volatile automatically. Other spaces are ordinary unless the operation explicitly requests volatile access.
- Widths larger than the target remain lossless integer carriers. Dereference requires explicit narrowing to the native address width.

## LLVM address spaces

Native-width domains use address spaces 0 through 5. Explicit non-native widths use `width + domain` only when the target backend can address that width. Wider values use `i64` or `i128` carriers so LLVM is never asked to select an impossible address mode.

## Machine carriers

- Native width: one native register.
- Narrow width: one narrowed register with explicit zero extension or truncation.
- 64 on a 32-bit target, and 128 on a 64-bit target: a register pair.
- 128 on a 32-bit target: four registers.

The machine plan includes raw add/subtract, unsigned compare, load/store, extend/truncate, and indirect call recipes for x86_64, i386, AArch64, ARMv7/Thumb-2, RV32IMAC, and RV64.

## Evidence

`examples/06_memory/pointer_targets_complete.n` is the executable hosted example. Unit tests emit and ask Clang to assemble one real object for each of the six target triples. AST and machine tests cover function, member, fixed-array, flexible-array, aggregate, address-space, and carrier classification.
