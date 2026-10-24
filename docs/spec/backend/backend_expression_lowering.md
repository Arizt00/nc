# ncc Backend Expression Lowering

Status: closed for the scalar expression surface covered by this pass.

This document records the backend contract for lowering scalar expressions from nC AST into
`ScalarExprIR` and then into concrete backend output.

## Lowered Expression Surface

The scalar lowering path now accepts:

- numeric literals;
- boolean literals;
- byte-sized character literals (`'A'`, `'\n'`);
- string literals;
- local/context symbol references;
- direct and nested struct context references;
- array and vector literals where the target storage supports them;
- unary expressions;
- binary expressions;
- deterministic compact post-increment over a simple mutable integer or
  `ptr<T>` symbol;
- compact compound update statements (`i--`, `i += n`, `i -= n`, `x *= n`)
  normalized into typed projection plus binary expression before backend
  lowering;
- address-of syntax (`&x`) as the compact form of `addrOf(x)`;
- pointer and region intrinsics;
- runtime render intrinsics;
- array and dynamic-array operations;
- calls and qualified calls;
- strict-check expressions as typed value wrappers;
- if/else expressions;
- dev trap expressions;
- field post-increment (`obj.field++`, `obj.a.b++`) over integer-like struct
  fields;
- pointer post-increment read (`*p++`) with C precedence, meaning `*(p++)`.

`Closure` is not treated as an integer scalar expression. When a closure literal is used as a call
argument, it lowers as a closure value descriptor and is passed by descriptor pointer.

## LLVM / MCT Lowering

The LLVM adapter lowers:

- `StringLiteral` to a function-namespaced, deduplicated
  `private unnamed_addr constant` with a trailing NUL and a pointer obtained
  via `getelementptr`. The storage therefore remains valid when a local
  function returns `str`; returning a pointer to a callee stack slot is never
  permitted.
- `CharLiteral` to a scalar byte-compatible codepoint. nC keeps `char` as a one-byte value for
  backend lowering; Unicode text with wider codepoints belongs to `str`.
- `StrictCheckExpr` to the wrapped value while preserving typed lowering through the surrounding
  projection/return context.
- `IfElse` to real LLVM control-flow blocks and a `phi` result, avoiding eager evaluation of both
  branches.
- `DevTrap` to `call void @llvm.trap()`.
- `>>>` to `lshr`, while signed `>>` remains `ashr`.
- `PostIncrement` to a real `load`, add and `store` on the target slot. Integer
  targets add `1`; `ptr<T>` targets use `getelementptr i8` by `sizeof(T)`.
  nC defines the expression value as the incremented value, so `i = i++;` is
  deterministic instead of inheriting C/C++ sequencing traps.
- `i--`, `i += n`, `i -= n` and `x *= n` do not introduce a separate
  expression ABI. They are normalized as typed projections whose value is a
  binary expression over the same target, so MCT/LLVM sees ordinary
  `load`/arithmetic/`store` through the existing assignment path.
- `AddressOf` to the concrete stack/storage address for local scalar and fixed
  aggregate slots.
- `FieldRead`, `FieldWrite` and `FieldPostIncrement` to field-address
  resolution plus typed `load`/`store`. Nested paths walk each struct layout in
  order and emit chained `getelementptr` instructions; vector/array element
  fields use the element address plus the same field-path walk.
- `PointerPostIncrementRead` to `load old pointer`, `load pointee`, `getelementptr i8` by the
  element byte size and `store` the advanced pointer back to the pointer slot.
- `RuntimeRender` to real text bytes for scalar values. `renderInt`,
  `renderUInt`, `renderFloat`, `renderChar`, `renderDebug` and `renderStable`
  use `snprintf` into stack-owned buffers; `renderBinary` / `renderBin`
  materialize `-`, `0` and `1` bytes with explicit bit scanning and byte
  stores; `renderBool` selects stable literal globals `"true"` / `"false"`;
  `renderType` materializes the lowered static type name when available.
- Binary `+` whose source operands carry the typed `str` contract lowers to
  `strlen` plus `malloc(len_left + len_right + 1)`, two `llvm.memcpy` calls and
  a final NUL byte. The decision uses nC symbol and function return types, not
  LLVM's opaque `ptr`, so integer addition and pointer arithmetic retain their
  existing lowering.

The module declares `@llvm.trap` only when a lowered expression uses it.

## Native-Fast Lowering

The native scalar builder lowers `IfElse` expressions into labels, conditional jump, branch bodies,
and a join label.

Native-fast materializes the previously missing aggregate-value edge cases:

- string literal call arguments become function-local descriptors in `.rodata.<function>` with the
  layout `{ data_ptr, byte_len }`; text code loads the descriptor address through typed `Abs64`
  relocations, and the descriptor's `data_ptr` relocates to a local NUL-terminated byte payload;
- closure literal call arguments become function-local closure descriptors in `.rodata.<function>`
  with the layout `{ code_ptr, env_ptr, param_count, capture_count }`; `code_ptr` relocates to a
  local native trap stub so the value is material rather than a null placeholder;
- `DevTrap` emits a native trap opcode (`ud2` on x86_64) and then materializes the expression result
  as zero for the surrounding typed expression path.
- `stc` scalar globals materialize into native object sections: integer and boolean initializers use
  `.data.<container>.stc`, string initializers use `.rodata.<container>.stc` descriptors with local
  byte payloads, and x86_64 emits a real `ret` init wrapper symbol.
- Fixed `stc` arrays materialize as contiguous zero-filled object storage. Native-fast computes
  global array addresses by finishing the index expression first and loading the array base
  afterwards, so reloc scratch registers used by global loads cannot overwrite the live array base.
- `local` initializers are folded into the native state-init path when they are container-level
  initial state rather than per-call scratch state.
- `to main` bodies lower to a physical native entry function before object emission, matching
  `ref main ->> ...;` for the native run path.
- Text concatenation that returns `str` values is materialized in the scalar
  native-fast lane as backend ops: the emitter counts the left/right NUL-terminated
  byte strings, calls `malloc(len_left + len_right + 1)`, copies both byte ranges
  with native load/store loops, writes the final NUL byte and returns the new
  pointer. The hosted launcher must not provide `__ncc_native_concat2`.
- `renderInt`, `renderUInt` and `renderChar` used as `str` values lower to a
  native heap buffer plus `snprintf`; `renderBinary` / `renderBin` lower to a
  native heap buffer, bit-scan loop, byte stores and final NUL terminator;
  `renderBool` selects function-local `.rodata` literals. Assigning the
  resulting `str` to `stc` stores the pointer with native qword width, so it
  does not leave a `render*` symbol for the linker.
- `PostIncrement` emits a real scalar load/add/store path and returns the
  incremented value in the requested destination register. For `ptr<T>`, the
  immediate added to the pointer value is `sizeof(T)`, not a hardcoded byte.
- `AddressOf` emits a material stack address with `lea` for locals and fixed
  aggregates.
- `PointerPostIncrementRead` emits a material pointer load, memory read, byte-stride pointer
  advance and pointer-slot store; the returned value is the old pointee.
- `ptr<T>` storage uses the native pointer word width (`qword` on SysV64). Only the pointee
  access uses `sizeof(T)`. The backend must never derive the storage width of the pointer slot
  from the element type.
- Direct scalar struct field paths use the same type-contract layout metadata
  as LLVM/MCT. Native-fast reserves a stack block with the struct's computed
  size/alignment, computes nested field addresses as
  `base_offset + field_offset_0 + ... + field_offset_n`, and emits typed
  `LoadMem`/`StoreMem` by the final field's real width.
- `FieldRead`, `FieldWrite` and `FieldPostIncrement` are native-fast material
  for direct scalar field paths. `FieldPostIncrement` returns the old field
  value and stores the incremented value back to the field address.
- Fixed `array<Struct,N>` element fields are native-fast material for
  `ArrayFieldRead` / `ArrayFieldWrite`. The builder allocates a contiguous stack
  block using `sizeof(Struct) * N`, computes `base + index * sizeof(Struct)`,
  adds the nested field-path offset, and emits typed `LoadMem`/`StoreMem` by the
  final field width.
- Local `vector<Struct>` / `dinArray<Struct>` values initialized by literal are
  native-fast material for `push`, `len`, `capacity`, indexed field read/write
  and `.get(index).field`. The native descriptor stores a real contiguous backing
  block plus a runtime `len` word and fixed local capacity; `push` copies the
  struct bytes using the type-contract ABI layout and advances `len`.
- Local fixed `list<T>` / dynamic-list literals that are represented as stack
  array slots are native-fast material for `.get`, `.set`, `.len`,
  `listCapacity`, `listClone`, `listIndexOf` and `listContains`. In this stack
  form, `listCapacity` is the fixed literal length; heap growth and descriptor
  ownership remain outside this closure.
- `print` / `println` lower to native hosted `printf` calls for the scalar
  native-fast path. Text concatenation is emitted as ordered print pieces for
  string literals, `RuntimeRender` values (`renderInt`, `renderUInt`,
  `renderBool`, `renderChar`, `renderDebug`, `renderStable`) and color wrappers
  such as `colorBold`; numeric `+` inside a non-text print expression remains a
  numeric addition.
- Heap growth for dynamic vector/list descriptors remains on the LLVM/MCT
  executable lane. Native-fast currently claims the local fixed-capacity
  descriptor shape above, not cross-function descriptor ownership.
- Cross-function `stc` list persistence and fully dynamic list descriptors still
  route through LLVM/MCT; native-fast only claims the stack-local list shape
  above.

## Analysis Integration

The new expression forms are visible to:

- source needle reconstruction;
- vector symbol collection;
- auto-vectorizer indirect-read analysis;
- scalar call and memory-op detection;
- scalar liveness dependency collection;
- register overwrite avoidance;
- constant expression evaluation where safe.

String literals, `RuntimeRender`, `PostIncrement`, `FieldPostIncrement` and
`PointerPostIncrementRead` are treated as memory-bearing for optimization
gating. `DevTrap` disables immediate return folding. `IfElse` propagates
dependency and call/memory analysis through condition and both branches.

## Validation

Baseical tests:

```text
cargo test -p ncc-ir mct_llvm_lowers_scalar_expression_surface -- --nocapture
cargo test -p ncc-cli compact_post_increment_example_checks_and_runs -- --nocapture
cargo test -p ncc-cli compact_c_like_post_increment_forms_check_run_and_emit_mct -- --nocapture
cargo test -p ncc-cli ncc_run_native_fast_executes_post_increment_value -- --nocapture
cargo test -p ncc-cli ncc_run_native_fast_executes_pointer_post_increment_read -- --nocapture
cargo test -p ncc-cli ncc_run_native_fast_executes_compact_pointer_c_forms -- --nocapture
cargo test -p ncc-ir mct_llvm_lowers_pointer_post_increment_by_element_size -- --nocapture
cargo test -p ncc-ir mct_llvm_lowers_typed_string_plus_to_concat_without_pointer_addition -- --nocapture
cargo test -p ncc-cli ncc_run_native_fast_executes_struct_field_read_write -- --nocapture
cargo test -p ncc-cli ncc_run_native_fast_executes_struct_field_post_increment -- --nocapture
cargo test -p ncc-cli ncc_emit_object_materializes_struct_field_backend_ops -- --nocapture
cargo test -p ncc-ir mct_llvm_lowers_nested_struct_field_access -- --nocapture
cargo test -p ncc-ir mct_llvm_lowers_vector_struct_nested_field_access_by_index_and_get -- --nocapture
cargo test -p ncc-cli --test native_fast_path ncc_run_native_fast_executes_vector_struct_push_and_field_access -- --nocapture
cargo test -p ncc-cli ncc_run_native_fast_executes_nested_struct_field_read_write -- --nocapture
cargo test -p ncc-cli ncc_emit_object_materializes_nested_struct_field_backend_ops -- --nocapture
cargo test -p ncc-sema fixed_array_struct_nested_field_access_passes_sema -- --nocapture
cargo test -p ncc-cli ncc_run_native_fast_executes_struct_array_field_read_write -- --nocapture
cargo test -p ncc-cli ncc_emit_object_materializes_struct_array_field_backend_ops -- --nocapture
cargo test -p ncc-ir mct_llvm_lowers_runtime_render_to_host_bytes -- --nocapture
cargo test -p ncc-ir emit_backend_materializes_string_literal_call_arg_with_abs64_reloc -- --nocapture
cargo test -p ncc-ir emit_backend_materializes_closure_call_arg_as_rodata_descriptor -- --nocapture
cargo test -p ncc-ir emit_backend_lowers_dev_trap_to_native_ud2 -- --nocapture
cargo test -p ncc-cli --test native_fast_path ncc_run_native_fast_executes_runtime_list_print_helpers -- --nocapture
cargo test -p ncc-cli --test native_fast_path ncc_run_native_fast_executes_to_main_high_level_surface -- --nocapture
cargo test -p ncc-cli --test native_fast_path ncc_run_native_fast_executes_string_concat_and_state_init_surface -- --nocapture
cargo test -p ncc-cli --test native_fast_path ncc_run_native_fast_executes_global_array_write_with_call_surface -- --nocapture
```

The wider LLVM adapter smoke is:

```text
cargo test -p ncc-ir mct_llvm_ -- --nocapture
```
