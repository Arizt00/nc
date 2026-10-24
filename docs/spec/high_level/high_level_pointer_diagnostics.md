# High Level Pointer Diagnostics

Status: closed.

The following codes belong to opt-in pointer auditing, bounded views, FFI validation, or legacy reports. They do not reject normal raw `p<T>` compilation:

| Code | Meaning |
| --- | --- |
| `E_PTR_NULL_DEREF_001` | Optional audit evidence for a tracked null raw pointer, or a non-raw reference contract. |
| `E_PTR_OUT_OF_BOUNDS_001` | A bounded region/span/slice access is outside its contract. Raw `ptr<T>` is exempt. |
| `E_PTR_STALE_SITE_001` | Optional audit or bounded-site evidence after free/realloc. |
| `E_PTR_ALIAS_MUT_001` | Optional audit evidence for a hazardous alias relation. |
| `E_PTR_ALIGNMENT_001` | Optional audit evidence for a potentially misaligned raw access. |
| `E_PTR_PROVENANCE_001` | Optional audit evidence for missing or uninitialized provenance. |
| `E_PTR_FFI_CONTRACT_001` | FFI pointer crossing lacks required ABI/layout/ownership/nullability contract. |
| `E_PTR_RAW_REQUIRED_001` | Legacy/strict-only gate for old safe-surface pointer policy. Pointer Model V2 does not require `risk` for C/C++-grade raw pointer operations in pointer-capable nC surfaces. |

Normal compilation checks type formation, but admits null, misalignment, aliasing, UAF, double free, dangling values, out-of-range access, and overflow. Run `ncc compiler pointer-audit <file.n>` to request pointer-state findings. Machine, basal, primitive, and kernel diagnostics remain independent.

## Optional Audit Fixtures

- Implicit null: `pointer_implicit_null_fail.n`
- Nullable deref: `pointer_nullable_deref_fail.n`
- Legacy raw-required gate: retained only for strict-policy documentation; Pointer Model V2 fixtures now pass as raw backend intent.
- Missing provenance/uninitialized pointer: `pointer_provenance_uninit_fail.n`
- Static memRegion bounds: `pointer_out_of_bounds_fail.n`
- Address-to-pointer alignment: `pointer_alignment_fail.n`
- Stale pointer/site: `pointer_stale_site_fail.n`
- Invalid alias/mutability: `pointer_alias_mut_fail.n`
- FFI contract: `pointer_ffi_contract_fail.n`

## Deterministic Fuzz

Deterministic audit fuzz may require reproducible `E_PTR_*` findings. Native execution of undefined behavior must never be tested for one fixed result.
