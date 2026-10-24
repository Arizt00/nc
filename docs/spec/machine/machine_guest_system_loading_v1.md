# Machine guest-system loading v1

## Scope

The guest loader accepts three explicit boot contracts. It never guesses the image type and never
silently delegates to QEMU.

| Contract | Materialized state |
| --- | --- |
| Linux `bzImage` | setup header, zero page, E820, GDT, command line, initrd, ACPI, VirtIO-MMIO disks and protected-mode entry |
| UEFI | validated PE32/PE32+ image, architecture fallback path, GPT/FAT32 ESP, initrd media, UTF-16 load options and ResetSystem contract |
| OpenSBI | FW_DYNAMIC/FW_JUMP region, dynamic-info record, aligned Linux payload, FDT, initrd, VirtIO nodes and SBI system-reset contract |

`GuestBootPlan::validate` rejects overlapping regions, out-of-memory placements, invalid firmware,
wrong architectures, malformed command lines, duplicate firmware tables and invalid disks before any
guest byte is installed.

## Linux direct host backend

On x86 Linux, `boot-guest` opens `/dev/kvm` directly. It installs guest RAM, CPUID, irqchip, PIT,
boot registers, ACPI and userspace VirtIO-MMIO. UART, ACPI S5 and reboot ports are handled without a
QEMU process.

The VirtIO split-ring contract includes:

- direct and indirect descriptors;
- volatile DMA reads and writes;
- release ordering before `used.idx` and IRQ injection;
- `VIRTIO_F_RING_EVENT_IDX`, including `used_event`, `avail_event` and index wrap-around;
- legacy `VIRTQ_AVAIL_F_NO_INTERRUPT` suppression;
- ISR assertion, acknowledgement and KVM GSI deassertion;
- read-only and writable block backends, sector bounds, flush, discard and write-zeroes.

`--mmio-trace-out <path>` retains bounded register, queue and indirect-descriptor evidence. The trace
is also persisted on timeout. `--virtio-no-indirect` and `--virtio-no-event-idx` are diagnostic feature
negotiation controls; the default path enables both features.

## Evidence rules

`prepare-guest` certifies parsing and byte materialization only. `boot-guest --bundle ...` additionally
requires a direct backend, no QEMU, the expected serial marker, a clean shutdown/reboot and at least
one completed request for every configured disk. The verifier recomputes all source and bundle hashes.
Bundles retain kernel, firmware, initrd and disks under `sources/`. They remain verifiable after the
originals are removed or the bundle is relocated: artifact paths are certificate-relative when the
recorded root is unavailable, and legacy WSL/Windows paths are translated without weakening hashes.

The direct execution backend in v1 is Linux x86 `bzImage`. UEFI and OpenSBI plans are fully parsed and
materialized, but physical execution requires respectively an x86/AArch64 firmware host or a RISC-V
H/KVM host. A prepared certificate must never be reported as an executed certificate.

## Primary contracts

- Linux x86 boot protocol: <https://www.kernel.org/doc/html/latest/arch/x86/boot.html>
- KVM userspace API: <https://www.kernel.org/doc/html/latest/virt/kvm/api.html>
- UEFI 2.10: <https://uefi.org/specs/UEFI/2.10/>
- OpenSBI firmware: <https://github.com/riscv-software-src/opensbi/blob/master/docs/firmware/fw.md>
- VirtIO 1.3: <https://docs.oasis-open.org/virtio/virtio/v1.3/virtio-v1.3.html>
