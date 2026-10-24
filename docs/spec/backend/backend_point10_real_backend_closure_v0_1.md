# Backend Point 10 Real Backend Closure v0.1

Status: closed for the current declared backend surface.

Point 10 closes the backend as a coordinated execution matrix, not as a single
provider. The order is intentional:

1. MCT as byte contract.
2. LLVM for hosted executables.
3. native-fast for critical paths.
4. machine for low authority.
5. parity tests between declared overlaps.

The official command is:

```bash
ncc mct parity --json
```

It emits `nc.backend.point10.parity.v1`.

## MCT Byte Contract

MCT is the first truth layer. A backend route cannot claim execution if its
activation/import/materialization surface does not expose deterministic bytes.

Evidence:

- `ncc mct imports --json`;
- `ncc imports backend --json`;
- `UseBackendLoweringManifest.emitted_bytes`;
- `UseBackendLoweringManifest.runtime_symbols`;
- `status = closed_real_backing_bytes` for imports with real runtime backing;
- diagnostic `E_USE_BACKING_DEFERRED_001` for catalog-only imports.

Failure diagnostic:

```text
E_BACKEND_PARITY_MCT_BYTES_001
```

## LLVM Executables

LLVM is the hosted executable route. It emits real textual `.ll`, then uses
Clang/LLVM tools for object or executable smoke when available.

Evidence:

- `ncc mct emit-llvm ...`;
- `ncc mct llvm-smoke ...`;
- `ncc mct llvm-exe-smoke ...`;
- pass pipeline, debug metadata and target attributes in the MCT JSON report.

Failure diagnostics:

```text
E_BACKEND_PARITY_LLVM_EXEC_001
E_BACKEND_TOOLCHAIN_MISSING_001
```

Missing external tools are not success. They are reported as missing
environment.

## native-fast Critical Paths

native-fast is the speed route for the current native execution subset:
scalars, arrays, pointers, structs, calls, compact operators and memory
fixtures.

Evidence:

- `ncc run <file.n> --native-fast --show-return`;
- `route=native-fast-path`;
- `crates/ncc-cli/tests/native_fast_path.rs`;
- `examples/performance/native_fast/*`.

Failure diagnostic:

```text
E_BACKEND_PARITY_NATIVE_FAST_001
```

## machine Low Authority

`[@machine]` remains the authority-low route. It validates and encodes machine
instructions and rejects runtime/high-level imports in the machine layer.

Evidence:

- `ncc machine check ...`;
- `ncc machine emit-bytes ... --json`;
- machine diagnostics and encoded-byte reports.

Failure diagnostic:

```text
E_BACKEND_PARITY_MACHINE_AUTHORITY_001
```

## Parity Tests

Parity means agreement across declared overlaps, not total universe parity with
LLVM/GCC/Rust. The current matrix proves:

- MCT reports byte contracts;
- LLVM reports real IR/executable route;
- native-fast reports direct native execution route;
- machine reports low-authority validation/encoding route;
- missing tools are visible diagnostics, not fake success.

Failure diagnostic:

```text
E_BACKEND_PARITY_DIVERGENCE_001
```

## Truth Boundary

Closed:

- MCT byte contract;
- runtime-backed imports expose real runtime symbols and are registered as
  `closed_real_backing_bytes`;
- `ncc run archivo.n` uses those runtime APIs without `--real`;
- each runtime-backed import and runtime ABI symbol declares route rows for
  `ncc_run`, MCT, native-fast, LLVM and the machine/basal bridge boundary;
- LLVM executable route for the declared subset;
- native-fast critical path route;
- machine low-authority route;
- parity manifest/tests for declared overlap.

## Runtime Symbol Parity

Priority 10 adds symbol-level runtime parity on top of this backend matrix.
The authoritative evidence is:

- `ncc runtime abi --json`, which emits `backend_routes` for every real
  `nC.runtime.*` symbol;
- `ncc imports backend --json`, which emits the same route set for every
  compact import facade;
- `docs/spec/backend/runtime_backend_parity_priority10_contract.md`.

Catalog-only imports remain visible but carry `E_USE_BACKING_DEFERRED_001`
instead of claiming native-fast, LLVM or real runtime execution.

Not claimed:

- full LLVM universe parity;
- full GCC universe parity;
- full Rust/rustc universe parity;
- arbitrary future language construct parity without new fixtures.

## Validation

```bash
cargo test -p ncc-cli --test backend_point10 -- --nocapture
cargo test -p ncc-cli --test backend_runtime_parity_priority10 -- --nocapture
cargo test -p ncc-cli --test native_fast_path -- --nocapture
cargo test -p ncc-cli --test compact_surface -- --nocapture
cargo run --bin ncc -- mct parity --json
cargo run --bin ncc -- compiler diagnostics --json
```
