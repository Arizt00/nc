# Backend LLVM Aggregates F1

Status: closed for direct `type:` struct layout and direct/nested field access
in the LLVM/MCT adapter.

F1 adds aggregate layout to the existing LLVM lane without changing nC language
semantics. The adapter reads lowered `TypeContractIR` structs from regular
containers, emits named LLVM struct types, reports field layout metadata, and
lowers direct field reads and writes, including nested paths such as
`@packet.header.mark`.

## Closed Behavior

- `type:` structs in regular containers are collected into LLVM layouts.
- Each layout has a named LLVM type: `%struct.<container>_<type>`.
- Field order follows source order inside `public/private/internal/strictly`
  field clauses.
- Struct layout uses natural alignment:
  - each field offset is `align_up(cursor, field_align)`;
  - the cursor advances by `field_size`;
  - struct alignment is the maximum field alignment;
  - struct size is `align_up(final_cursor, struct_align)`.
- Nested by-value struct fields use the nested struct's computed size and
  alignment, including type contracts declared later in the same container.
- Each field records:
  - source name;
  - nC type;
  - LLVM type;
  - field index;
  - natural offset;
  - size;
  - alignment.
- Direct `@value.field` and nested `@value.a.b` expressions lower to LLVM field
  loads.
- Direct `@value.field >> T : expr` and nested `@value.a.b >> T : expr`
  projections lower to LLVM field stores.
- LLVM IR uses one `getelementptr inbounds %struct.X, ptr %value.addr, i64 0,
  i32 <field-index>` per path segment. The next segment is resolved using the
  previous field's struct layout.
- Object smoke through `clang` is covered by `ncc mct llvm-smoke`.
- Hosted executable smoke is covered by `ncc mct llvm-exe-smoke`.

## Example

```nc
mctLlvmStructLayout {
    Packet :: type {
        public:
            tag >> int;
            count >> int;
            total >> qword;
    }

    stc:
        packet >> Packet;
        gotTag >> int;
        gotCount >> int;
        ok >> bool;

    boot :: in() -> bool {
        packet.tag = 7;
        packet.count = 35;
        gotTag = packet.tag;
        gotCount = packet.count;
        ok = gotTag == 7 && gotCount == 35;
        return ok;
    }

    ref main ->> boot;

    fa:
        return true;
}
```

The emitted LLVM includes:

```llvm
%struct.mctLlvmStructLayout_Packet = type { i32, i32, i64 }
%packet.addr = alloca %struct.mctLlvmStructLayout_Packet, align 8
%field = getelementptr inbounds %struct.mctLlvmStructLayout_Packet, ptr %packet.addr, i64 0, i32 0
store i32 7, ptr %field, align 4
```

For `Packet { tag: int; count: int; total: qword; }`, the reported layout is:

| Field | Offset | Size | Align |
| --- | ---: | ---: | ---: |
| `tag` | 0 | 4 | 4 |
| `count` | 4 | 4 | 4 |
| `total` | 8 | 8 | 8 |

The struct size is 16 bytes and the struct alignment is 8 bytes. A nested
forward field follows the same rule; for example `Envelope { kind: byte;
footer: Footer; tail: int; }` with `Footer { code: qword; flag: byte; }`
places `footer` at offset 8, `tail` at offset 24, and gives `Envelope` size 32
and alignment 8.

## Boundaries

F1 does not claim full C/C++/Rust aggregate ABI parity. Still pending:

- aggregate pass-by-value ABI classification;
- aggregate return ABI classification;
- explicit packed layout;
- explicit `repr(C)`/target ABI layout controls;
- by-value cyclic struct diagnostics and final policy;
- complete debug metadata for aggregate fields;
- dynamic aggregate construction and destructuring.

Those belong to later LLVM/MCT phases.
