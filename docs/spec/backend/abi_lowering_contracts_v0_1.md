# nC v0.1 — F11 tranche B/F — ABI + lowering + emission + object + ELF64 contracts

## Scope
This tranche extends F11 in the active line without breaking the stable parser/sema/runtime path.

It is aligned with the ABI base of:
- `_archive/deprecated/context_legacy/pdfs/nC_abi_calling_data_layout_ffi_especificacion_tecn_260330_195638.pdf`
- `_archive/deprecated/context_legacy/pdfs/nC_linking_loader_abi_ffi_especificacion_tecnica_260330_194357.pdf`

## Implemented

`FunctionIR` now exposes typed ABI + backend metadata:

- `params: Vec<FunctionParamIR>`
- `return_type: String`
- `abi: FunctionAbiIR`
- `backend_plan: FunctionBackendPlanIR`
- `call_links: Vec<FunctionCallLinkIR>`

### ABI contract (typed)

`FunctionAbiTagIR`:
- `version`
- `call_class`
- `return_class`
- `unwind_policy`
- `mode_class`
- `foreign_boundary`

`FunctionAbiIR`:
- `tag`
- `frame_class`

ABI enums:
- `AbiCallClassIR`
- `AbiModeClassIR`
- `AbiUnwindPolicyIR`
- `AbiReturnClassIR`
- `AbiPassClassIR`
- `AbiFrameClassIR`

### Backend physical plan (typed)

`FunctionBackendPlanIR`:
- `target`
- `calling_convention`
- `data_layout`
- `param_locations`
- `return_location`
- `frame_layout`
- `call_sites`

Typed supporting contracts:
- `BackendTargetIR`
- `BackendCallingConventionIR`
- `BackendDataLayoutIR`
- `FunctionBackendParamLocationIR`
- `FunctionBackendReturnLocationIR`
- `FunctionBackendFrameLayoutIR`
- `FunctionBackendCallSiteIR`
- `BackendValueLocationIR`
- `BackendBridgeKindIR`
- `BackendRegisterIR`

### Material emission layer (typed)

`ncc-ir` now exposes a first material emission API on top of typed lowering:

- `emit_backend(&ProgramIR) -> BackendModuleIR`
- `render_backend_text(&BackendModuleIR) -> String`

Emission contracts:
- `BackendModuleIR`
- `BackendUnitIR`
- `BackendFunctionUnitIR`
- `BackendFuncOpIR`
- `BackendMachineUnitIR`
- `BackendMachineOpIR`
- `BackendPrimitiveUnitIR`

### Stable type ABI layout (typed)

Named `type:` contracts now expose a stable storage ABI descriptor:

- `TypeAbiLayoutIR`
- `TypeAbiFieldIR`
- `BackendTypeContractUnitIR.abi_layouts`
- `MctLlvmModuleIR.type_abi_layouts`

The descriptor records schema, status, qualified type name, kind, byte size,
alignment, source-order field offsets, field sizes, field alignments and a
deterministic fingerprint. It is emitted in backend text, object `.meta.type.*`
bytes and MCT LLVM comments. This closes the storage/data-layout side of the
type ABI; full foreign aggregate pass/return classification remains a separate
ABI gate.

The complete storage-family table is executable through
`backend_type_abi_storage_matrix()` and documented in
`backend_type_abi_layout_v1.md`. It covers scalar, pointer/reference/text,
fixed arrays, fixed byte buffers, dynamic descriptors, SIMD vectors, union
payloads, named type contracts and explicit opaque fallback rows.

CLI integration:
- `ncc emit-backend <file.n>`
- `ncc emit-object <file.n>`
- `ncc emit-elf <file.n> <out.o>`

## Active lowering rules

### ABI derivation
- `call_class` and `mode_class` derive from container context (`[@]`, `[@machine]`, `[@runtime]`, etc.).
- `unwind_policy` is `NoUnwind` for `[@machine]` and `MayUnwind` for other active contexts.
- `pass_class` per param:
  - pointer-family => `PointerWord`
  - scalar simple => `Register`
  - aggregate/generic => `ByRef`
- `return_class` per return type:
  - `void` => `Void`
  - pointer-family => `PointerWord`
  - scalar simple => `Scalar`
  - aggregate/generic => `AggregateByRef` (or `AggregateByValue` for simple non-scalar)
- `foreign_boundary=true` when function body contains non-local qualified imported call paths.
- `frame_class` is `Leaf`/`NonLeaf` by call presence in body.

### Call-link derivation
`FunctionCallLinkIR` captures:
- `surface` (`Unqualified` / `Qualified` / `Method`)
- `kind` (`Local` / `SameFile` / `Import` / `Root` / `Unknown`)
- `target`
- `arg_count`
- per-argument typed descriptors:
  - `pass_class` (`Register` / `PointerWord` / `ByRef` / `ByValue`)
  - `source` (`Symbol` / literal forms / expression fallback)
- `target_mode`
- `requires_bridge`

LLVM direct-call emission preserves the typed source scope established by
parser and sema. An unqualified call is resolved against
`<current-container>.<function>` before the compatibility simple-name index is
consulted. An explicitly qualified call is looked up by its exact path and is
never rebound to a same-named function in another container. This ordering is
required when several containers intentionally expose functions with the same
name but different signatures.

### Physical backend plan derivation
Current active target line:
- `target = X86_64 | AArch64`
- `calling_convention` profile is typed and explicit:
  - `NcSysV64` (default active line)
  - `NcWin64` (typed lowering profile in active line)
  - `NcAapcs64` (typed lowering profile in active line)
- `data_layout = { pointer_bytes: 8, word_bytes: 8, stack_align_bytes: 16 }`

Parameter location policy:
- `NcSysV64` register window order: `RDI, RSI, RDX, RCX, R8, R9`
- `NcWin64` register window order: `RCX, RDX, R8, R9`
- `NcAapcs64` register window order: `X0..X7`
- register-eligible pass classes: `Register`, `PointerWord`, `ByRef`
- `ByValue` is conservatively forced to stack slots
- stack slots use aligned word-sized offsets

Return location policy:
- `Void` => `Void`
- `Scalar` / `PointerWord` / `AggregateByValue` => `Register(RAX)`
- `AggregateByRef` =>
  - `NcSysV64`: `HiddenSRet { pointer_register: RDI, echo_register: RAX }`
  - `NcWin64`: `HiddenSRet { pointer_register: RCX, echo_register: RAX }`
  - `NcAapcs64`: `HiddenSRet { pointer_register: X8, echo_register: X0 }`

Bridge/lowering call-site policy:
- `requires_bridge=false` => `bridge=None`
- `kind=Root` + bridge => `RootBoundary`
- `kind=Import` + bridge => `ImportBoundary`
- remaining bridged calls => `CrossMode`
- bridged call-sites reserve typed `bridge_scratch_bytes`
- outgoing call stack bytes:
  - computed from typed call-arg classes (not only by raw `arg_count`)
  - `Register`/`PointerWord`/`ByRef` consume register slots while available, otherwise stack
  - `ByValue` is materialized directly as stack argument in active line
  - `NcSysV64`: typed stack args as above
  - `NcWin64`: typed stack args as above + mandatory 32-byte shadow space
  - `NcAapcs64`: typed stack args as above
- per call-site typed arg placement is preserved in `arg_locations`:
  - each argument records `{ index, pass_class, source, location }`
  - `location` is explicitly typed as `Register(<reg>)` or `Stack { byte_offset, byte_size }`

Frame layout policy:
- derives `callee_saved` from frame + bridge needs
- convention-specific bridge scratch callee-saved set in active line:
  - `NcSysV64`: `R12`, `R13`
  - `NcWin64`: `R14`, `R15`
  - `NcAapcs64`: `X19`, `X20`
- tracks `callee_saved_bytes`, `outgoing_arg_bytes`, `bridge_scratch_bytes`, `local_spill_bytes`
- computes aligned `total_bytes` for backend materialization

### Material emission policy

- function units:
  - symbol naming: `<container>.<function>`
  - frame materialization from `backend_plan` (`SaveCallee`, `ReserveStack`, `ReleaseStack`, `RestoreCallee`)
  - non-leaf frame stabilization in active line:
    - `SetFramePointer` emits typed pair by convention:
      - `NcSysV64` / `NcWin64`: `mov rbp, rsp`
      - `NcAapcs64`: `mov x29, sp`
    - incoming stack-parameter addressing is anchored to official frame-pointer base (`sysv64: +16`, `win64: +48`, `aapcs64: +16`)
  - bridge-aware call emission (`BridgeEnter`/`Call`/`BridgeLeave`)
  - call-site stack marshalling ops in active line:
    - `ReserveCallStack(bytes)` before call
    - `PrepareCallArg { ... }` per typed call argument placement
    - `ReleaseCallStack(bytes)` after call
    - `bytes` follows typed `stack_arg_bytes` from call-site plan (including Win64 shadow-space)
  - `PrepareCallArg` native materialization in active line:
    - supports `Symbol` sources from typed parameter locations (register and incoming stack forms)
    - supports `NumberLiteral` / `BoolLiteral` immediates
    - supports `StringLiteral` via function-local `.rodata.<function>` symbols + typed `Abs64`
      relocations
  - call mnemonic in object textual payload:
    - `x86_64`: `call`
    - `aarch64`: `bl`
- machine units:
  - typed `MachineInstIR` and `MachineFinalOpIR` are emitted as material backend machine ops
- primitive units:
  - bridge/capability/operation/anchor/window/signal/port/signature/entry metadata is emitted as typed backend manifest unit

### Object layer policy

- typed object projection from backend module:
  - `emit_object(&BackendModuleIR) -> BackendObjectIR`
  - `render_object_text(&BackendObjectIR) -> String`
- object contracts:
  - sections: `.text.<fn>`, `.machine.<container>`, `.meta.<primitive>`
  - payload: `BackendObjectSectionIR.bytes: Option<Vec<u8>>`
    - native bytes when encoding is available
    - explicit textual fallback when native encoding is not yet safe/available
  - symbol table: local/global typed entries
  - local machine labels resolve to qualified symbols (`<machine_symbol>.<label>`) in relocations
  - relocations:
    - `CallNear32` for function/machine call-sites
    - `JumpNear32` for machine jump/final-jump targets
  - deterministic pseudo-size accounting for section offsets and relocation anchors

### Native opcode emission in active line

- function units emit native x86_64 bytes for:
  - callee-save push/pop
  - frame-pointer setup (`mov rbp, rsp`)
  - stack reserve/release (`sub/add rsp, imm32`)
  - call-site stack marshalling reserve/release (`sub/add rsp, imm32`) around call-sites with `stack_arg_bytes > 0`
  - call-arg value moves (`PrepareCallArg`) for register/stack placements, including literal pointer moves through `Abs64` relocations
  - `call rel32` (with typed relocation)
  - `ret`
- machine units emit native x86_64 bytes for:
  - control-flow (`call`, `jmp`, `jmp.eq/ne/gt/lt/ge/le`, `ret`, `nop`, `wait`, `halt`)
  - register/memory forms (`mov`, `cmp`, `test`, `add`, `sub`, `and`, `or`, `xor`, `not.bin`, `inc`, `dec`, `push`, `pop`, `load`, `store`)
  - typed memory addressing for `load/store`:
    - `mem[base]`
    - `mem[base + index]`
    - `mem[base + index*scale]` with `scale ∈ {1,2,4,8}`
    - optional signed displacement (`+disp` / `-disp`)
  - `shl/shr/rol/ror` with arbitrary shift-count source through typed `rcx/cl` count channel
  - when `rcx` is owned by a symbolic machine register, emitter preserves/restores it via non-mapped typed scratch register when available
- if a machine unit includes non-encodable forms in active line, object emission preserves explicit textual fallback for that section instead of inventing semantics.
  - fallback remains explicit when no scratch physical register is available for safe preservation/restore.
- Mach-O fallback rendering in CLI:
  - `x86_64`: keeps Intel syntax path
  - `aarch64`: uses arm64 syntax fallback for textual function sections (`bl`, `mov x29, sp`, `str/ldr` stack save/restore)

### ELF64 layer policy

- typed object to native relocatable bytes:
  - `emit_object_elf64(&BackendObjectIR) -> Result<Vec<u8>, String>`
- current targets:
  - `ET_REL` + `EM_X86_64` ELF64 little-endian
  - `ET_REL` + `EM_AARCH64` ELF64 little-endian
- emitted tables:
  - section headers
  - `.symtab`
  - `.strtab`
  - `.shstrtab`
  - `.rela*` sections for typed call/jump relocations
- relocation mapping:
  - x86_64:
    - `CallNear32` -> `R_X86_64_PC32`
    - `JumpNear32` -> `R_X86_64_PC32`
    - `Abs64` -> `R_X86_64_64`
  - aarch64:
    - `CallNear32` -> `R_AARCH64_CALL26`
    - `JumpNear32` -> `R_AARCH64_JUMP26`
    - `Abs64` -> `R_AARCH64_ABS64`

## Baseical test evidence

`crates/ncc-ir/src/lib.rs`:
- `abi_ir_tests::lower_function_abi_tag_for_root_marks_foreign_boundary_and_param_classes`
- `abi_ir_tests::lower_function_abi_tag_for_runtime_aggregate_return_stays_local_leaf`
- `abi_ir_tests::lower_function_backend_plan_spills_after_register_window_and_keeps_byvalue_on_stack`
- `abi_ir_tests::lower_function_call_links_capture_mode_targets_and_bridge_flags`
- `backend_emit_ir_tests::emit_backend_materializes_function_bridges_and_frame_ops`
- `backend_emit_ir_tests::emit_backend_materializes_machine_and_primitive_units`
- `backend_emit_ir_tests::emit_machine_object_falls_back_to_text_payload_for_non_encodable_shift_count_register`
- `backend_emit_ir_tests::emit_machine_object_encodes_shift_rotate_with_non_rcx_source_when_rcx_is_free_scratch`
- `backend_emit_ir_tests::emit_machine_object_encodes_shift_with_rcx_destination_using_typed_scratch_register`
- `backend_emit_ir_tests::emit_machine_object_encodes_indexed_load_store_native_when_addressing_is_typed`
- `abi_ir_tests::lower_function_backend_plan_win64_maps_register_window_shadow_space_and_sret`
- `abi_ir_tests::lower_function_backend_plan_aapcs64_maps_register_window_and_return_register`
- `backend_emit_ir_tests::emit_backend_win64_materializes_convention_specific_frame_and_native_push_pop`
- `backend_emit_ir_tests::emit_object_elf64_materializes_aarch64_header_when_profile_is_aapcs64`
- `backend_emit_ir_tests::emit_backend_materializes_string_literal_call_arg_with_abs64_reloc`

`crates/ncc-cli/src/main.rs`:
- `backend_emit_cli_tests::emit_backend_materializes_function_units_for_root_fixture`
- `backend_emit_cli_tests::emit_backend_materializes_machine_and_primitive_units_for_fixture`
- `backend_emit_cli_tests::emit_object_materializes_sections_symbols_and_relocations`
- `backend_emit_cli_tests::emit_elf_materializes_elf64_relocatable_bytes`
- `backend_emit_cli_tests::render_macho_asm_aarch64_branch_relocs_in_native_bytes_are_rewritten`
- `backend_emit_cli_tests::render_coff_asm_materializes_sections_and_branches`
- `backend_emit_cli_tests::emit_macho_with_clang_fallback_writes_macho_object`
- `backend_emit_cli_tests::emit_coff_with_clang_fallback_writes_coff_object`

## Closure scope (active line)

This tranche is considered operationally closed for F11 in the active line:
- typed lowering/emission/object is end-to-end stable
- native ELF64 relocatable serialization is integrated
- COFF/Mach-O and linker integration are available through the typed external pipeline, without breaking typed contracts

Further deepening (broader native serializers, broader ISA-native closure, richer in-repo linking stages) is treated as post-F11 versioned evolution and does not reopen F11 closure.

F20 v1.0.0 operational hardening on top of this typed base (artifact/profile compatibility, non-empty material output guarantees, deterministic ELF evidence) is documented in:
- `docs/spec/backend/backend_material_defensible_v1_0.md`

CLI profile selection in active line:
- `ncc emit-backend <file.n> --cc <nC.sysv64|nC.win64|nC.aapcs64>`
- `ncc emit-object <file.n> --cc <nC.sysv64|nC.win64|nC.aapcs64>`
- `ncc emit-elf <file.n> <out.o> --cc <nC.sysv64|nC.win64|nC.aapcs64>`
- `ncc emit-coff <file.n> <out.obj> [--objcopy <tool>] [--clang <tool>] --cc <...>`
- `ncc emit-macho <file.n> <out.o> [--objcopy <tool>] [--clang <tool>] --cc <...>`
- `ncc build-exe <file.n> <out> [--linker <tool>] [--entry <symbol>] --cc <...>`
