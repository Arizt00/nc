# Manual/raw source execution

Status: canonical specification for nC 2.0.0.2.

The compact surface is the official source form. The compiler is a mechanical
translator: it may parse, type-check, lower, encode and apply
semantics-preserving optimizations, but it cannot add a material effect that was
not requested by a source operation. A verbose internal representation cannot
change program semantics.

## Single execution contract

Every normal source resolves to `ManualRaw`. There is no second execution mode.
The material contract is:

- no implicit bounds, overflow, null, provenance, lifetime or alias checks;
- no implicit cleanup, allocation, collection growth, scheduler or dispatch;
- no unrequested I/O, process, thread, lock, recovery or release operation;
- raw pointers may alias, escape, dangle and address bytes outside an object;
- explicit operations remain in the generated IR and backend;
- an invalid access lowers as an address calculation, not as a compiler trap.

The program owns the consequences of undefined or invalid host behavior, as it
does in low-level C. `SourceExecutionContract::require_fully_manual` is checked
before IR lowering. A future compiler change that enables hidden work is
rejected with `E_SOURCE_MANUAL_CONTRACT_001`.

## What manual means

Manual refers to observable and material behavior. The source must request every
allocation, release, memory access, I/O operation, process, thread,
synchronization operation, scheduler transition, recovery path and dynamic
dispatch.

Syntax translation is not a hidden effect. `rd<byte>(p)` is an explicit pointer
read and `value <- expression` is an explicit request for compile-time type
inference. The fully spelled declaration remains available when the programmer
wants to state the type directly:

```n
value: int = 7;
bytePtr: p<byte> = asptr<byte>(rawPtr);
```

Neither spelling may allocate, free, trap, lock or call a runtime unless that
operation is itself present in the source contract.

String `+` uses automatic storage and never promotes itself to heap merely
because the pointer escapes. Returning that pointer may therefore create a
dangling value. `strConcat(left, right)` is the explicit heap-producing form and
must be paired with an explicit release when required.

`listAdd`, `listPush`, `queuePush` and equivalent dynamic collection operations
do not grow a full descriptor in `ManualRaw`; they report failure. Capacity is
changed only by an explicit create-with-capacity, reserve or realloc operation.
String elements are stored as raw pointers without an implicit clone.

## Open package profiles

`profile:` accepts any lexical identifier:

```n
package app { profile: raw; }
package app { profile: release_fast; }
package app { profile: vendor_board_x1; }
```

A profile is build, target or vendor metadata. It never enables checks or
hidden work. `manual` and `raw` are conventional labels for the only execution
contract, but other names are valid and resolve to the same contract.

## Explicit work

Allocation, release, pointer access, synchronization and waiting happen only
when source code requests them. The native prelude exposes these names without
source imports:

```text
alloc calloc realloc free
ptrRead ptrWrite ptrOffset
sizeOf alignOf offsetOf
atomicLoad atomicStore fence
sleepMs monotonicMs
```

The compiler records a canonical intrinsic or runtime symbol for each explicit
call and selects its backend. Prelude availability is name resolution only: it
does not emit a call on its own. A local declaration with the same name takes
precedence.

The optional pointer-state auditor is an explicit sema entry point. Normal
`analyze` and normal builds do not invoke it.

## Kernel boundary

Kernel lowering keeps the same raw contract and additionally forbids hosted
runtime, implicit heap, hidden cleanup, scheduler work and dynamic dispatch.
Memory/layout intrinsics and native atomics remain available when explicitly
called. Hosted sleep and console operations require an admitted kernel service
instead of silently importing a hosted runtime.

## Material evidence

The contract is represented by:

- `ProgramIR.source_contract` and `BackendModuleIR.source_contract`;
- `SourceExpansionReport`, which inventories explicit source operations and
  records zero compiler-owned runtime effects;
- normal build manifests and the source-execution JSON report.

No dedicated object or LLVM section is emitted for the contract. Executable
semantics come from the lowered instructions and calls, not metadata bytes.

## Cross-feature rule

Binary layout, C11 atomics, error propagation, monomorphized generics, pattern
decision trees, iterators, generators and coroutine state machines consume the
same contract. None may invent heap use, checks, dispatch or scheduling.
