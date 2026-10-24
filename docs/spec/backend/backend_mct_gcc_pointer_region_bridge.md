# MCT GCC Pointer Region Bridge

Status: integrated for declared pointer/region intrinsic lowering.

This phase closes the GCC C99 bridge surface for pointer and region operations
already present in checked `ProgramIR`.

## Lowering

- `ptrRead<T>(p)` emits a typed load through `*((T *)p)`.
- `ptrWrite<T>(p, value)` emits a typed store through `*((T *)p) = value`.
- `ptrOffset<T>(p, i)` emits byte-addressed pointer arithmetic:

```text
(uint8_t *)p + i * sizeof(T)
```

- `ptrDiff<T>(left, right)` emits signed address subtraction divided by
  `sizeof(T)`.
- `regionFromPtr<T>(p, len)` initializes a descriptor with `data = p` and
  `len = len`.
- `regionPtrAt<T>(region, i)` emits `region.data + i * sizeof(T)`.

The bridge uses `intptr_t` for pointer difference and `uint8_t *` for byte
addressing. It does not claim provenance, alias-analysis, or bounds-check parity
beyond the declared C99 bridge surface.

## Closed Surface

- parser/sema acceptance of the pointer/region fixture;
- `array<int>` backing descriptor;
- `memRegion<int>` descriptor projection from `regionFromPtr<int>`;
- `ptrRead<int>` and `ptrWrite<int>`;
- `ptrOffset<int>` and `ptrDiff<int>`;
- `regionPtrAt<int>`;
- GCC compile/link/run smoke.

## Evidence

```sh
cargo test -p ncc-ir mct_gcc_bridge --all-targets -- --nocapture
cargo test -p ncc-cli mct_cli_gcc_smoke_pointer_region_access_when_gcc_is_available --lib -- --nocapture
ncc mct gcc-smoke examples/backend/mct_gcc_pointer_region_access.n --out-dir target/tmp/mct-gcc-pointer --json --cc nC.sysv64 --allow-missing-tools
```
