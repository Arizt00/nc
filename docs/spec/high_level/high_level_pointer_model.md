# High Level Pointer Model

Status: closed for normal/high-level nC.

Scope: this model covers normal nC and high-level nC. It does not reopen basal, machine, primitive or future kernel doctors.

## Baseical Pointer Family

The official pointer family is:

- `addr`: raw untyped address.
- `p<T>`: official compact spelling of `ptr<T>`.
- `ptr<T>`: raw typed address; it may be null, dangling, misaligned, aliased, or outside an allocation.
- `ptr mut<T>`: legacy typed spelling retained for source compatibility; raw pointers do not carry exclusive ownership.
- `ref<T>`: safe shared reference surface.
- `ref mut<T>`: safe mutable reference surface; it may upcast to `ref<T>`.
- `site<T>`: site/provenance pointer surface documented for memory-site ownership.
- `memRegion<T>`: memory region with element type and runtime-region import contract.

Raw pointers accept null directly; a nullable union remains useful only when APIs want to model null in their type contract:

```n
p: ptr<byte> | null;
region: memRegion<byte> | null;
```

This raw form also compiles:

```n
p: p<byte>;
@p >> p<byte> : null;
@value >> byte : pRead(@p); // Compiles; execution is undefined.
```

## Provenance

The optional auditor may classify pointer values by provenance:

- `stack_local`
- `heap_site`
- `static_global`
- `ffi`
- `raw_addr`
- `machine_basal_memory`
- `unknown`

Normal compilation does not require provenance. `E_PTR_PROVENANCE_001` and `E_PTR_STALE_SITE_001` belong to opt-in pointer auditing or bounded-view/runtime diagnostics.

## Alias And Mutability

Raw aliases are always accepted and no `noalias`, ownership, lifetime, or generation promise is inferred. Type-compatible projections remain available for:

- `ptr mut<T>` to `ptr<T>`
- `ref mut<T>` to `ref<T>`
- `site<T>` to `memRegion<T>` where the site is represented as a region

Address-to-pointer casts need no `risk`, `silentPointer`, ownership, or provenance authority. Type-shape errors remain ordinary type diagnostics; hazardous alias relations may be reported by the optional auditor.

## Bounds And Alignment

Array, vector and `memRegion<T>` paths keep bounds-aware element typing. Direct pointer indexing and `ptrRead`/`ptrWrite` use raw machine-address semantics:

```n
@value >> int : @p[0]; // Raw access; no allocation-range gate.
```

Use `memRegion<T>` when an operation must retain a checked span. When a region is created from a literal bound, the checker preserves that bound:

```n
@region >> memRegion<byte> : regionFromPtr(@bytes, 4);
@value >> byte : @region[4]; // E_PTR_OUT_OF_BOUNDS_001
```

The pointee width determines load/store width and ABI alignment, but raw casts are not rejected for misalignment:

```n
@base >> addr : 0x1003;
@p !> p<qword> : @base; // Compiles; a misaligned access may be undefined.
```

Bounded-view diagnostics remain active. Alignment diagnostics are optional audit evidence for raw pointers:

- `E_PTR_ALIGNMENT_001`
- `E_PTR_OUT_OF_BOUNDS_001`

## Raw Pointers

Raw pointer power is the only normal pointer semantics. A `risk` section is legacy optional evidence and does not enable a different mode:

```n
risk:
    allow rawPtr;
    allow rawAddr;
    accept rawPtr "manual pointer provenance boundary";
    report pointerHazardEvidenceBoundary;
```

Pointer arithmetic wraps at machine width, uses `sizeof(T)` stride, and may leave any allocation. Null dereference, misalignment, aliasing violations, UAF, double free, dangling access, and overflow compile. Like C, the language does not guarantee what an undefined execution returns or whether it traps.

## FFI Pointer Contracts

Any foreign signature that mentions pointer-like data must declare pointer contracts:

- ABI
- layout/repr where required
- ownership
- nullability
- side effects
- lifetime/free/destructor where ownership crosses the boundary

Missing pointer nullability in a pointer-like FFI signature is `E_PTR_FFI_CONTRACT_001`.

## Lowering Contract

Lowering to machine preserves:

- width
- endian/profile
- alignment
- volatility/atomic/barrier intent when declared
- relocation and symbol address intent

Raw lowering requires no authority. It carries no provenance, ownership, lifetime, generation, `inbounds`, `nsw`, `nuw`, or inferred `noalias` promise.

## CLI

```sh
ncc high-level pointer-model --json
ncc compiler pointer-audit file.n --json
ncc doctor --json
```

`ncc doctor` includes the pointer model under the normal/high-level scope and still excludes basal, machine, primitive and future kernel doctors by design.

## Examples

Positive:

- `examples/high_level/pointers/pointer_model_safe_ok.n`
- `examples/high_level/pointers/pointer_model_bounds_alignment_ok.n`
- `examples/high_level/pointers/pointer_model_raw_hazard_ok.n`
- `examples/high_level/pointers/pointer_model_ffi_contract_ok.n`
- `examples/high_level/pointers/pointer_raw_ptrv2_no_risk_ok.n`
- `examples/high_level/pointers/pointer_silent_ptrv2_no_authority_ok.n`

Optional audit fixtures (they compile under the canonical raw contract):

- `examples/high_level/pointers/negative/pointer_implicit_null_fail.n`
- `examples/high_level/pointers/negative/pointer_nullable_deref_fail.n`
- `examples/high_level/pointers/negative/pointer_provenance_uninit_fail.n`
- `examples/high_level/pointers/negative/pointer_out_of_bounds_fail.n`
- `examples/high_level/pointers/negative/pointer_alignment_fail.n`
- `examples/high_level/pointers/negative/pointer_stale_site_fail.n`
- `examples/high_level/pointers/negative/pointer_alias_mut_fail.n`
- `examples/high_level/pointers/negative/pointer_ffi_contract_fail.n`
