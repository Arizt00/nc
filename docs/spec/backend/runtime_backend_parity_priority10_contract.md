# Runtime Backend Parity Priority 10 Contract

Status: closed for the currently declared runtime surface.

Priority 10 closes the gap between calls that already work through `ncc run`
and the backend contracts that must also know how to materialize those calls.
Every real runtime symbol now exposes a route row for:

- `ncc_run`;
- `mct`;
- `native_fast`;
- `llvm`;
- `machine`;
- `basal`.

## Contract

`ncc run archivo.n` is the real executor route. Runtime APIs in the ABI table
do not require `--real`.

MCT is the byte contract route. Import activation rows expose deterministic
bytes and bind the runtime symbols they are allowed to call.

native-fast lowers runtime calls through an nC runtime-value thunk. The thunk
uses the ABI row declared by `ncc runtime abi --json`.

LLVM lowers runtime calls as hosted external declarations/calls using
`nC.runtime-value-v1` layouts. This is not inferred as a raw C ABI.

`[@machine]` and basal do not import high-level runtime directly. When a
runtime side effect applies to those layers, it must cross through an explicit
typed bridge/trap row. Direct imports remain forbidden.

## Negative Boundary

Catalog-only imports are still visible because the language must diagnose them
cleanly, but they do not claim native-fast, LLVM or real runtime support. Their
route rows carry `E_USE_BACKING_DEFERRED_001`.

## Evidence

```bash
ncc runtime abi --json
ncc imports backend --json
ncc mct imports --json
ncc run examples/runtime/backendRuntimeParityPriority10.n
cargo test -p ncc-cli --test backend_runtime_parity_priority10 -- --nocapture
```
