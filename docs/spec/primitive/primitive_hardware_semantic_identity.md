# nC primitive[@] — Hardware Semantic Identity

Status: BASE ACTIVE
Version: v2.0.0-primitive-hardware-identity

## Purpose

This document freezes the semantic identity of `primitive[@]` as hardware-physical
contracts inside nC.

`primitive[@]` is not:
- runtime,
- `[@machine]`,
- high-level console programming,
- human input/output flow.

`primitive[@]` is:
- material entity declaration,
- material witness requirement,
- target capability-checked lowering,
- explicit physical effects and failure semantics.

## Core Law

In `primitive[@]`, an operation without material witness does not exist.

Mandatory checks:
- target declared,
- environment declared,
- capability declared,
- lowering available,
- side effects declared when applicable,
- volatility/fence contracts for MMIO and device operations.

## Mode Boundaries

- `.nbb`: binary origin layer.
- `[@]`: semantic root layer.
- `primitive[@]`: physical-material semantic layer.
- `[@machine]`: machine instruction and register-readable layer.

## Human I/O Prohibition

`primitive[@]` forbids runtime/human input-output pathways:
- no `readLine`, `readInt`, `stdin`,
- no runtime output fallback,
- no semantic substitution of hardware operations with stdout/println.

Equivalent physical forms:
- `sample`, `latch`, `emit`, `load`, `volatileRead`, `volatileWrite`,
- `portRead`, `portWrite`, `mmioRead`, `mmioWrite`,
- bit/mask/status-based state extraction.

## Material Entities

`primitive[@]` maintains official construct families:
- `anchor`, `slot`, `pattern`, `mask`, `window`, `signal`, `bus`, `state`,
  `timing`, `emit`, `sample`, `latch`, `assert`, `rupture`, `bridge`, `seal`.

Extended material identity (target-gated):
- electrical: `voltage`, `current`, `edge`, `noise`, `metastability`,
  `power`, `temperature`.
- temporal: `clock`, `latency`, `settle`, `timeout`, `watchdog`.
- microarchitecture: `cache`, `tlb`, `pipeline`, `reorder`, `branchPredictor`.
- platform: `dmaChannel`, `interruptLine`, `firmwareRegion`, `bootRom`,
  `registerWindow`, `mmioWindow`, `portWindow`.

## Firmware/Boot Closure

Firmware and boot contracts are semantically coupled, not decorative labels.
The following relations are mandatory in `primitive[@]`:

- extended firmware directives (`bootRomRegion`, `biosRegion`, `uefiHandoff`,
  `resetVector`, `bootWindow`, `bootAnchor`, `firmwareTable`, `deviceTree`,
  `acpiTable`, `microcodeState`) require `firmwareRegion`;
- `resetVector` requires both `bootWindow` and `bootAnchor`;
- `bootAnchor` must resolve to a declared `anchor` symbol;
- `bootRomRegion` requires both `resetVector` and `bootWindow`;
- `biosRegion` requires both `resetVector` and `bootWindow`;
- `uefiHandoff` requires `firmwareTable`;
- `microcodeState` requires `firmwareTable`;
- `firmwareTable` requires at least one platform table (`deviceTree` or
  `acpiTable`);
- declaring `deviceTree`/`acpiTable` without `firmwareTable` is invalid.

Baseical failures:
- `E_PRIMITIVE_FIRMWARE_001`
- `E_PRIMITIVE_BOOTROM_001`

## Emulation Clause

QEMU is a supported emulated environment for material smoke tests, but it is
not physical hardware certification.

The compiler/tooling MUST preserve this distinction in diagnostics and reports.
