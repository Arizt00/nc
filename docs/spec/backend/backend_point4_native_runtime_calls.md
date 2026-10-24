# Backend Point 4 - Native runtime calls

Status: implemented for backend admission, object reporting and linker-visible
runtime symbols.

This point closes the backend side of native runtime calls: a runtime call that
reaches the native build/object boundary must be recognized from the official
catalog, reported in the build manifest, and handed to the machine/internal OS
linker as a real symbol. The backend no longer keeps a private manual runtime
allowlist that can drift from the language catalog.

## Closed

- Runtime native-call admission is derived from:
  - `ncc_ast::runtime_api_family_specs()`;
  - `ncc_ast::use_backend_lowering_manifest_all()`;
  - the backend-owned C/OS bridge symbols used by MCT and Phase12.
- The admission set accepts:
  - full symbols such as `nC.runtime.fs.fswrite`;
  - module symbols such as `runtime.fs.fswrite`;
  - leaf symbols such as `fswrite`;
  - bridge spellings such as `__imp_Sleep`, `Sleep`, `usleep`, `write`,
    `printf`, `memcpy`, `ShellExecuteA`, `WriteFile` and related OS calls.
- Catalog-only activation symbols are intentionally excluded. An import such as
  `use stoud.io { entropy };` can still retain deterministic activation bytes,
  but it is not reported as a complete native runtime call until it has real
  backing symbols.
- The standard build manifest now includes:

```text
machine_backend.runtime_import_materialization.native_call_admission
```

The payload records catalog authority, official runtime function count, backend
import entries, admitted alias count, and `manual_runtime_allowlist: false`.

- Existing direct MCT builtins remain native:
  - console/output bytes;
  - render and string helpers;
  - memory allocation and string libc calls;
  - math FP intrinsics/calls;
  - sleep/wait calls from Backend Point 3.
- Runtime calls that are not direct MCT builtins stay linker-visible as runtime
  symbols instead of being hidden as interpreter-only behavior.

## Native call boundary

Backend is responsible for symbol admission, ABI visibility, object/report
evidence and handoff to machine/linker. Runtime remains responsible for the
semantics of complex families such as scheduler, networking, process registries,
permissions, handles and async task state.

That split mirrors C/C++: the compiler emits a native call and preserves ABI
facts; the runtime/library provides the implementation.

## Evidence

- `crates/ncc-cli/src/commands/tooling.rs`
  - `nbuild_runtime_import_symbol_set`
  - `nbuild_is_runtime_import_symbol`
  - `nbuild_runtime_native_call_admission_payload`
- Unit tests:
  - `nbuild_runtime_import_symbol_admission_covers_official_runtime_catalog`
  - `nbuild_runtime_import_symbol_admission_covers_backend_import_manifest`
  - `nbuild_runtime_import_symbol_admission_covers_host_c_and_os_symbols`
  - `nbuild_runtime_import_symbol_admission_keeps_catalog_only_activation_out`
  - `nbuild_runtime_native_call_admission_payload_records_catalog_authority`

## Required behavior

If a new runtime family or function is added to the official runtime catalog, it
is automatically admitted at the backend runtime-call boundary. If it has no real
backing symbols, it remains catalog-only and must not be reported as complete
native materialization.

This is the source-of-truth rule for Point 4.
