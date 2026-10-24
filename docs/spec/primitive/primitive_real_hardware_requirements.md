# primitive[@] real hardware requirements

This document defines the minimum requirements before primitive[@] can claim
real hardware evidence. It exists to prevent accidental upgrades from parser,
QEMU, or freestanding evidence to physical hardware claims.

## Required classification

Current local status:

```text
status: blocked_by_environment
claim: hardware_not_verified
reason: WSL2 host, no real authorized serial device, no /dev/port, no safe MMIO map, no allowlist device.
```

This status is release-safe and intentionally conservative.

## Host requirements

A real hardware run requires a host where the target device is visible as an
actual device endpoint and can be opened by the primitive command under policy.

WSL2 is accepted for:

- parsing;
- semantic checks;
- witness generation;
- map generation;
- lowering;
- freestanding artifacts;
- QEMU preflight.

WSL2 is not accepted for:

- privileged port I/O closure;
- arbitrary MMIO closure;
- physical hardware verification without an explicitly forwarded and
  allowlisted device plus retained readback/capture.

## Serial requirements

To close UART or serial physical evidence, the host must provide:

- a concrete serial device path, preferably `/dev/serial/by-id/...`;
- an exact allowlist entry in `primitive.hardware.allowlist.json`;
- a declared baud/configuration policy when required by the target;
- an expected output or readback pattern;
- retained run evidence;
- retained capture/readback evidence;
- a verification report that hashes the artifact and retained evidence.

`/dev/ttyS*` nodes alone do not prove a physical device. They are only eligible
after explicit allowlisting and successful observed readback/capture.

## Port I/O requirements

Port I/O requires:

- a non-WSL privileged host;
- `/dev/port` or equivalent capability;
- a safe port whitelist;
- an explicit policy allowing the selected operation;
- retained evidence proving the operation was blocked or observed as expected.

Without those items, the official diagnostic is
`E_PRIMITIVE_PORT_IO_PERMISSION_001`.

## MMIO requirements

MMIO requires:

- a safe map document identifying the address window;
- access width;
- allowed read/write mode;
- target ownership and safety notes;
- explicit write policy for any write;
- retained evidence for observed effects.

Without a safe map, the official diagnostics are
`E_PRIMITIVE_MMIO_PERMISSION_001` and `E_PRIMITIVE_MMIO_UNSAFE_ADDRESS_001`.

## QEMU is not physical evidence

QEMU preflight is useful and required for emulated confidence. It is classified
as `qemu_emulated` and cannot be promoted to real hardware evidence.

Valid QEMU claim:

```text
claim: qemu_emulated
physical_hardware_certified: false
```

Invalid release claim:

```text
claim: hardware_verified
source: qemu
```

## Baseical blocked diagnostic

When a physical device is not present, authorized, or verified, reports must use:

```text
E_PRIMITIVE_REAL_HARDWARE_MISSING_001
```

Message:

```text
no allowlisted real hardware device has been verified on this host
```

## Closure checklist

- Real device path is visible.
- Device is explicitly allowlisted.
- Run command opens only the allowlisted device.
- Capture/readback is retained.
- Evidence hash is recorded.
- QEMU remains classified as emulated.
- Port I/O uses a safe whitelist.
- MMIO uses a safe map.
- No report marks real hardware as verified without retained physical evidence.
