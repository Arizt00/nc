# nC primitive[@] — Material Environment Law

Status: BASE ACTIVE
Version: v2.0.0-primitive-environment-law

## Purpose

Define that `primitive[@]` does not promise generic hardware behavior.
It binds to explicit target + material environment.

## Environment Law

Every primitive material operation REQUIRES:
- target,
- material environment,
- capability,
- witness,
- lowering path.

If any element is missing, operation MUST fail with official diagnostics.

## No Universal Hardware Contract

Primitive operations are environment-dependent:
- `x86_64` may expose `port I/O`,
- `aarch64` usually relies on MMIO and may not expose x86-style ports,
- `riscv64` depends on platform/SoC capabilities.

No silent emulation fallback is allowed.

## Capability Governance

Target capability registry MUST govern:
- port support,
- MMIO support,
- interrupt model,
- volatile/fence requirements,
- protocol availability,
- firmware/boot visibility.

Unsupported capabilities MUST fail with explicit errors.

## Protocol and Firmware Law

Protocol families (UART, PCIe, USB, SATA, NVMe, I2C, SPI, Ethernet, HDMI, DP)
are target-environment capabilities, not string decorations.

Firmware/boot entities (BIOS/UEFI/bootROM/microcode/handoff) are explicit
environment contracts and fail if unavailable or opaque.

Firmware/boot chain integrity is enforced:
- extended firmware directives require `firmwareRegion`;
- `resetVector` requires `bootWindow` + `bootAnchor`;
- `bootAnchor` must resolve to declared `anchor`;
- `bootRomRegion` requires `resetVector` + `bootWindow`;
- `biosRegion` requires `resetVector` + `bootWindow`;
- `uefiHandoff` requires `firmwareTable`;
- `microcodeState` requires `firmwareTable`;
- `firmwareTable` requires `deviceTree` or `acpiTable`.

## No False Depth Closure

The following contracts are mandatory and enforced by primitive validation:

- protocol surfaces that imply hardware control planes (`USB`, `SATA`, `NVME`,
  `PCIE`, `ETHERNET`, `I2C`, `SPI`, `HDMI`, `DISPLAYPORT`) require explicit
  `controller` declaration (`E_PRIMITIVE_TARGET_MATERIAL_001`);
- `sensor*` surfaces require explicit `peripheral` declaration
  (`E_PRIMITIVE_TARGET_MATERIAL_001`);
- `motor*` surfaces require explicit `controller` declaration
  (`E_PRIMITIVE_TARGET_MATERIAL_001`);
- `radio*` surfaces require target RF capability
  (`E_PRIMITIVE_TARGET_CAPABILITY_001`);
- `screen*`/`display*` surfaces require explicit display-link protocol contract
  (`HDMI` or `DISPLAYPORT`) (`E_PRIMITIVE_DISPLAY_001`).

## Emulation Classification

QEMU-based evidence is valid for emulation-path verification and regression
proof, but MUST be tagged as emulated environment evidence.

It MUST NOT be reported as physical-hardware certification.
