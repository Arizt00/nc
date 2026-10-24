# Runtime Lists (v2.1.0)

Status: implemented as runtime alias surface backed by `runtime.queue` contracts.

Baseical imports supported:
- `use alloc.collections { list };`
- `use nC.runtime.list.*;`
- `use nC.runtime.list.core;`
- `use nC.runtime.list.index;`
- `use nC.runtime.list.memory;`
- `use nC.list;`
- `use nC.list.*;`
- `use nC.list.core;`
- `use nC.list.index;`
- `use nC.list.memory;`
- symbol imports (individual): `use nC.runtime.list.listCreate;` etc.
- symbol imports (individual): `use nC.list.listCreate;` etc.

Exported alias API (surface):
- `listCreate`, `listCreateWithCapacity`, `listFromArray`
- `listLen`, `listCapacity`, `listIsEmpty`
- `listAdd`, `listPush`, `listPop`
- `listGet`, `listSet`, `listInsert`, `listRemoveAt`, `listClear`
- `listContains`, `listIndexOf`, `listSlice`, `listAsArray`, `listAsRegion`
- `listClone`, `listCopy`, `listMove`, `listSwap`

Note:
- list surface is runtime dynamic; binary stable layout is not guaranteed by default.
- `listAsRegion` materializes a typed `memRegion<T>` snapshot only when element type is resolvable
  (declared list type or homogeneous runtime values). If not resolvable, runtime returns
  `E_LIST_AS_REGION_001`.

Array/List/DinArray distinction:
- `array<T>`: indexable collection; can be fixed or runtime-sized depending declaration.
- `array<T, N>`: fixed shape with stronger static bounds and stable layout eligibility.
- `list<T>`: runtime dynamic container (alias family over `runtime.queue`), non-stable binary layout by default.
- `dinArray.binary<T>`: explicit stable binary layout path (when `T` is stable).
- `dinArray.unbinary<T>`: runtime-oriented dynamic layout path.
