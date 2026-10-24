# MCT GCC Bridge

Status: integrated for the declared scalar, dynamic aggregate, pointer/region
and aggregate ABI C99 bridge gate.

The GCC provider now has a real MCT bridge adapter. It lowers checked nC
`ProgramIR` into C99 source that can be compiled by `gcc` or `cc` and smoked as
a hosted executable when the toolchain is present.

This is not a full GCC parity claim. It is the first GCC lane after declared
LLVM/MCT parity, now including the vector-of-structs dynamic aggregate gate, the
declared pointer/region intrinsic gate and aggregate parameters/returns through
the host C ABI.

## Closed Surface

- `nc.mct.gcc.bridge_module.v1` JSON module.
- `ncc mct emit-gcc-c <file.n> --out <file.c>`.
- `ncc mct gcc-smoke <file.n> --out-dir <dir> --json`.
- Scalar integer, bool and string literal values.
- Internal function calls.
- Structured C control flow for the scalar gate.
- `print`, `println`, `print.sp`, `print.nl` and `print.err` through `stdio`.
- `ref main ->> ...;` hosted executable wrapper.
- Struct layout metadata with C declarations.
- Dynamic descriptor metadata for `array`, `slice`, `span`, `memRegion`,
  `dinArray` and `vector<Struct>`.
- `vector<Struct>` as a `dinArray` descriptor with:
  - `data` backing pointer;
  - `len`;
  - `cap`.
- `vector<Struct>` empty literal projection with real backing allocation.
- `vector<Struct>.push(value)` with `realloc` growth and
  `sizeof(*descriptor.data)` stride.
- `vector<Struct>` field read/write through `descriptor.data[(size_t)index].field`.
- `packets[i].field` and `packets.get(i).field` lowering to the same descriptor
  access form.
- `ptrRead<T>` and `ptrWrite<T>` through typed C pointer casts.
- `ptrOffset<T>` through byte-addressed pointer arithmetic:
  `((uint8_t *)base) + index * sizeof(T)`.
- `ptrDiff<T>` through `intptr_t` subtraction divided by `sizeof(T)`.
- `regionFromPtr<T>` descriptor projection with `data` and `len`.
- `regionPtrAt<T>` lowering over descriptor `data`.
- Struct parameters and returns by value through C99 functions.
- Dynamic descriptor parameters and returns by value through C99 functions.
- Honest missing-tool reporting for `gcc`/`cc`.

## Not Claimed

- Full GCC plugin or GIMPLE parity.
- Full dynamic aggregate method family beyond the closed vector-of-structs
  push/get/set surface.
- Exhaustive per-target aggregate ABI classification independent of the host C
  compiler.

## CLI

```sh
ncc mct emit-gcc-c examples/backend/mct_gcc_scalar_bridge.n --out target/tmp/mct-gcc/scalar.c --json --cc nC.sysv64
ncc mct gcc-smoke examples/backend/mct_gcc_scalar_bridge.n --out-dir target/tmp/mct-gcc --json --cc nC.sysv64 --allow-missing-tools
ncc mct gcc-smoke examples/backend/mct_gcc_vector_struct_access.n --out-dir target/tmp/mct-gcc-vector --json --cc nC.sysv64 --allow-missing-tools
ncc mct gcc-smoke examples/backend/mct_gcc_pointer_region_access.n --out-dir target/tmp/mct-gcc-pointer --json --cc nC.sysv64 --allow-missing-tools
ncc mct gcc-smoke examples/backend/mct_gcc_aggregate_abi.n --out-dir target/tmp/mct-gcc-aggregate-abi --json --cc nC.sysv64 --allow-missing-tools
```

`gcc-smoke` writes C, invokes `gcc` or `cc`, runs the executable when compile
succeeds, and reports `gcc_smoke_missing_tool` rather than inventing evidence.

## Truth Boundary

- `declared_gcc_bridge_adapter = true`.
- `full_gcc_parity = false`.
- LLVM/MCT declared parity remains closed.
- The next GCC gate is multi-host certification and trend evidence.
