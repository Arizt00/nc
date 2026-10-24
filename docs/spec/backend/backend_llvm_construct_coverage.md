# Backend LLVM/MCT Construct Coverage

Status: closed for the current nC construct catalog.

The LLVM adapter no longer leaves current nC language constructs as loose
"not represented" caveats. Every MCT LLVM module carries a structured construct
coverage matrix in `MctLlvmModuleIR.construct_coverage`, and
`ncc mct emit-llvm --json` exports the same matrix as `construct_coverage`.

## Purpose

The matrix records what the current LLVM adapter can represent and what is
forbidden by mode. This is a truth surface, not a claim that nC embeds the whole
LLVM/Clang/rustc frontend universe. Current nC constructs that enter ProgramIR
now have either a represented adapter row or an explicit modal-forbidden row.

## JSON Shape

`construct_coverage` contains:

- `schema = nc.mct.llvm.construct_coverage.v1`.
- `status = llvm_construct_coverage_closed` when no present construct is marked
  `not_represented_yet`.
- `status = llvm_construct_coverage_partial` when the module contains at least
  one present construct that the adapter does not represent yet.
- `represented_count`.
- `not_represented_count`.
- `forbidden_count`.
- `present_not_represented_count`.
- `rows`.
- `diagnostics`.

Each row contains:

- `construct`.
- `family`.
- `status`: `represented`, `not_represented_yet` or `forbidden_by_mode`.
- `present_in_module`.
- `lowering`.
- `diagnostic`.
- `reason`.
- `evidence`.

## Represented Families

The current represented surface covers regular/root/security containers,
function signatures, scalar returns, projections, scalar expressions, print/dev
statements, raw statement/return boundaries, structured control flow, labels,
main refs, function links, scalar literals, symbols, pointer
reads/writes/offsets/diffs, region views, struct field access, enum tag layouts,
tagged-union tag/payload layouts, array projection storage, array literal value
storage, array access, dinArray descriptor operations, calls, unary/binary ops,
conditional expressions and dev traps.

Enum contracts lower to deterministic `i32` tag layouts. Tagged unions lower to
an `i32` tag plus an aligned byte payload sized from the largest variant. Raw
statements and raw returns either parse through scalar fragments or lower to an
explicit `llvm.trap` boundary with typed return fallback. Array literals used as
values allocate typed aggregate storage and return the aggregate or first-element
pointer according to the expected LLVM type.

## Forbidden By Mode

The LLVM adapter keeps modal authority boundaries:

- `container.primitive`: `E_MCT_LLVM_MODE_FORBIDDEN_001`.
- `container.machine`: `E_MCT_LLVM_MACHINE_FORBIDDEN_001`.

Those modes stay outside the LLVM adapter. They are not hidden as future work,
and they are not silently lowered through LLVM.

## Evidence

Use:

```sh
ncc mct emit-llvm examples/backend/mct_llvm_debug_dwarf.n \
  --out target/tmp/mct-llvm/coverage.ll \
  --cc nC.sysv64 --json
```

Expected:

- `construct_coverage.schema = nc.mct.llvm.construct_coverage.v1`.
- `construct_coverage.status = llvm_construct_coverage_closed` for the current
  catalog.
- `construct_coverage.rows[]` includes represented and forbidden rows.
- `present_not_represented_count = 0`.

## Boundary

This closes current nC construct representation in the LLVM adapter. It does not
claim global LLVM optimizer parity, Clang frontend parity, rustc frontend parity,
or future constructs that have not yet entered the nC ProgramIR catalog.
