# Compiler Type Contract Sections

The compiler parses `type:` into `Section::TypeContracts(Vec<TypeContractDecl>)`.

Type headers:
- `Name`
- `Name<T...>`
- `Name struct`
- `Name union`
- `Name enum`
- `Name tagged union`

`compiler type-contracts --json` now reports:
- `kind` (`struct|enum|tagged_union`)
- `generic_params`
- `variants` (name + typed payload)
- `clauses`

Supported internal clauses include:
- `public/private/internal/strictly`
- `layout`, `bounds`, `invariant`
- `constructure`, `destructure`
- `copy`, `clone`, `move`
- `ownership`, `nullability`, `drop`, `ffi`
- `methods`, `associated`, `validate`, `default`, `compare`, `hash`, `debug`, `stable`

CLI surfaces:
- `ncc compiler types <file> --json`
- `ncc compiler type-contracts <file> --json`

`compiler doctor` reports explicit type-contract capability flags:
- `type_contracts`
- `type_sections`
- `type_layouts`
- `type_invariants`
- `type_copy_clone_move`

For LLVM/MCT struct type contracts, `type_layouts` means the compiler reports
the material layout for each struct:
- struct `size_bytes`;
- struct `align_bytes`;
- per-field index, offset, size and alignment;
- nested by-value struct fields, including structs declared later in the same
  container.

Natural layout aligns every field cursor, advances by field size and rounds the
aggregate to maximum alignment. `layout:` can replace that default with
`repr(C)`, `repr(native)`, `packed`, `align(N)`, `size(N)`, explicit endian,
bitfield storage and a final flexible array member. Real unions overlap all
members at offset zero. See `../language/explicit_binary_layout.md`.
