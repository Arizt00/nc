# MCT GCC Dynamic Aggregate Bridge

Status: integrated for `vector<Struct>` descriptor lowering.

This phase closes the first dynamic aggregate surface in the GCC C99 bridge.
`vector<Struct>` lowers as a `dinArray` descriptor:

- `data`: pointer to the backing element buffer;
- `len`: current element count;
- `cap`: current allocated capacity.

The backing buffer is allocated with `malloc` for literal projection and grown
with `realloc` for `push`. Element addressing is emitted as C indexing over the
typed backing pointer, which is equivalent to:

```text
addr = descriptor.data + index * sizeof(Struct)
```

The emitted C uses `sizeof(*descriptor.data)` for allocation and growth so the
stride is tied to the real element type.

## Closed Surface

- struct layout metadata: size, alignment, field offsets;
- C struct declarations for aggregate elements;
- descriptor declarations for `vector<Struct>`;
- empty literal projection, for example `@packets >> vector<Packet> : []`;
- `@packets.push(@packet)`;
- `@packets[@i].field >> T : value`;
- `@x >> T : @packets[@i].field`;
- `@x >> T : @packets.get(@i).field`;
- GCC smoke through `ncc mct gcc-smoke`.

## Boundary

This is not a full dynamic runtime claim. Pointer/region intrinsics, aggregate
ABI passing/returning and the full dynamic method family remain separate gates.

## Evidence

```sh
cargo test -p ncc-ir mct_gcc_bridge --all-targets -- --nocapture
cargo test -p ncc-cli mct_cli_gcc_smoke_vector_struct_access_when_gcc_is_available --lib -- --nocapture
ncc mct gcc-smoke examples/backend/mct_gcc_vector_struct_access.n --out-dir target/tmp/mct-gcc-vector --json --cc nC.sysv64 --allow-missing-tools
```
