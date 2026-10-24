# Manual raw optimizer contract v1

## Authority

This contract governs every optimization applied to `p<T>`, integer-derived
addresses, escaped storage, MMIO, DMA and firmware memory in manual/raw mode.
The programmer owns validity, alignment, lifetime, allocation, synchronization
and recovery. The compiler must neither add safety policy nor invent facts that
the source and material layout do not prove.

## Alias rule

The default relation between two raw accesses is `MayAlias`.

`NoAlias` is permitted only for disjoint exact numeric ranges or separately
sealed compiler-owned objects whose addresses have not escaped. Different
names, globals, symbols, types or address spaces are not by themselves proof of
non-aliasing. `MustAlias` requires the same material identity and exact range.

Raw LLVM lowering therefore does not emit:

- `!tbaa` based only on source types;
- `noalias` or `alias.scope` based only on pointer spelling;
- `getelementptr inbounds` for arbitrary raw arithmetic;
- hidden runtime alias checks to enable speculative vectorization.

Explicit user contracts may select stronger semantics in a future profile, but
manual/raw never infers them.

## Legal transformations

DCE, store forwarding, CSE/GVN, LICM and vectorization may reorder or remove a
raw access only after proving the exact bytes cannot be observed through any
intervening access. Unknown calls, escaped addresses, MMIO, DMA, volatile and
atomic operations are observable memory barriers at the ncc-owned optimizer.
Implicit vectorization additionally requires a material range identity and an
explicit compiler proof that ranges from different loop iterations are
disjoint; it never inserts a hidden runtime alias-versioning branch.

Ordinary `memcpy`, `memmove` and `memset` retain their C byte contracts:

- `memcpy` requires non-overlap as a programmer precondition;
- `memmove` supports overlap;
- `memset` writes the low byte of its value;
- none adds bounds, lifetime, provenance or allocation checks.

Volatile accesses remain present and ordered according to volatile semantics.
Atomics preserve width and requested C11 order. They do not make adjacent
non-atomic memory safe or race-free.

## Machine materialization

The machine authority exposes explicit memory-copy, memory-set, volatile,
fence and atomic recipes for x86_64, i386, AArch64, ARMv7/Thumb, RV32IMAC and
RV64. Recipes use each target's actual string, load/store, barrier and atomic
instruction families. They do not allocate, validate ranges or select a memory
policy for the programmer.

## Certification

`ncc-certification-v43 certify-raw-optimizer` ([V43 native build](../../NATIVE_TOOLS_V43_COMMANDS.md)) verifies parser, sema, typed IR, LLVM,
native object, disassembly, link and execution. It also builds the six machine
profiles, runs the raw memory and kernel stress suites, and can invoke physical
board certification from explicit manifests.

A software run without a physical manifest must report
`not_executed_no_hardware_manifest`. QEMU, WSL and hosted execution cannot be
reported as physical certification.

Normative optimizer behavior follows LLVM's alias lattice and memory semantics,
while the differential build follows GCC's documented optimization controls:

- https://llvm.org/docs/AliasAnalysis.html
- https://llvm.org/docs/LangRef.html
- https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html
