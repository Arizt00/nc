# Pointer Industrial Closure

The industrial pointer closure defines one canonical C-like raw model. `p<T>`/`ptr<T>` operations compile without ownership, lifetime, bounds, generation, provenance, nullability, or alignment gates. Auditing is optional, read-only, and codegen-neutral.

## Core Types

`addr` is a raw address value. It is not a typed pointer and does not carry element type, ownership, bounds, or lifetime by itself.

`p<T>` is the compact spelling of `ptr<T>`. It is a raw address whose pointee type selects stride, load/store width, and ABI shape; it carries no safety state.

`ptr mut<T>` remains source-compatible, but raw aliases are accepted and exclusivity is never inferred.

`ptr<T> | null` remains available for API modeling, but plain `p<T>` may contain null. Dereferencing it compiles and may have undefined behavior.

`ref<T>` and `ref mut<T>` model shared and mutable references. They are pointer-family types but represent safer contracts than raw address authority.

`site<T>` and `memRegion<T>` model memory with known provenance and bounds. `memRegion<T>` index access is checked against known length when available.

## Authority Model

Raw pointer power is available by default. A risk authority section can still record `rawPtr`, `rawAddr`, `rawSite`, and `silentPointer` as optional audit evidence, but it does not unlock a separate execution mode or turn dangerous work into safe work.

Direct raw pointer casts, arithmetic, indexing, reads, writes, free, and realloc need no authority. Null, misalignment, aliasing, UAF, double free, dangling values, out-of-range access, and overflow compile. Their behavior is undefined as in C and is never promised to be deterministic. Bounded arrays, vectors, slices, buffers, and `memRegion<T>` remain separate checked types.

## Closed Surfaces

This closure covers parser/AST recognition, normal raw sema/type rules, optional heap/ownership/alias/lifetime/site auditing, FFI ABI contracts, machine lowering evidence, fixtures, and doctors.

The native backend wires the pointer family into real addresses. `ptrWrite<T>` emits a typed store, `ptrRead<T>` a typed load, `ptrOffset<T>` wrapping element-width movement, and `ptrDiff<T>` an address difference in element units. State hazards are optional audit findings, not compilation gates or injected machine checks.

The concise heap surface is also closed: `alloc`, `free`, `realloc`, and `zero` are native-prelude operations and remain available through their legacy qualified aliases. The generic pointer functions preserve width and raw address arithmetic without hidden state checks.

Basal receives the model as records, not as a semantic rewrite. `BasalExecutableSurfaceIr.pointer_accesses` records `addr`, `mem[...]`, raw pointer hazards, region bridges, and evidence boundaries. This keeps basal closed while still making pointer use visible for reports, NBB executable-surface metadata, and later release evidence.

The compiler lowering plan is exposed through `ncc compiler pointer-lowering --json`. It maps pointer read/write/offset/diff/address/region operations to machine intents such as load, store, LEA/add, subtraction plus scaling, and relocation/frame address materialization. This does not change [@machine] syntax or instruction base.

For C/C++ parity, nC now records the boundary explicitly instead of pretending all foreign pointer behavior is safe. C ABI crossings require header, symbol, layout, alignment, ownership, nullability, and who-frees contracts. C++ ABI crossings are backed by the real `ncc compiler ffi-cpp-abi --json --strict` smoke: native C++17 name mangling, template instantiation, vtable dispatch, virtual destructor execution, exception-boundary rejection, object files, static archives, and shared libraries are compiled, linked, inspected, and executed through the host toolchain. ASM bridges require register, stack, object, relocation, and call-preserved-register metadata. RAII-like lifetime, moves, references, smart-pointer-like ownership, and template-style pointer operations are modeled as nC lifetime/generic contracts with raw authority when ambiguity is intentional.

The IDE/LSP catalog includes pointer functions and diagnostics so hover, completion, and auto-import candidates reflect the same surface that `ncc` can execute.

The closure deliberately does not modify [@machine], primitive, ghor, or future kernel semantics.
