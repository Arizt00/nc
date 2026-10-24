# Backend LLVM/MCT: dynamic aggregate arrays

Status: closed for `vector<S>` where `S` is a type-contract `struct` on the
LLVM/MCT local executable lane.

This phase closes vector-of-structs as dynamic aggregate arrays. The source
surface remains:

```n
@packets >> vector<Packet> : [];
@packets.push(@packet);
@packets[@i].tag >> int : 42;
@tagA >> int : @packets[@i].tag;
@tagB >> int : @packets.get(@i).tag;
@packets[@i].header.mark >> int : 42;
@packets.get(@i).header.mark >> int : 43;
@markA >> int : @packets[@i].header.mark;
@markB >> int : @packets.get(@i).header.mark;
```

Semantic contract:

- `vector<S>` is a dynamic material sequence when `S` is a type-contract
  `struct`.
- It is not a SIMD register vector and must not lower to `<N x %struct...>`.
- The official descriptor is the same owned `dinArray<S>` shape:
  `{ data: ptr, len: i64, capacity: i64 }`.
- The descriptor is 24 bytes and 8-byte aligned on hosted 64-bit targets.
- The backing buffer stores contiguous `S` elements using the computed struct
  layout: size, alignment and field offsets come from the type contract.

Lowering contract:

```text
element_addr = data + index * sizeof(S)
field_addr   = gep S, element_addr, 0, field_index [, nested field geps...]
read_value   = load field_type, field_addr, align field_align
store field_type write_value, field_addr, align field_align
```

Supported field-access forms lower through this contract:

- `packets[i].tag`
- `packets.get(i).tag`
- `packets[i].tag >> T : value`
- `packets[i].header.mark`
- `packets.get(i).header.mark`
- `packets[i].header.mark >> T : value`
- `packets.get(i).header.mark >> T : value`, normalized by the parser to the
  same `ArrayFieldWrite` contract as indexed lvalue syntax

The parser represents no-argument postfix properties as method-call syntax in
the AST. Sema gives that shape field semantics only when the receiver type is a
record/struct and the property name is a declared field. IR then lowers indexed
or `get` receivers to `ArrayFieldRead`, preserving the target array symbol,
index expression and full field path. Indexed field lvalue projection lowers
through `ArrayFieldWrite`, preserving the same target, index and field path
metadata and emitting a typed LLVM store.

Closed evidence:

- Parser tests cover `packets[i].tag`, `packets.get(i).tag`,
  `packets[i].tag >> T : value` and nested indexed lvalues such as
  `packets[i].header.mark >> T : value`.
- Sema tests resolve read and indexed write forms against `vector<Packet>` and
  reject the old "unknown method on list" path.
- IR tests assert read forms lower to `ArrayFieldRead` and the indexed lvalue
  projection lowers to `ArrayFieldWrite`, preserving full field paths.
- LLVM tests assert the real `%struct.<container>_Packet` layout, the
  `dinArray` descriptor, `sizeof(Packet)` stride multiplication, direct and
  nested field GEPs, typed field load and typed field store.
- CLI tests emit LLVM for `examples/backend/mct_llvm_vector_struct_access.n`.
- When `clang` is available, `mct llvm-exe-smoke` links and executes the same
  fixture and expects process status 0.

Primary fixture:

- `examples/backend/mct_llvm_vector_struct_access.n`

Expected smoke:

```sh
ncc check examples/backend/mct_llvm_vector_struct_access.n
ncc mct emit-llvm examples/backend/mct_llvm_vector_struct_access.n --out target/tmp/mct-llvm/vector-struct-access.ll --cc nC.sysv64 --json
ncc mct llvm-exe-smoke examples/backend/mct_llvm_vector_struct_access.n --out-dir target/tmp/mct-llvm/vector-struct-access --json --cc nC.sysv64
```

Boundaries:

- This phase covers read access to direct/nested fields of vector elements and
  indexed field writes through `packets[i].field >> T : value` and
  `packets[i].a.b >> T : value`.
- Whole-element `push`, `get/readAt` and `set/writeAt` are covered by the F7
  dinArray growth contract.
- Descriptor ownership, destructor insertion and deep clone policy remain
  outside this phase.
