# Imports Backend Lowering

This document defines the backend/MCT lowering surface for compact standard imports,
`use::auth` and `use::dom`.

The import surface has two layers:

1. Semantic expansion: `use stoud.io { output };` expands to the official
   internal path `nC.runtime.output`.
2. Backend activation: every supported import item receives a deterministic
   backend symbol and concrete activation bytes for the official MCT profiles.
3. Runtime call lowering: imports with real backing also expose the concrete
   runtime symbols that `ncc run archivo.n` calls without requiring `--real`.

The activation symbol is not a replacement for runtime side effects. It is the
backend-visible proof that the import exists, is grouped, is linkable, and can be
materialized as bytes. Concrete operations such as filesystem access, process
spawning, networking or rendering are lowered to the resolved runtime/system
symbols recorded in `runtime_symbols`.

## Baseical Commands

```text
ncc imports backend --json
ncc imports backend examples/imports/stoud_io_basic.n --json
ncc imports runtime-table --json
ncc mct imports --json
```

`ncc imports backend --json` emits the complete grouped import lowering
manifest. Passing a source file filters the manifest to imports used by that
file.

`ncc mct imports --json` exposes the same manifest from the Modal Container
Translate side.

`ncc imports runtime-table --json` exposes the single runnable/catalog status
for each import. Its allowed statuses are `closed_real_backing_bytes`,
`closed_native_backend`, `closed_interpreter_runtime` and
`catalog_only_with_diagnostic`.

`ncc run archivo.n` uses those runtime APIs directly. No `--real` flag is part
of this contract for the runtime-backed imports listed as
`closed_real_backing_bytes`.

## Backend Symbols

Each import item receives a symbol:

```text
__nc_import_activate_<domain>_<family>_<item>
```

Example:

```text
use stoud.io { input };
__nc_import_activate_stoud_io_input
```

Runtime-backed entries also expose:

```json
{
  "status": "closed_real_backing_bytes",
  "backing_status": "closed_real_backing_bytes",
  "closed_real_backing_bytes": true,
  "runtime_call_lowering": "direct_runtime_symbol_call_without_real_flag",
  "runtime_symbols": ["nC.runtime.process.spawn"]
}
```

Runtime-backed imports are packaged as the external `ncc-runtime-stdlib`
component installed with the toolchain. The backend manifest keeps MCT
activation bytes in-repo, while hosted runtime symbols are resolved from that
stdlib component. Provider-specific calls can still keep deterministic language
semantics: for example, `renderBinary` may use OS/libc `%b` only after a probe
confirms support, and otherwise falls back to nC's exact byte renderer.

The compact facade keeps `sleep()` under the time family:

```n

```

lowers to:

```text
nC.runtime.time.sleep
```

`nC.runtime.sleep` remains a separate registered runtime family for explicit
sleep utilities such as `sleepMs` and `sleepSec`; it is not the target of the
compact `sleep()` facade.

Catalog-only entries remain visible for MCT byte accounting, but keep:

```json
{
  "status": "closed_catalog_only_with_activation_bytes",
  "closed_real_backing_bytes": false,
  "runtime_call_lowering": "catalog_only_activation_bytes_no_runtime_call_lowering",
  "diagnostics": ["E_USE_BACKING_DEFERRED_001: existe en catálogo pero no está bajado a backend"]
}
```

## Emitted Bytes

Each import activation symbol has concrete bytes for:

- `nC.sysv64`: x86_64 ELF64.
- `nC.win64`: x86_64 COFF.
- `nC.aapcs64`: AArch64 Mach-O.
- `nC.portable`: wasm32/WASI expression bytes.

The bytes return a stable import id:

- x86_64: `mov eax, import_id; ret`.
- AArch64: `mov w0, import_id16; ret`.
- wasm32: `i32.const import_id; end`.

These bytes are real instruction bytes. They are intentionally tiny activation
thunks; the runtime action itself is still performed by the expanded target
symbol.

## Families

`stoud` families lower as safe or medium-risk runtime import thunks:

- `io`
- `text`
- `collections`
- `memory`
- `time`
- `task`
- `sync`
- `audit`
- `net`

`auth` families lower as authority gate thunks and require policy:

- `system`
- `security`
- `resource`

`dom` families lower as domain service thunks:

- `compiler`
- `high`
- `inspect`
- `ai`
- `tooling`

## Truth Boundary

Closed:

- Every supported standard, `auth` and `dom` item has a deterministic backend symbol.
- Every supported item emits concrete activation bytes for the MCT profiles.
- Runtime-backed imports are registered as `closed_real_backing_bytes`.
- Runtime-backed imports list concrete runtime symbols for filesystem, TCP,
  process, thread and time/sleep calls.
- `ncc run archivo.n` uses those APIs without `--real`.
- Imports are grouped by domain and family.
- MCT can inspect the same manifest.
- Machine, primitive, basal and kernel remain import-forbidden.

Not claimed:

- `auth` imports do not fake OS side effects inside the import itself.
- Runtime function implementation is not replaced by import activation bytes.
- This does not turn machine, primitive, basal or kernel into import-based modes.
