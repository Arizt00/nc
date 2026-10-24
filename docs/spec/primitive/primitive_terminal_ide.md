# nC primitive[@] - Official Terminal IDE Surface

Status: BASE ACTIVE
Version: v2.0.0-primitive-terminal-ide

## Purpose

This document defines the official terminal IDE flow for `primitive[@]`.
The terminal IDE belongs to `[@tooling]`, while `primitive[@]` remains the
hardware-physical semantic layer.

The terminal IDE does not humanize primitive semantics and does not relax
material contracts.

## Official Command Surface

The official terminal subcommand family is:

- `ncc primitive doctor`
- `ncc primitive check`
- `ncc primitive witness`
- `ncc primitive map`
- `ncc primitive lower`
- `ncc primitive build`
- `ncc primitive qemu`
- `ncc primitive report`
- `ncc primitive target`
- `ncc primitive clean`

Common options when applicable:

- `--json`
- `--md`
- `--no-color`
- `--strict`
- `--target <triple>`
- `--environment <name>`

## Contract Law

Terminal commands MUST preserve the primitive hard constraints:

- no runtime fallback in `primitive[@]`,
- no stdout simulation as hardware output,
- no silent capability downgrade,
- no implicit universal hardware assumption.

If a target capability is absent, commands MUST fail with explicit
`E_PRIMITIVE_*` diagnostics.

## Material Witness and Lowering Visibility

`ncc primitive witness` and `ncc primitive lower` must expose:

- target and environment,
- required capabilities and protocols,
- declared material entities,
- volatility/fence/side-effect constraints,
- lowering path (`primitive -> IR -> machine -> artifact`),
- environment classification (`emulated` vs physical).

`ncc primitive witness` and `ncc primitive map` include a
`firmware_boot_model` block with:
- declared firmware/boot entities,
- firmware/boot contract checks (resolved anchor, boot-chain, BIOS/UEFI, and
  firmware-table relations),
- `verified` status for firmware/boot semantic integrity.

## QEMU Classification

QEMU is allowed for emulation smoke evidence.
It MUST be tagged as emulated environment evidence and MUST NOT be declared as
physical hardware certification.

In strict mode, certification-sensitive commands MUST reject QEMU-class
environments with `E_PRIMITIVE_QEMU_NOT_REAL_HW_001`:

- `ncc primitive check --strict`
- `ncc primitive witness --strict`
- `ncc primitive map --strict`
- `ncc primitive lower --strict`
- `ncc primitive build --strict`
- `ncc primitive report --strict`
- `ncc primitive target <target> --strict` when target/environment is emulated-qemu
- `ncc primitive doctor --strict --target <target> --environment <env>` when env is qemu/emulated

`ncc primitive qemu` remains an emulation smoke flow and must keep explicit
emulation labeling in its output.
