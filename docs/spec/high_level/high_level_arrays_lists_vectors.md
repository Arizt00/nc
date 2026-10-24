# High Level Arrays, Lists, And Vectors

Status: closed.

Closed forms:

- `array<T,N>` for fixed arrays;
- `T[]` for vector surface;
- `vector<T>` and current vector-family aliases;
- `list<T>` with explicit list/runtime imports;
- homogeneous collection literals.

Rules:

- collection literals must have compatible element types;
- `array<T,N>` literal length is checked when `N` is explicit;
- list and mem-list surfaces require import authority;
- list runtime families require heap/list imports under the current runtime contract.

`vector<S>` has a specialized high-level meaning when `S` is a type-contract
`struct`: it is a dynamic sequence of material values, not a SIMD register lane.
The official material form is the same owned dynamic descriptor as
`dinArray<S>`:

```text
{ data: ptr, len: i64, capacity: i64 }
```

The descriptor points to a runtime backing buffer that stores contiguous `S`
elements using the struct layout defined by the type contract. On the hosted
64-bit backend family the descriptor is 24 bytes with 8-byte alignment. Copying
or passing the descriptor is shallow under the current memory model; it does not
deep-clone the backing buffer or recursively clone pointer-containing fields.
Future destructor/finalizer policy may define release points, but the current
semantic contract is descriptor plus backing buffer.

Scalar storage vectors such as `vector<dword>` remain the SIMD/storage-vector
lane and are not rewritten to `dinArray`.

For `vector<S>` where `S` is a struct, both `items[i].field` and
`items.get(i).field` are supported read forms, and `items[i].field >> T : value`
is the supported indexed field-write form. They name the field of the material
element at `i`; the backend computes `i * sizeof(S)` before selecting the struct
field.

Examples:

```n
@fixed >> array<int, 3> : [1, 2, 3];
@values >> int[] : [4, 5, 6];
@items >> list<int> : [10, 20, 30];
@packets[@i].tag >> int : 42;
@tag >> int : @packets[@i].tag;
@tag2 >> int : @packets.get(@i).tag;
```

Invalid examples:

```n
@items >> array<int, 2> : [1, 2, 3];
@items >> list<int> : [1, "bad"];
```
