# nC — primitive[@] Material Execution Closure
Status: Active Base (workspace scope)
Version: 2.0.0

## 1. Objective
`primitive[@]` is the physical boundary mode of nC. It defines material contracts and low-level execution surfaces, not high-level runtime behavior.

## 2. Runtime policy
`primitive[@]` does not consume `nC.runtime.*`. Runtime imports in primitive context are rejected by sema.

## 3. Primitive constructs (active)
The parser/AST/sema/IR surface includes:
- `anchor`
- `slot`
- `pattern`
- `mask`
- `window`
- `signal`
- `bus`
- `state`
- `timing`
- `emit`
- `sample`
- `latch`
- `assert`
- `rupture`
- `bridge`
- `seal`

## 4. Material contracts
Primitive lowering carries typed metadata for:
- capability contracts
- bitfield contracts
- operation contracts
- window/signal/port/signature/entry contracts
- bridge contracts

## 5. Target and linker contracts
Baseical freestanding target file:
- `targets/x86_64-unknown-none.nctarget`

Baseical linker script:
- `link/x86_64_baremetal.ld`

For primitive material builds (`build-sovereign` with primitive containers), CLI requires:
- `--target x86_64-unknown-none`
- `--linker link/x86_64_baremetal.ld`
- `--entry <symbol>`

## 6. MMIO/port/volatile/fence policy
Primitive surface and typed contracts include MMIO/port classes and fence semantics as low-level clauses. Target capabilities are validated from target spec for material builds.

Lowering route (material):
- `portWrite` / official `write <port> <- <byte>`
- `portRead`
- `mmioWrite` / `mmioRead`
- `volatileWrite` / `volatileRead`
- `fence` / `barrier`
- `halt` / `haltLoop`
- `cpuPause`

Enforcement:
- MMIO operations REQUIRE explicit `volatile`.
- Port/MMIO operations REQUIRE explicit `fence` or `barrier`.
- Unsupported target capability yields official `E_PRIMITIVE_PORT_UNSUPPORTED_001` or `E_PRIMITIVE_MMIO_UNSUPPORTED_001`.

## 7. print policy in primitive
`print.sensory` is restricted to `primitive[@]`; non-primitive usage is rejected in sema.

Material print integration:
- `print.mmio` and `print.port` are lowered only when target capabilities exist.
- `print.sensory` REQUIRES:
  - sensory bridge target,
  - explicit capability authority,
  - expert profile (`--profile primitive-expert`),
  - sensory lowering backend support; otherwise it fails with official diagnostics.

## 8. Baseical material example
- `examples/primitive/boot_serial_hello.ncp`

This fixture defines serial capability/ports/signals and material entry contract `kernelEntry`.

## 9. Build command (official)
```bash
ncc build-sovereign examples/primitive/boot_serial_hello.ncp out/boot_serial_hello.sov \
  --profile primitive \
  --target x86_64-unknown-none \
  --entry kernelEntry \
  --linker link/x86_64_baremetal.ld

ncc build-sovereign examples/primitive/boot_serial_hello.ncp out/boot_serial_hello.elf \
  --profile primitive \
  --target x86_64-unknown-none \
  --entry kernelEntry \
  --linker link/x86_64_baremetal.ld \
  --image-format elf
```

## 10. QEMU smoke command
```bash
tools/scripts/run_primitive_qemu_smoke.sh
```

Behavior:
- fails if QEMU is missing (no false success),
- fails if expected serial text is not observed,
- passes only on real positive serial match.
- always validates material artifacts before QEMU:
  - raw image size must be 512 bytes,
  - raw image signature at offset `0x1FE` must be `0x55AA`,
  - machine opcodes required for serial path (`mov dx`, `out dx,al`, `in al,dx`, `test al`, `hlt`) must exist,
  - ELF magic must be `0x7F454C46`.
- writes material evidence to:
  - `var/primitive/qemu-smoke/material_evidence.txt`

## 10.1 Material execution status
For `primitive[@]` + `--target x86_64-unknown-none`, `build-sovereign` now emits a real boot-sector image:
- 512 bytes exact size,
- boot signature `0x55AA` at offset `0x1FE`,
- executable x86 real-mode opcodes for COM1 serial output.

Freestanding ELF emission is also active for the same primitive contract:
- pass `--image-format elf` (or use `.elf` output extension),
- emits ELF64 executable header (`ET_EXEC`, `EM_X86_64`),
- payload is backed by the same primitive-lowered machine bytes.

Primitive entry lowering consumes official `operation` write clauses (`write <port> <- <byte>`) and emits serial port I/O machine instructions.
The QEMU smoke gate validates observable serial output (`nC primitive alive`) from the booted image.

## 11. Diagnostics (material build path)
Primitive diagnostics surface is officialized with explicit `E_PRIMITIVE_*` codes:
- `E_PRIMITIVE_RUNTIME_001`
- `E_PRIMITIVE_RUNTIME_PATH_001`
- `E_PRIMITIVE_INPUT_001`
- `E_PRIMITIVE_OUTPUT_001`
- `E_PRIMITIVE_ANCHOR_MISSING_001`
- `E_PRIMITIVE_ANCHOR_INVALID_001`
- `E_PRIMITIVE_WINDOW_INVALID_001`
- `E_PRIMITIVE_MASK_WIDTH_001`
- `E_PRIMITIVE_SIGNAL_UNDECLARED_001`
- `E_PRIMITIVE_BUS_CONFLICT_001`
- `E_PRIMITIVE_TIMING_UNSUPPORTED_001`
- `E_PRIMITIVE_SEAL_MISSING_001`
- `E_PRIMITIVE_BRIDGE_MACHINE_MISSING_001`
- `E_PRIMITIVE_SENSORY_BRIDGE_MISSING_001`
- `E_PRIMITIVE_SENSORY_AUTHORITY_001`
- `E_PRIMITIVE_SENSORY_PROFILE_EXPERT_001`
- `E_PRIMITIVE_SENSORY_LOWERING_UNAVAILABLE_001`
- `E_PRIMITIVE_TARGET_MISSING_001`
- `E_PRIMITIVE_TARGET_UNSUPPORTED_001`
- `E_PRIMITIVE_PORT_UNSUPPORTED_001`
- `E_PRIMITIVE_MMIO_UNSUPPORTED_001`
- `E_PRIMITIVE_VOLATILE_REQUIRED_001`
- `E_PRIMITIVE_FENCE_REQUIRED_001`
- `E_PRIMITIVE_LINKER_MISSING_001`
- `E_PRIMITIVE_ENTRY_MISSING_001`
- `E_PRIMITIVE_QEMU_SMOKE_001`
- `E_PRIMITIVE_LOWERING_UNAVAILABLE_001`
- `E_PRIMITIVE_ELF_EMIT_001`

## 12. Validation checklist
Run:
```bash
cargo fmt --all
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace primitive -- --nocapture
tools/scripts/run_primitive_qemu_smoke.sh
```

If QEMU is unavailable, smoke must fail with explicit dependency diagnostic (not pass).
